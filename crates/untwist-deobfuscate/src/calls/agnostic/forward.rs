use oxc_ast::ast::*;
use oxc_semantic::Scoping;
use oxc_span::Span;
use oxc_syntax::symbol::SymbolId;

fn resolves_to(scoping: &Scoping, expr: &Expression<'_>, symbol: SymbolId) -> bool {
    let Expression::Identifier(id) = expr.without_parentheses() else {
        return false;
    };
    id.reference_id
        .get()
        .and_then(|reference| scoping.get_reference(reference).symbol_id())
        == Some(symbol)
}

fn param_symbol(param: &FormalParameter<'_>) -> Option<SymbolId> {
    match (&param.pattern, &param.initializer) {
        (BindingPattern::BindingIdentifier(id), None) => id.symbol_id.get(),
        _ => None,
    }
}

pub(super) fn forwarding_call(scoping: &Scoping, func: &Function<'_>) -> Option<Span> {
    if func.r#async || func.generator || func.params.rest.is_some() || func.id.is_some() {
        return None;
    }
    let (first, rest) = func.params.items.split_first()?;
    let first = param_symbol(first)?;
    let [Statement::ReturnStatement(ret)] = func.body.as_ref()?.statements.as_slice() else {
        return None;
    };
    let Expression::CallExpression(call) = ret.argument.as_ref()?.without_parentheses() else {
        return None;
    };
    let forwarded = !call.optional
        && call.arguments.len() == rest.len()
        && resolves_to(scoping, &call.callee, first)
        && call.arguments.iter().zip(rest).all(|(argument, param)| {
            param_symbol(param).is_some_and(|symbol| {
                argument
                    .as_expression()
                    .is_some_and(|value| resolves_to(scoping, value, symbol))
            })
        });
    forwarded.then_some(call.span)
}

pub(super) fn passing_call<'k, 'a>(
    scoping: &Scoping,
    func: &'k Function<'a>,
) -> Option<&'k CallExpression<'a>> {
    if func.r#async || func.generator || func.params.rest.is_some() || func.id.is_some() {
        return None;
    }
    let [Statement::ReturnStatement(ret)] = func.body.as_ref()?.statements.as_slice() else {
        return None;
    };
    let Expression::CallExpression(call) = ret.argument.as_ref()?.without_parentheses() else {
        return None;
    };
    let passed = !call.optional
        && !func.params.items.is_empty()
        && call.arguments.len() == func.params.items.len()
        && call
            .arguments
            .iter()
            .zip(&func.params.items)
            .all(|(argument, param)| {
                param_symbol(param).is_some_and(|symbol| {
                    argument
                        .as_expression()
                        .is_some_and(|value| resolves_to(scoping, value, symbol))
                })
            });
    passed.then_some(&**call)
}
