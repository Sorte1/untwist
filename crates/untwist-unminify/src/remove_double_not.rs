use oxc_allocator::ReplaceWith;
use oxc_ast::ast::*;
use untwist_core::pass::{
    Cx, ExpressionKind, ExpressionVariants, Interest, Locality, Pass, StatementKind,
    StatementVariants,
};
use untwist_core::semantic::builtins::{Reliance, Unpatched};

const ARRAY_METHODS: [&str; 7] = [
    "filter",
    "find",
    "findLast",
    "findIndex",
    "findLastIndex",
    "some",
    "every",
];

static RELIED_ON: Reliance = Reliance {
    roots: &["Array"],
    members: &[
        ("Array", false, &["prototype"]),
        ("Array", true, &ARRAY_METHODS),
    ],
};

pub struct RemoveDoubleNot {
    unpatched: Unpatched,
}

impl Default for RemoveDoubleNot {
    fn default() -> Self {
        Self {
            unpatched: Unpatched::new(&RELIED_ON),
        }
    }
}

fn not_argument<'e, 'a>(expr: &'e Expression<'a>) -> Option<&'e Expression<'a>> {
    let Expression::UnaryExpression(unary) = expr else {
        return None;
    };
    if unary.operator != UnaryOperator::LogicalNot {
        return None;
    }
    Some(&unary.argument)
}

fn is_double_not(expr: &Expression<'_>) -> bool {
    let Some(inner) = not_argument(expr) else {
        return false;
    };
    not_argument(inner).is_some()
}

pub fn would_rewrite(conditional: &ConditionalExpression<'_>, _cx: &Cx<'_, '_>) -> bool {
    is_double_not(&conditional.test)
}

fn strip_double_not(expr: &mut Expression<'_>) -> bool {
    if !is_double_not(expr) {
        return false;
    }
    let mut stripped = false;
    expr.replace_with(|old| {
        let outer = match old {
            Expression::UnaryExpression(outer) => outer,
            other => return other,
        };
        if !matches!(outer.argument, Expression::UnaryExpression(_)) {
            return Expression::UnaryExpression(outer);
        }
        let Expression::UnaryExpression(inner) = outer.unbox().argument else {
            unreachable!()
        };
        stripped = true;
        inner.unbox().argument
    });
    stripped
}

fn is_array_predicate_call(call: &CallExpression<'_>) -> bool {
    if call.arguments.len() != 1 {
        return false;
    }
    match &call.callee {
        Expression::StaticMemberExpression(member) => {
            matches!(member.object, Expression::ArrayExpression(_))
                && ARRAY_METHODS.contains(&member.property.name.as_str())
        }
        Expression::ComputedMemberExpression(member) => {
            let Expression::StringLiteral(property) = &member.expression else {
                return false;
            };
            matches!(member.object, Expression::ArrayExpression(_))
                && property
                    .value
                    .as_str()
                    .is_some_and(|name| ARRAY_METHODS.contains(&name))
        }
        _ => false,
    }
}

const ACTED_ON_EXPRESSIONS: ExpressionVariants = ExpressionVariants::of(&[
    ExpressionKind::ConditionalExpression,
    ExpressionKind::UnaryExpression,
    ExpressionKind::CallExpression,
]);

const ACTED_ON_STATEMENTS: StatementVariants = StatementVariants::of(&[
    StatementKind::IfStatement,
    StatementKind::WhileStatement,
    StatementKind::DoWhileStatement,
    StatementKind::ForStatement,
]);

impl<'a> Pass<'a> for RemoveDoubleNot {
    fn name(&self) -> &'static str {
        "remove-double-not"
    }

    fn locality(&self) -> Locality {
        Locality::Neutral
    }

    fn interest(&self) -> Interest {
        (Interest::ENTER_PROGRAM | Interest::EXIT_STATEMENT | Interest::EXIT_EXPRESSION)
            .on_expressions(ACTED_ON_EXPRESSIONS)
            .on_statements(ACTED_ON_STATEMENTS)
    }

    fn enter_program(&mut self, program: &mut Program<'a>, cx: &mut Cx<'a, '_>) {
        self.unpatched.learn(program, cx);
    }

    fn exit_statement(&mut self, stmt: &mut Statement<'a>, cx: &mut Cx<'a, '_>) {
        let test = match stmt {
            Statement::IfStatement(node) => &mut node.test,
            Statement::WhileStatement(node) => &mut node.test,
            Statement::DoWhileStatement(node) => &mut node.test,
            Statement::ForStatement(node) => match node.test.as_mut() {
                Some(test) => test,
                None => return,
            },
            _ => return,
        };
        if strip_double_not(test) {
            cx.changed();
        }
    }

    fn exit_expression(&mut self, expr: &mut Expression<'a>, cx: &mut Cx<'a, '_>) {
        match expr {
            Expression::ConditionalExpression(conditional) => {
                if strip_double_not(&mut conditional.test) {
                    cx.changed();
                }
            }
            Expression::UnaryExpression(unary) => {
                if unary.operator == UnaryOperator::LogicalNot
                    && strip_double_not(&mut unary.argument)
                {
                    cx.changed();
                }
            }
            Expression::CallExpression(call) => {
                if !is_array_predicate_call(call) {
                    return;
                }
                let Some(Argument::ArrowFunctionExpression(arrow)) = call.arguments.first_mut()
                else {
                    return;
                };
                let Some(body) = arrow.body.as_expression_mut() else {
                    return;
                };
                if !is_double_not(body) || !self.unpatched.holds(cx) {
                    return;
                }
                if strip_double_not(body) {
                    cx.changed();
                }
            }
            _ => {}
        }
    }
}

pub fn boxed<'a>() -> Box<dyn Pass<'a> + 'a> {
    Box::new(RemoveDoubleNot::default())
}
