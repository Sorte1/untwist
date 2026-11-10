use oxc_ast::ast::*;
use oxc_ast_visit::{walk_js, VisitJs};
use oxc_syntax::scope::ScopeFlags;
use rustc_hash::FxHashSet;

use crate::matcher::{call_parts, const_member};
use crate::webpack::esm_proof::marks_exports;

const MODULE_SCOPE: [&str; 6] = [
    "module",
    "exports",
    "require",
    "__filename",
    "__dirname",
    "arguments",
];
const REBOUND: [&str; 4] = ["module", "exports", "require", "arguments"];
const ARGUMENTS: &str = "arguments";
const EVAL: &str = "eval";

#[derive(Default)]
struct Body<'a> {
    functions: u32,
    arrows: u32,
    escapes: bool,
    reads_this: bool,
    declared: FxHashSet<&'a str>,
    hoisted_functions: FxHashSet<&'a str>,
}

impl Body<'_> {
    fn own_level(&self) -> bool {
        self.functions + self.arrows == 0
    }
}

impl<'a> VisitJs<'a> for Body<'a> {
    fn visit_function(&mut self, it: &Function<'a>, flags: ScopeFlags) {
        if let (true, true, Some(id)) = (self.own_level(), it.is_declaration(), &it.id) {
            self.declared.insert(id.name.as_str());
            self.hoisted_functions.insert(id.name.as_str());
        }
        self.functions += 1;
        walk_js::walk_function(self, it, flags);
        self.functions -= 1;
    }

    fn visit_arrow_function_expression(&mut self, it: &ArrowFunctionExpression<'a>) {
        self.arrows += 1;
        walk_js::walk_arrow_function_expression(self, it);
        self.arrows -= 1;
    }

    fn visit_binding_identifier(&mut self, it: &BindingIdentifier<'a>) {
        if self.own_level() {
            self.declared.insert(it.name.as_str());
        }
    }

    fn visit_identifier_reference(&mut self, it: &IdentifierReference<'a>) {
        self.escapes |= it.name == EVAL || (it.name == ARGUMENTS && self.functions == 0);
    }

    fn visit_this_expression(&mut self, _it: &ThisExpression) {
        self.reads_this |= self.functions == 0;
    }

    fn visit_return_statement(&mut self, it: &ReturnStatement<'a>) {
        self.escapes |= self.own_level();
        walk_js::walk_return_statement(self, it);
    }

    fn visit_new_target(&mut self, _it: &NewTarget) {
        self.escapes |= self.functions == 0;
    }
}

#[derive(Default)]
struct Names<'a> {
    read: FxHashSet<&'a str>,
}

impl<'a> VisitJs<'a> for Names<'a> {
    fn visit_identifier_reference(&mut self, it: &IdentifierReference<'a>) {
        self.read.insert(it.name.as_str());
    }
}

fn names_read<'a>(argument: &Argument<'a>) -> FxHashSet<&'a str> {
    let mut names = Names::default();
    names.visit_argument(argument);
    names.read
}

fn parameter_names<'a>(function: &Function<'a>) -> Option<Vec<&'a str>> {
    let names: Vec<&str> = function
        .params
        .items
        .iter()
        .filter(|item| item.initializer.is_none())
        .filter_map(|item| match &item.pattern {
            BindingPattern::BindingIdentifier(id) => Some(id.name.as_str()),
            _ => None,
        })
        .collect();
    let distinct: FxHashSet<&str> = names.iter().copied().collect();
    let plain = function.params.rest.is_none()
        && names.len() == function.params.items.len()
        && distinct.len() == names.len()
        && !REBOUND.iter().any(|name| distinct.contains(name));
    plain.then_some(names)
}

fn arguments_stay_put(params: &[&str], arguments: &[Argument<'_>], body: &Body<'_>) -> bool {
    arguments.iter().all(|argument| {
        let read = names_read(argument);
        let reads_parameter = params.iter().any(|name| read.contains(name));
        let reads_declared = read.iter().any(|name| body.declared.contains(name));
        !reads_parameter && !reads_declared && !read.contains(EVAL)
    })
}

fn body_moves(function: &Function<'_>, arguments: &[Argument<'_>], keeps_this: bool) -> bool {
    let (Some(params), Some(statements)) = (parameter_names(function), function.body.as_deref())
    else {
        return false;
    };
    let plain = function.id.is_none() && !function.r#async && !function.generator;
    if !plain || params.len() != arguments.len() || !statements.directives.is_empty() {
        return false;
    }
    let mut body = Body::default();
    body.visit_function_body(statements);
    let shadows = MODULE_SCOPE.iter().any(|name| body.declared.contains(name));
    let overrides = params
        .iter()
        .any(|name| body.hoisted_functions.contains(name));
    let this_moves = keeps_this || !body.reads_this;
    !body.escapes
        && !shadows
        && !overrides
        && this_moves
        && arguments_stay_put(&params, arguments, &body)
}

pub fn proven_wrapper(statements: &[Statement<'_>]) -> bool {
    let Some((Statement::ExpressionStatement(wrapper), before)) = statements.split_last() else {
        return false;
    };
    let Some((callee, arguments)) = call_parts(&wrapper.expression) else {
        return false;
    };
    let Some(Expression::FunctionExpression(function)) = const_member(callee, "call") else {
        return false;
    };
    let Some((receiver, injected)) = arguments.split_first() else {
        return false;
    };
    let keeps_this = matches!(receiver, Argument::ThisExpression(_));
    let spread = injected
        .iter()
        .any(|argument| matches!(argument, Argument::SpreadElement(_)));
    !spread && before.iter().all(marks_exports) && body_moves(function, injected, keeps_this)
}
