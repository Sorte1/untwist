mod base64;
mod cost;
mod exec;
mod rc4;
mod shape;
#[cfg(test)]
mod tests;

use std::cell::Cell;

use oxc_ast::ast::{ForStatement, Statement};

use crate::compile::scope::Resolved;
use crate::compile::{Lazy, LazyExpr, Reach};
use crate::frame::FrameRef;
use crate::interp::Interp;
use crate::value::{Eval, FnCode};

const MAX_PARAMS: usize = 2;
const MAX_STATEMENTS: usize = 16;
const MAX_SPAN_BYTES: u32 = 4096;
const LOOPS: [usize; 2] = [2, 3];

thread_local! {
    static ENABLED: Cell<bool> = const { Cell::new(true) };
}

#[cfg(feature = "test-hooks")]
thread_local! {
    static RUNS: Cell<u64> = const { Cell::new(0) };
}

pub(crate) fn enabled() -> bool {
    ENABLED.with(Cell::get)
}

#[cfg(feature = "test-hooks")]
pub(crate) fn set_enabled(enabled: bool) {
    ENABLED.with(|cell| cell.set(enabled));
}

#[cfg(feature = "test-hooks")]
pub(crate) fn runs() -> u64 {
    RUNS.with(Cell::get)
}

pub(crate) struct Plan<'a> {
    steps: Box<[Option<Step<'a>>]>,
}

pub(crate) struct Step<'a>(Kind<'a>);

enum Kind<'a> {
    Decode(base64::Decode<'a>),
    Percent(base64::Percent),
    Fill(rc4::Fill),
    Schedule(rc4::Schedule),
    Stream(rc4::Stream<'a>),
}

struct Planning<'c, 'a> {
    interp: &'c mut Interp<'a>,
    shape: &'c shape::Shape<'a>,
    body: &'a [Statement<'a>],
    lazies: &'c [Lazy<'a>],
}

impl<'a> Planning<'_, 'a> {
    fn loops(&self) -> Vec<usize> {
        self.body
            .iter()
            .enumerate()
            .filter(|(_, statement)| matches!(statement, Statement::ForStatement(_)))
            .map(|(index, _)| index)
            .collect()
    }

    fn no_steps(&self) -> Box<[Option<Step<'a>>]> {
        self.body.iter().map(|_| None).collect()
    }

    fn for_statement(&self, at: usize) -> Option<&'a ForStatement<'a>> {
        match self.body.get(at)? {
            Statement::ForStatement(node) => Some(node),
            _ => None,
        }
    }

    fn slot(&mut self, at: usize, variable: usize) -> Option<u32> {
        let name = self.interp.names.intern(self.shape.names.get(variable)?);
        match self.lazies.get(at)?.scope().resolve(name) {
            Resolved::Local(slot) => Some(slot),
            _ => None,
        }
    }

    fn optional_slot(&mut self, at: usize, variable: Option<usize>) -> Option<Option<u32>> {
        match variable {
            Some(variable) => self.slot(at, variable).map(Some),
            None => Some(None),
        }
    }

    fn read(&mut self, at: usize, variable: usize) -> Option<LazyExpr<'a>> {
        if self.slot(at, variable).is_some() {
            return None;
        }
        let node = (*self.shape.reads.get(variable)?)?;
        Some(LazyExpr::new(node, self.lazies.get(at)?.scope().clone()))
    }
}

impl<'a> Plan<'a> {
    pub(crate) fn step(&self, index: usize) -> Option<&Step<'a>> {
        self.steps.get(index)?.as_ref()
    }
}

impl<'a> Interp<'a> {
    pub(crate) fn plan_native_body(
        &mut self,
        code: FnCode<'a>,
        lazies: &[Lazy<'a>],
    ) -> Option<Box<Plan<'a>>> {
        if !ENABLED.with(Cell::get) {
            return None;
        }
        let FnCode::Function(function) = code else {
            return None;
        };
        let body = &function.body.as_ref()?.statements;
        let small = function.params.items.len() <= MAX_PARAMS
            && body.len() <= MAX_STATEMENTS
            && function.span.size() <= MAX_SPAN_BYTES
            && lazies.len() == body.len()
            && matches!(body.last(), Some(Statement::ReturnStatement(_)));
        if !small {
            return None;
        }
        let loops = body
            .iter()
            .filter(|statement| matches!(statement, Statement::ForStatement(_)))
            .count();
        if !LOOPS.contains(&loops) {
            return None;
        }
        let shape = shape::function_shape(function)?;
        let mut cx = Planning {
            interp: self,
            shape: &shape,
            body,
            lazies,
        };
        let steps = base64::plan(&mut cx).or_else(|| rc4::plan(&mut cx))?;
        Some(Box::new(Plan { steps }))
    }

    pub(crate) fn run_native_step(
        &mut self,
        step: &Step<'a>,
        at: (&FrameRef<'a>, Reach),
    ) -> Eval<'a, bool> {
        let native = match &step.0 {
            Kind::Decode(plan) => self.run_decode(plan, at),
            Kind::Percent(plan) => self.run_percent(plan, at),
            Kind::Fill(plan) => self.run_fill(plan, at.0),
            Kind::Schedule(plan) => self.run_schedule(plan, at.0),
            Kind::Stream(plan) => self.run_stream(plan, at),
        }?;
        #[cfg(feature = "test-hooks")]
        if native {
            RUNS.with(|runs| runs.set(runs.get() + 1));
        }
        Ok(native)
    }
}
