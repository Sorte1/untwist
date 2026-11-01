use oxc_ast::ast::*;
use oxc_ast_visit::VisitJs;
use oxc_semantic::{ReferenceId, Scoping};
use oxc_syntax::scope::{ScopeFlags, ScopeId};
use untwist_core::pass::Cx;

struct Guarded<'a> {
    names: Vec<&'a IdentifierReference<'a>>,
    value: bool,
}

#[derive(Default)]
struct LeadingScan<'a> {
    written: Vec<&'a str>,
    writes_member: bool,
    runs_code: bool,
}

#[inline(never)]
pub fn is_hoist_order_safe<'a>(assign: &AssignmentExpression<'a>, cx: &Cx<'a, '_>) -> bool {
    let compound = assign.operator != AssignmentOperator::Assign;
    let Some(guarded) = guarded_parts(&assign.left, compound) else {
        return false;
    };
    if guarded.names.is_empty() && !guarded.value {
        return true;
    }
    let Expression::SequenceExpression(seq) = &assign.right else {
        return true;
    };
    let mut scan = LeadingScan::default();
    let leading = seq.expressions.len().saturating_sub(1);
    for part in seq.expressions.iter().take(leading) {
        scan.visit_expression(part);
    }
    if guarded.value && (scan.writes_member || scan.runs_code) {
        return false;
    }
    guarded.names.iter().all(|name| {
        !scan.written.contains(&name.name.as_str())
            && (!scan.runs_code || is_uncaptured_local(name, cx))
    })
}

fn guarded_parts<'a>(target: &'a AssignmentTarget<'a>, compound: bool) -> Option<Guarded<'a>> {
    let mut names = Vec::new();
    let value = match target {
        AssignmentTarget::AssignmentTargetIdentifier(id) => {
            if compound {
                names.push(&**id);
            }
            false
        }
        AssignmentTarget::StaticMemberExpression(member) => {
            names.push(identifier(&member.object)?);
            compound
        }
        AssignmentTarget::ComputedMemberExpression(member) => {
            names.push(identifier(&member.object)?);
            if let Expression::Identifier(key) = &member.expression {
                names.push(&**key);
            }
            compound
        }
        _ => return None,
    };
    Some(Guarded { names, value })
}

fn identifier<'a>(expr: &'a Expression<'a>) -> Option<&'a IdentifierReference<'a>> {
    match expr {
        Expression::Identifier(id) => Some(&**id),
        _ => None,
    }
}

fn is_uncaptured_local(name: &IdentifierReference<'_>, cx: &Cx<'_, '_>) -> bool {
    let scoping = cx.scoping();
    let Some(reference) = name.reference_id.get() else {
        return false;
    };
    let Some(symbol) = scoping.get_reference(reference).symbol_id() else {
        return false;
    };
    let current = cx.current_scope_id();
    let here = function_scope(scoping, current);
    let home = function_scope(scoping, scoping.symbol_scope_id(symbol));
    if home != here
        || scoping.scope_flags(home).contains_direct_eval()
        || is_inside_with(scoping, current, here)
    {
        return false;
    }
    scoping
        .get_resolved_reference_ids(symbol)
        .iter()
        .all(|id| !writes_from_elsewhere(scoping, *id, here))
}

fn writes_from_elsewhere(scoping: &Scoping, id: ReferenceId, here: ScopeId) -> bool {
    let reference = scoping.get_reference(id);
    reference.is_write() && function_scope(scoping, reference.scope_id()) != here
}

fn is_inside_with(scoping: &Scoping, start: ScopeId, until: ScopeId) -> bool {
    untwist_core::semantic::with_observes()
        && scoping
            .scope_ancestors(start)
            .take_while(|scope| *scope != until)
            .any(|scope| scoping.scope_flags(scope).contains(ScopeFlags::With))
}

fn function_scope(scoping: &Scoping, start: ScopeId) -> ScopeId {
    let boundary = ScopeFlags::Function | ScopeFlags::Arrow | ScopeFlags::ClassStaticBlock;
    let mut scope = start;
    loop {
        if scoping.scope_flags(scope).intersects(boundary) {
            return scope;
        }
        match scoping.scope_parent_id(scope) {
            Some(parent) => scope = parent,
            None => return scope,
        }
    }
}

impl<'a> LeadingScan<'a> {
    fn note_target(&mut self, target: &AssignmentTarget<'a>) {
        let mut names = TargetNames::default();
        names.visit_assignment_target(target);
        self.written.extend(names.names);
        self.writes_member |= names.member;
    }

    fn note_simple(&mut self, target: &SimpleAssignmentTarget<'a>) {
        match target {
            SimpleAssignmentTarget::AssignmentTargetIdentifier(id) => {
                self.written.push(id.name.as_str())
            }
            _ => self.writes_member = true,
        }
    }
}

impl<'a> VisitJs<'a> for LeadingScan<'a> {
    fn visit_function(&mut self, _it: &Function<'a>, _flags: ScopeFlags) {}

    fn visit_arrow_function_expression(&mut self, _it: &ArrowFunctionExpression<'a>) {}

    fn visit_expression(&mut self, it: &Expression<'a>) {
        match it {
            Expression::AssignmentExpression(node) => self.note_target(&node.left),
            Expression::UpdateExpression(node) => self.note_simple(&node.argument),
            Expression::UnaryExpression(node) if node.operator == UnaryOperator::Delete => {
                match &node.argument {
                    Expression::Identifier(id) => self.written.push(id.name.as_str()),
                    _ => self.writes_member = true,
                }
            }
            _ => {}
        }
        self.runs_code |= !is_inert(it);
        oxc_ast_visit::walk_js::walk_expression(self, it);
    }
}

fn is_inert(expr: &Expression<'_>) -> bool {
    matches!(
        expr,
        Expression::NumericLiteral(_)
            | Expression::StringLiteral(_)
            | Expression::BooleanLiteral(_)
            | Expression::NullLiteral(_)
            | Expression::BigIntLiteral(_)
            | Expression::Identifier(_)
            | Expression::FunctionExpression(_)
            | Expression::ArrowFunctionExpression(_)
            | Expression::SequenceExpression(_)
            | Expression::ParenthesizedExpression(_)
    ) || matches!(
        expr,
        Expression::AssignmentExpression(node)
            if matches!(node.left, AssignmentTarget::AssignmentTargetIdentifier(_))
    )
}

#[derive(Default)]
struct TargetNames<'a> {
    names: Vec<&'a str>,
    member: bool,
}

impl<'a> VisitJs<'a> for TargetNames<'a> {
    fn visit_identifier_reference(&mut self, it: &IdentifierReference<'a>) {
        self.names.push(it.name.as_str());
    }

    fn visit_member_expression(&mut self, _it: &MemberExpression<'a>) {
        self.member = true;
    }

    fn visit_simple_assignment_target(&mut self, it: &SimpleAssignmentTarget<'a>) {
        match it {
            SimpleAssignmentTarget::AssignmentTargetIdentifier(id) => {
                self.names.push(id.name.as_str())
            }
            _ => self.member = true,
        }
    }
}
