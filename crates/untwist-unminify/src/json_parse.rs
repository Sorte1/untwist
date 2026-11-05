use oxc_allocator::GetAllocator;
use oxc_ast::ast::*;
use oxc_parser::Parser;
use oxc_span::SourceType;
use untwist_core::depth::{
    limits_for, scan_as, Goal, Limits, C_P1, PRINT_GROWTH, PRINT_LENGTH_FACTOR,
};
use untwist_core::pass::{
    Cx, ExpressionKind, ExpressionVariants, InputRoom, Interest, Locality, Pass,
};
use untwist_eval::stack::room;

#[derive(Default)]
pub struct JsonParse;

pub fn boxed<'a>() -> Box<dyn Pass<'a> + 'a> {
    Box::new(JsonParse)
}

fn is_json_object(expr: &Expression<'_>) -> bool {
    matches!(expr, Expression::Identifier(ident) if ident.name.as_str() == "JSON")
}

fn is_json_parse_callee(callee: &Expression<'_>) -> bool {
    match callee {
        Expression::StaticMemberExpression(member) => {
            !member.optional
                && member.property.name.as_str() == "parse"
                && is_json_object(&member.object)
        }
        Expression::ComputedMemberExpression(member) => {
            !member.optional
                && matches!(&member.expression, Expression::StringLiteral(literal)
                    if literal.value.as_str() == "parse")
                && is_json_object(&member.object)
        }
        _ => false,
    }
}

#[inline(never)]
fn sets_a_prototype(text: &str, parsed: &Expression<'_>) -> bool {
    let may_spell = text.contains("__proto__") || text.contains("\\u");
    may_spell && has_proto_key(parsed)
}

fn has_proto_key(expr: &Expression<'_>) -> bool {
    match expr {
        Expression::ObjectExpression(object) => object.properties.iter().any(|property| {
            let ObjectPropertyKind::ObjectProperty(entry) = property else {
                return true;
            };
            let proto = matches!(&entry.key, PropertyKey::StringLiteral(key)
                if key.value.as_str() == "__proto__");
            proto || has_proto_key(&entry.value)
        }),
        Expression::ArrayExpression(array) => array
            .elements
            .iter()
            .any(|element| element.as_expression().is_none_or(has_proto_key)),
        _ => false,
    }
}

const ACTED_ON_EXPRESSIONS: ExpressionVariants =
    ExpressionVariants::of(&[ExpressionKind::CallExpression]);

impl<'a> Pass<'a> for JsonParse {
    fn name(&self) -> &'static str {
        "json-parse"
    }

    fn locality(&self) -> Locality {
        Locality::Neutral
    }

    fn interest(&self) -> Interest {
        Interest::EXIT_EXPRESSION.on_expressions(ACTED_ON_EXPRESSIONS)
    }

    fn exit_expression(&mut self, expr: &mut Expression<'a>, cx: &mut Cx<'a, '_>) {
        let text = {
            let Expression::CallExpression(call) = &*expr else {
                return;
            };
            if call.optional || call.arguments.len() != 1 {
                return;
            }
            if !is_json_parse_callee(&call.callee) {
                return;
            }
            let Some(Argument::StringLiteral(literal)) = call.arguments.first() else {
                return;
            };
            literal.value.as_str()
        };
        replace_with_literal(expr, text, cx);
    }
}

fn json_limits(input: Option<&InputRoom>, text: &str) -> Limits {
    let limits = limits_for(room(), text.len());
    #[cfg(target_arch = "wasm32")]
    let limits = Limits {
        nesting: untwist_core::depth::capped_above_the_input(limits.nesting, 1),
        ..limits
    };
    let Some(input) = input else {
        return limits;
    };
    Limits {
        nesting: limits.nesting.min(input.nesting().saturating_add(C_P1)),
        ..limits
    }
}

fn line_breaks(text: &str) -> u64 {
    let breaks = text
        .bytes()
        .filter(|byte| matches!(byte, b'{' | b'}' | b'[' | b']' | b','))
        .count();
    u64::try_from(breaks).unwrap_or(u64::MAX)
}

fn printed_bound(text: &str, site_depth: usize, nesting: u32) -> u64 {
    let len = u64::try_from(text.len()).unwrap_or(u64::MAX);
    let levels = u64::try_from(site_depth)
        .unwrap_or(u64::MAX)
        .saturating_add(u64::from(nesting))
        .saturating_add(1);
    let lines = line_breaks(text).saturating_add(1);
    PRINT_LENGTH_FACTOR
        .saturating_mul(len)
        .saturating_add(PRINT_GROWTH.saturating_mul(levels).saturating_mul(lines))
}

#[inline(never)]
fn replace_with_literal<'a>(expr: &mut Expression<'a>, text: &'a str, cx: &mut Cx<'a, '_>) {
    if !cx.is_global_name("JSON") {
        return;
    }
    if serde_json::from_str::<serde::de::IgnoredAny>(text).is_err() {
        return;
    }
    let input = cx.opts.room.as_ref();
    let Ok(found) = scan_as(text, &json_limits(input, text), Goal::Module) else {
        return;
    };
    let printed = printed_bound(text, cx.tcx.ancestors_depth(), found.nesting);
    if input.is_some_and(|input| printed > input.print()) {
        return;
    }
    let Ok(parsed) = Parser::new(cx.allocator(), text, SourceType::mjs()).parse_expression() else {
        return;
    };
    if sets_a_prototype(text, &parsed) {
        return;
    }
    if input.is_some_and(|input| !input.spend_print(printed)) {
        return;
    }
    *expr = parsed;
    cx.changed();
}
