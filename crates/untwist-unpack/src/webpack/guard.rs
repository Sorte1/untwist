use oxc_allocator::Allocator;
use oxc_ast::ast::*;
use rustc_hash::FxHashSet;

use crate::model::BundleKind;
use crate::runtime::{js_string, runtime_path, synthetic_statements};

pub const DIRNAME: &str = "__dirname";
const HOST_REQUIRE: &str = "require";

fn is_name(expr: &Expression<'_>, name: &str) -> bool {
    matches!(expr.without_parentheses(), Expression::Identifier(id) if id.name == name)
}

fn is_undefined(expr: &Expression<'_>) -> bool {
    match expr.without_parentheses() {
        Expression::Identifier(id) => id.name == "undefined",
        Expression::UnaryExpression(unary) => {
            unary.operator == UnaryOperator::Void
                && matches!(unary.argument, Expression::NumericLiteral(_))
        }
        _ => false,
    }
}

fn guard_pair(
    subject: &Expression<'_>,
    other: &Expression<'_>,
    require: &str,
    strict: bool,
) -> bool {
    if is_name(subject, require) {
        let null = matches!(other.without_parentheses(), Expression::NullLiteral(_));
        return is_undefined(other) || (!strict && null);
    }
    let Expression::UnaryExpression(unary) = subject.without_parentheses() else {
        return false;
    };
    unary.operator == UnaryOperator::Typeof
        && is_name(&unary.argument, require)
        && matches!(other.without_parentheses(), Expression::StringLiteral(text) if text.value == "undefined")
}

fn proven_guard(test: &Expression<'_>, require: &str) -> bool {
    match test.without_parentheses() {
        Expression::Identifier(id) => id.name == require,
        Expression::BinaryExpression(compare) => {
            let strict = match compare.operator {
                BinaryOperator::StrictInequality => true,
                BinaryOperator::Inequality => false,
                _ => return false,
            };
            guard_pair(&compare.left, &compare.right, require, strict)
                || guard_pair(&compare.right, &compare.left, require, strict)
        }
        _ => false,
    }
}

pub fn guarded<'k, 'a>(
    statement: &'k Statement<'a>,
    require: &str,
) -> Option<(&'k Expression<'a>, &'k AssignmentExpression<'a>)> {
    let Statement::IfStatement(branch) = statement else {
        return None;
    };
    if branch.alternate.is_some() || !proven_guard(&branch.test, require) {
        return None;
    }
    let inner = match &branch.consequent {
        Statement::BlockStatement(block) => match &block.body[..] {
            [only] => only,
            _ => return None,
        },
        other => other,
    };
    let Statement::ExpressionStatement(statement) = inner else {
        return None;
    };
    match &statement.expression {
        Expression::AssignmentExpression(assignment) => Some((&branch.test, &**assignment)),
        _ => None,
    }
}

pub fn dirname_suffix<'k>(assignment: &'k AssignmentExpression<'_>) -> Option<&'k str> {
    let Expression::BinaryExpression(sum) = &assignment.right else {
        return None;
    };
    let dirname = matches!(&sum.left, Expression::Identifier(id) if id.name == DIRNAME);
    match &sum.right {
        Expression::StringLiteral(suffix)
            if dirname && sum.operator == BinaryOperator::Addition =>
        {
            suffix.value.as_str()
        }
        _ => None,
    }
}

pub fn dirname_helper<'a>(
    alloc: &'a Allocator,
    require_name: &str,
    name: &str,
    suffix: &str,
) -> Option<Statement<'a>> {
    let depth = runtime_path(BundleKind::Webpack).matches('/').count();
    let root = (0..depth).fold(DIRNAME.to_string(), |inner, _| {
        format!("require(\"path\").dirname({inner})")
    });
    let text = format!("{}.{name} = {root} + {};", require_name, js_string(suffix));
    synthetic_statements(alloc, text)?.pop()
}

pub fn dirname_movable(require_name: &str, declared: &FxHashSet<&str>) -> bool {
    require_name != HOST_REQUIRE && !declared.contains(HOST_REQUIRE) && !declared.contains(DIRNAME)
}
