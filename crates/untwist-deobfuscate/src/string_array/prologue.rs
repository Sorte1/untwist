use oxc_ast::ast::*;
use oxc_ast_visit::{walk_mut, VisitMut};
use oxc_syntax::operator::{AssignmentOperator, BinaryOperator, LogicalOperator, UnaryOperator};
use oxc_syntax::scope::ScopeFlags;
use untwist_core::ast_util::matcher::{call_parts, iife};
use untwist_core::ast_util::prop::{identifier_name, member_parts};

const GRAB_TEXTS: [&str; 2] = [
    "return (function() {}.constructor(\"return this\")( ));",
    "return this",
];

const POLYFILLED: &str = "atob";

fn literal_text(expr: &Expression<'_>, into: &mut String) -> bool {
    match expr.without_parentheses() {
        Expression::StringLiteral(literal) => match literal.value.as_str() {
            Some(text) => {
                into.push_str(text);
                true
            }
            None => false,
        },
        Expression::BinaryExpression(binary) if binary.operator == BinaryOperator::Addition => {
            literal_text(&binary.left, into) && literal_text(&binary.right, into)
        }
        _ => false,
    }
}

fn is_grab_text(arguments: &[Argument<'_>]) -> bool {
    let [argument] = arguments else {
        return false;
    };
    let mut text = String::new();
    argument
        .as_expression()
        .is_some_and(|expr| literal_text(expr, &mut text))
        && GRAB_TEXTS.contains(&text.as_str())
}

fn assigns<'k, 'a>(stmt: &'k Statement<'a>, name: &str) -> Option<&'k Expression<'a>> {
    let Statement::ExpressionStatement(stmt) = stmt else {
        return None;
    };
    let Expression::AssignmentExpression(assignment) = stmt.expression.without_parentheses() else {
        return None;
    };
    let AssignmentTarget::AssignmentTargetIdentifier(target) = &assignment.left else {
        return None;
    };
    (assignment.operator == AssignmentOperator::Assign && target.name == name)
        .then_some(&assignment.right)
}

fn makes_grab_function(expr: &Expression<'_>) -> bool {
    call_parts(expr.without_parentheses()).is_some_and(|(inner, text)| {
        identifier_name(inner) == Some("Function") && is_grab_text(text)
    })
}

fn calls_code_text(expr: &Expression<'_>) -> bool {
    let Some((callee, arguments)) = call_parts(expr.without_parentheses()) else {
        return false;
    };
    arguments.is_empty() && makes_grab_function(callee)
}

fn calls_named(expr: &Expression<'_>, name: &str) -> bool {
    call_parts(expr.without_parentheses()).is_some_and(|(callee, arguments)| {
        arguments.is_empty() && identifier_name(callee) == Some(name)
    })
}

fn grabs_in_block(block: &BlockStatement<'_>, name: &str) -> bool {
    match block.body.as_slice() {
        [stmt] => assigns(stmt, name).is_some_and(calls_code_text),
        [Statement::VariableDeclaration(declaration), stmt] => {
            let [declarator] = declaration.declarations.as_slice() else {
                return false;
            };
            let BindingPattern::BindingIdentifier(maker) = &declarator.id else {
                return false;
            };
            let maker = maker.name.as_str();
            maker != name
                && declarator.init.as_ref().is_some_and(makes_grab_function)
                && assigns(stmt, name).is_some_and(|value| calls_named(value, maker))
        }
        _ => false,
    }
}

fn grabbing_try(attempt: &TryStatement<'_>, name: &str) -> bool {
    let Some(handler) = attempt.handler.as_ref() else {
        return false;
    };
    let caught = single(&handler.body)
        .and_then(|stmt| assigns(stmt, name))
        .and_then(identifier_name)
        == Some("window");
    attempt.finalizer.is_none() && caught && grabs_in_block(&attempt.block, name)
}

fn single<'k, 'a>(block: &'k BlockStatement<'a>) -> Option<&'k Statement<'a>> {
    match block.body.as_slice() {
        [stmt] => Some(stmt),
        _ => None,
    }
}

fn is_grab_function(function: &Function<'_>) -> bool {
    if function.id.is_some() || !function.params.items.is_empty() || function.params.rest.is_some()
    {
        return false;
    }
    let Some(body) = function.body.as_ref() else {
        return false;
    };
    let [Statement::VariableDeclaration(declaration), Statement::TryStatement(attempt), Statement::ReturnStatement(ret)] =
        body.statements.as_slice()
    else {
        return false;
    };
    let [declarator] = declaration.declarations.as_slice() else {
        return false;
    };
    let BindingPattern::BindingIdentifier(id) = &declarator.id else {
        return false;
    };
    let name = id.name.as_str();
    declarator.init.is_none()
        && grabbing_try(attempt, name)
        && ret.argument.as_ref().and_then(identifier_name) == Some(name)
}

fn grab_call(expr: &Expression<'_>, grabs: &[&str]) -> bool {
    if let Some((function, arguments)) = iife(expr.without_parentheses()) {
        return arguments.is_empty() && is_grab_function(function);
    }
    call_parts(expr.without_parentheses()).is_some_and(|(callee, arguments)| {
        arguments.is_empty() && identifier_name(callee).is_some_and(|name| grabs.contains(&name))
    })
}

fn polyfills(stmt: &Statement<'_>, holders: &[&str]) -> bool {
    let Statement::ExpressionStatement(stmt) = stmt else {
        return false;
    };
    let Expression::LogicalExpression(logical) = stmt.expression.without_parentheses() else {
        return false;
    };
    let Some((holder, key)) = member_parts(logical.left.without_parentheses()) else {
        return false;
    };
    let Expression::AssignmentExpression(assignment) = logical.right.without_parentheses() else {
        return false;
    };
    let Some(target) = assignment.left.as_simple_assignment_target() else {
        return false;
    };
    let Some(member) = target.as_member_expression() else {
        return false;
    };
    let written = match member {
        MemberExpression::StaticMemberExpression(node) => {
            Some((&node.object, node.property.name.as_str() == POLYFILLED))
        }
        MemberExpression::ComputedMemberExpression(node) => Some((
            &node.object,
            matches!(&node.expression, Expression::StringLiteral(key) if key.value == POLYFILLED),
        )),
        _ => None,
    };
    let Some((written_holder, same_key)) = written else {
        return false;
    };
    let holder = identifier_name(holder);
    logical.operator == LogicalOperator::Or
        && key == POLYFILLED
        && same_key
        && assignment.operator == AssignmentOperator::Assign
        && holder.is_some_and(|name| holders.contains(&name))
        && identifier_name(written_holder) == holder
        && matches!(assignment.right, Expression::FunctionExpression(_))
}

fn is_prologue_body(statements: &[Statement<'_>]) -> bool {
    let Some((last, declarations)) = statements.split_last() else {
        return false;
    };
    let mut grabs: Vec<&str> = Vec::new();
    let mut holders: Vec<&str> = Vec::new();
    let mut bare: Vec<&str> = Vec::new();
    for stmt in declarations {
        if let Statement::TryStatement(attempt) = stmt {
            let Some(name) = bare
                .iter()
                .copied()
                .find(|name| grabbing_try(attempt, name))
            else {
                return false;
            };
            holders.push(name);
            continue;
        }
        let Statement::VariableDeclaration(declaration) = stmt else {
            return false;
        };
        let [declarator] = declaration.declarations.as_slice() else {
            return false;
        };
        let BindingPattern::BindingIdentifier(id) = &declarator.id else {
            return false;
        };
        match declarator
            .init
            .as_ref()
            .map(Expression::without_parentheses)
        {
            None => bare.push(id.name.as_str()),
            Some(Expression::FunctionExpression(function)) if is_grab_function(function) => {
                grabs.push(id.name.as_str());
            }
            Some(init) if grab_call(init, &grabs) => holders.push(id.name.as_str()),
            Some(Expression::StringLiteral(_)) => {}
            _ => return false,
        }
    }
    polyfills(last, &holders)
}

pub(super) fn is_atob_prologue(expr: &Expression<'_>) -> bool {
    let call = match expr.without_parentheses() {
        Expression::UnaryExpression(unary) if unary.operator == UnaryOperator::LogicalNot => {
            unary.argument.without_parentheses()
        }
        other => other,
    };
    let Some((function, arguments)) = iife(call) else {
        return false;
    };
    let Some(body) = function.body.as_ref() else {
        return false;
    };
    arguments.is_empty()
        && function.params.items.is_empty()
        && function.params.rest.is_none()
        && body.directives.is_empty()
        && is_prologue_body(&body.statements)
}

struct Strip {
    depth: usize,
    removed: usize,
}

impl<'a> VisitMut<'a> for Strip {
    fn visit_function(&mut self, it: &mut Function<'a>, flags: ScopeFlags) {
        self.depth += 1;
        if self.depth == 1 {
            walk_mut::walk_function(self, it, flags);
        }
        self.depth -= 1;
    }

    fn visit_statements(&mut self, it: &mut oxc_allocator::Vec<'a, Statement<'a>>) {
        let before = it.len();
        it.retain(|stmt| {
            !matches!(stmt, Statement::ExpressionStatement(node) if is_atob_prologue(&node.expression))
        });
        self.removed += before - it.len();
        walk_mut::walk_statements(self, it);
    }

    fn visit_sequence_expression(&mut self, it: &mut SequenceExpression<'a>) {
        let before = it.expressions.len();
        if it.expressions.iter().any(|expr| !is_atob_prologue(expr)) {
            it.expressions.retain(|expr| !is_atob_prologue(expr));
        }
        self.removed += before - it.expressions.len();
        walk_mut::walk_sequence_expression(self, it);
    }
}

pub(super) fn strip_atob_prologues(decoder: &mut Statement<'_>) -> usize {
    let mut strip = Strip {
        depth: 0,
        removed: 0,
    };
    strip.visit_statement(decoder);
    strip.removed
}
