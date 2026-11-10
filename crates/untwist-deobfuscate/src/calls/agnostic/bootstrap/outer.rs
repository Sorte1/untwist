use oxc_ast::ast::*;
use oxc_ast_visit::VisitJs;
use oxc_semantic::Scoping;
use oxc_syntax::scope::ScopeFlags;
use oxc_syntax::symbol::SymbolId;

use super::super::bind::reads_arguments;
use super::{binding_symbol, is_function, table_of};

fn outer_plain(call: &CallExpression<'_>, outer: &Function<'_>) -> bool {
    !call.optional
        && !outer.generator
        && !outer.r#async
        && outer.params.rest.is_none()
        && !reads_arguments(outer)
}

#[derive(Default)]
struct ArrowArguments {
    found: bool,
}

impl<'a> VisitJs<'a> for ArrowArguments {
    fn visit_function(&mut self, _it: &Function<'a>, _flags: ScopeFlags) {}
    fn visit_identifier_reference(&mut self, it: &IdentifierReference<'a>) {
        self.found |= it.name == "arguments";
    }
}

fn arrow_plain(
    call: &CallExpression<'_>,
    arrow: &ArrowFunctionExpression<'_>,
    body: &FunctionBody<'_>,
) -> bool {
    let mut scan = ArrowArguments::default();
    scan.visit_function_body(body);
    !call.optional && !arrow.r#async && arrow.params.rest.is_none() && !scan.found
}

pub(super) struct Outer<'a> {
    pub(super) body: &'a FunctionBody<'a>,
    pub(super) table: SymbolId,
    pub(super) elements: &'a Expression<'a>,
    pub(super) direct: bool,
    pub(super) plain: bool,
}

fn declared_tables<'a>(
    scoping: &Scoping,
    body: &'a FunctionBody<'a>,
) -> Vec<(SymbolId, &'a Expression<'a>)> {
    let mut out = Vec::new();
    for statement in &body.statements {
        let Statement::VariableDeclaration(decl) = statement else {
            continue;
        };
        for declarator in &decl.declarations {
            let (BindingPattern::BindingIdentifier(id), Some(init)) =
                (&declarator.id, &declarator.init)
            else {
                continue;
            };
            let Some(symbol) = id.symbol_id.get() else {
                continue;
            };
            if table_of(scoping, init).is_some_and(|table| !table.functions.is_empty()) {
                out.push((symbol, init));
            }
        }
    }
    out
}

pub(super) fn outers<'a>(scoping: &Scoping, call: &'a CallExpression<'a>) -> Vec<Outer<'a>> {
    match call.callee.without_parentheses() {
        Expression::FunctionExpression(outer) if !outer.params.items.is_empty() => {
            let (Some(table), Some(elements), Some(body)) = (
                outer.params.items.first().and_then(binding_symbol),
                call.arguments.first().and_then(Argument::as_expression),
                outer.body.as_ref(),
            ) else {
                return Vec::new();
            };
            vec![Outer {
                body,
                table,
                elements,
                direct: false,
                plain: outer_plain(call, outer),
            }]
        }
        Expression::FunctionExpression(outer) => {
            let Some(body) = outer.body.as_ref() else {
                return Vec::new();
            };
            let plain = outer_plain(call, outer);
            declared(scoping, body, plain)
        }
        Expression::ArrowFunctionExpression(arrow) => {
            let ArrowFunctionBody::FunctionBody(body) = &arrow.body else {
                return Vec::new();
            };
            let plain = arrow_plain(call, arrow, body);
            if arrow.params.items.is_empty() {
                return declared(scoping, body, plain);
            }
            let (Some(table), Some(elements)) = (
                arrow.params.items.first().and_then(binding_symbol),
                call.arguments.first().and_then(Argument::as_expression),
            ) else {
                return Vec::new();
            };
            vec![Outer {
                body,
                table,
                elements,
                direct: false,
                plain,
            }]
        }
        _ => Vec::new(),
    }
}

fn declared<'a>(scoping: &Scoping, body: &'a FunctionBody<'a>, plain: bool) -> Vec<Outer<'a>> {
    declared_tables(scoping, body)
        .into_iter()
        .map(|(table, elements)| Outer {
            body,
            table,
            elements,
            direct: true,
            plain,
        })
        .collect()
}

pub(in crate::calls::agnostic) fn declared_candidate(call: &CallExpression<'_>) -> bool {
    let body = match call.callee.without_parentheses() {
        Expression::FunctionExpression(func) if func.params.items.is_empty() => func.body.as_ref(),
        Expression::ArrowFunctionExpression(arrow) if arrow.params.items.is_empty() => {
            match &arrow.body {
                ArrowFunctionBody::FunctionBody(body) => Some(body),
                _ => None,
            }
        }
        _ => None,
    };
    body.is_some_and(|body| {
        body.statements.iter().any(|statement| {
            matches!(statement, Statement::VariableDeclaration(decl) if decl.declarations.iter().any(|declarator| {
                declarator.init.as_ref().is_some_and(|init| matches!(init.without_parentheses(), Expression::ObjectExpression(object) if object.properties.iter().any(|property| matches!(property, ObjectPropertyKind::ObjectProperty(p) if is_function(&p.value)))))
            }))
        })
    })
}
