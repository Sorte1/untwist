use oxc_ast::ast::Expression;
use untwist_eval::fold::{fold_layer_text, LayerText};

const KEY_FUEL: u64 = 1_000_000;

const KEY_HEAP: u64 = 1 << 24;

const CONSTRUCTOR: &str = "constructor";

const ARRAY_METHODS: [&str; 25] = [
    "concat",
    "entries",
    "every",
    "fill",
    "filter",
    "find",
    "flat",
    "forEach",
    "includes",
    "indexOf",
    "join",
    "keys",
    "map",
    "pop",
    "push",
    "reduce",
    "reverse",
    "shift",
    "slice",
    "some",
    "sort",
    "splice",
    "unshift",
    "values",
    "lastIndexOf",
];

pub fn is_function_constructor(expr: &Expression<'_>) -> bool {
    match expr.without_parentheses() {
        Expression::Identifier(id) => id.name == "Function",
        other => member_parts(other).is_some_and(|(object, key)| {
            key.as_deref() == Some(CONSTRUCTOR) && is_builtin_function(object)
        }),
    }
}

fn is_builtin_function(expr: &Expression<'_>) -> bool {
    let Some((object, Some(key))) = member_parts(expr) else {
        return false;
    };
    match object.without_parentheses() {
        Expression::ArrayExpression(array) if array.elements.is_empty() => {
            key == CONSTRUCTOR || ARRAY_METHODS.contains(&key.as_str())
        }
        Expression::ObjectExpression(object) if object.properties.is_empty() => key == CONSTRUCTOR,
        Expression::NumericLiteral(_)
        | Expression::StringLiteral(_)
        | Expression::BooleanLiteral(_) => key == CONSTRUCTOR,
        _ => false,
    }
}

fn member_parts<'p, 'a>(expr: &'p Expression<'a>) -> Option<(&'p Expression<'a>, Option<String>)> {
    match expr.without_parentheses() {
        Expression::StaticMemberExpression(member) if !member.optional => {
            Some((&member.object, Some(member.property.name.to_string())))
        }
        Expression::ComputedMemberExpression(member) if !member.optional => {
            Some((&member.object, key_text(&member.expression)))
        }
        _ => None,
    }
}

fn key_text(expr: &Expression<'_>) -> Option<String> {
    match fold_layer_text(expr, KEY_FUEL, KEY_HEAP).text {
        LayerText::Text(text) => Some(text),
        _ => None,
    }
}
