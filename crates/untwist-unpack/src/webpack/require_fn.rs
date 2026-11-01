use oxc_ast::ast::*;
use oxc_ast_visit::{walk_js, VisitJs};
use oxc_syntax::operator::AssignmentOperator;
use untwist_core::ast_util::prop::identifier_name;

use crate::matcher::{
    argument_expression, call_parts, const_member, expression_statement, literal_id,
    single_param_name,
};

pub struct RequireFunction<'k, 'a> {
    pub name: &'k str,
    pub body: &'k FunctionBody<'a>,
}

pub fn require_function<'k, 'a>(
    stmt: &'k Statement<'a>,
    container: &str,
) -> Option<RequireFunction<'k, 'a>> {
    let Statement::FunctionDeclaration(function) = stmt else {
        return None;
    };
    if function.generator || function.r#async {
        return None;
    }
    let name = function.id.as_ref()?.name.as_str();
    single_param_name(&function.params)?;
    let body = function.body.as_deref()?;
    let dispatches = body
        .statements
        .iter()
        .filter_map(expression_statement)
        .any(|expr| is_factory_dispatch(expr, container));
    if !dispatches {
        return None;
    }
    Some(RequireFunction { name, body })
}

fn is_factory_dispatch(expr: &Expression<'_>, container: &str) -> bool {
    let Some((callee, _)) = call_parts(expr) else {
        return false;
    };
    if is_container_index(callee, container) {
        return true;
    }
    const_member(callee, "call").is_some_and(|object| is_container_index(object, container))
}

fn is_container_index(expr: &Expression<'_>, container: &str) -> bool {
    let Expression::ComputedMemberExpression(member) = expr else {
        return false;
    };
    if member.optional || !matches!(member.expression, Expression::Identifier(_)) {
        return false;
    }
    identifier_name(&member.object) == Some(container)
}

struct EntryScan<'s> {
    require_name: &'s str,
    assigned: bool,
    found: Option<String>,
}

impl<'a> VisitJs<'a> for EntryScan<'_> {
    fn visit_assignment_expression(&mut self, it: &AssignmentExpression<'a>) {
        if self.assigned
            && self.found.is_none()
            && it.operator == AssignmentOperator::Assign
            && is_entry_slot(&it.left, self.require_name)
        {
            self.found = literal_id(&it.right);
        }
        walk_js::walk_assignment_expression(self, it);
    }

    fn visit_call_expression(&mut self, it: &CallExpression<'a>) {
        if !self.assigned
            && self.found.is_none()
            && it.arguments.len() == 1
            && identifier_name(&it.callee) == Some(self.require_name)
        {
            self.found = it
                .arguments
                .first()
                .and_then(argument_expression)
                .and_then(literal_id);
        }
        walk_js::walk_call_expression(self, it);
    }
}

pub const ENTRY_MARKER: &str = "s";

fn is_entry_slot(target: &AssignmentTarget<'_>, require_name: &str) -> bool {
    match target {
        AssignmentTarget::StaticMemberExpression(member) => {
            member.property.name == ENTRY_MARKER
                && identifier_name(&member.object) == Some(require_name)
        }
        AssignmentTarget::ComputedMemberExpression(member) => {
            matches!(&member.expression, Expression::StringLiteral(literal) if literal.value == ENTRY_MARKER)
                && identifier_name(&member.object) == Some(require_name)
        }
        _ => false,
    }
}

fn scan(
    body: &[Statement<'_>],
    require_name: &str,
    skip_index: Option<usize>,
    assigned: bool,
) -> Option<String> {
    let mut scan = EntryScan {
        require_name,
        assigned,
        found: None,
    };
    for (index, stmt) in body.iter().enumerate() {
        if Some(index) == skip_index {
            continue;
        }
        scan.visit_statement(stmt);
        if scan.found.is_some() {
            break;
        }
    }
    scan.found
}

pub fn find_assigned_entry_id(
    body: &[Statement<'_>],
    require_name: &str,
    skip_index: Option<usize>,
) -> Option<String> {
    scan(body, require_name, skip_index, true)
}

pub fn find_required_entry_id(
    body: &[Statement<'_>],
    require_name: &str,
    skip_index: Option<usize>,
) -> Option<String> {
    scan(body, require_name, skip_index, false)
}

pub fn count_required_entry_ids(body: &[Statement<'_>], require_name: &str) -> usize {
    body.iter()
        .filter(|stmt| {
            let mut scan = EntryScan {
                require_name,
                assigned: false,
                found: None,
            };
            scan.visit_statement(stmt);
            scan.found.is_some()
        })
        .count()
}
