use oxc_ast::ast::*;
use oxc_ast_visit::{walk_js, VisitJs};
use oxc_syntax::operator::AssignmentOperator;
use rustc_hash::FxHashMap;
use untwist_core::ast_util::prop::identifier_name;

use crate::matcher::{
    argument_expression, call_parts, const_member, expression_statement, literal_id,
    single_param_name,
};

pub struct RequireFunction<'k, 'a> {
    pub name: &'k str,
    pub body: &'k FunctionBody<'a>,
}

fn dispatcher<'k, 'a>(stmt: &'k Statement<'a>) -> Option<RequireFunction<'k, 'a>> {
    let Statement::FunctionDeclaration(function) = stmt else {
        return None;
    };
    if function.generator || function.r#async {
        return None;
    }
    let name = function.id.as_ref()?.name.as_str();
    single_param_name(&function.params)?;
    let body = function.body.as_deref()?;
    Some(RequireFunction { name, body })
}

fn dispatched<'k, 'a>(body: &'k FunctionBody<'a>) -> impl Iterator<Item = &'k str> + use<'k, 'a> {
    body.statements
        .iter()
        .filter_map(expression_statement)
        .filter_map(dispatched_container)
}

fn dispatched_container<'k>(expr: &'k Expression<'_>) -> Option<&'k str> {
    let (callee, _) = call_parts(expr)?;
    container_index(callee).or_else(|| const_member(callee, "call").and_then(container_index))
}

fn container_index<'k>(expr: &'k Expression<'_>) -> Option<&'k str> {
    let Expression::ComputedMemberExpression(member) = expr else {
        return None;
    };
    if member.optional || !matches!(member.expression, Expression::Identifier(_)) {
        return None;
    }
    identifier_name(&member.object)
}

pub fn require_function<'k, 'a>(
    stmt: &'k Statement<'a>,
    container: &str,
) -> Option<RequireFunction<'k, 'a>> {
    let function = dispatcher(stmt)?;
    dispatched(function.body)
        .any(|name| name == container)
        .then_some(function)
}

pub struct Dispatchers<'k> {
    by_container: FxHashMap<&'k str, Vec<(usize, &'k str)>>,
}

impl<'k> Dispatchers<'k> {
    pub fn of(body: &'k [Statement<'_>]) -> Self {
        let mut by_container: FxHashMap<&'k str, Vec<(usize, &'k str)>> = FxHashMap::default();
        for (index, stmt) in body.iter().enumerate() {
            let Some(function) = dispatcher(stmt) else {
                continue;
            };
            for container in dispatched(function.body) {
                let found = by_container.entry(container).or_default();
                if found.last().map(|(at, _)| *at) != Some(index) {
                    found.push((index, function.name));
                }
            }
        }
        Self { by_container }
    }

    pub fn is_empty(&self) -> bool {
        self.by_container.is_empty()
    }

    pub fn first_after(&self, container: &str, index: usize) -> Option<&'k str> {
        let found = self.by_container.get(container)?;
        found
            .get(found.partition_point(|(at, _)| *at <= index))
            .map(|(_, name)| *name)
    }
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
