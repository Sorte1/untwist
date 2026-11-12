use oxc_allocator::{Allocator, CloneIn};
use oxc_ast::ast::*;
use oxc_ast_visit::{walk_js, VisitJs};
use oxc_semantic::Scoping;
use oxc_span::{GetSpan, Span};
use oxc_syntax::scope::ScopeFlags;
use rustc_hash::FxHashSet;
use untwist_core::semantic::build_scoping;

use crate::frame::Frame;
use crate::matcher::{argument_expression, literal_id, param_names};
use crate::model::{module_from_body_with, Bodies, Bundle, Module, RequireParam, Runtime};
use crate::rename::{rename_factory_parameters, rename_wrapper_bindings};
use crate::webpack::container::module_factories;
use crate::webpack::extract::WEBPACK_PARAMS;
use crate::webpack::helpers::{declared_in, pattern_names};
use crate::webpack::runtime_module::Needs;

const NO_MODULE: &str = "\0module";
const NO_EXPORTS: &str = "\0exports";
const USE_STRICT: &str = "use strict";
const BIND: &str = "bind";
const FAKE_NAMESPACE: &str = "t";
const ENTRY_ID: &str = "entry";
const HOST_NAMES: [&str; 6] = [
    "module",
    "exports",
    "require",
    "__filename",
    "__dirname",
    "eval",
];

pub struct Site<'k, 'a> {
    pub span: Span,
    body: &'k FunctionBody<'a>,
    container: &'k Expression<'a>,
    declared: FxHashSet<&'k str>,
    exports: Vec<(&'k str, Span)>,
}

fn last_expression<'k, 'a>(body: &'k [Statement<'a>]) -> Option<&'k Expression<'a>> {
    let Statement::ExpressionStatement(statement) = body.last()? else {
        return None;
    };
    match &statement.expression {
        Expression::SequenceExpression(sequence) => sequence.expressions.last(),
        expression => Some(expression),
    }
}

fn entry_body<'k, 'a>(expression: &'k Expression<'a>) -> Option<&'k FunctionBody<'a>> {
    let Expression::CallExpression(call) = expression.without_parentheses() else {
        return None;
    };
    if call.optional || !call.arguments.is_empty() {
        return None;
    }
    let Expression::ArrowFunctionExpression(arrow) = call.callee.without_parentheses() else {
        return None;
    };
    let ArrowFunctionBody::FunctionBody(body) = &arrow.body else {
        return None;
    };
    let plain = !arrow.r#async && arrow.params.items.is_empty() && arrow.params.rest.is_none();
    plain.then_some(&**body)
}

fn strict(directives: &[Directive<'_>]) -> bool {
    directives
        .iter()
        .any(|directive| directive.directive.as_str() == USE_STRICT)
}

struct Enclosing<'k, 'a> {
    top: &'k [Statement<'a>],
    params: Option<&'k FormalParameters<'a>>,
    strict: bool,
}

fn enclosing<'k, 'a>(
    source: &'k Program<'a>,
    body: &'k [Statement<'a>],
) -> Option<Enclosing<'k, 'a>> {
    let outer = strict(&source.directives);
    if std::ptr::eq(body, source.body.as_slice()) {
        return Some(Enclosing {
            top: &[],
            params: None,
            strict: outer,
        });
    }
    let frame = Frame::of(source);
    let wrapper = frame.wrapper?;
    if !std::ptr::eq(wrapper.body(), body) {
        return None;
    }
    Some(Enclosing {
        top: frame.top,
        params: Some(wrapper.function.params),
        strict: outer || strict(&wrapper.function.body.directives),
    })
}

fn exports_candidates<'k>(body: &'k [Statement<'_>]) -> Vec<(&'k str, Span)> {
    body.iter()
        .filter_map(|statement| {
            let Statement::VariableDeclaration(declaration) = statement else {
                return None;
            };
            let [declarator] = declaration.declarations.as_slice() else {
                return None;
            };
            let BindingPattern::BindingIdentifier(id) = &declarator.id else {
                return None;
            };
            let empty = matches!(&declarator.init, Some(Expression::ObjectExpression(object)) if object.properties.is_empty());
            empty.then(|| (id.name.as_str(), declaration.span))
        })
        .collect()
}

pub fn site<'k, 'a>(
    source: &'k Program<'a>,
    (body, container): (&'k [Statement<'a>], &'k Expression<'a>),
) -> Option<Site<'k, 'a>> {
    let expression = last_expression(body)?;
    let entry = entry_body(expression)?;
    let around = enclosing(source, body)?;
    if around.strict && !strict(&entry.directives) {
        return None;
    }
    if around.top.iter().any(Statement::is_module_declaration)
        || evaluates(body)
        || evaluates(around.top)
    {
        return None;
    }
    let mut declared = FxHashSet::default();
    declared_in(body, &mut declared);
    declared_in(around.top, &mut declared);
    for item in around.params.iter().flat_map(|params| &params.items) {
        pattern_names(&item.pattern, &mut declared);
    }
    Some(Site {
        span: expression.span(),
        body: entry,
        container,
        declared,
        exports: exports_candidates(body),
    })
}

#[derive(Default)]
struct DirectEval {
    found: bool,
}

impl<'a> VisitJs<'a> for DirectEval {
    fn visit_call_expression(&mut self, it: &CallExpression<'a>) {
        self.found |= matches!(it.callee.without_parentheses(), Expression::Identifier(id) if id.name == "eval");
        walk_js::walk_call_expression(self, it);
    }

    fn visit_function(&mut self, _it: &Function<'a>, _flags: ScopeFlags) {}

    fn visit_arrow_function_expression(&mut self, _it: &ArrowFunctionExpression<'a>) {}

    fn visit_class(&mut self, _it: &Class<'a>) {}
}

fn evaluates(statements: &[Statement<'_>]) -> bool {
    let mut eval = DirectEval::default();
    for statement in statements {
        eval.visit_statement(statement);
    }
    eval.found
}

fn is_name_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_' || c == '$' || !c.is_ascii()
}

fn skipped_spans(site: &Site<'_, '_>, name: &str, declared: Span) -> Vec<Span> {
    let mut spans: Vec<Span> = module_factories(site.container)
        .into_iter()
        .filter(|(_, factory)| param_names(factory.params).contains(&name))
        .map(|(_, factory)| Span::new(factory.params.span.start, factory.body.span.end))
        .collect();
    spans.push(site.span);
    spans.push(declared);
    spans.sort_unstable_by_key(|span| span.start);
    spans
}

fn inside(spans: &[Span], at: usize) -> bool {
    let after = spans.partition_point(|span| span.start as usize <= at);
    after > 0 && at < spans[after - 1].end as usize
}

fn mentioned_outside(text: &str, name: &str, skipped: &[Span]) -> bool {
    text.match_indices(name).any(|(at, _)| {
        let inside = inside(skipped, at);
        let before = text[..at].chars().next_back().is_some_and(is_name_char);
        let after = text[at + name.len()..]
            .chars()
            .next()
            .is_some_and(is_name_char);
        !inside && !before && !after
    })
}

fn exports_name<'s>(
    scoping: &Scoping,
    site: &Site<'s, '_>,
    (text, require): (&str, &str),
) -> Option<Option<&'s str>> {
    let mut found = None;
    for name in scoping.root_unresolved_references().keys() {
        let name = name.as_str();
        if name == require {
            continue;
        }
        let exports = site
            .exports
            .iter()
            .find(|(candidate, _)| *candidate == name);
        if let Some((candidate, declared)) = exports {
            let skipped = skipped_spans(site, name, *declared);
            if found.is_none() && !mentioned_outside(text, name, &skipped) {
                found = Some(*candidate);
                continue;
            }
        }
        if HOST_NAMES.contains(&name) || site.declared.contains(name) {
            return None;
        }
    }
    Some(found)
}

struct Uses<'s> {
    scoping: &'s Scoping,
    require: &'s str,
    needs: Option<Needs>,
    depth: u32,
    allowed: FxHashSet<usize>,
    refused: bool,
}

fn address<T>(it: &T) -> usize {
    std::ptr::from_ref(it) as usize
}

impl Uses<'_> {
    fn unresolved(&self, id: &IdentifierReference<'_>) -> bool {
        crate::scan::is_free(Some(self.scoping), id)
    }

    fn is_free_require(&self, id: &IdentifierReference<'_>) -> bool {
        id.name == self.require && self.unresolved(id)
    }

    fn free_require<'k>(
        &self,
        expression: &'k Expression<'_>,
    ) -> Option<&'k IdentifierReference<'k>> {
        let Expression::Identifier(id) = expression else {
            return None;
        };
        self.is_free_require(id).then_some(&**id)
    }

    fn routes(&self, name: &str) -> bool {
        self.needs.as_ref().is_some_and(|needs| needs.routes(name))
    }

    fn allow(&mut self, id: &IdentifierReference<'_>) {
        self.allowed.insert(address(id));
    }

    fn bound(&self, call: &CallExpression<'_>) -> Option<usize> {
        let Expression::StaticMemberExpression(member) = &call.callee else {
            return None;
        };
        if member.optional || member.property.name != BIND {
            return None;
        }
        let object = match &member.object {
            Expression::StaticMemberExpression(inner)
                if !inner.optional && inner.property.name == FAKE_NAMESPACE =>
            {
                &inner.object
            }
            object => object,
        };
        self.free_require(object)?;
        let args: Vec<&Expression<'_>> = call
            .arguments
            .iter()
            .map(argument_expression)
            .collect::<Option<_>>()?;
        let shape = match (args.as_slice(), std::ptr::eq(object, &member.object)) {
            ([this, id], true) => literal_id(id).map(|_| this),
            ([this, id, Expression::NumericLiteral(_)], false) => literal_id(id).map(|_| this),
            _ => None,
        }?;
        self.free_require(shape).map(address)
    }

    fn written(&self, target: &SimpleAssignmentTarget<'_>) -> bool {
        let object = match target {
            SimpleAssignmentTarget::StaticMemberExpression(member) => &member.object,
            SimpleAssignmentTarget::ComputedMemberExpression(member) => &member.object,
            SimpleAssignmentTarget::PrivateFieldExpression(member) => &member.object,
            _ => return false,
        };
        self.free_require(object).is_some()
    }
}

impl<'a> VisitJs<'a> for Uses<'_> {
    fn visit_call_expression(&mut self, it: &CallExpression<'a>) {
        if let Some(callee) = self.free_require(&it.callee) {
            let [argument] = it.arguments.as_slice() else {
                self.refused = true;
                return;
            };
            if argument_expression(argument).and_then(literal_id).is_some() {
                self.allow(callee);
            }
        }
        if let Some(this) = self.bound(it) {
            self.allowed.insert(this);
        }
        walk_js::walk_call_expression(self, it);
    }

    fn visit_static_member_expression(&mut self, it: &StaticMemberExpression<'a>) {
        if let Some(object) = self.free_require(&it.object) {
            let name = it.property.name.as_str();
            if !it.optional && (self.routes(name) || name == BIND) {
                self.allow(object);
            }
        }
        walk_js::walk_static_member_expression(self, it);
    }

    fn visit_simple_assignment_target(&mut self, it: &SimpleAssignmentTarget<'a>) {
        self.refused |= self.written(it);
        walk_js::walk_simple_assignment_target(self, it);
    }

    fn visit_unary_expression(&mut self, it: &UnaryExpression<'a>) {
        if it.operator == UnaryOperator::Delete {
            if let Some(member) = it.argument.without_parentheses().as_member_expression() {
                self.refused |= self.free_require(member.object()).is_some();
            }
        }
        walk_js::walk_unary_expression(self, it);
    }

    fn visit_identifier_reference(&mut self, it: &IdentifierReference<'a>) {
        if self.is_free_require(it) && !self.allowed.contains(&address(it)) {
            self.refused = true;
        }
        let top_arguments = it.name == "arguments" && self.depth == 0;
        self.refused |= top_arguments && self.unresolved(it);
    }

    fn visit_this_expression(&mut self, _it: &ThisExpression) {
        self.refused |= self.depth == 0;
    }

    fn visit_with_statement(&mut self, _it: &WithStatement<'a>) {
        self.refused = true;
    }

    fn visit_function(&mut self, it: &Function<'a>, flags: ScopeFlags) {
        self.depth += 1;
        walk_js::walk_function(self, it, flags);
        self.depth -= 1;
    }

    fn visit_property_definition(&mut self, it: &PropertyDefinition<'a>) {
        self.visit_decorators(&it.decorators);
        self.visit_property_key(&it.key);
        self.depth += 1;
        if let Some(value) = &it.value {
            self.visit_expression(value);
        }
        self.depth -= 1;
    }

    fn visit_accessor_property(&mut self, it: &AccessorProperty<'a>) {
        self.visit_decorators(&it.decorators);
        self.visit_property_key(&it.key);
        self.depth += 1;
        if let Some(value) = &it.value {
            self.visit_expression(value);
        }
        self.depth -= 1;
    }

    fn visit_static_block(&mut self, it: &StaticBlock<'a>) {
        self.depth += 1;
        walk_js::walk_static_block(self, it);
        self.depth -= 1;
    }
}

fn proven<'s>(
    program: &Program<'_>,
    site: &Site<'s, '_>,
    (text, require): (&str, &str),
    runtime: Option<&Runtime<'_>>,
) -> Option<Option<&'s str>> {
    let scoping = build_scoping(program).scoping;
    let exports = exports_name(&scoping, site, (text, require))?;
    let mut uses = Uses {
        scoping: &scoping,
        require,
        needs: runtime.map(Needs::of),
        depth: 0,
        allowed: FxHashSet::default(),
        refused: false,
    };
    uses.visit_program(program);
    (!uses.refused).then_some(exports)
}

#[derive(Default)]
struct Strings {
    seen: FxHashSet<String>,
}

impl<'a> VisitJs<'a> for Strings {
    fn visit_string_literal(&mut self, it: &StringLiteral<'a>) {
        if let Some(value) = it
            .value
            .as_str()
            .filter(|value| value.starts_with(ENTRY_ID))
        {
            self.seen.insert(value.to_string());
        }
    }
}

fn entry_id(bundle: &Bundle<'_>, program: &Program<'_>) -> String {
    let mut strings = Strings::default();
    strings.visit_program(program);
    for module in bundle.modules.values() {
        strings.visit_program(module.program());
    }
    (1..)
        .map(|index| match index {
            1 => ENTRY_ID.to_string(),
            index => format!("{ENTRY_ID}{index}"),
        })
        .find(|id| !bundle.modules.contains_key(id) && !strings.seen.contains(id))
        .unwrap_or_default()
}

pub fn attach<'a>(
    alloc: &'a Allocator,
    source: &Program<'a>,
    (site, require): (&Site<'_, 'a>, &str),
    bundle: &mut Bundle<'a>,
    bodies: &mut Bodies<'a>,
) {
    let planned = matches!(bodies, Bodies::Planned(_));
    let mut program = module_from_body_with(alloc, source, site.body, bodies);
    program.directives = site.body.directives.clone_in(alloc);
    let exports = match planned {
        true => None,
        false => {
            let runtime = bundle.runtime.as_ref();
            let Some(exports) = proven(&program, site, (source.source_text, require), runtime)
            else {
                return;
            };
            exports
        }
    };
    let params = [NO_MODULE, exports.unwrap_or(NO_EXPORTS), require];
    rename_factory_parameters(alloc, &mut program, (&params, &WEBPACK_PARAMS), false);
    rename_wrapper_bindings(alloc, &mut program, &params);
    let id = entry_id(bundle, &program);
    let mut module = Module::new(id.clone(), program, true);
    module.runtime_require = RequireParam::Default;
    bundle.modules.shift_insert(0, id.clone(), module);
    bundle.entry_id = Some(id);
}
