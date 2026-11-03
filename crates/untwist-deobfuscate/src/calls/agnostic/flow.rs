mod collect;
mod funcs;
#[cfg(test)]
mod tests;
mod walk;

use oxc_ast::ast::Program;
use oxc_ast_visit::VisitJs;
use oxc_semantic::Scoping;
use oxc_span::Span;
use rustc_hash::FxHashMap;

use super::sites::span_key;
use crate::value_flow::{flow_rule_disabled, VALUE_FLOW_SINKS};
use collect::Collector;
use funcs::FuncScan;
use untwist_eval::value::JsString;

const UNREACHED: u32 = u32::MAX;

pub(super) struct Flow {
    reads: FxHashMap<u64, u32>,
    reached: Vec<u32>,
    sinks: Vec<Span>,
}

impl Flow {
    pub(super) fn collect<'a>(
        program: &'a Program<'a>,
        scoping: &Scoping,
        resolve: Option<&FxHashMap<Span, &JsString>>,
    ) -> Self {
        let mut funcs = FxHashMap::default();
        FuncScan { funcs: &mut funcs }.visit_program(program);
        let mut collector = Collector::new(scoping, resolve, funcs);
        collector.visit_program(program);
        collector.solve()
    }

    pub(super) fn sink_of(&self, source: Span) -> Option<Span> {
        if flow_rule_disabled(VALUE_FLOW_SINKS) {
            return None;
        }
        let Some(&node) = self.reads.get(&span_key(source)) else {
            return Some(source);
        };
        match self.reached.get(node as usize) {
            Some(&UNREACHED) => None,
            Some(&sink) => self.sinks.get(sink as usize).copied().or(Some(source)),
            None => Some(source),
        }
    }
}
