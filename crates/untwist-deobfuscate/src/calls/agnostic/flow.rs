mod collect;
mod funcs;
pub(super) mod holds;
#[cfg(test)]
mod tests;
mod walk;

use std::cell::OnceCell;

use oxc_ast::ast::Program;
use oxc_ast_visit::VisitJs;
use oxc_semantic::Scoping;
use oxc_span::Span;
use rustc_hash::FxHashMap;

use super::sites::span_key;
use crate::value_flow::{flow_rule_disabled, VALUE_FLOW_SINKS};
use collect::Collector;
use funcs::Prescan;
use untwist_eval::value::JsString;

const UNREACHED: u32 = u32::MAX;

struct Graph {
    reads: FxHashMap<u64, u32>,
    reached: Vec<u32>,
    sinks: Vec<Span>,
}

pub(super) struct Flow<'f, 'a> {
    program: &'a Program<'a>,
    scoping: &'f Scoping,
    resolve: Option<&'f FxHashMap<Span, &'f JsString>>,
    graph: OnceCell<Graph>,
}

impl<'f, 'a> Flow<'f, 'a> {
    pub(super) fn new(
        program: &'a Program<'a>,
        scoping: &'f Scoping,
        resolve: Option<&'f FxHashMap<Span, &'f JsString>>,
    ) -> Self {
        Self {
            program,
            scoping,
            resolve,
            graph: OnceCell::new(),
        }
    }

    fn graph(&self) -> &Graph {
        self.graph.get_or_init(|| {
            let mut prescan = Prescan::new(self.scoping);
            prescan.visit_program(self.program);
            let mut collector = Collector::new(self.scoping, self.resolve, prescan.finish());
            collector.visit_program(self.program);
            collector.solve()
        })
    }

    pub(super) fn sink_of(&self, source: Span) -> Option<Span> {
        if flow_rule_disabled(VALUE_FLOW_SINKS) {
            return None;
        }
        let graph = self.graph();
        let Some(&node) = graph.reads.get(&span_key(source)) else {
            return Some(source);
        };
        match graph.reached.get(node as usize) {
            Some(&UNREACHED) => None,
            Some(&sink) => graph.sinks.get(sink as usize).copied().or(Some(source)),
            None => Some(source),
        }
    }
}
