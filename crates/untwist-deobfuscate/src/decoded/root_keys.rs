use oxc_ast::ast::*;
use oxc_ast_visit::{walk_js, VisitJs};
use oxc_semantic::Scoping;
use oxc_span::Span;
use oxc_syntax::scope::ScopeFlags;
use rustc_hash::FxHashSet;
use untwist_eval::decode::DecoderProgram;
use untwist_eval::object::PROTOTYPE_LINKS;

use super::UnitRun;
use crate::proof::PreScan;

struct RootReads<'s, 'a> {
    scoping: &'s Scoping,
    keys: Vec<(&'a str, Option<&'a str>)>,
}

impl<'a> RootReads<'_, 'a> {
    fn root_name(&self, object: &Expression<'a>) -> Option<&'a str> {
        let Expression::Identifier(id) = object.without_parentheses() else {
            return None;
        };
        let unresolved = id
            .reference_id
            .get()
            .is_none_or(|r| self.scoping.get_reference(r).symbol_id().is_none());
        unresolved.then_some(id.name.as_str())
    }
}

impl<'a> VisitJs<'a> for RootReads<'_, 'a> {
    fn visit_static_member_expression(&mut self, it: &StaticMemberExpression<'a>) {
        if let Some(root) = self.root_name(&it.object) {
            self.keys.push((root, Some(it.property.name.as_str())));
        }
        walk_js::walk_static_member_expression(self, it);
    }

    fn visit_computed_member_expression(&mut self, it: &ComputedMemberExpression<'a>) {
        if let Some(root) = self.root_name(&it.object) {
            let key = match it.expression.without_parentheses() {
                Expression::StringLiteral(key) => key.value.as_str(),
                _ => None,
            };
            self.keys.push((root, key));
        }
        walk_js::walk_computed_member_expression(self, it);
    }
}

fn reads_on_roots<'a>(
    program: &DecoderProgram<'a>,
    scoping: &Scoping,
) -> Vec<(&'a str, Option<&'a str>)> {
    let mut reads = RootReads {
        scoping,
        keys: Vec::new(),
    };
    for function in program.functions() {
        reads.visit_function(function, ScopeFlags::Function);
    }
    if let Some(rotator) = program.rotator {
        reads.visit_expression(rotator);
    }
    reads.keys
}

fn written_names(scan: &PreScan, resolve: &dyn Fn(Span) -> Option<String>) -> FxHashSet<String> {
    scan.written
        .iter()
        .filter_map(|key| key.resolved(resolve))
        .collect()
}

fn root_written(root: &str, reads: &[(&str, Option<&str>)], written: &FxHashSet<String>) -> bool {
    written.contains(root)
        || reads.iter().any(|(read_root, key)| {
            *read_root == root && key.is_none_or(|key| written.contains(key))
        })
}

pub(super) fn refuse_written_roots(
    names: &mut FxHashSet<&'static str>,
    scan: &PreScan,
    runs: &[UnitRun<'_, '_>],
    resolve: &dyn Fn(Span) -> Option<String>,
    scoping: &Scoping,
) {
    if scan.written.is_empty() {
        return;
    }
    let written = written_names(scan, resolve);
    for run in runs {
        let pending: Vec<&'static str> = run
            .evaluator
            .builtin_roots
            .iter()
            .copied()
            .filter(|root| *root != "Object" && *root != PROTOTYPE_LINKS && !names.contains(root))
            .collect();
        if pending.is_empty() {
            continue;
        }
        let reads = reads_on_roots(&run.request.program, scoping);
        for root in pending {
            if root_written(root, &reads, &written) {
                names.insert(root);
            }
        }
    }
}

pub(super) fn written_spans(scan: &PreScan, spans: &mut FxHashSet<Span>) {
    for key in &scan.written {
        key.pending_spans(spans);
    }
}
