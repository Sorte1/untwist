use oxc_ast::ast::*;
use oxc_ast_visit::walk_js;

use super::Scan;

impl Scan<'_> {
    pub(super) fn visit_delete(&mut self, it: &UnaryExpression<'_>) {
        let argument = &it.argument;
        if self.deletes_a_root_name(argument) || self.member_write(argument) {
            self.written();
            return;
        }
        if self.deletes_window_key(argument) {
            self.not_allowed();
        }
        if self.reaches_promise(argument) {
            self.promise_written = true;
        }
        walk_js::walk_unary_expression(self, it);
    }

    fn deletes_a_root_name(&self, argument: &Expression<'_>) -> bool {
        match argument.without_parentheses() {
            Expression::Identifier(id) => self.global_named(id, self.reliance.roots),
            _ => false,
        }
    }

    pub(super) fn target_reads_a_root(&self, target: &AssignmentTarget<'_>) -> bool {
        let Some(simple) = target.as_simple_assignment_target() else {
            return true;
        };
        match simple {
            SimpleAssignmentTarget::AssignmentTargetIdentifier(id) => {
                self.allow_root_name(id).is_some()
            }
            SimpleAssignmentTarget::StaticMemberExpression(_)
            | SimpleAssignmentTarget::ComputedMemberExpression(_)
            | SimpleAssignmentTarget::PrivateFieldExpression(_) => {
                simple.as_member_expression().is_none_or(|member| {
                    let read = member.as_expression();
                    self.is_root(read) || self.reads_prototypes(read)
                })
            }
            SimpleAssignmentTarget::TSAsExpression(_)
            | SimpleAssignmentTarget::TSSatisfiesExpression(_)
            | SimpleAssignmentTarget::TSNonNullExpression(_)
            | SimpleAssignmentTarget::TSTypeAssertion(_) => true,
        }
    }
}
