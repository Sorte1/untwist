use oxc_ast::ast::*;
use oxc_syntax::operator::AssignmentOperator;

use super::{has_no_params, is_named, is_number, is_string, param_name, returned};

const STATE_WORD: &str = "newState";
pub(super) const STATE_HEAD: &str = r"\w+ *\(\) *{\w+ *";
pub(super) const STATE_TAIL: &str = r#"['|"].+['|"];? *}"#;
const STATE_SLOTS: usize = 5;
const STATE_COUNTERS: [f64; 3] = [1.0, 0.0, 0.0];

fn this_assignment_value<'k, 'a>(expr: &'k Expression<'a>) -> Option<&'k Expression<'a>> {
    let Expression::AssignmentExpression(assign) = expr else {
        return None;
    };
    let on_this = match &assign.left {
        AssignmentTarget::StaticMemberExpression(member) => {
            matches!(member.object, Expression::ThisExpression(_))
        }
        AssignmentTarget::ComputedMemberExpression(member) => {
            matches!(member.object, Expression::ThisExpression(_))
        }
        _ => false,
    };
    (assign.operator == AssignmentOperator::Assign && on_this).then_some(&assign.right)
}

pub(super) fn state_target<'k, 'a>(
    function: &'k Function<'a>,
    statements: &'k [Statement<'a>],
) -> Option<&'k Function<'a>> {
    match statements {
        [Statement::ExpressionStatement(only)] => match &only.expression {
            Expression::SequenceExpression(sequence)
                if sequence.expressions.len() == STATE_SLOTS =>
            {
                state_values(function, sequence.expressions.iter())
            }
            _ => None,
        },
        five if five.len() == STATE_SLOTS => state_values(
            function,
            five.iter().filter_map(|statement| match statement {
                Statement::ExpressionStatement(each) => Some(&each.expression),
                _ => None,
            }),
        ),
        _ => None,
    }
}

fn is_counters(expr: &Expression<'_>) -> bool {
    let Expression::ArrayExpression(array) = expr else {
        return false;
    };
    array.elements.len() == STATE_COUNTERS.len()
        && array
            .elements
            .iter()
            .zip(STATE_COUNTERS)
            .all(|(element, value)| {
                element
                    .as_expression()
                    .is_some_and(|expr| is_number(expr, value))
            })
}

fn state_function<'k, 'a>(expr: &'k Expression<'a>) -> Option<&'k Function<'a>> {
    let Expression::FunctionExpression(function) = expr else {
        return None;
    };
    let body = function.body.as_ref()?;
    let returns_word = matches!(body.statements.as_slice(),
        [only] if returned(only).is_some_and(|value| is_string(value, STATE_WORD)));
    (has_no_params(function) && body.directives.is_empty() && returns_word).then_some(function)
}

fn state_values<'k, 'a>(
    function: &'k Function<'a>,
    expressions: impl Iterator<Item = &'k Expression<'a>>,
) -> Option<&'k Function<'a>> {
    let mut values = expressions.map(this_assignment_value);
    let mut next = || values.next().flatten();
    let param = param_name(function, 0)?;
    let (seed, counters, word) = (next()?, next()?, next()?);
    let (head, tail) = (next()?, next()?);
    let target = state_function(word)?;
    let shaped = is_named(seed, param)
        && is_counters(counters)
        && is_string(head, STATE_HEAD)
        && is_string(tail, STATE_TAIL);
    shaped.then_some(target)
}
