use oxc_ast::ast::*;
use oxc_ast_visit::{walk_js, VisitJs};
use oxc_semantic::Scoping;
use oxc_span::Span;
use oxc_syntax::operator::UnaryOperator;
use rustc_hash::{FxHashMap, FxHashSet};
use untwist_core::ast_util::search::statements_at;

use crate::mutants::legacy_rotation_patches_trusted;
use crate::notes;
use crate::proof::{
    alias_refusal, alias_sites, builtin_aliases, patch_refusal, record_definer_patch,
    record_identifier_patch, record_member_patch, statement_used_names, PatchKey, PatchSite,
    UsedNames,
};

#[derive(Default)]
struct PatchScan<'a> {
    sites: Vec<PatchSite<'a>>,
}

impl<'a> VisitJs<'a> for PatchScan<'a> {
    fn visit_simple_assignment_target(&mut self, it: &SimpleAssignmentTarget<'a>) {
        match it {
            SimpleAssignmentTarget::AssignmentTargetIdentifier(id) => {
                self.sites.extend(record_identifier_patch(id));
            }
            other => {
                self.sites
                    .extend(other.as_member_expression().and_then(record_member_patch));
            }
        }
        walk_js::walk_simple_assignment_target(self, it);
    }

    fn visit_unary_expression(&mut self, it: &UnaryExpression<'a>) {
        if it.operator == UnaryOperator::Delete {
            self.sites.extend(
                it.argument
                    .without_parentheses()
                    .as_member_expression()
                    .and_then(record_member_patch),
            );
        }
        walk_js::walk_unary_expression(self, it);
    }

    fn visit_call_expression(&mut self, it: &CallExpression<'a>) {
        self.sites.extend(record_definer_patch(it));
        walk_js::walk_call_expression(self, it);
    }
}

fn unused_key(key: PatchKey<'_>, used: &UsedNames<'_>) -> bool {
    matches!(key, PatchKey::Named(name) if !used.uses(name))
}

pub(super) fn rotator_builtins_patched(
    program: &Program<'_>,
    scoping: &Scoping,
    (rotator, keys): (Span, FxHashMap<Span, String>),
) -> Option<(&'static str, Span)> {
    if legacy_rotation_patches_trusted() {
        return None;
    }
    let Some(Some(statement)) = statements_at(program, &[rotator]).into_iter().next() else {
        return Some((notes::BUILTIN_PATCHED, rotator));
    };
    let used = statement_used_names(scoping, statement);
    let mut scan = PatchScan::default();
    scan.visit_program(program);
    let verdict = patch_refusal(&scan.sites, scoping, &used, &FxHashSet::default());
    if let Some(refusal) = verdict.refusal {
        return Some((refusal.note, refusal.span));
    }
    if let Some(pending) = verdict.pending_prototypes.first() {
        return Some((notes::BUILTIN_PATCHED, pending.root_key));
    }
    if let Some(write) = verdict
        .proto_writes
        .iter()
        .find(|write| !unused_key(write.key, &used))
    {
        return Some((notes::BUILTIN_PATCHED, write.span));
    }
    let aliases = builtin_aliases(program, scoping, keys);
    alias_refusal(&alias_sites(program, &aliases, &[rotator]), &used)
        .map(|refusal| (refusal.note, refusal.span))
}
