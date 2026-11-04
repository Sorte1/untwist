use crate::bail::Bail;
use crate::budget::{loop_cap, Budget, CALL_DEPTH_CAP, NESTING_DEPTH_CAP};
use crate::builtins::args::reserve_text;
use crate::builtins::shape::SHAPES;
use crate::compile::{LazyExpr, Reach};
use crate::frame::{FrameRef, Stored};
use crate::host::Host;
use crate::interp::Interp;
use crate::value::{Eval, JsString, ObjKind, PropKey, Value};

const NESTING_ROOM: u32 = 256;
const SLACK_BYTES: u64 = 64 << 10;

pub(super) type Native = (&'static str, &'static str);

pub(super) struct Run {
    snapshot: Budget,
    owed: u64,
    at: u64,
    called: bool,
}

impl Run {
    pub(super) fn owe(&mut self, steps: u64) {
        self.owed += steps;
    }

    pub(super) fn part(&mut self) {
        self.at = 0;
    }

    pub(super) fn to(&mut self, mark: u64) {
        self.owed += mark.saturating_sub(self.at);
        self.at = mark;
    }

    pub(super) fn end(&mut self, total: u64) {
        self.owed += total.saturating_sub(self.at);
        self.at = 0;
    }
}

pub(super) fn read_slot<'a>(env: &FrameRef<'a>, slot: u32) -> Option<Value<'a>> {
    env.borrow().body.read_slot(slot)
}

pub(super) fn text_slot<'a>(env: &FrameRef<'a>, slot: u32) -> Option<JsString> {
    match read_slot(env, slot)? {
        Value::Str(text) => Some(text),
        _ => None,
    }
}

pub(super) fn number_slot(env: &FrameRef<'_>, slot: u32) -> Option<f64> {
    match read_slot(env, slot)? {
        Value::Number(number) => Some(number),
        _ => None,
    }
}

const UNWRITABLE: &str = "a native decoder loop whose local the frame refused to store";

fn store_slot<'a>(env: &FrameRef<'a>, slot: u32, value: Value<'a>) -> bool {
    matches!(
        env.borrow_mut().body.store_slot(slot, value),
        Some(Stored::Done)
    )
}

pub(super) fn writable(env: &FrameRef<'_>, slots: &[u32]) -> bool {
    slots.iter().all(|slot| match read_slot(env, *slot) {
        Some(value) => store_slot(env, *slot, value),
        None => false,
    })
}

pub(super) fn commit<'a>(env: &FrameRef<'a>, slot: u32, value: Value<'a>) -> Eval<'a, ()> {
    match store_slot(env, slot, value) {
        true => Ok(()),
        false => Err(Bail::Unsupported(&UNWRITABLE).into()),
    }
}

impl<'a> Interp<'a> {
    pub(super) fn begin(&self) -> Run {
        Run {
            snapshot: self.budget,
            owed: 0,
            at: 0,
            called: false,
        }
    }

    pub(super) fn settle(&mut self, run: &mut Run) -> Eval<'a, ()> {
        let owed = std::mem::take(&mut run.owed);
        self.budget.spend(owed)?;
        Ok(())
    }

    pub(super) fn refund(&mut self, run: &Run) {
        self.budget = run.snapshot;
    }

    pub(super) fn has_room(&self, iterations: usize, transient: u64) -> bool {
        let heap = match &self.heap {
            Some(ledger) => {
                !ledger.is_over()
                    && ledger
                        .live()
                        .saturating_add(transient)
                        .saturating_add(SLACK_BYTES)
                        <= ledger.limit()
            }
            None => true,
        };
        heap && (iterations as u64) < loop_cap(self.loop_len_hint)
            && self.budget.call_depth() < CALL_DEPTH_CAP
            && self.budget.nesting_depth().saturating_add(NESTING_ROOM) <= NESTING_DEPTH_CAP
    }

    pub(super) fn first_call(&mut self, run: &mut Run, mark: u64) -> Eval<'a, ()> {
        if run.called {
            return Ok(());
        }
        run.to(mark);
        self.settle(run)?;
        self.budget.enter_call()?;
        self.budget.leave_call();
        run.called = true;
        Ok(())
    }

    pub(super) fn lookup(
        &mut self,
        run: &mut Run,
        (receiver, key, mark): (&Value<'a>, &str, u64),
        native: Native,
    ) -> Eval<'a, bool> {
        run.to(mark);
        self.settle(run)?;
        let value = self.get_property(receiver, &PropKey::Name(JsString::from_str(key)))?;
        run.at += 1;
        Ok(self.is_native(&value, native))
    }

    pub(super) fn read_lazy(
        &mut self,
        run: &mut Run,
        (read, mark): (&LazyExpr<'a>, u64),
        (env, reach): (&FrameRef<'a>, Reach),
    ) -> Eval<'a, Value<'a>> {
        run.to(mark);
        self.settle(run)?;
        let value = read.eval(self, env, reach)?;
        run.at += 1;
        Ok(value)
    }

    pub(super) fn is_native(&self, value: &Value<'a>, (owner, name): Native) -> bool {
        let Some(obj) = value.as_object() else {
            return false;
        };
        let id = match &obj.borrow().kind {
            ObjKind::Native(id, _) => *id,
            _ => return false,
        };
        let table = self.realm.natives();
        let Some((_, shape)) = table.get(id) else {
            return false;
        };
        SHAPES.iter().any(|entry| {
            entry.owner == owner && entry.name == name && std::ptr::eq(&entry.shape, shape)
        })
    }

    pub(super) fn charged_text(&mut self, units: Vec<u16>) -> Eval<'a, JsString> {
        let charge = reserve_text(self, units.len())?;
        Ok(JsString::charged(units, charge))
    }
}
