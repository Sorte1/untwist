use oxc_ast::ast::ForStatement;

use super::base64::walked;
use super::cost::{concat_charge, Event};
use super::exec::{commit, number_slot, read_slot, text_slot, writable, Native, Run};
use super::{Kind, Planning, Step};
use crate::coerce::to_int32;
use crate::compile::{LazyExpr, Reach};
use crate::frame::FrameRef;
use crate::interp::Interp;
use crate::value::{Eval, JsString, ObjKind, ObjRef, PropKey, Value};

pub(super) const STATEMENTS: &str = r"F(v0,v1,){var(v2=[],);var(v3=#0,);var(v4,);var(v5='',);e(a=(v0,call(v6,v0)));for(var(v7=#0,);b<(v7,#256);post++(v7)){e(a=(i(v2,v7),v7));};for(a=(v7,#0);b<(v7,#256);post++(v7)){e(a=(v3,b%(b+(b+(v3,i(v2,v7)),call(m(v1).'charCodeAt',b%(v7,m(v1).'length'))),#256)));e(a=(v4,i(v2,v7)));e(a=(i(v2,v7),i(v2,v3)));e(a=(i(v2,v3),v4));};e(a=(v7,#0));e(a=(v3,#0));for(var(v8=#0,);b<(v8,m(v0).'length');post++(v8)){e(a=(v7,b%(b+(v7,#1),#256)));e(a=(v3,b%(b+(v3,i(v2,v7)),#256)));e(a=(v4,i(v2,v7)));e(a=(i(v2,v7),i(v2,v3)));e(a=(i(v2,v3),v4));e(a+=(v5,call(m(v9).'fromCharCode',b^(call(m(v0).'charCodeAt',v8),i(v2,b%(b+(i(v2,v7),i(v2,v3)),#256))))));};ret(v5);}";
pub(super) const SEQUENCES: &str = r"F(v0,v1,){var(v2=[],);var(v3=#0,);var(v4,);var(v5='',);e(a=(v0,call(v6,v0)));var(v7,);for(a=(v7,#0);b<(v7,#256);post++(v7)){e(a=(i(v2,v7),v7));};for(a=(v7,#0);b<(v7,#256);post++(v7)){e(s(a=(v3,b%(b+(b+(v3,i(v2,v7)),call(m(v1).'charCodeAt',b%(v7,m(v1).'length'))),#256)),a=(v4,i(v2,v7)),a=(i(v2,v7),i(v2,v3)),a=(i(v2,v3),v4),));};e(s(a=(v7,#0),a=(v3,#0),));for(var(v8=#0,);b<(v8,m(v0).'length');post++(v8)){e(s(a=(v7,b%(b+(v7,#1),#256)),a=(v3,b%(b+(v3,i(v2,v7)),#256)),a=(v4,i(v2,v7)),a=(i(v2,v7),i(v2,v3)),a=(i(v2,v3),v4),a+=(v5,call(m(v9).'fromCharCode',b^(call(m(v0).'charCodeAt',v8),i(v2,b%(b+(i(v2,v7),i(v2,v3)),#256))))),));};ret(v5);}";

const STATE_LEN: usize = 256;
const FILL_BYTES: u64 = 16 << 10;

const CHAR_CODE_AT: Native = ("String.prototype", "charCodeAt");
const FROM_CHAR_CODE: Native = ("String", "fromCharCode");

const TEXT: usize = 0;
const KEY: usize = 1;
const STATE: usize = 2;
const MIXER: usize = 3;
const SWAP: usize = 4;
const OUTPUT: usize = 5;
const INDEX: usize = 7;
const STRING: usize = 9;

#[derive(Clone, Copy)]
struct Costs {
    head: u64,
    test: u64,
    body: u64,
    update: u64,
}

pub(super) struct Fill {
    state: u32,
    index: u32,
    costs: Costs,
}

pub(super) struct Schedule {
    state: u32,
    mixer: u32,
    swap: u32,
    key: u32,
    index: u32,
    costs: Costs,
    get: u64,
    call: u64,
}

pub(super) struct Stream<'a> {
    text: u32,
    state: u32,
    index: u32,
    mixer: u32,
    swap: u32,
    output: u32,
    string: LazyExpr<'a>,
    costs: Costs,
    marks: [u64; 4],
}

fn costs<'a>(
    node: &'a ForStatement<'a>,
    calls: usize,
) -> Option<(Costs, super::cost::Walk<'a, 'static>)> {
    let head = walked(&[], |walk| walk.for_init(node.init.as_ref()))?;
    let test = walked(&[], |walk| walk.expression(node.test.as_ref()?))?;
    let body = walked(&[], |walk| walk.statement(&node.body))?;
    let update = walked(&[], |walk| walk.expression(node.update.as_ref()?))?;
    let quiet = test.count(Event::Call) + update.count(Event::Call) + head.count(Event::Call) == 0;
    if !quiet || body.count(Event::Call) != calls {
        return None;
    }
    let costs = Costs {
        head: 1 + head.total,
        test: test.total,
        body: body.total,
        update: update.total,
    };
    Some((costs, body))
}

fn plan_fill<'a>(cx: &mut Planning<'_, 'a>, at: usize) -> Option<Fill> {
    let (costs, body) = costs(cx.for_statement(at)?, 0)?;
    (body.count(Event::Concat) == 0).then_some(())?;
    Some(Fill {
        state: cx.slot(at, STATE)?,
        index: cx.slot(at, INDEX)?,
        costs,
    })
}

fn plan_schedule<'a>(cx: &mut Planning<'_, 'a>, at: usize) -> Option<Schedule> {
    let (costs, body) = costs(cx.for_statement(at)?, 1)?;
    (body.count(Event::Concat) == 0).then_some(())?;
    Some(Schedule {
        state: cx.slot(at, STATE)?,
        mixer: cx.slot(at, MIXER)?,
        swap: cx.slot(at, SWAP)?,
        key: cx.slot(at, KEY)?,
        index: cx.slot(at, INDEX)?,
        costs,
        get: body.at(Event::Get("charCodeAt"))?,
        call: body.first(Event::Call)?,
    })
}

fn plan_stream<'a>(cx: &mut Planning<'_, 'a>, at: usize) -> Option<Stream<'a>> {
    let (costs, body) = costs(cx.for_statement(at)?, 2)?;
    let string = *cx.shape.names.get(STRING)?;
    (body.count(Event::Concat) == 1).then_some(())?;
    Some(Stream {
        text: cx.slot(at, TEXT)?,
        state: cx.slot(at, STATE)?,
        index: cx.slot(at, INDEX)?,
        mixer: cx.slot(at, MIXER)?,
        swap: cx.slot(at, SWAP)?,
        output: cx.slot(at, OUTPUT)?,
        string: cx.read(at, STRING)?,
        costs,
        marks: [
            body.at(Event::Read(string))?,
            body.at(Event::Get("fromCharCode"))?,
            body.at(Event::Get("charCodeAt"))?,
            body.first(Event::Call)?,
        ],
    })
}

pub(super) fn plan<'a>(cx: &mut Planning<'_, 'a>) -> Option<Box<[Option<Step<'a>>]>> {
    if cx.shape.text != STATEMENTS && cx.shape.text != SEQUENCES {
        return None;
    }
    let loops = cx.loops();
    let [fill, schedule, stream] = loops.as_slice() else {
        return None;
    };
    let mut steps = cx.no_steps();
    steps[*fill] = Some(Step(Kind::Fill(plan_fill(cx, *fill)?)));
    steps[*schedule] = Some(Step(Kind::Schedule(plan_schedule(cx, *schedule)?)));
    steps[*stream] = Some(Step(Kind::Stream(plan_stream(cx, *stream)?)));
    Some(steps)
}

fn byte(value: f64) -> Option<usize> {
    let index = value as usize;
    (index < STATE_LEN && index as f64 == value).then_some(index)
}

fn state_bytes<'a>(env: &FrameRef<'a>, slot: u32) -> Option<(ObjRef<'a>, [usize; STATE_LEN])> {
    let Some(Value::Object(obj)) = read_slot(env, slot) else {
        return None;
    };
    let mut bytes = [0_usize; STATE_LEN];
    {
        let borrowed = obj.borrow();
        let ObjKind::Array(items) = &borrowed.kind else {
            return None;
        };
        if items.len() != STATE_LEN {
            return None;
        }
        for (into, item) in bytes.iter_mut().zip(items.iter()) {
            let Value::Number(number) = item else {
                return None;
            };
            *into = byte(*number)?;
        }
    }
    Some((obj, bytes))
}

fn write_state(obj: &ObjRef<'_>, bytes: &[usize; STATE_LEN]) {
    if let ObjKind::Array(items) = &mut obj.borrow_mut().kind {
        for (item, value) in items.iter_mut().zip(bytes.iter()) {
            *item = Value::Number(*value as f64);
        }
    }
}

fn store_numbers<'a>(env: &FrameRef<'a>, pairs: &[(u32, f64)]) -> Eval<'a, bool> {
    for (slot, value) in pairs {
        commit(env, *slot, Value::Number(*value))?;
    }
    Ok(true)
}

impl<'a> Interp<'a> {
    pub(super) fn run_fill(&mut self, plan: &Fill, env: &FrameRef<'a>) -> Eval<'a, bool> {
        let Some(Value::Object(state)) = read_slot(env, plan.state) else {
            return Ok(false);
        };
        if !writable(env, &[plan.index]) || !self.has_room(STATE_LEN, FILL_BYTES) {
            return Ok(false);
        }
        let mut run = self.begin();
        run.owe(plan.costs.head);
        for index in 0..STATE_LEN {
            run.owe(plan.costs.test + plan.costs.body);
            self.settle(&mut run)?;
            let slot = PropKey::Index(index as u32);
            self.write_member(
                Value::Object(state.clone()),
                slot,
                Value::Number(index as f64),
            )?;
            run.owe(plan.costs.update);
        }
        run.owe(plan.costs.test);
        self.settle(&mut run)?;
        store_numbers(env, &[(plan.index, STATE_LEN as f64)])
    }

    pub(super) fn run_schedule(&mut self, plan: &Schedule, env: &FrameRef<'a>) -> Eval<'a, bool> {
        let (Some((state, mut bytes)), Some(mixer), Some(key)) = (
            state_bytes(env, plan.state),
            number_slot(env, plan.mixer).and_then(byte),
            text_slot(env, plan.key),
        ) else {
            return Ok(false);
        };
        let slots = [plan.index, plan.mixer, plan.swap];
        if key.is_empty() || !writable(env, &slots) || !self.has_room(STATE_LEN, 0) {
            return Ok(false);
        }
        let mut run = self.begin();
        run.owe(plan.costs.head + plan.costs.test);
        run.part();
        let receiver = Value::Str(key.clone());
        if !self.intact_member(&mut run, (&receiver, "charCodeAt", plan.get), CHAR_CODE_AT)? {
            self.refund(&run);
            return Ok(false);
        }
        self.first_call(&mut run, plan.call)?;
        run.end(plan.costs.body);
        let (mut j, mut swap) = (mixer, 0);
        for index in 0..STATE_LEN {
            if index > 0 {
                run.owe(plan.costs.test + plan.costs.body);
            }
            let code = usize::from(key.units()[index % key.len()]);
            j = (j + bytes[index] + code) % STATE_LEN;
            swap = bytes[index];
            bytes.swap(index, j);
            run.owe(plan.costs.update);
        }
        run.owe(plan.costs.test);
        self.settle(&mut run)?;
        write_state(&state, &bytes);
        let numbers = [
            (plan.index, STATE_LEN as f64),
            (plan.mixer, j as f64),
            (plan.swap, swap as f64),
        ];
        store_numbers(env, &numbers)
    }

    pub(super) fn run_stream(
        &mut self,
        plan: &Stream<'a>,
        at: (&FrameRef<'a>, Reach),
    ) -> Eval<'a, bool> {
        let env = at.0;
        let (Some(text), Some((state, bytes)), Some(i), Some(j), Some(output)) = (
            text_slot(env, plan.text),
            state_bytes(env, plan.state),
            number_slot(env, plan.index).and_then(byte),
            number_slot(env, plan.mixer).and_then(byte),
            text_slot(env, plan.output),
        ) else {
            return Ok(false);
        };
        let len = text.len();
        let transient = 4 * (96 + 2 * (output.len() + len) as u64);
        let slots = [plan.index, plan.mixer, plan.swap, plan.output];
        if !writable(env, &slots) || !self.has_room(len, transient) {
            return Ok(false);
        }
        let mut run = self.begin();
        run.owe(plan.costs.head);
        let mut out = output.units().to_vec();
        let mut keystream = Keystream {
            bytes,
            i,
            j,
            swap: None,
        };
        for position in 0..len {
            run.owe(plan.costs.test);
            if position == 0 && !self.stream_checks(plan, &mut run, &text, at)? {
                self.refund(&run);
                return Ok(false);
            }
            if position > 0 {
                run.owe(plan.costs.body);
            }
            run.owe(concat_charge(out.len(), 1));
            let mixed = to_int32(f64::from(text.units()[position])) ^ keystream.next() as i32;
            out.push(mixed as u16);
            run.owe(plan.costs.update);
        }
        run.owe(plan.costs.test);
        self.settle(&mut run)?;
        self.finish_stream(plan, env, (&state, keystream), out, len)
    }

    fn stream_checks(
        &mut self,
        plan: &Stream<'a>,
        run: &mut Run,
        text: &JsString,
        at: (&FrameRef<'a>, Reach),
    ) -> Eval<'a, bool> {
        run.part();
        let string = self.read_lazy(run, (&plan.string, plan.marks[0]), at)?;
        if !self.intact_member(
            run,
            (&string, "fromCharCode", plan.marks[1]),
            FROM_CHAR_CODE,
        )? {
            return Ok(false);
        }
        let receiver = Value::Str(text.clone());
        if !self.intact_member(run, (&receiver, "charCodeAt", plan.marks[2]), CHAR_CODE_AT)? {
            return Ok(false);
        }
        self.first_call(run, plan.marks[3])?;
        run.end(plan.costs.body);
        Ok(true)
    }

    fn finish_stream(
        &mut self,
        plan: &Stream<'a>,
        env: &FrameRef<'a>,
        (state, keystream): (&ObjRef<'a>, Keystream),
        out: Vec<u16>,
        len: usize,
    ) -> Eval<'a, bool> {
        if len == 0 {
            return Ok(true);
        }
        write_state(state, &keystream.bytes);
        let numbers = [
            (plan.index, keystream.i as f64),
            (plan.mixer, keystream.j as f64),
            (plan.swap, keystream.swap.unwrap_or_default() as f64),
        ];
        let output = self.charged_text(out)?;
        store_numbers(env, &numbers)?;
        commit(env, plan.output, Value::Str(output))?;
        Ok(true)
    }
}

struct Keystream {
    bytes: [usize; STATE_LEN],
    i: usize,
    j: usize,
    swap: Option<usize>,
}

impl Keystream {
    fn next(&mut self) -> usize {
        self.i = (self.i + 1) % STATE_LEN;
        self.j = (self.j + self.bytes[self.i]) % STATE_LEN;
        self.swap = Some(self.bytes[self.i]);
        self.bytes.swap(self.i, self.j);
        self.bytes[(self.bytes[self.i] + self.bytes[self.j]) % STATE_LEN]
    }
}
