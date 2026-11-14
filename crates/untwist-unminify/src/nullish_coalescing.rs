use crate::transpile_util::{
    assignment_temp_symbol, has_reference_counts, identifier_symbol_is, is_repeatable,
    is_undefined_identifier, take_binary_left_assignment_value, take_logical_left_assignment_value,
    TempVars,
};
use oxc_allocator::{Box as ABox, Vec as AVec};
use oxc_ast::ast::*;
use oxc_span::{ContentEq, Span, SPAN};
use oxc_syntax::scope::ScopeId;
use oxc_syntax::symbol::SymbolId;
use untwist_core::ast_util::scope::binding_symbol;
use untwist_core::pass::{Cx, ExpressionKind, ExpressionVariants, Interest, Locality, Pass};

#[derive(Default)]
pub struct NullishCoalescing {
    temps: TempVars,
}

pub fn boxed<'a>() -> Box<dyn Pass<'a> + 'a> {
    Box::new(NullishCoalescing::default())
}

const ACTED_ON_EXPRESSIONS: ExpressionVariants = ExpressionVariants::of(&[
    ExpressionKind::CallExpression,
    ExpressionKind::ConditionalExpression,
]);

impl<'a> Pass<'a> for NullishCoalescing {
    fn name(&self) -> &'static str {
        "nullish-coalescing"
    }

    fn locality(&self) -> Locality {
        Locality::Local
    }

    fn interest(&self) -> Interest {
        (Interest::ENTER_PROGRAM
            | Interest::EXIT_VARIABLE_DECLARATION
            | Interest::EXIT_EXPRESSION
            | Interest::EXIT_STATEMENTS)
            .on_expressions(ACTED_ON_EXPRESSIONS)
    }

    fn enter_program(&mut self, node: &mut Program<'a>, cx: &mut Cx<'a, '_>) {
        self.temps.reset();
        cx.observe_names(node);
    }

    fn exit_variable_declaration(
        &mut self,
        node: &mut VariableDeclaration<'a>,
        cx: &mut Cx<'a, '_>,
    ) {
        self.temps.record_declaration(node, cx);
    }

    fn exit_expression(&mut self, node: &mut Expression<'a>, cx: &mut Cx<'a, '_>) {
        let rewritten = rewrite_iife(node, cx)
            || rewrite_temp_strict(node, &mut self.temps, cx)
            || rewrite_temp_loose(node, &mut self.temps, cx)
            || rewrite_simple(node, cx);
        if rewritten {
            cx.changed();
        }
    }

    fn exit_statements(&mut self, node: &mut AVec<'a, Statement<'a>>, _cx: &mut Cx<'a, '_>) {
        self.temps.strip_declarations(node);
    }
}

fn strict_temp_test(test: &Expression<'_>, cx: &Cx<'_, '_>) -> Option<SymbolId> {
    let Expression::LogicalExpression(logical) = test else {
        return None;
    };
    if logical.operator != LogicalOperator::And {
        return None;
    }
    let Expression::BinaryExpression(first) = &logical.left else {
        return None;
    };
    if first.operator != BinaryOperator::StrictInequality
        || !matches!(first.right, Expression::NullLiteral(_))
    {
        return None;
    }
    let symbol = assignment_temp_symbol(&first.left, cx)?;
    let Expression::BinaryExpression(second) = &logical.right else {
        return None;
    };
    if second.operator != BinaryOperator::StrictInequality
        || !is_undefined_identifier(&second.right, cx)
        || !identifier_symbol_is(&second.left, symbol, cx)
    {
        return None;
    }
    Some(symbol)
}

fn loose_temp_test(test: &Expression<'_>, cx: &Cx<'_, '_>) -> Option<SymbolId> {
    let Expression::BinaryExpression(binary) = test else {
        return None;
    };
    if binary.operator != BinaryOperator::Inequality
        || !matches!(binary.right, Expression::NullLiteral(_))
    {
        return None;
    }
    assignment_temp_symbol(&binary.left, cx)
}

fn simple_test<'k, 'a>(test: &'k Expression<'a>, cx: &Cx<'_, '_>) -> Option<&'k Expression<'a>> {
    match test {
        Expression::LogicalExpression(logical) if logical.operator == LogicalOperator::And => {
            let Expression::BinaryExpression(first) = &logical.left else {
                return None;
            };
            let Expression::BinaryExpression(second) = &logical.right else {
                return None;
            };
            if first.operator != BinaryOperator::StrictInequality
                || second.operator != BinaryOperator::StrictInequality
                || !matches!(first.right, Expression::NullLiteral(_))
                || !is_undefined_identifier(&second.right, cx)
                || !ContentEq::content_eq(&first.left, &second.left)
            {
                return None;
            }
            Some(&first.left)
        }
        Expression::BinaryExpression(binary) if binary.operator == BinaryOperator::Inequality => {
            matches!(binary.right, Expression::NullLiteral(_)).then_some(&binary.left)
        }
        _ => None,
    }
}

fn take_conditional<'a>(
    node: &mut Expression<'a>,
    cx: &mut Cx<'a, '_>,
) -> Option<ConditionalExpression<'a>> {
    let old = std::mem::replace(node, Expression::new_null_literal(SPAN, cx));
    match old {
        Expression::ConditionalExpression(conditional) => Some(conditional.unbox()),
        other => {
            *node = other;
            None
        }
    }
}

fn take_simple_left<'a>(
    test: Expression<'a>,
    cx: &mut Cx<'a, '_>,
) -> Result<Expression<'a>, Expression<'a>> {
    match test {
        Expression::LogicalExpression(logical) => {
            let logical = logical.unbox();
            let logical_span = logical.span;
            let logical_operator = logical.operator;
            match logical.left {
                Expression::BinaryExpression(binary) => Ok(binary.unbox().left),
                other => Err(Expression::new_logical_expression(
                    logical_span,
                    other,
                    logical_operator,
                    logical.right,
                    cx,
                )),
            }
        }
        Expression::BinaryExpression(binary) => Ok(binary.unbox().left),
        other => Err(other),
    }
}

fn rewrite_temp_strict<'a>(
    node: &mut Expression<'a>,
    temps: &mut TempVars,
    cx: &mut Cx<'a, '_>,
) -> bool {
    let Expression::ConditionalExpression(conditional) = &*node else {
        return false;
    };
    let Some(symbol) = strict_temp_test(&conditional.test, cx) else {
        return false;
    };
    if !identifier_symbol_is(&conditional.consequent, symbol, cx) || !temps.is_temp(symbol, 2, cx) {
        return false;
    }
    let span = conditional.span;
    let Some(conditional) = take_conditional(node, cx) else {
        return false;
    };
    let value = match take_logical_left_assignment_value(conditional.test, cx) {
        Ok(value) => value,
        Err(test) => {
            *node = Expression::new_conditional_expression(
                span,
                test,
                conditional.consequent,
                conditional.alternate,
                cx,
            );
            return false;
        }
    };
    temps.consume(symbol);
    *node = Expression::new_logical_expression(
        span,
        value,
        LogicalOperator::Coalesce,
        conditional.alternate,
        cx,
    );
    true
}

fn rewrite_temp_loose<'a>(
    node: &mut Expression<'a>,
    temps: &mut TempVars,
    cx: &mut Cx<'a, '_>,
) -> bool {
    let Expression::ConditionalExpression(conditional) = &*node else {
        return false;
    };
    let Some(symbol) = loose_temp_test(&conditional.test, cx) else {
        return false;
    };
    if !identifier_symbol_is(&conditional.consequent, symbol, cx) || !temps.is_temp(symbol, 1, cx) {
        return false;
    }
    let span = conditional.span;
    let Some(conditional) = take_conditional(node, cx) else {
        return false;
    };
    let value = match take_binary_left_assignment_value(conditional.test, cx) {
        Ok(value) => value,
        Err(test) => {
            *node = Expression::new_conditional_expression(
                span,
                test,
                conditional.consequent,
                conditional.alternate,
                cx,
            );
            return false;
        }
    };
    temps.consume(symbol);
    *node = Expression::new_logical_expression(
        span,
        value,
        LogicalOperator::Coalesce,
        conditional.alternate,
        cx,
    );
    true
}

fn rewrite_simple<'a>(node: &mut Expression<'a>, cx: &mut Cx<'a, '_>) -> bool {
    let Expression::ConditionalExpression(conditional) = &*node else {
        return false;
    };
    let Some(left) = simple_test(&conditional.test, cx) else {
        return false;
    };
    if !ContentEq::content_eq(left, &conditional.consequent) || !is_repeatable(left, cx) {
        return false;
    }
    let span = conditional.span;
    let Some(conditional) = take_conditional(node, cx) else {
        return false;
    };
    let value = match take_simple_left(conditional.test, cx) {
        Ok(value) => value,
        Err(test) => {
            *node = Expression::new_conditional_expression(
                span,
                test,
                conditional.consequent,
                conditional.alternate,
                cx,
            );
            return false;
        }
    };
    *node = Expression::new_logical_expression(
        span,
        value,
        LogicalOperator::Coalesce,
        conditional.alternate,
        cx,
    );
    true
}

pub fn would_rewrite(conditional: &ConditionalExpression<'_>, cx: &Cx<'_, '_>) -> bool {
    let consequent_is = |symbol| identifier_symbol_is(&conditional.consequent, symbol, cx);
    strict_temp_test(&conditional.test, cx).is_some_and(consequent_is)
        || loose_temp_test(&conditional.test, cx).is_some_and(consequent_is)
        || simple_test(&conditional.test, cx).is_some_and(|left| {
            ContentEq::content_eq(left, &conditional.consequent) && is_repeatable(left, cx)
        })
}

fn iife_matches(call: &CallExpression<'_>, cx: &Cx<'_, '_>) -> bool {
    if call.optional || !call.arguments.is_empty() {
        return false;
    }
    let Expression::ArrowFunctionExpression(arrow) = &call.callee else {
        return false;
    };
    if arrow.r#async || arrow.params.rest.is_some() || arrow.params.items.len() != 1 {
        return false;
    }
    let parameter = &arrow.params.items[0];
    if parameter.initializer.is_some() {
        return false;
    }
    let Some(symbol) = binding_symbol(&parameter.pattern) else {
        return false;
    };
    let ArrowFunctionBody::ConditionalExpression(conditional) = &arrow.body else {
        return false;
    };
    strict_temp_test(&conditional.test, cx) == Some(symbol)
        && identifier_symbol_is(&conditional.consequent, symbol, cx)
        && has_reference_counts(symbol, 2, 1, cx)
}

struct CallMeta<'a> {
    span: Span,
    type_arguments: Option<ABox<'a, TSTypeParameterInstantiation<'a>>>,
    arguments: AVec<'a, Argument<'a>>,
    optional: bool,
    pure: bool,
}

fn rebuild_call<'a>(
    meta: CallMeta<'a>,
    callee: Expression<'a>,
    cx: &mut Cx<'a, '_>,
) -> Expression<'a> {
    Expression::new_call_expression_with_pure(
        meta.span,
        callee,
        meta.type_arguments,
        meta.arguments,
        meta.optional,
        meta.pure,
        cx,
    )
}

struct ArrowMeta<'a> {
    span: Span,
    r#async: bool,
    type_parameters: Option<ABox<'a, TSTypeParameterDeclaration<'a>>>,
    params: ABox<'a, FormalParameters<'a>>,
    return_type: Option<ABox<'a, TSTypeAnnotation<'a>>>,
    scope_id: Option<ScopeId>,
    pure: bool,
    pife: bool,
}

fn rebuild_arrow<'a>(
    meta: ArrowMeta<'a>,
    body: ArrowFunctionBody<'a>,
    cx: &mut Cx<'a, '_>,
) -> Expression<'a> {
    match meta.scope_id {
        Some(scope_id) => {
            Expression::new_arrow_function_expression_with_scope_id_and_pure_and_pife(
                meta.span,
                meta.r#async,
                meta.type_parameters,
                meta.params,
                meta.return_type,
                body,
                scope_id,
                meta.pure,
                meta.pife,
                cx,
            )
        }
        None => Expression::new_arrow_function_expression(
            meta.span,
            meta.r#async,
            meta.type_parameters,
            meta.params,
            meta.return_type,
            body,
            cx,
        ),
    }
}

fn take_iife_call<'a>(
    node: &mut Expression<'a>,
    cx: &mut Cx<'a, '_>,
) -> Option<CallExpression<'a>> {
    match std::mem::replace(node, Expression::new_null_literal(SPAN, cx)) {
        Expression::CallExpression(call) => Some(call.unbox()),
        other => {
            *node = other;
            None
        }
    }
}

fn take_iife_conditional<'a>(
    node: &mut Expression<'a>,
    call_meta: CallMeta<'a>,
    callee: Expression<'a>,
    cx: &mut Cx<'a, '_>,
) -> Option<(CallMeta<'a>, ArrowMeta<'a>, ConditionalExpression<'a>)> {
    let arrow = match callee {
        Expression::ArrowFunctionExpression(arrow) => arrow.unbox(),
        other => {
            *node = rebuild_call(call_meta, other, cx);
            return None;
        }
    };
    let arrow_meta = ArrowMeta {
        span: arrow.span,
        r#async: arrow.r#async,
        type_parameters: arrow.type_parameters,
        params: arrow.params,
        return_type: arrow.return_type,
        scope_id: arrow.scope_id.get(),
        pure: arrow.pure,
        pife: arrow.pife,
    };
    match arrow.body {
        ArrowFunctionBody::ConditionalExpression(conditional) => {
            Some((call_meta, arrow_meta, conditional.unbox()))
        }
        other => {
            let rebuilt_arrow = rebuild_arrow(arrow_meta, other, cx);
            *node = rebuild_call(call_meta, rebuilt_arrow, cx);
            None
        }
    }
}

fn rewrite_iife<'a>(node: &mut Expression<'a>, cx: &mut Cx<'a, '_>) -> bool {
    let Expression::CallExpression(call) = &*node else {
        return false;
    };
    if !iife_matches(call, cx) {
        return false;
    }
    let Some(call) = take_iife_call(node, cx) else {
        return false;
    };
    let call_meta = CallMeta {
        span: call.span,
        type_arguments: call.type_arguments,
        arguments: call.arguments,
        optional: call.optional,
        pure: call.pure,
    };
    let Some((call_meta, arrow_meta, conditional)) =
        take_iife_conditional(node, call_meta, call.callee, cx)
    else {
        return false;
    };
    let value = match take_logical_left_assignment_value(conditional.test, cx) {
        Ok(value) => value,
        Err(test) => {
            let rebuilt_body = ArrowFunctionBody::new_conditional_expression(
                conditional.span,
                test,
                conditional.consequent,
                conditional.alternate,
                cx,
            );
            let rebuilt_arrow = rebuild_arrow(arrow_meta, rebuilt_body, cx);
            *node = rebuild_call(call_meta, rebuilt_arrow, cx);
            return false;
        }
    };
    *node = Expression::new_logical_expression(
        call_meta.span,
        value,
        LogicalOperator::Coalesce,
        conditional.alternate,
        cx,
    );
    true
}
