use crate::transpile_util::{
    assignment_temp_symbol, identifier_symbol_is, is_repeatable, is_undefined_identifier,
    take_binary_left_assignment_value, take_logical_left_assignment_value, TempVars,
};
use oxc_allocator::Vec as AVec;
use oxc_ast::ast::*;
use oxc_span::{ContentEq, Span, SPAN};
use oxc_traverse::Ancestor;
use untwist_core::pass::{Cx, ExpressionKind, ExpressionVariants, Interest, Locality, Pass};

#[derive(Default)]
pub struct OptionalChaining {
    temps: TempVars,
}

pub fn boxed<'a>() -> Box<dyn Pass<'a> + 'a> {
    Box::new(OptionalChaining::default())
}

const ACTED_ON_EXPRESSIONS: ExpressionVariants =
    ExpressionVariants::of(&[ExpressionKind::ConditionalExpression]);

impl<'a> Pass<'a> for OptionalChaining {
    fn name(&self) -> &'static str {
        "optional-chaining"
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

    fn enter_program(&mut self, _node: &mut Program<'a>, _cx: &mut Cx<'a, '_>) {
        self.temps.reset();
    }

    fn exit_variable_declaration(
        &mut self,
        node: &mut VariableDeclaration<'a>,
        cx: &mut Cx<'a, '_>,
    ) {
        self.temps.record_declaration(node, cx);
    }

    fn exit_expression(&mut self, node: &mut Expression<'a>, cx: &mut Cx<'a, '_>) {
        let Expression::ConditionalExpression(_) = &*node else {
            return;
        };
        if is_reference_position(cx) {
            return;
        }
        if rewrite_simple(node, cx) || rewrite_temp(node, &mut self.temps, cx) {
            cx.changed();
        }
    }

    fn exit_statements(&mut self, node: &mut AVec<'a, Statement<'a>>, _cx: &mut Cx<'a, '_>) {
        self.temps.strip_declarations(node);
    }
}

fn is_reference_position(cx: &Cx<'_, '_>) -> bool {
    let parent = cx
        .tcx
        .ancestors()
        .find(|ancestor| !matches!(ancestor, Ancestor::ParenthesizedExpressionExpression(_)));
    match parent {
        Some(Ancestor::CallExpressionCallee(_) | Ancestor::TaggedTemplateExpressionTag(_)) => true,
        Some(Ancestor::UnaryExpressionArgument(unary)) => {
            *unary.operator() == UnaryOperator::Delete
        }
        _ => false,
    }
}

fn nullish_or_test<'k, 'a>(
    test: &'k Expression<'a>,
    cx: &Cx<'_, '_>,
) -> Option<(&'k Expression<'a>, &'k Expression<'a>)> {
    let Expression::LogicalExpression(logical) = test else {
        return None;
    };
    if logical.operator != LogicalOperator::Or {
        return None;
    }
    let Expression::BinaryExpression(first) = &logical.left else {
        return None;
    };
    let Expression::BinaryExpression(second) = &logical.right else {
        return None;
    };
    if first.operator != BinaryOperator::StrictEquality
        || second.operator != BinaryOperator::StrictEquality
        || !matches!(first.right, Expression::NullLiteral(_))
        || !is_undefined_identifier(&second.right, cx)
    {
        return None;
    }
    Some((&first.left, &second.left))
}

fn loose_null_test<'k, 'a>(test: &'k Expression<'a>) -> Option<&'k Expression<'a>> {
    let Expression::BinaryExpression(binary) = test else {
        return None;
    };
    (binary.operator == BinaryOperator::Equality
        && matches!(binary.right, Expression::NullLiteral(_)))
    .then_some(&binary.left)
}

fn tested<'k, 'a>(
    test: &'k Expression<'a>,
    cx: &Cx<'_, '_>,
) -> Option<(&'k Expression<'a>, &'k Expression<'a>)> {
    nullish_or_test(test, cx).or_else(|| loose_null_test(test).map(|left| (left, left)))
}

fn member_object<'k, 'a>(expr: &'k Expression<'a>) -> Option<&'k Expression<'a>> {
    match expr {
        Expression::StaticMemberExpression(member) if !member.optional => Some(&member.object),
        Expression::ComputedMemberExpression(member) if !member.optional => Some(&member.object),
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

fn chain_element_to_expression(element: ChainElement<'_>) -> Expression<'_> {
    match element {
        ChainElement::CallExpression(call) => Expression::CallExpression(call),
        ChainElement::TSNonNullExpression(assertion) => Expression::TSNonNullExpression(assertion),
        ChainElement::StaticMemberExpression(member) => Expression::StaticMemberExpression(member),
        ChainElement::ComputedMemberExpression(member) => {
            Expression::ComputedMemberExpression(member)
        }
        ChainElement::PrivateFieldExpression(member) => Expression::PrivateFieldExpression(member),
    }
}

fn flatten_chain_object<'a>(object: Expression<'a>) -> Expression<'a> {
    let Expression::ChainExpression(chain) = object else {
        return object;
    };
    chain_element_to_expression(chain.unbox().expression)
}

enum ChainMember<'a> {
    Static(StaticMemberExpression<'a>),
    Computed(ComputedMemberExpression<'a>),
}

fn take_chain_member<'a>(alternate: Expression<'a>) -> Result<ChainMember<'a>, Expression<'a>> {
    match alternate {
        Expression::StaticMemberExpression(member) => Ok(ChainMember::Static(member.unbox())),
        Expression::ComputedMemberExpression(member) => Ok(ChainMember::Computed(member.unbox())),
        other => Err(other),
    }
}

fn rebuild_chain_member<'a>(member: ChainMember<'a>, cx: &mut Cx<'a, '_>) -> Expression<'a> {
    match member {
        ChainMember::Static(member) => Expression::new_static_member_expression(
            member.span,
            member.object,
            member.property,
            member.optional,
            cx,
        ),
        ChainMember::Computed(member) => Expression::new_computed_member_expression(
            member.span,
            member.object,
            member.expression,
            member.optional,
            cx,
        ),
    }
}

fn finish_chain<'a>(
    member: ChainMember<'a>,
    object: Option<Expression<'a>>,
    span: Span,
    cx: &mut Cx<'a, '_>,
) -> Expression<'a> {
    let element = match member {
        ChainMember::Static(member) => {
            let object = flatten_chain_object(object.unwrap_or(member.object));
            ChainElement::new_static_member_expression(span, object, member.property, true, cx)
        }
        ChainMember::Computed(member) => {
            let object = flatten_chain_object(object.unwrap_or(member.object));
            ChainElement::new_computed_member_expression(span, object, member.expression, true, cx)
        }
    };
    Expression::new_chain_expression(span, element, cx)
}

fn take_chain_member_or_restore<'a>(
    node: &mut Expression<'a>,
    conditional: ConditionalExpression<'a>,
    span: Span,
    cx: &mut Cx<'a, '_>,
) -> Option<(ChainMember<'a>, Expression<'a>, Expression<'a>)> {
    match take_chain_member(conditional.alternate) {
        Ok(member) => Some((member, conditional.test, conditional.consequent)),
        Err(alternate) => {
            *node = Expression::new_conditional_expression(
                span,
                conditional.test,
                conditional.consequent,
                alternate,
                cx,
            );
            None
        }
    }
}

fn rewrite_simple<'a>(node: &mut Expression<'a>, cx: &mut Cx<'a, '_>) -> bool {
    let Expression::ConditionalExpression(conditional) = &*node else {
        return false;
    };
    let Some((first, second)) = tested(&conditional.test, cx) else {
        return false;
    };
    if !ContentEq::content_eq(first, second)
        || !is_undefined_identifier(&conditional.consequent, cx)
    {
        return false;
    }
    let Some(object) = member_object(&conditional.alternate) else {
        return false;
    };
    if !ContentEq::content_eq(first, object) || !is_repeatable(first, cx) {
        return false;
    }
    let span = conditional.span;
    let Some(conditional) = take_conditional(node, cx) else {
        return false;
    };
    let Some((member, _, _)) = take_chain_member_or_restore(node, conditional, span, cx) else {
        return false;
    };
    *node = finish_chain(member, None, span, cx);
    true
}

fn rewrite_temp<'a>(node: &mut Expression<'a>, temps: &mut TempVars, cx: &mut Cx<'a, '_>) -> bool {
    let Expression::ConditionalExpression(conditional) = &*node else {
        return false;
    };
    let loose = loose_null_test(&conditional.test).is_some();
    let Some((first, second)) = tested(&conditional.test, cx) else {
        return false;
    };
    let Some(symbol) = assignment_temp_symbol(first, cx) else {
        return false;
    };
    if !(loose || identifier_symbol_is(second, symbol, cx))
        || !is_undefined_identifier(&conditional.consequent, cx)
    {
        return false;
    }
    let Some(object) = member_object(&conditional.alternate) else {
        return false;
    };
    let reads = if loose { 1 } else { 2 };
    if !identifier_symbol_is(object, symbol, cx) || !temps.is_temp(symbol, reads, cx) {
        return false;
    }
    let span = conditional.span;
    let Some(conditional) = take_conditional(node, cx) else {
        return false;
    };
    let Some((member, test, consequent)) =
        take_chain_member_or_restore(node, conditional, span, cx)
    else {
        return false;
    };
    let taken = if loose {
        take_binary_left_assignment_value(test, cx)
    } else {
        take_logical_left_assignment_value(test, cx)
    };
    let object = match taken {
        Ok(object) => object,
        Err(test) => {
            let alternate = rebuild_chain_member(member, cx);
            *node = Expression::new_conditional_expression(span, test, consequent, alternate, cx);
            return false;
        }
    };
    temps.consume(symbol);
    *node = finish_chain(member, Some(object), span, cx);
    true
}

pub fn would_rewrite(conditional: &ConditionalExpression<'_>, cx: &Cx<'_, '_>) -> bool {
    let loose = loose_null_test(&conditional.test).is_some();
    let Some((first, second)) = tested(&conditional.test, cx) else {
        return false;
    };
    if !is_undefined_identifier(&conditional.consequent, cx) {
        return false;
    }
    let Some(object) = member_object(&conditional.alternate) else {
        return false;
    };
    let simple = ContentEq::content_eq(first, second)
        && ContentEq::content_eq(first, object)
        && is_repeatable(first, cx);
    simple
        || assignment_temp_symbol(first, cx).is_some_and(|symbol| {
            (loose || identifier_symbol_is(second, symbol, cx))
                && identifier_symbol_is(object, symbol, cx)
        })
}
