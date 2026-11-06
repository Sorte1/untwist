use oxc_ast::ast::*;
use oxc_semantic::Scoping;

const CODE_MODULES: &[&str] = &[
    "vm",
    "node:vm",
    "module",
    "node:module",
    "inspector",
    "node:inspector",
    "repl",
    "node:repl",
];

fn is_require(callee: &Expression<'_>, scoping: &Scoping) -> bool {
    match callee.without_parentheses() {
        Expression::Identifier(id) => {
            id.name == "require"
                && id
                    .reference_id
                    .get()
                    .is_none_or(|r| scoping.get_reference(r).symbol_id().is_none())
        }
        other => other
            .as_member_expression()
            .and_then(MemberExpression::static_property_name)
            .and_then(|name| name.as_str())
            .is_some_and(|name| name == "require"),
    }
}

pub(super) fn loads_code_module(call: &CallExpression<'_>, scoping: &Scoping) -> bool {
    if !is_require(&call.callee, scoping) {
        return false;
    }
    match call.arguments.first().and_then(Argument::as_expression) {
        Some(Expression::StringLiteral(name)) => CODE_MODULES.iter().any(|m| name.value == *m),
        _ => true,
    }
}
