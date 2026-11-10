use crate::string_literal_in_template::{escape_template, join_raw};
use oxc_allocator::{Box as ABox, Vec as AVec};
use oxc_ast::ast::*;
use oxc_span::SPAN;
use oxc_str::Str;
use untwist_core::pass::{
    Cx, ExpressionKind, ExpressionVariants, Interest, Locality, Pass, Safety,
};

#[derive(Default)]
pub struct TemplateLiterals;

pub fn boxed<'a>() -> Box<dyn Pass<'a> + 'a> {
    Box::new(TemplateLiterals)
}

const ACTED_ON_EXPRESSIONS: ExpressionVariants = ExpressionVariants::of(&[
    ExpressionKind::BinaryExpression,
    ExpressionKind::CallExpression,
]);

impl<'a> Pass<'a> for TemplateLiterals {
    fn name(&self) -> &'static str {
        "template-literals"
    }

    fn locality(&self) -> Locality {
        Locality::Neutral
    }

    fn interest(&self) -> Interest {
        Interest::EXIT_EXPRESSION.on_expressions(ACTED_ON_EXPRESSIONS)
    }

    fn safety(&self) -> Safety {
        Safety::Unsafe
    }

    fn exit_expression(&mut self, node: &mut Expression<'a>, cx: &mut Cx<'a, '_>) {
        if merge_concatenation(node, cx) || convert_concat_call(node, cx) {
            cx.changed();
        }
    }
}

fn expression_has_lone_surrogates(expr: &Expression<'_>) -> bool {
    match expr {
        Expression::StringLiteral(literal) => literal.value.has_lone_surrogate(),
        Expression::TemplateLiteral(template) => template.quasis.iter().any(|quasi| {
            quasi
                .value
                .cooked
                .is_some_and(|cooked| cooked.has_lone_surrogate())
        }),
        _ => false,
    }
}

fn empty_element<'a>(cx: &mut Cx<'a, '_>) -> TemplateElement<'a> {
    let value = TemplateElementValue {
        raw: Str::from_str_in("", cx),
        cooked: None,
    };
    TemplateElement::new(SPAN, value, false, cx)
}

fn append_raw<'a>(template: &mut TemplateLiteral<'a>, text: &str, cx: &mut Cx<'a, '_>) {
    let Some(last) = template.quasis.last_mut() else {
        return;
    };
    let mut raw = last.value.raw.as_str().to_string();
    join_raw(&mut raw, text);
    last.value.raw = Str::from_str_in(&raw, cx);
    last.value.cooked = None;
}

fn prepend_raw<'a>(template: &mut TemplateLiteral<'a>, text: &str, cx: &mut Cx<'a, '_>) {
    let Some(first) = template.quasis.first_mut() else {
        return;
    };
    let mut raw = text.to_string();
    join_raw(&mut raw, first.value.raw.as_str());
    first.value.raw = Str::from_str_in(&raw, cx);
    first.value.cooked = None;
}

fn normalize_tails(template: &mut TemplateLiteral<'_>) {
    let last = template.quasis.len().saturating_sub(1);
    for (index, quasi) in template.quasis.iter_mut().enumerate() {
        quasi.tail = index == last;
    }
}

fn push<'a>(template: &mut TemplateLiteral<'a>, value: Expression<'a>, cx: &mut Cx<'a, '_>) {
    if let Some(text) = plain_string(&value) {
        append_raw(template, &escape_template(text), cx);
        return;
    }
    match value {
        Expression::TemplateLiteral(other) => {
            let other = other.unbox();
            let mut quasis = other.quasis.into_iter();
            if let Some(first) = quasis.next() {
                let text = first.value.raw.as_str().to_string();
                append_raw(template, &text, cx);
            }
            template.expressions.extend(other.expressions);
            template.quasis.extend(quasis);
        }
        other => {
            template.expressions.push(other);
            let element = empty_element(cx);
            template.quasis.push(element);
        }
    }
}

fn plain_string<'a>(value: &Expression<'a>) -> Option<&'a str> {
    match value {
        Expression::StringLiteral(literal) => literal.value.as_str(),
        _ => None,
    }
}

fn unshift<'a>(template: &mut TemplateLiteral<'a>, value: Expression<'a>, cx: &mut Cx<'a, '_>) {
    if let Some(text) = plain_string(&value) {
        prepend_raw(template, &escape_template(text), cx);
        return;
    }
    template.expressions.insert(0, value);
    let element = empty_element(cx);
    template.quasis.insert(0, element);
}

fn take_binary<'a>(node: &mut Expression<'a>, cx: &mut Cx<'a, '_>) -> Option<BinaryExpression<'a>> {
    match std::mem::replace(node, Expression::new_null_literal(SPAN, cx)) {
        Expression::BinaryExpression(binary) => Some(binary.unbox()),
        other => {
            *node = other;
            None
        }
    }
}

fn merge_left_template<'a>(
    node: &mut Expression<'a>,
    span: Span,
    left: Expression<'a>,
    right: Expression<'a>,
    cx: &mut Cx<'a, '_>,
) -> Option<ABox<'a, TemplateLiteral<'a>>> {
    match left {
        Expression::TemplateLiteral(mut template) => {
            push(&mut template, right, cx);
            Some(template)
        }
        other => {
            *node =
                Expression::new_binary_expression(span, other, BinaryOperator::Addition, right, cx);
            None
        }
    }
}

fn merge_right_template<'a>(
    node: &mut Expression<'a>,
    span: Span,
    left: Expression<'a>,
    right: Expression<'a>,
    cx: &mut Cx<'a, '_>,
) -> Option<ABox<'a, TemplateLiteral<'a>>> {
    match right {
        Expression::TemplateLiteral(mut template) => {
            unshift(&mut template, left, cx);
            Some(template)
        }
        other => {
            *node =
                Expression::new_binary_expression(span, left, BinaryOperator::Addition, other, cx);
            None
        }
    }
}

fn merge_concatenation<'a>(node: &mut Expression<'a>, cx: &mut Cx<'a, '_>) -> bool {
    let Expression::BinaryExpression(binary) = &*node else {
        return false;
    };
    if binary.operator != BinaryOperator::Addition {
        return false;
    }
    let left_is_template = matches!(binary.left, Expression::TemplateLiteral(_));
    let right_is_template = matches!(binary.right, Expression::TemplateLiteral(_));
    if !left_is_template && !right_is_template {
        return false;
    }
    if expression_has_lone_surrogates(&binary.left) || expression_has_lone_surrogates(&binary.right)
    {
        return false;
    }
    let Some(binary) = take_binary(node, cx) else {
        return false;
    };
    let BinaryExpression {
        span, left, right, ..
    } = binary;
    let merged = if left_is_template {
        merge_left_template(node, span, left, right, cx)
    } else {
        merge_right_template(node, span, left, right, cx)
    };
    let Some(mut template) = merged else {
        return false;
    };
    template.span = span;
    normalize_tails(&mut template);
    *node = Expression::TemplateLiteral(template);
    true
}

fn concat_receiver<'k, 'a>(callee: &'k Expression<'a>) -> Option<&'k Expression<'a>> {
    let (object, named_concat) = match callee {
        Expression::StaticMemberExpression(member) if !member.optional => {
            (&member.object, member.property.name.as_str() == "concat")
        }
        Expression::ComputedMemberExpression(member) if !member.optional => {
            let named = matches!(
                member.expression.without_parentheses(),
                Expression::StringLiteral(key) if key.value == "concat"
            );
            (&member.object, named)
        }
        _ => return None,
    };
    if !named_concat {
        return None;
    }
    matches!(
        object.without_parentheses(),
        Expression::StringLiteral(_) | Expression::TemplateLiteral(_)
    )
    .then_some(object)
}

fn take_concat_receiver<'a>(callee: Expression<'a>) -> Result<Expression<'a>, Expression<'a>> {
    match callee {
        Expression::StaticMemberExpression(member) => Ok(member.unbox().object),
        Expression::ComputedMemberExpression(member) => Ok(member.unbox().object),
        other => Err(other),
    }
}

fn take_call<'a>(node: &mut Expression<'a>, cx: &mut Cx<'a, '_>) -> Option<CallExpression<'a>> {
    match std::mem::replace(node, Expression::new_null_literal(SPAN, cx)) {
        Expression::CallExpression(call) => Some(call.unbox()),
        other => {
            *node = other;
            None
        }
    }
}

fn take_receiver_or_restore<'a>(
    node: &mut Expression<'a>,
    call: CallExpression<'a>,
    cx: &mut Cx<'a, '_>,
) -> Option<(Span, Expression<'a>, AVec<'a, Argument<'a>>)> {
    let CallExpression {
        span,
        callee,
        type_arguments,
        arguments,
        optional,
        pure,
        ..
    } = call;
    match take_concat_receiver(callee) {
        Ok(receiver) => Some((span, receiver, arguments)),
        Err(callee) => {
            *node = Expression::new_call_expression_with_pure(
                span,
                callee,
                type_arguments,
                arguments,
                optional,
                pure,
                cx,
            );
            None
        }
    }
}

fn convert_concat_call<'a>(node: &mut Expression<'a>, cx: &mut Cx<'a, '_>) -> bool {
    let Expression::CallExpression(call) = &*node else {
        return false;
    };
    if call.optional || !call.arguments.iter().all(Argument::is_expression) {
        return false;
    }
    let Some(receiver) = concat_receiver(&call.callee) else {
        return false;
    };
    if expression_has_lone_surrogates(receiver) {
        return false;
    }
    if call.arguments.iter().any(|argument| {
        argument
            .as_expression()
            .is_some_and(expression_has_lone_surrogates)
    }) {
        return false;
    }
    let Some(call) = take_call(node, cx) else {
        return false;
    };
    let Some((span, receiver, arguments)) = take_receiver_or_restore(node, call, cx) else {
        return false;
    };
    let mut quasis: AVec<'a, TemplateElement<'a>> = AVec::new_in(cx);
    let element = empty_element(cx);
    quasis.push(element);
    let expressions: AVec<'a, Expression<'a>> = AVec::new_in(cx);
    let mut template = ABox::new_in(TemplateLiteral::new(span, quasis, expressions, cx), cx);
    push(&mut template, receiver, cx);
    for argument in arguments {
        let Ok(value) = Expression::try_from(argument) else {
            continue;
        };
        push(&mut template, value, cx);
    }
    normalize_tails(&mut template);
    *node = Expression::TemplateLiteral(template);
    true
}
