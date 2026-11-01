use oxc_ast::ast::{
    AssignmentTarget, BindingIdentifier, BindingPattern, Expression, Function, Statement,
};
use oxc_ast_visit::{walk_js, VisitJs};
use oxc_span::{GetSpan, Span};
use oxc_syntax::scope::ScopeFlags;
use rustc_hash::FxHashSet;

use crate::stack;

pub(super) const INTRINSICS: [&str; 19] = [
    "parseInt",
    "parseFloat",
    "String",
    "Number",
    "Boolean",
    "Math",
    "Array",
    "RegExp",
    "decodeURIComponent",
    "encodeURIComponent",
    "decodeURI",
    "encodeURI",
    "escape",
    "unescape",
    "isNaN",
    "isFinite",
    "undefined",
    "NaN",
    "Infinity",
];

pub(super) const FORBIDDEN_KEYS: [&str; 5] =
    ["constructor", "__proto__", "prototype", "caller", "callee"];

pub(super) const FORBIDDEN_NAMES: [&str; 3] = ["arguments", "eval", "Function"];

pub(super) fn skipped(skip: &[Span], span: Span) -> bool {
    skip.contains(&span)
}

struct Bindings<'s, 'a> {
    skip: &'s [Span],
    names: FxHashSet<&'a str>,
}

impl<'a> VisitJs<'a> for Bindings<'_, 'a> {
    fn visit_assignment_target(&mut self, it: &AssignmentTarget<'a>) {
        stack::maybe_grow_dyn(&mut || walk_js::walk_assignment_target(self, it));
    }

    fn visit_binding_pattern(&mut self, it: &BindingPattern<'a>) {
        stack::maybe_grow_dyn(&mut || walk_js::walk_binding_pattern(self, it));
    }

    fn visit_statement(&mut self, it: &Statement<'a>) {
        if !skipped(self.skip, it.span()) {
            stack::maybe_grow_dyn(&mut || walk_js::walk_statement(self, it));
        }
    }

    fn visit_expression(&mut self, it: &Expression<'a>) {
        if !skipped(self.skip, it.span()) {
            stack::maybe_grow_dyn(&mut || walk_js::walk_expression(self, it));
        }
    }

    fn visit_binding_identifier(&mut self, it: &BindingIdentifier<'a>) {
        self.names.insert(it.name.as_str());
    }
}

pub(super) fn declared_in<'a>(function: &Function<'a>, skip: &[Span]) -> FxHashSet<&'a str> {
    let mut bindings = Bindings {
        skip,
        names: FxHashSet::default(),
    };
    bindings.visit_function(function, ScopeFlags::Function);
    bindings.names
}

pub(super) fn param_names<'a>(function: &Function<'a>) -> FxHashSet<&'a str> {
    function
        .params
        .items
        .iter()
        .filter_map(|param| match &param.pattern {
            oxc_ast::ast::BindingPattern::BindingIdentifier(id) => Some(id.name.as_str()),
            _ => None,
        })
        .collect()
}

pub(super) fn outer_names<'a>(function: &Function<'a>) -> FxHashSet<&'a str> {
    let mut names = param_names(function);
    let Some(body) = &function.body else {
        return names;
    };
    for statement in &body.statements {
        if let Statement::VariableDeclaration(declaration) = statement {
            for declarator in &declaration.declarations {
                if let oxc_ast::ast::BindingPattern::BindingIdentifier(id) = &declarator.id {
                    names.insert(id.name.as_str());
                }
            }
        }
    }
    names
}

pub(super) fn array_locals<'a>(function: &Function<'a>, array: &str) -> FxHashSet<&'a str> {
    let Some(body) = &function.body else {
        return FxHashSet::default();
    };
    let mut locals = FxHashSet::default();
    for statement in &body.statements {
        let Statement::VariableDeclaration(declaration) = statement else {
            continue;
        };
        for declarator in &declaration.declarations {
            let (oxc_ast::ast::BindingPattern::BindingIdentifier(id), Some(init)) =
                (&declarator.id, &declarator.init)
            else {
                continue;
            };
            let calls_array = matches!(init, Expression::CallExpression(call)
                if call.arguments.is_empty()
                    && matches!(&call.callee, Expression::Identifier(callee) if callee.name == array));
            if calls_array {
                locals.insert(id.name.as_str());
            }
        }
    }
    locals
}
