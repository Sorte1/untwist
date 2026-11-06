use oxc_ast::ast::ForStatement;

use super::cost::{concat_charge, length_charge, scan_charge, Event, Walk};
use super::exec::{commit, text_slot, writable, Native, Run};
use super::{Kind, Planning, Step};
use crate::coerce::{number_to_js_string, number_to_radix_string, to_int32};
use crate::compile::{LazyExpr, Reach};
use crate::frame::FrameRef;
use crate::interp::Interp;
use crate::value::{Eval, JsString, Value};

pub(super) const PLAIN: &str = r"F(v0,){var(v1='abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789+/=',);var(v2='',);var(v3='',);for(var(v4=#0,v5,v6,v7=#0,);a=(v6,call(m(v0).'charAt',post++(v7)));c(l&&(u~(v6),s(a=(v5,c(b%(v4,#4),b+(b*(v5,#64),v6),v6)),b%(post++(v4),#4),)),a+=(v2,call(m(v8).'fromCharCode',b&(#255,b>>(v5,b&(b*(#-2,v4),#6))))),#0)){e(a=(v6,call(m(v1).'indexOf',v6)));};for(var(v9=#0,v10=m(v2).'length',);b<(v9,v10);post++(v9)){e(a+=(v3,b+('%',call(m(b+('00',call(m(call(m(v2).'charCodeAt',v9)).'toString',#16))).'slice',#-2))));};ret(call(v11,v3));}";
pub(super) const SELF_TEXT: &str = r"F(v0,){var(v1='abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789+/=',);var(v2='',);var(v3='',);var(v4=b+(v2,v5),);for(var(v6=#0,v7,v8,v9=#0,);a=(v8,call(m(v0).'charAt',post++(v9)));c(l&&(u~(v8),s(a=(v7,c(b%(v6,#4),b+(b*(v7,#64),v8),v8)),b%(post++(v6),#4),)),a+=(v2,c(b!==(b-(call(m(v4).'charCodeAt',b+(v9,#10)),#10),#0),call(m(v10).'fromCharCode',b&(#255,b>>(v7,b&(b*(#-2,v6),#6)))),v6)),#0)){e(a=(v8,call(m(v1).'indexOf',v8)));};for(var(v11=#0,v12=m(v2).'length',);b<(v11,v12);post++(v11)){e(a+=(v3,b+('%',call(m(b+('00',call(m(call(m(v2).'charCodeAt',v11)).'toString',#16))).'slice',#-2))));};ret(call(v13,v3));}";
pub(super) const FLAGGED: &str = r"F(v0,){var(v1='abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789+/=',);var(v2='',);var(v3='',);var(v4=b+(v2,v5),);var(v6=b!==(call(m(b+('',F(){ret(#0);})).'indexOf','\u000a'),#-1),);for(var(v7=#0,v8,v9,v10=#0,);a=(v9,call(m(v0).'charAt',post++(v10)));c(l&&(u~(v9),s(a=(v8,c(b%(v7,#4),b+(b*(v8,#64),v9),v9)),b%(post++(v7),#4),)),a+=(v2,c(l||(v6,b!==(b-(call(m(v4).'charCodeAt',b+(v10,#10)),#10),#0)),call(m(v11).'fromCharCode',b&(#255,b>>(v8,b&(b*(#-2,v7),#6)))),v7)),#0)){e(a=(v9,call(m(v1).'indexOf',v9)));};for(var(v12=#0,v13=m(v2).'length',);b<(v12,v13);post++(v12)){e(a+=(v3,b+('%',call(m(b+('00',call(m(call(m(v2).'charCodeAt',v12)).'toString',#16))).'slice',#-2))));};ret(call(v14,v3));}";

const ALPHABET: &str = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789+/=";
const PROBE_OFFSET: usize = 11;
const PROBE_UNIT: f64 = 10.0;
const UNIT_PAGES: usize = 256;
const PAGE_BOUND: u64 = 8 << 10;

const CHAR_AT: Native = ("String.prototype", "charAt");
const INDEX_OF: Native = ("String.prototype", "indexOf");
const CHAR_CODE_AT: Native = ("String.prototype", "charCodeAt");
const SLICE: Native = ("String.prototype", "slice");
const FROM_CHAR_CODE: Native = ("String", "fromCharCode");
const NUMBER_TO_STRING: Native = ("Number.prototype", "toString");

#[derive(Clone, Copy)]
pub(super) enum Variant {
    Plain,
    Probed,
    Flagged,
}

#[derive(Clone, Copy)]
struct Roles {
    input: usize,
    alphabet: usize,
    output: usize,
    temp: usize,
    text: Option<usize>,
    flag: Option<usize>,
    string: usize,
}

const fn roles(variant: Variant) -> Roles {
    let (text, flag, string) = match variant {
        Variant::Plain => (None, None, 8),
        Variant::Probed => (Some(4), None, 10),
        Variant::Flagged => (Some(4), Some(6), 11),
    };
    Roles {
        input: 0,
        alphabet: 1,
        output: 2,
        temp: 3,
        text,
        flag,
        string,
    }
}

#[derive(Clone, Copy)]
struct Part {
    total: u64,
    get: u64,
}

#[derive(Clone, Copy)]
struct CharPath {
    total: u64,
    string: u64,
    from: u64,
}

#[derive(Clone, Copy)]
struct Emit {
    char: CharPath,
    count: Option<(u64, u64)>,
}

pub(super) struct Decode<'a> {
    input: u32,
    alphabet: u32,
    output: u32,
    text: Option<u32>,
    flag: Option<u32>,
    string: LazyExpr<'a>,
    head: u64,
    test: Part,
    call: u64,
    body: Part,
    miss: u64,
    phase: u64,
    emits: [Option<Emit>; 2],
}

pub(super) struct Percent {
    source: u32,
    output: u32,
    head: u64,
    test: u64,
    body: u64,
    update: u64,
    gets: [u64; 3],
    call: u64,
}

pub(super) fn walked<'a, 'p>(
    path: &'p [bool],
    walk: impl FnOnce(&mut Walk<'a, 'p>) -> Option<()>,
) -> Option<Walk<'a, 'p>> {
    let mut walker = Walk::new(path);
    walk(&mut walker)?;
    walker.finished()
}

fn char_path<'a>(
    node: &'a ForStatement<'a>,
    path: &[bool],
    string: &str,
) -> Option<(CharPath, Option<u64>)> {
    let update = node.update.as_ref()?;
    let walk = walked(path, |walk| walk.expression(update))?;
    let char = CharPath {
        total: walk.total,
        string: walk.at(Event::Read(string))?,
        from: walk.at(Event::Get("fromCharCode"))?,
    };
    (walk.count(Event::Concat) == 1).then_some((char, walk.at(Event::Get("charCodeAt"))))
}

fn count_path<'a>(node: &'a ForStatement<'a>, path: &[bool]) -> Option<(u64, u64)> {
    let update = node.update.as_ref()?;
    let walk = walked(path, |walk| walk.expression(update))?;
    let probe = walk.at(Event::Get("charCodeAt"))?;
    (walk.count(Event::Concat) == 1).then_some((walk.total, probe))
}

fn probed_emit<'a>(node: &'a ForStatement<'a>, prefix: &[bool], string: &str) -> Option<Emit> {
    let mut on = prefix.to_vec();
    on.push(true);
    let mut off = prefix.to_vec();
    off.push(false);
    let (char, probe) = char_path(node, &on, string)?;
    let count = count_path(node, &off)?;
    (probe == Some(count.1)).then_some(Emit {
        char,
        count: Some(count),
    })
}

fn emits<'a>(
    node: &'a ForStatement<'a>,
    variant: Variant,
    string: &str,
) -> Option<[Option<Emit>; 2]> {
    const LEAD: [bool; 3] = [true, true, true];
    match variant {
        Variant::Plain => {
            let (char, probe) = char_path(node, &LEAD, string)?;
            probe
                .is_none()
                .then_some([Some(Emit { char, count: None }), None])
        }
        Variant::Probed => Some([Some(probed_emit(node, &LEAD, string)?), None]),
        Variant::Flagged => {
            let off = probed_emit(node, &[true, true, true, true], string)?;
            let (char, probe) = char_path(node, &[true, true, true, false, true], string)?;
            probe
                .is_none()
                .then_some([Some(off), Some(Emit { char, count: None })])
        }
    }
}

fn head_cost<'a>(node: &'a ForStatement<'a>) -> Option<u64> {
    let walk = walked(&[], |walk| walk.for_init(node.init.as_ref()))?;
    Some(1 + walk.total)
}

fn decode_parts<'a>(node: &'a ForStatement<'a>) -> Option<(Part, u64, Part, u64, u64)> {
    let test = walked(&[], |walk| walk.expression(node.test.as_ref()?))?;
    let body = walked(&[], |walk| walk.statement(&node.body))?;
    let update = node.update.as_ref()?;
    let miss = walked(&[false, false], |walk| walk.expression(update))?;
    let phase = walked(&[true, false, false], |walk| walk.expression(update))?;
    let quiet = [&test, &body, &miss, &phase]
        .iter()
        .all(|walk| walk.count(Event::Concat) == 0);
    quiet.then_some((
        Part {
            total: test.total,
            get: test.at(Event::Get("charAt"))?,
        },
        test.at(Event::Call)?,
        Part {
            total: body.total,
            get: body.at(Event::Get("indexOf"))?,
        },
        miss.total,
        phase.total,
    ))
}

fn plan_decode<'a>(cx: &mut Planning<'_, 'a>, at: usize, variant: Variant) -> Option<Decode<'a>> {
    let node = cx.for_statement(at)?;
    let roles = roles(variant);
    let string_name = *cx.shape.names.get(roles.string)?;
    let (test, call, body, miss, phase) = decode_parts(node)?;
    Some(Decode {
        input: cx.slot(at, roles.input)?,
        alphabet: cx.slot(at, roles.alphabet)?,
        output: cx.slot(at, roles.output)?,
        text: cx.optional_slot(at, roles.text)?,
        flag: cx.optional_slot(at, roles.flag)?,
        string: cx.read(at, roles.string)?,
        head: head_cost(node)?,
        test,
        call,
        body,
        miss,
        phase,
        emits: emits(node, variant, string_name)?,
    })
}

fn plan_percent<'a>(cx: &mut Planning<'_, 'a>, at: usize, variant: Variant) -> Option<Percent> {
    let node = cx.for_statement(at)?;
    let roles = roles(variant);
    let test = walked(&[], |walk| walk.expression(node.test.as_ref()?))?;
    let body = walked(&[], |walk| walk.statement(&node.body))?;
    let update = walked(&[], |walk| walk.expression(node.update.as_ref()?))?;
    let calls = (body.count(Event::Call), body.count(Event::Concat));
    if calls != (3, 3) || test.count(Event::Call) + update.count(Event::Call) > 0 {
        return None;
    }
    Some(Percent {
        source: cx.slot(at, roles.output)?,
        output: cx.slot(at, roles.temp)?,
        head: head_cost(node)?,
        test: test.total,
        body: body.total,
        update: update.total,
        gets: [
            body.at(Event::Get("charCodeAt"))?,
            body.at(Event::Get("toString"))?,
            body.at(Event::Get("slice"))?,
        ],
        call: body.first(Event::Call)?,
    })
}

pub(super) fn plan<'a>(cx: &mut Planning<'_, 'a>) -> Option<Box<[Option<Step<'a>>]>> {
    let variant = match cx.shape.text.as_str() {
        PLAIN => Variant::Plain,
        SELF_TEXT => Variant::Probed,
        FLAGGED => Variant::Flagged,
        _ => return None,
    };
    let loops = cx.loops();
    let [decode, percent] = loops.as_slice() else {
        return None;
    };
    let mut steps = cx.no_steps();
    steps[*decode] = Some(Step(Kind::Decode(plan_decode(cx, *decode, variant)?)));
    steps[*percent] = Some(Step(Kind::Percent(plan_percent(cx, *percent, variant)?)));
    Some(steps)
}

const NOT_FOUND: u8 = u8::MAX;
const HEX: &[u8; 16] = b"0123456789abcdef";

const fn alphabet_table() -> [u8; 128] {
    let mut table = [NOT_FOUND; 128];
    let bytes = ALPHABET.as_bytes();
    let mut index = 0;
    while index < bytes.len() {
        table[bytes[index] as usize] = index as u8;
        index += 1;
    }
    table
}

const ALPHABET_INDEX: [u8; 128] = alphabet_table();

fn alphabet_index(unit: u16) -> Option<u32> {
    let found = *ALPHABET_INDEX.get(usize::from(unit))?;
    (found != NOT_FOUND).then_some(u32::from(found))
}

fn hex_digits(unit: u16) -> usize {
    match unit {
        0..=0xf => 1,
        0x10..=0xff => 2,
        0x100..=0xfff => 3,
        _ => 4,
    }
}

fn text_bytes(units: usize) -> u64 {
    96 + 2 * units as u64
}

struct Decoding {
    input: JsString,
    text: Option<JsString>,
    emit: Emit,
    out: Vec<u16>,
    count: f64,
    bits: f64,
    checks: [bool; 4],
}

impl<'a> Decode<'a> {
    fn state(&self, env: &FrameRef<'a>) -> Option<Decoding> {
        let alphabet = text_slot(env, self.alphabet)?;
        if alphabet.units().iter().copied().ne(ALPHABET.encode_utf16()) {
            return None;
        }
        let text = match self.text {
            Some(slot) => Some(text_slot(env, slot)?),
            None => None,
        };
        let flagged = match self.flag {
            Some(slot) => match super::exec::read_slot(env, slot)? {
                Value::Bool(flag) => flag,
                _ => return None,
            },
            None => false,
        };
        let emit = self.emits[usize::from(flagged)]?;
        Some(Decoding {
            input: text_slot(env, self.input)?,
            text,
            emit,
            out: text_slot(env, self.output)?.units().to_vec(),
            count: 0.0,
            bits: 0.0,
            checks: [false; 4],
        })
    }
}

impl<'a> Interp<'a> {
    pub(super) fn run_decode(
        &mut self,
        plan: &Decode<'a>,
        at: (&FrameRef<'a>, Reach),
    ) -> Eval<'a, bool> {
        let Some(mut state) = plan.state(at.0) else {
            return Ok(false);
        };
        if !writable(at.0, &[plan.output]) {
            return Ok(false);
        }
        let len = state.input.len();
        let transient = 4 * text_bytes(state.out.len() + len)
            + len as u64 * text_bytes(1)
            + len.min(UNIT_PAGES) as u64 * PAGE_BOUND;
        if !self.has_room(len, transient) {
            return Ok(false);
        }
        let before = state.out.len();
        let mut run = self.begin();
        run.owe(plan.head);
        if !self.decode_units(plan, &mut state, &mut run, at)? {
            self.refund(&run);
            return Ok(false);
        }
        self.settle(&mut run)?;
        if state.out.len() == before {
            return Ok(true);
        }
        let output = self.charged_text(state.out)?;
        commit(at.0, plan.output, Value::Str(output))?;
        Ok(true)
    }

    fn decode_units(
        &mut self,
        plan: &Decode<'a>,
        state: &mut Decoding,
        run: &mut Run,
        at: (&FrameRef<'a>, Reach),
    ) -> Eval<'a, bool> {
        let input = state.input.clone();
        for position in 0..=input.len() {
            let unit = input.code_unit_at(position);
            if !self.decode_test(plan, state, run, unit)? {
                return Ok(false);
            }
            let Some(unit) = unit else {
                return Ok(true);
            };
            if !self.decode_body(plan, state, run)? {
                return Ok(false);
            }
            let Some(found) = alphabet_index(unit) else {
                run.owe(plan.miss);
                continue;
            };
            if !self.decode_found(plan, state, run, (found, position), at)? {
                return Ok(false);
            }
        }
        Ok(true)
    }

    fn decode_test(
        &mut self,
        plan: &Decode<'a>,
        state: &mut Decoding,
        run: &mut Run,
        unit: Option<u16>,
    ) -> Eval<'a, bool> {
        run.part();
        if !state.checks[0] {
            let receiver = Value::Str(state.input.clone());
            let found = self.intact_member(run, (&receiver, "charAt", plan.test.get), CHAR_AT)?;
            if !found {
                return Ok(false);
            }
            state.checks[0] = true;
        }
        self.first_call(run, plan.call)?;
        if let Some(unit) = unit {
            run.to(plan.call);
            self.settle(run)?;
            self.realm.unit_string(unit)?;
        }
        run.end(plan.test.total);
        Ok(true)
    }

    fn decode_body(
        &mut self,
        plan: &Decode<'a>,
        state: &mut Decoding,
        run: &mut Run,
    ) -> Eval<'a, bool> {
        run.part();
        if !state.checks[1] {
            let receiver = Value::Str(JsString::from_str(ALPHABET));
            let found = self.intact_member(run, (&receiver, "indexOf", plan.body.get), INDEX_OF)?;
            if !found {
                return Ok(false);
            }
            state.checks[1] = true;
        }
        run.end(plan.body.total);
        run.owe(scan_charge(ALPHABET.len(), 1));
        Ok(true)
    }

    fn decode_found(
        &mut self,
        plan: &Decode<'a>,
        state: &mut Decoding,
        run: &mut Run,
        (found, position): (u32, usize),
        at: (&FrameRef<'a>, Reach),
    ) -> Eval<'a, bool> {
        let phase = state.count % 4.0;
        state.bits = match phase != 0.0 {
            true => state.bits * 64.0 + f64::from(found),
            false => f64::from(found),
        };
        state.count += 1.0;
        if phase == 0.0 {
            run.owe(plan.phase);
            return Ok(true);
        }
        run.part();
        match self.decode_counted(state, run, position)? {
            Some(true) => Ok(true),
            Some(false) => self.decode_char(plan, state, run, at),
            None => Ok(false),
        }
    }

    fn decode_counted(
        &mut self,
        state: &mut Decoding,
        run: &mut Run,
        position: usize,
    ) -> Eval<'a, Option<bool>> {
        let (Some(text), Some((total, probe))) = (state.text.clone(), state.emit.count) else {
            return Ok(Some(false));
        };
        if !state.checks[2] {
            let receiver = Value::Str(text.clone());
            let found = self.intact_member(run, (&receiver, "charCodeAt", probe), CHAR_CODE_AT)?;
            if !found {
                return Ok(None);
            }
            state.checks[2] = true;
        }
        let code = text
            .code_unit_at(position + PROBE_OFFSET)
            .map_or(f64::NAN, f64::from);
        if code - PROBE_UNIT != 0.0 {
            return Ok(Some(false));
        }
        let digits = number_to_js_string(state.count);
        run.end(total);
        run.owe(concat_charge(state.out.len(), digits.len()));
        state.out.extend_from_slice(digits.units());
        Ok(Some(true))
    }

    fn decode_char(
        &mut self,
        plan: &Decode<'a>,
        state: &mut Decoding,
        run: &mut Run,
        at: (&FrameRef<'a>, Reach),
    ) -> Eval<'a, bool> {
        let path = state.emit.char;
        if !state.checks[3] {
            let string = self.read_lazy(run, (&plan.string, path.string), at)?;
            let found =
                self.intact_member(run, (&string, "fromCharCode", path.from), FROM_CHAR_CODE)?;
            if !found {
                return Ok(false);
            }
            state.checks[3] = true;
        }
        let shift = (to_int32(-2.0 * state.count) & 6) as u32;
        let byte = (255 & (to_int32(state.bits) >> shift)) as u16;
        run.end(path.total);
        run.owe(concat_charge(state.out.len(), 1));
        state.out.push(byte);
        Ok(true)
    }

    pub(super) fn run_percent(
        &mut self,
        plan: &Percent,
        at: (&FrameRef<'a>, Reach),
    ) -> Eval<'a, bool> {
        let (Some(source), Some(output)) =
            (text_slot(at.0, plan.source), text_slot(at.0, plan.output))
        else {
            return Ok(false);
        };
        let len = source.len();
        if !writable(at.0, &[plan.output])
            || !self.has_room(len, 4 * text_bytes(output.len() + 3 * len))
        {
            return Ok(false);
        }
        let mut out = output.units().to_vec();
        out.reserve(3 * len);
        let mut run = self.begin();
        run.owe(plan.head);
        for (index, unit) in source.units().iter().enumerate() {
            run.owe(plan.test);
            if index == 0 && !self.percent_checks(plan, &mut run, (&source, *unit))? {
                self.refund(&run);
                return Ok(false);
            }
            if index > 0 {
                run.owe(plan.body + concat_charge(2, hex_digits(*unit)));
            }
            run.owe(length_charge(2) + concat_charge(1, 2) + concat_charge(out.len(), 3));
            let low = usize::from(*unit & 0xff);
            out.extend([b'%', HEX[low >> 4], HEX[low & 0xf]].map(u16::from));
            run.owe(plan.update);
        }
        run.owe(plan.test);
        self.settle(&mut run)?;
        if len == 0 {
            return Ok(true);
        }
        let output = self.charged_text(out)?;
        commit(at.0, plan.output, Value::Str(output))?;
        Ok(true)
    }

    fn percent_checks(
        &mut self,
        plan: &Percent,
        run: &mut Run,
        (source, unit): (&JsString, u16),
    ) -> Eval<'a, bool> {
        run.part();
        let text = Value::Str(source.clone());
        if !self.intact_member(run, (&text, "charCodeAt", plan.gets[0]), CHAR_CODE_AT)? {
            return Ok(false);
        }
        self.first_call(run, plan.call)?;
        let code = Value::Number(f64::from(unit));
        if !self.intact_member(run, (&code, "toString", plan.gets[1]), NUMBER_TO_STRING)? {
            return Ok(false);
        }
        let hex = number_to_radix_string(f64::from(unit), 16);
        run.owe(concat_charge(2, hex_digits(unit)));
        let piece = Value::Str(JsString::from_str(&format!("00{hex}")));
        if !self.intact_member(run, (&piece, "slice", plan.gets[2]), SLICE)? {
            return Ok(false);
        }
        run.end(plan.body);
        Ok(true)
    }
}
