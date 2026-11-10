use oxc_ast::ast::*;
use oxc_syntax::operator::{AssignmentOperator, BinaryOperator, LogicalOperator, UnaryOperator};

use crate::amd::factory::{
    callback, is_relative, pair_up, Callback, Dependency, Returns, EXPORTS, MODULE, REQUIRE,
};
use crate::frame::{Frame, Shape};
use crate::matcher::{anonymous_factory, argument_expression};

const RESERVED_PARAMS: [&str; 4] = ["define", "exports", "module", "require"];

pub struct Umd<'k, 'a> {
    pub callback: Callback<'k, 'a>,
    pub pairs: Vec<Dependency>,
    pub returns: Returns,
}

fn typeof_of(expr: &Expression<'_>) -> Option<&'static str> {
    match expr.without_parentheses() {
        Expression::Identifier(id) => match id.name.as_str() {
            "define" => Some("undefined"),
            "exports" | "module" => Some("object"),
            _ => None,
        },
        Expression::StaticMemberExpression(member) => {
            let named =
                matches!(&member.object, Expression::Identifier(id) if id.name == "exports");
            (named && member.property.name == "nodeName").then_some("undefined")
        }
        _ => None,
    }
}

fn compared(binary: &BinaryExpression<'_>) -> Option<bool> {
    let equal = match binary.operator {
        BinaryOperator::StrictEquality | BinaryOperator::Equality => true,
        BinaryOperator::StrictInequality | BinaryOperator::Inequality => false,
        _ => return None,
    };
    let (probe, literal) = match (&binary.left, &binary.right) {
        (Expression::UnaryExpression(unary), Expression::StringLiteral(literal))
        | (Expression::StringLiteral(literal), Expression::UnaryExpression(unary)) => {
            (unary, literal)
        }
        _ => return None,
    };
    if probe.operator != UnaryOperator::Typeof {
        return None;
    }
    Some((literal.value == typeof_of(&probe.argument)?) == equal)
}

fn truth(expr: &Expression<'_>) -> Option<bool> {
    match expr.without_parentheses() {
        Expression::BooleanLiteral(literal) => Some(literal.value),
        Expression::UnaryExpression(unary) if unary.operator == UnaryOperator::LogicalNot => {
            truth(&unary.argument).map(|value| !value)
        }
        Expression::BinaryExpression(binary) => compared(binary),
        Expression::LogicalExpression(logical) => match logical.operator {
            LogicalOperator::And if !truth(&logical.left)? => Some(false),
            LogicalOperator::Or if truth(&logical.left)? => Some(true),
            LogicalOperator::And | LogicalOperator::Or => truth(&logical.right),
            LogicalOperator::Coalesce => None,
        },
        Expression::Identifier(id) => match id.name.as_str() {
            "exports" | "module" => Some(true),
            _ => None,
        },
        Expression::StaticMemberExpression(member) => {
            let module =
                matches!(&member.object, Expression::Identifier(id) if id.name == "module");
            (module && member.property.name == "exports").then_some(true)
        }
        _ => None,
    }
}

fn reach_expression<'k, 'a>(expr: &'k Expression<'a>) -> Option<&'k Expression<'a>> {
    match expr.without_parentheses() {
        Expression::ConditionalExpression(conditional) => {
            let branch = if truth(&conditional.test)? {
                &conditional.consequent
            } else {
                &conditional.alternate
            };
            reach_expression(branch)
        }
        Expression::LogicalExpression(logical) if logical.operator == LogicalOperator::And => {
            truth(&logical.left)?
                .then_some(&logical.right)
                .and_then(reach_expression)
        }
        Expression::LogicalExpression(logical) if logical.operator == LogicalOperator::Or => {
            (!truth(&logical.left)?)
                .then_some(&logical.right)
                .and_then(reach_expression)
        }
        other => Some(other),
    }
}

fn reach<'k, 'a>(statement: &'k Statement<'a>) -> Option<&'k Expression<'a>> {
    match statement {
        Statement::IfStatement(branch) if truth(&branch.test)? => reach(&branch.consequent),
        Statement::IfStatement(branch) => reach(branch.alternate.as_ref()?),
        Statement::BlockStatement(block) if block.body.len() == 1 => reach(&block.body[0]),
        Statement::ExpressionStatement(statement) => reach_expression(&statement.expression),
        _ => None,
    }
}

fn is_module_exports(target: &AssignmentTarget<'_>) -> bool {
    let AssignmentTarget::StaticMemberExpression(member) = target else {
        return false;
    };
    matches!(&member.object, Expression::Identifier(id) if id.name == "module")
        && member.property.name == "exports"
}

fn invocation<'k, 'a>(
    terminal: &'k Expression<'a>,
    factory: &str,
) -> Option<(Returns, &'k CallExpression<'a>)> {
    let (returns, call) = match terminal {
        Expression::AssignmentExpression(assign)
            if assign.operator == AssignmentOperator::Assign && is_module_exports(&assign.left) =>
        {
            (Returns::Assigned, &assign.right)
        }
        other => (Returns::Dropped, other),
    };
    let Expression::CallExpression(call) = call.without_parentheses() else {
        return None;
    };
    let Expression::Identifier(callee) = call.callee.without_parentheses() else {
        return None;
    };
    (callee.name == factory).then_some((returns, call))
}

fn dependency_of(arg: &Expression<'_>) -> Option<String> {
    match arg.without_parentheses() {
        Expression::Identifier(id) => match id.name.as_str() {
            "exports" => Some(EXPORTS.to_string()),
            "module" => Some(MODULE.to_string()),
            "require" => Some(REQUIRE.to_string()),
            _ => None,
        },
        Expression::CallExpression(call) => {
            let Expression::Identifier(callee) = call.callee.without_parentheses() else {
                return None;
            };
            let [Argument::StringLiteral(literal)] = call.arguments.as_slice() else {
                return None;
            };
            let specifier = literal.value.as_str()?;
            let usable = callee.name == "require"
                && !call.optional
                && !specifier.is_empty()
                && !is_relative(specifier)
                && !specifier.starts_with('/');
            usable.then(|| specifier.to_string())
        }
        _ => None,
    }
}

fn defined_name<'k>(test: &'k Expression<'_>) -> Option<&'k str> {
    let Expression::BinaryExpression(binary) = test.without_parentheses() else {
        return None;
    };
    if !matches!(
        binary.operator,
        BinaryOperator::StrictInequality | BinaryOperator::Inequality
    ) {
        return None;
    }
    let (probe, literal) = match (&binary.left, &binary.right) {
        (Expression::UnaryExpression(unary), other)
        | (other, Expression::UnaryExpression(unary)) => (unary, other),
        _ => return None,
    };
    let undefined = matches!(literal.without_parentheses(), Expression::StringLiteral(text) if text.value == "undefined");
    match probe.argument.without_parentheses() {
        Expression::Identifier(id) if undefined && probe.operator == UnaryOperator::Typeof => {
            Some(id.name.as_str())
        }
        _ => None,
    }
}

fn inert_root(expr: &Expression<'_>) -> bool {
    match expr.without_parentheses() {
        Expression::ThisExpression(_)
        | Expression::NullLiteral(_)
        | Expression::BooleanLiteral(_)
        | Expression::NumericLiteral(_)
        | Expression::StringLiteral(_) => true,
        Expression::ConditionalExpression(choice) => {
            let Some(name) = defined_name(&choice.test) else {
                return false;
            };
            let guarded = matches!(choice.consequent.without_parentheses(),
                Expression::Identifier(id) if id.name == name);
            (guarded || inert_root(&choice.consequent)) && inert_root(&choice.alternate)
        }
        _ => false,
    }
}

pub fn parts<'k, 'a>(frame: &Frame<'k, 'a>) -> Option<Umd<'k, 'a>> {
    let wrapper = frame.wrapper.as_ref()?;
    if wrapper.shape != Shape::Umd || frame.top.len() != 1 || wrapper.index != 0 {
        return None;
    }
    if !matches!(frame.top[0], Statement::ExpressionStatement(_)) {
        return None;
    }
    let mut functions = wrapper
        .arguments
        .iter()
        .enumerate()
        .filter(|(_, argument)| {
            argument
                .as_expression()
                .and_then(anonymous_factory)
                .is_some()
        });
    let (position, argument) = functions.next()?;
    if functions.next().is_some() {
        return None;
    }
    let roots_inert = wrapper
        .arguments
        .iter()
        .enumerate()
        .filter(|(index, _)| *index != position)
        .all(|(_, argument)| argument_expression(argument).is_some_and(inert_root));
    if !roots_inert {
        return None;
    }
    let callback = callback(argument.as_expression()?)?;
    let params = wrapper.function.params;
    if params.rest.is_some() || params.items.len() < wrapper.arguments.len().min(position + 1) {
        return None;
    }
    let names = crate::matcher::param_names(params);
    if names.len() != params.items.len() || names.iter().any(|name| RESERVED_PARAMS.contains(name))
    {
        return None;
    }
    let factory = *names.get(position)?;
    let [only] = wrapper.body() else {
        return None;
    };
    let (returns, call) = invocation(reach(only)?, factory)?;
    let dependencies: Vec<String> = call
        .arguments
        .iter()
        .map(|argument| dependency_of(argument_expression(argument)?))
        .collect::<Option<_>>()?;
    let own = callback.names()?;
    Some(Umd {
        pairs: pair_up(&own, dependencies),
        callback,
        returns,
    })
}
