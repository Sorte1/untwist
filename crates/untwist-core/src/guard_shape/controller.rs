use oxc_ast::ast::*;
use oxc_ast_visit::{walk_js, VisitJs};
use oxc_syntax::operator::{AssignmentOperator, UnaryOperator};
use oxc_syntax::scope::ScopeFlags;

use super::{
    has_no_params, is_named, is_null, is_reset, param_name, returned, single_var, statements_of,
    var_declaration,
};
use crate::ast_util::literal::{is_false_literal, is_true_literal};

const CALL_ARGUMENTS: &str = "arguments";
const BRANCH_DEPTH: u32 = 4;

pub(super) fn controller_target<'k, 'a>(
    statements: &'k [Statement<'a>],
) -> Option<&'k Function<'a>> {
    let [leading @ .., declaration, handoff] = statements else {
        return None;
    };
    let (flag, Some(init)) = single_var(declaration)? else {
        return None;
    };
    if !is_true_literal(init) {
        return None;
    }
    let Some(Expression::FunctionExpression(guarded)) = returned(handoff) else {
        return None;
    };
    if !leading.iter().all(is_leading_declaration) {
        return None;
    }
    guarded_target(guarded, Some(flag))
}

pub(super) fn handed_out_target<'k, 'a>(guarded: &'k Function<'a>) -> Option<&'k Function<'a>> {
    guarded_target(guarded, None)
}

pub(super) fn handed_out_targets<'k, 'a>(guarded: &'k Function<'a>) -> Vec<&'k Function<'a>> {
    guarded_targets(guarded, None)
}

fn is_leading_declaration(statement: &Statement<'_>) -> bool {
    var_declaration(statement).is_some() && !is_choice(statement)
}

fn is_choice(statement: &Statement<'_>) -> bool {
    matches!(
        single_var(statement),
        Some((_, Some(Expression::ConditionalExpression(_))))
    )
}

fn guarded_target<'k, 'a>(
    guarded: &'k Function<'a>,
    flag: Option<&str>,
) -> Option<&'k Function<'a>> {
    if super::narrowed() {
        return None;
    }
    guarded_targets(guarded, flag).into_iter().next()
}

fn guarded_targets<'k, 'a>(guarded: &'k Function<'a>, flag: Option<&str>) -> Vec<&'k Function<'a>> {
    let mut lists = Vec::new();
    if guarded.params.items.len() != 2 || guarded.params.rest.is_some() {
        return Vec::new();
    }
    let (Some(caller), Some(callee), Some(statements)) = (
        param_name(guarded, 0),
        param_name(guarded, 1),
        statements_of(guarded),
    ) else {
        return Vec::new();
    };
    template_lists(statements, BRANCH_DEPTH, &mut lists);
    lists
        .into_iter()
        .filter_map(|list| matched_target(list, flag, [caller, callee]))
        .collect()
}

fn matched_target<'k, 'a>(
    list: &'k [Statement<'a>],
    flag: Option<&str>,
    [caller, callee]: [&str; 2],
) -> Option<&'k Function<'a>> {
    let (first, rest) = list.split_first()?;
    let flag = match flag {
        Some(flag) => flag,
        None => choice_flag(first, [caller, callee])?,
    };
    let (result, target) = guarded_choice(first, flag)?;
    let closed = closes_skipping_declarations(rest, flag, result);
    (closed && runs_once(target, caller, callee)).then_some(target)
}

fn closes_skipping_declarations(rest: &[Statement<'_>], flag: &str, result: &str) -> bool {
    let mut live = rest
        .iter()
        .filter(|statement| !matches!(statement, Statement::FunctionDeclaration(_)));
    let (first, second) = (live.next(), live.next());
    if let (Some(Statement::ExpressionStatement(reset)), Some(last)) = (first, second) {
        if is_reset(&reset.expression, flag, is_false_flag)
            && returned(last).is_some_and(|expr| is_named(expr, result))
        {
            return true;
        }
    }
    let Some(Expression::SequenceExpression(sequence)) = first.and_then(returned) else {
        return false;
    };
    matches!(sequence.expressions.as_slice(),
        [reset, last] if is_reset(reset, flag, is_false_flag) && is_named(last, result))
}

fn template_lists<'k, 'a>(
    statements: &'k [Statement<'a>],
    depth: u32,
    lists: &mut Vec<&'k [Statement<'a>]>,
) {
    for (index, statement) in statements.iter().enumerate() {
        match statement {
            _ if is_choice(statement) => lists.extend(statements.get(index..)),
            Statement::BlockStatement(block) if depth > 0 => {
                template_lists(&block.body, depth - 1, lists);
            }
            Statement::IfStatement(branch) if depth > 0 => {
                branch_lists(&branch.consequent, depth - 1, lists);
                if let Some(alternate) = &branch.alternate {
                    branch_lists(alternate, depth - 1, lists);
                }
            }
            _ => {}
        }
    }
}

fn branch_lists<'k, 'a>(
    branch: &'k Statement<'a>,
    depth: u32,
    lists: &mut Vec<&'k [Statement<'a>]>,
) {
    match branch {
        Statement::BlockStatement(block) => template_lists(&block.body, depth, lists),
        Statement::IfStatement(_) => template_lists(std::slice::from_ref(branch), depth, lists),
        _ => {}
    }
}

fn is_false_flag(expr: &Expression<'_>) -> bool {
    if is_false_literal(expr) {
        return true;
    }
    matches!(expr.without_parentheses(), Expression::UnaryExpression(not)
        if not.operator == UnaryOperator::LogicalNot
            && matches!(&not.argument, Expression::NumericLiteral(n) if n.value != 0.0 && !n.value.is_nan()))
}

fn choice_flag<'k>(statement: &'k Statement<'_>, params: [&str; 2]) -> Option<&'k str> {
    let (_, Some(Expression::ConditionalExpression(choice))) = single_var(statement)? else {
        return None;
    };
    match &choice.test {
        Expression::Identifier(flag) if !params.contains(&flag.name.as_str()) => {
            Some(flag.name.as_str())
        }
        _ => None,
    }
}

pub(super) fn opens_with_a_choice(statements: &[Statement<'_>]) -> bool {
    let mut lists = Vec::new();
    template_lists(statements, BRANCH_DEPTH, &mut lists);
    lists.iter().any(|list| {
        matches!(list.first().and_then(single_var), Some((_, Some(Expression::ConditionalExpression(choice))))
            if matches!(choice.consequent, Expression::FunctionExpression(_)))
    })
}

fn is_empty_function(expr: &Expression<'_>) -> bool {
    matches!(expr, Expression::FunctionExpression(function)
        if has_no_params(function)
            && function.body.as_ref().is_some_and(|body| body.statements.is_empty() && body.directives.is_empty()))
}

fn guarded_choice<'k, 'a>(
    statement: &'k Statement<'a>,
    flag: &str,
) -> Option<(&'k str, &'k Function<'a>)> {
    let (result, Some(Expression::ConditionalExpression(choice))) = single_var(statement)? else {
        return None;
    };
    let Expression::FunctionExpression(target) = &choice.consequent else {
        return None;
    };
    let shaped = is_named(&choice.test, flag)
        && is_empty_function(&choice.alternate)
        && has_no_params(target);
    shaped.then_some((result, target))
}

struct Forwarding<'n> {
    caller: &'n str,
    callee: &'n str,
    forwards: bool,
    resets: bool,
}

impl<'a> VisitJs<'a> for Forwarding<'_> {
    fn visit_function(&mut self, _: &Function<'a>, _: ScopeFlags) {}

    fn visit_call_expression(&mut self, it: &CallExpression<'a>) {
        self.forwards |= is_forwarding_call(it, self.caller, self.callee);
        walk_js::walk_call_expression(self, it);
    }

    fn visit_assignment_expression(&mut self, it: &AssignmentExpression<'a>) {
        self.resets |= it.operator == AssignmentOperator::Assign
            && matches!(&it.left, AssignmentTarget::AssignmentTargetIdentifier(id) if id.name == self.callee)
            && is_null(&it.right);
        walk_js::walk_assignment_expression(self, it);
    }
}

fn runs_once(target: &Function<'_>, caller: &str, callee: &str) -> bool {
    let Some(body) = &target.body else {
        return false;
    };
    let mut found = Forwarding {
        caller,
        callee,
        forwards: false,
        resets: false,
    };
    found.visit_function_body(body);
    found.forwards && found.resets
}

fn is_forwarding_call(call: &CallExpression<'_>, caller: &str, callee: &str) -> bool {
    let forwards = matches!(call.arguments.as_slice(),
        [Argument::Identifier(this), Argument::Identifier(rest)]
            if this.name == caller && rest.name == CALL_ARGUMENTS);
    !call.optional
        && forwards
        && call
            .callee
            .as_member_expression()
            .is_some_and(|member| !member.optional() && is_named(member.object(), callee))
}
