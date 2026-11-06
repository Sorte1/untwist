use std::collections::hash_map::DefaultHasher;
use std::hash::Hasher;

use oxc_ast::ast::Program;
use untwist_core::clock::StageInstant as Instant;
use untwist_core::depth::{
    limits_for, print_estimate, scan, DepthRefusal, Limits, UNIT_STACK_BYTES,
};
use untwist_core::pass::{InputRoom, Note, NoteKind, StopReason};
use untwist_core::semantic::Semantics;
use untwist_deobfuscate::layers::notes::{peeled_message, refused_message};
use untwist_deobfuscate::layers::{
    self, DecodeCx, Decoded, Layer, LayerForm, Probe, Refusal, Refused, FUEL_LAYERS, LAYER_CAP,
    LAYER_PASS, TEXT_COPIES,
};
use untwist_eval::budget::{heap_budget, STEP_BUDGET, STRING_LEN_CAP};
use untwist_eval::stack::room;
use untwist_eval::stack_reserve::DECODED_TEXT_STACK_BYTES;

use super::stages::{round_of, StageCx};
use super::Pipeline;
use crate::report::StageReport;

const MIB: u64 = 1 << 20;

#[derive(Clone, Copy, Debug)]
pub(super) struct RoomSeed {
    print: u64,
    nesting: u32,
}

impl RoomSeed {
    pub(super) fn of(room: &InputRoom) -> Self {
        Self {
            print: room.print(),
            nesting: room.nesting(),
        }
    }

    pub(super) fn room(self) -> InputRoom {
        InputRoom::new(self.print, self.nesting)
    }
}

#[derive(Clone, Copy)]
pub(super) struct Counters {
    peeled: u32,
    fuel_left: u64,
    heap_left: u64,
    hashes: usize,
}

pub(super) struct Peeled {
    pub(super) text: String,
    pub(super) room: RoomSeed,
    pub(super) form: LayerForm,
}

pub(super) struct Layering {
    input_len: usize,
    reserved_units: u32,
    fuel: u64,
    heap: u64,
    counters: Counters,
    hashes: Vec<u64>,
    kept: Vec<(u64, Refusal)>,
    runs: u32,
}

pub(super) fn reserved_units(reserved_bytes: u64) -> u32 {
    let unit = u64::try_from(UNIT_STACK_BYTES).unwrap_or(u64::MAX);
    let held = reserved_bytes
        .saturating_sub(DECODED_TEXT_STACK_BYTES)
        .max(MIB);
    u32::try_from(held / unit).unwrap_or(u32::MAX)
}

fn text_hash(text: &str) -> u64 {
    let mut hasher = DefaultHasher::new();
    hasher.write(text.trim().as_bytes());
    hasher.finish()
}

fn over_unit_cap(text: &str) -> bool {
    text.len() > STRING_LEN_CAP && text.encode_utf16().count() > STRING_LEN_CAP
}

fn depth_refusal(refusal: DepthRefusal) -> Refusal {
    match refusal {
        DepthRefusal::Nesting | DepthRefusal::SpeculativeParens => Refusal::TooDeep,
        DepthRefusal::PrintSize => Refusal::TooLarge,
        DepthRefusal::Typescript => Refusal::Unparsable,
    }
}

impl Layering {
    pub(super) fn new(input_len: usize, reserved_bytes: u64, fuel: u64, heap: u64) -> Self {
        Self {
            input_len,
            reserved_units: reserved_units(reserved_bytes),
            fuel,
            heap,
            counters: Counters {
                peeled: 0,
                fuel_left: fuel.saturating_mul(FUEL_LAYERS),
                heap_left: heap,
                hashes: 0,
            },
            hashes: Vec::new(),
            kept: Vec::new(),
            runs: 0,
        }
    }

    pub(super) fn peeled(&self) -> u32 {
        self.counters.peeled
    }

    pub(super) fn start_run(&mut self) -> bool {
        let rerun = self.runs > 0;
        self.runs = self.runs.saturating_add(1);
        rerun
    }

    pub(super) fn mark(&self) -> Counters {
        self.counters
    }

    pub(super) fn restore(&mut self, counters: Counters) {
        self.hashes.truncate(counters.hashes);
        self.counters = counters;
    }

    pub(super) fn keep(&mut self, text: &str, reason: Refusal) {
        self.kept.push((text_hash(text), reason));
    }

    fn cx(&self, unsafe_transforms: bool) -> DecodeCx {
        DecodeCx {
            unsafe_transforms,
            fuel: self.fuel.min(self.counters.fuel_left),
            heap: self.counters.heap_left,
        }
    }

    fn admit(&mut self, source: &str, layer: Layer) -> Result<(String, RoomSeed), Refused> {
        let refused = |reason| Refused {
            form: layer.form,
            reason,
        };
        if self.counters.peeled >= LAYER_CAP {
            return Err(refused(Refusal::Cap));
        }
        if layer.steps > self.fuel || layer.steps > self.counters.fuel_left {
            return Err(refused(Refusal::Fuel));
        }
        if self.hashes.is_empty() {
            self.hashes.push(text_hash(source));
        }
        let hash = text_hash(&layer.text);
        if let Some(&(_, reason)) = self.kept.iter().find(|(kept, _)| *kept == hash) {
            return Err(refused(reason));
        }
        if self.hashes.contains(&hash) {
            return Err(refused(Refusal::Cycle));
        }
        let room = self.readmit(&layer.text).map_err(refused)?;
        self.counters.fuel_left -= layer.steps;
        self.counters.heap_left = self.counters.heap_left.saturating_sub(layer.heap);
        self.counters.peeled += 1;
        self.hashes.push(hash);
        self.counters.hashes = self.hashes.len();
        Ok((layer.text, room))
    }

    fn readmit(&self, text: &str) -> Result<RoomSeed, Refusal> {
        let len = text.len();
        let cap = self.heap / TEXT_COPIES;
        if u64::try_from(len).unwrap_or(u64::MAX) > cap || over_unit_cap(text) {
            return Err(Refusal::TooLarge);
        }
        let stack = room();
        let base = limits_for(stack, len);
        let print = base
            .print_bytes
            .min(limits_for(stack, self.input_len).print_bytes);
        let limits = Limits {
            nesting: base.nesting.min(self.reserved_units),
            print_bytes: print,
            ..base
        };
        let found = scan(text, &limits).map_err(depth_refusal)?;
        Ok(RoomSeed {
            print: print.saturating_sub(print_estimate(&found, len)),
            nesting: found.nesting,
        })
    }
}

fn layer_note(kind: NoteKind, message: String) -> Note {
    Note {
        pass: LAYER_PASS,
        kind,
        message,
        span: None,
    }
}

fn peel_report(stages: &[StageReport], started: Instant) -> StageReport {
    let elapsed = started.elapsed();
    StageReport {
        stage: LAYER_PASS,
        round: round_of(stages, LAYER_PASS),
        iterations: 1,
        changes: 1,
        per_pass: vec![(LAYER_PASS, 1)],
        per_pass_elapsed: vec![(LAYER_PASS, elapsed)],
        fixpoint_per_pass: Vec::new(),
        fixpoint_per_pass_elapsed: Vec::new(),
        iteration_elapsed: Vec::new(),
        elapsed,
        stop: StopReason::Converged,
        #[cfg(feature = "test-hooks")]
        work: Vec::new(),
    }
}

fn note_refused(site: &mut StageCx<'_, '_>, refused: Refused, index: u32) {
    let message = refused_message(refused.reason, index, refused.form.name());
    if site.notes.iter().any(|note| note.message == message) {
        return;
    }
    site.notes.push(layer_note(NoteKind::Refusal, message));
}

impl Pipeline {
    pub(super) fn layer_budget(&self, input_len: usize) -> (u64, u64) {
        let fuel = self.options.fuel.unwrap_or(STEP_BUDGET);
        let heap = self
            .options
            .hooks()
            .heap_budget
            .unwrap_or_else(|| heap_budget(input_len));
        (fuel, heap)
    }

    fn layers_enabled(&self) -> bool {
        self.options.deobfuscate && self.options.extended() && !self.disabled(LAYER_PASS)
    }

    fn disabled(&self, pass: &str) -> bool {
        self.options.disabled_passes.contains(&pass)
    }

    pub(super) fn peel(
        &self,
        site: &mut StageCx<'_, '_>,
        sem: &Semantics,
        layering: &mut Layering,
    ) -> Option<Peeled> {
        if !self.layers_enabled() {
            return None;
        }
        let program: &Program<'_> = site.program;
        let probe = layers::probe(program)?;
        let index = layering.peeled().saturating_add(1);
        let started = self.begin(LAYER_PASS);
        let cx = layering.cx(self.options.unsafe_transforms);
        let decoded = self.decode_layer(program, sem, probe, &cx)?;
        let source = program.source_text;
        let bytes_in = source.len();
        let admitted = decoded.and_then(|layer| {
            let (form, heap) = (layer.form, layer.heap);
            layering
                .admit(source, layer)
                .map(|(text, room)| (text, room, form, heap))
        });
        let (text, room, form, heap) = match admitted {
            Ok(admitted) => admitted,
            Err(refused) => {
                note_refused(site, refused, index);
                return None;
            }
        };
        *site.heap_peak = (*site.heap_peak).max(heap);
        let message = peeled_message(index, form.name(), bytes_in, text.len());
        site.notes.push(layer_note(NoteKind::Progress, message));
        site.stages.push(peel_report(site.stages, started));
        Some(Peeled { text, room, form })
    }

    #[cfg(feature = "test-hooks")]
    fn decode_layer(
        &self,
        program: &Program<'_>,
        sem: &Semantics,
        probe: Probe<'_, '_>,
        cx: &DecodeCx,
    ) -> Decoded {
        match self.options.hooks().layer_stub {
            Some(table) => stub_decode(table, probe),
            None => layers::decode(program, &sem.scoping, probe, cx),
        }
    }

    #[cfg(not(feature = "test-hooks"))]
    fn decode_layer(
        &self,
        program: &Program<'_>,
        sem: &Semantics,
        probe: Probe<'_, '_>,
        cx: &DecodeCx,
    ) -> Decoded {
        layers::decode(program, &sem.scoping, probe, cx)
    }
}

#[cfg(feature = "test-hooks")]
fn stub_decode(table: &[(&str, &str)], probe: Probe<'_, '_>) -> Decoded {
    use oxc_ast::ast::{Argument, Expression};
    use untwist_deobfuscate::layers::LayerForm;
    if probe.form != LayerForm::Eval {
        return None;
    }
    let Expression::CallExpression(call) = probe.expression.without_parentheses() else {
        return None;
    };
    let [Argument::StringLiteral(literal)] = call.arguments.as_slice() else {
        return None;
    };
    let key = literal.value.as_str()?;
    let text = table
        .iter()
        .find(|(from, _)| *from == key)
        .map_or(key, |(_, to)| *to)
        .to_string();
    let steps = u64::try_from(text.len()).unwrap_or(u64::MAX);
    Some(Ok(Layer {
        form: LayerForm::Eval,
        text,
        steps,
        heap: 0,
    }))
}

#[cfg(test)]
mod tests {
    use super::{reserved_units, Layering, MIB};
    use untwist_core::depth::UNIT_STACK_BYTES;
    use untwist_eval::stack_reserve::DECODED_TEXT_STACK_BYTES;

    const UNIT: u64 = UNIT_STACK_BYTES as u64;

    #[test]
    fn the_reserved_units_are_the_admitted_stack_less_the_decoded_text_headroom() {
        let headroom = DECODED_TEXT_STACK_BYTES;
        let floor = u32::try_from(MIB / UNIT).unwrap_or(u32::MAX);
        assert_eq!(reserved_units(MIB + headroom), floor);
        assert_eq!(reserved_units(MIB), floor);
        assert_eq!(reserved_units(0), floor);
        assert_eq!(reserved_units(600 * UNIT + headroom), 600);
        assert_eq!(floor, if cfg!(debug_assertions) { 128 } else { 256 });
    }

    #[test]
    fn a_layer_over_the_text_budget_or_the_reserved_nesting_is_refused() {
        let layering = Layering::new(10, MIB + DECODED_TEXT_STACK_BYTES, 100, 4_000);
        assert!(layering.readmit(&"a".repeat(1_000)).is_ok());
        assert!(layering.readmit(&"a".repeat(1_001)).is_err());
        let units = reserved_units(MIB);
        let fits = format!(
            "{}0{}",
            "[".repeat(units as usize / 2 - 4),
            "]".repeat(units as usize / 2 - 4)
        );
        let deep = format!(
            "{}0{}",
            "[".repeat(units as usize / 2 + 1),
            "]".repeat(units as usize / 2 + 1)
        );
        let roomy = Layering::new(10, MIB, 100, u64::MAX);
        let verdicts = std::thread::Builder::new()
            .stack_size(64 << 20)
            .spawn(move || (roomy.readmit(&fits).is_ok(), roomy.readmit(&deep).is_err()))
            .and_then(|worker| worker.join().map_err(|_| std::io::Error::other("panicked")));
        assert_eq!(verdicts.ok(), Some((true, true)));
    }
}
