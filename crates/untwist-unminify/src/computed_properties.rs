use oxc_allocator::ReplaceWith;
use oxc_ast::ast::*;
use oxc_span::GetSpan;
use oxc_str::Ident;
use oxc_syntax::identifier::is_identifier_name;
use untwist_core::pass::{Cx, ExpressionKind, ExpressionVariants, Interest, Locality, Pass};

#[derive(Default)]
pub struct ComputedProperties;

pub fn boxed<'a>() -> Box<dyn Pass<'a> + 'a> {
    Box::new(ComputedProperties)
}

fn identifier_name<'a>(expr: &Expression<'a>) -> Option<&'a str> {
    let Expression::StringLiteral(literal) = expr else {
        return None;
    };
    literal
        .value
        .as_str()
        .filter(|name| is_identifier_name(name))
}

fn string_key<'a>(key: &PropertyKey<'a>) -> Option<&'a str> {
    let PropertyKey::StringLiteral(literal) = key else {
        return None;
    };
    literal.value.as_str()
}

fn key_identifier_name<'a>(key: &PropertyKey<'a>) -> Option<&'a str> {
    string_key(key).filter(|name| is_identifier_name(name))
}

enum KeyContext {
    ObjectProperty,
    ClassMember { is_static: bool },
}

fn keeps_computed_key(key: &PropertyKey<'_>, context: KeyContext) -> bool {
    match context {
        KeyContext::ObjectProperty => string_key(key) == Some("__proto__"),
        KeyContext::ClassMember { is_static } => {
            string_key(key) == Some("constructor")
                || (is_static && string_key(key) == Some("prototype"))
        }
    }
}

fn uncompute_key<'a>(key: &mut PropertyKey<'a>, computed: &mut bool, cx: &mut Cx<'a, '_>) -> bool {
    let Some(name) = key_identifier_name(key) else {
        return false;
    };
    let span = key.span();
    *key = PropertyKey::new_static_identifier(span, Ident::from(name), &*cx);
    *computed = false;
    true
}

fn computed_member_property<'a>(
    member: &ComputedMemberExpression<'a>,
    cx: &Cx<'a, '_>,
) -> Option<IdentifierName<'a>> {
    let name = identifier_name(&member.expression)?;
    Some(IdentifierName::new(
        member.expression.span(),
        Ident::from(name),
        cx,
    ))
}

fn rewrite_computed_member<'a>(expr: &mut Expression<'a>, cx: &mut Cx<'a, '_>) -> bool {
    let Expression::ComputedMemberExpression(member) = &*expr else {
        return false;
    };
    let Some(property) = computed_member_property(member, &*cx) else {
        return false;
    };
    let mut rewritten = false;
    expr.replace_with(|old| {
        let member = match old {
            Expression::ComputedMemberExpression(member) => member,
            other => return other,
        };
        let member = member.unbox();
        rewritten = true;
        Expression::new_static_member_expression(
            member.span,
            member.object,
            property,
            member.optional,
            &*cx,
        )
    });
    rewritten
}

fn rewrite_computed_assignment_target<'a>(
    target: &mut AssignmentTarget<'a>,
    cx: &mut Cx<'a, '_>,
) -> bool {
    let AssignmentTarget::ComputedMemberExpression(member) = &*target else {
        return false;
    };
    let Some(property) = computed_member_property(member, &*cx) else {
        return false;
    };
    let mut rewritten = false;
    target.replace_with(|old| {
        let member = match old {
            AssignmentTarget::ComputedMemberExpression(member) => member,
            other => return other,
        };
        let member = member.unbox();
        rewritten = true;
        AssignmentTarget::new_static_member_expression(
            member.span,
            member.object,
            property,
            member.optional,
            &*cx,
        )
    });
    rewritten
}

fn rewrite_computed_update_target<'a>(
    target: &mut SimpleAssignmentTarget<'a>,
    cx: &mut Cx<'a, '_>,
) -> bool {
    let SimpleAssignmentTarget::ComputedMemberExpression(member) = &*target else {
        return false;
    };
    let Some(property) = computed_member_property(member, &*cx) else {
        return false;
    };
    let mut rewritten = false;
    target.replace_with(|old| {
        let member = match old {
            SimpleAssignmentTarget::ComputedMemberExpression(member) => member,
            other => return other,
        };
        let member = member.unbox();
        rewritten = true;
        SimpleAssignmentTarget::new_static_member_expression(
            member.span,
            member.object,
            property,
            member.optional,
            &*cx,
        )
    });
    rewritten
}

fn rewrite_computed_chain_member<'a>(chain: &mut ChainExpression<'a>, cx: &mut Cx<'a, '_>) -> bool {
    let ChainElement::ComputedMemberExpression(member) = &chain.expression else {
        return false;
    };
    let Some(property) = computed_member_property(member, &*cx) else {
        return false;
    };
    let mut rewritten = false;
    chain.expression.replace_with(|old| {
        let member = match old {
            ChainElement::ComputedMemberExpression(member) => member,
            other => return other,
        };
        let member = member.unbox();
        rewritten = true;
        ChainElement::new_static_member_expression(
            member.span,
            member.object,
            property,
            member.optional,
            &*cx,
        )
    });
    rewritten
}

const ACTED_ON_EXPRESSIONS: ExpressionVariants = ExpressionVariants::of(&[
    ExpressionKind::ComputedMemberExpression,
    ExpressionKind::ChainExpression,
    ExpressionKind::UpdateExpression,
]);

impl<'a> Pass<'a> for ComputedProperties {
    fn name(&self) -> &'static str {
        "computed-properties"
    }

    fn locality(&self) -> Locality {
        Locality::Neutral
    }

    fn interest(&self) -> Interest {
        (Interest::EXIT_EXPRESSION
            | Interest::EXIT_OBJECT_PROPERTY
            | Interest::EXIT_METHOD_DEFINITION
            | Interest::EXIT_PROPERTY_DEFINITION
            | Interest::EXIT_ASSIGNMENT_TARGET)
            .on_expressions(ACTED_ON_EXPRESSIONS)
    }

    fn exit_expression(&mut self, expr: &mut Expression<'a>, cx: &mut Cx<'a, '_>) {
        if rewrite_computed_member(expr, cx) {
            cx.changed();
            return;
        }
        let rewritten = match expr {
            Expression::ChainExpression(chain) => rewrite_computed_chain_member(chain, cx),
            Expression::UpdateExpression(update) => {
                rewrite_computed_update_target(&mut update.argument, cx)
            }
            _ => return,
        };
        if rewritten {
            cx.changed();
        }
    }

    fn exit_object_property(&mut self, property: &mut ObjectProperty<'a>, cx: &mut Cx<'a, '_>) {
        if property.kind == PropertyKind::Init
            && !property.method
            && keeps_computed_key(&property.key, KeyContext::ObjectProperty)
        {
            return;
        }
        if uncompute_key(&mut property.key, &mut property.computed, cx) {
            cx.changed();
        }
    }

    fn exit_method_definition(&mut self, method: &mut MethodDefinition<'a>, cx: &mut Cx<'a, '_>) {
        let context = KeyContext::ClassMember {
            is_static: method.r#static,
        };
        if keeps_computed_key(&method.key, context) {
            return;
        }
        if uncompute_key(&mut method.key, &mut method.computed, cx) {
            cx.changed();
        }
    }

    fn exit_property_definition(
        &mut self,
        property: &mut PropertyDefinition<'a>,
        cx: &mut Cx<'a, '_>,
    ) {
        let context = KeyContext::ClassMember {
            is_static: property.r#static,
        };
        if keeps_computed_key(&property.key, context) {
            return;
        }
        if uncompute_key(&mut property.key, &mut property.computed, cx) {
            cx.changed();
        }
    }

    fn exit_assignment_target(&mut self, target: &mut AssignmentTarget<'a>, cx: &mut Cx<'a, '_>) {
        if rewrite_computed_assignment_target(target, cx) {
            cx.changed();
        }
    }
}
