use std::borrow::Cow;
use std::panic::{catch_unwind, AssertUnwindSafe};

use untwist_core::depth::{limits_for, print_estimate, scan, DepthRefusal, UNIT_STACK_BYTES};
use untwist_core::parse::strip_bookmarklet;
use untwist_core::pass::{InputRoom, Note, NoteKind};
use untwist_core::source_map::OwnedSourceMap;
use untwist_eval::stack::room;
use untwist_eval::stack_reserve::{
    StackReservation, StackReserve, DECODED_TEXT_STACK_BYTES, STACK_RESERVE_BYTES,
};

use super::boundary::Boundary;
use super::input::{FinalTextGuard, InputResult};
use super::ladder::{on_a_worker_stack, Ladder};
use super::unpack::Prior;
use super::{Output, OutputParts, Pipeline, BUDGET_UNPACK, UNPACK_PANICKED};
use crate::bundle::BundleOutput;
use crate::options::Progress;
use untwist_core::budget::{Hit, Limits};

pub const NESTED_TOO_DEEP: &str = "this input nests deeper than untwist can parse on its stack; \
     it is passed through unchanged";

pub const PARENTHESES_TOO_DEEP: &str = "this input nests parenthesised heads so deeply that \
     parsing it would take memory quadratic in their depth; it is passed through unchanged";

pub const PRINT_TOO_LARGE: &str = "this input nests statements so deeply that its printed \
     output would exceed the memory budget; it is passed through unchanged";

pub const UNPACK_TEXT_TOO_DEEP: &str = "a final text nests too deeply to parse again for \
     unpacking; the inputs are written without a bundle";

pub const TYPESCRIPT_IN_JAVASCRIPT: &str = "this input uses TypeScript syntax that untwist does \
     not read in JavaScript; it is passed through unchanged";

pub const WORKER_DIED: &str = "processing this input stopped the worker process, which ran out \
     of stack or memory; it is passed through unchanged";

pub const OVER_MEMORY_REFUSED: &str = "budget-memory: processing this input went over the memory \
     budget (--max-memory) before its passes could stop; it is passed through unchanged";

pub const OVER_TIME_REFUSED: &str = "budget-time: processing this input ran past the time budget \
     (--timeout) before its passes could stop; it is passed through unchanged";

pub const TOO_LARGE_FOR_THE_BUDGET: &str = "budget-memory: parsing this input alone would take \
     more than the memory budget (--max-memory); it is passed through unchanged";

pub const PARSE_BYTES_PER_INPUT_BYTE: u64 = 16;

pub const ADMIT_PANICKED: &str = "reading this input before its parse hit an internal error; it \
     is passed through unchanged";

const MIB: u64 = 1 << 20;

const ADMIT_PANIC_HOOK: &str = "a test hook panicked in admit";

const INPUT_REFUSALS: [&str; 9] = [
    NESTED_TOO_DEEP,
    PARENTHESES_TOO_DEEP,
    PRINT_TOO_LARGE,
    TYPESCRIPT_IN_JAVASCRIPT,
    WORKER_DIED,
    OVER_MEMORY_REFUSED,
    OVER_TIME_REFUSED,
    TOO_LARGE_FOR_THE_BUDGET,
    ADMIT_PANICKED,
];

type Admitted<'p> = Result<(StackReservation<'p>, Option<InputRoom>), &'static str>;

impl Output {
    pub fn depth_refusals(&self) -> Vec<(usize, &str)> {
        let texts = self.notes.iter().filter(|note| {
            note.pass == "pipeline" && INPUT_REFUSALS.contains(&note.message.as_str())
        });
        self.depth_refused
            .iter()
            .zip(texts)
            .map(|(index, note)| (*index, note.message.as_str()))
            .collect()
    }
}

pub(super) fn refusal_note(refusal: DepthRefusal) -> &'static str {
    match refusal {
        DepthRefusal::Nesting => NESTED_TOO_DEEP,
        DepthRefusal::SpeculativeParens => PARENTHESES_TOO_DEEP,
        DepthRefusal::PrintSize => PRINT_TOO_LARGE,
        DepthRefusal::Typescript => TYPESCRIPT_IN_JAVASCRIPT,
    }
}

fn pipeline_refusal(message: &'static str) -> Note {
    Note {
        pass: "pipeline",
        kind: NoteKind::Refusal,
        message: message.to_string(),
        span: None,
    }
}

fn stack_request(nesting: u32) -> u64 {
    let unit = u64::try_from(UNIT_STACK_BYTES).unwrap_or(u64::MAX);
    unit.saturating_mul(u64::from(nesting))
        .max(MIB)
        .saturating_add(DECODED_TEXT_STACK_BYTES)
        .min(STACK_RESERVE_BYTES)
}

impl InputResult {
    pub(super) fn refused(source: &str, message: &'static str) -> Self {
        InputResult {
            code: source.to_string(),
            notes: vec![pipeline_refusal(message)],
            refused: true,
            ..InputResult::default()
        }
    }
}

impl Pipeline {
    pub(super) fn admit_guarded<'s, 'p>(
        &self,
        index: usize,
        raw: &'s str,
        stack: &StackReserve<'p>,
    ) -> (Cow<'s, str>, Admitted<'p>) {
        let _boundary = Boundary::enter();
        let outcome = catch_unwind(AssertUnwindSafe(|| {
            let source = strip_bookmarklet(raw);
            let admitted = self.admit(index, &source, stack);
            (source, admitted)
        }));
        outcome.unwrap_or_else(|_| {
            stack.give_up(index);
            (Cow::Borrowed(raw), Err(ADMIT_PANICKED))
        })
    }

    fn parse_exceeds_the_budget(&self, input_len: usize) -> bool {
        let needed = u64::try_from(input_len)
            .unwrap_or(u64::MAX)
            .saturating_mul(PARSE_BYTES_PER_INPUT_BYTE);
        self.options.max_memory.is_some_and(|limit| needed > limit)
    }

    fn died_of(&self, index: usize) -> &'static str {
        let over = self.options.isolation.over_budget.iter();
        match over
            .filter(|(at, _)| *at == index)
            .map(|(_, hit)| *hit)
            .next()
        {
            Some(Hit::Memory) => OVER_MEMORY_REFUSED,
            Some(Hit::Time) => OVER_TIME_REFUSED,
            None => WORKER_DIED,
        }
    }

    fn admit<'p>(&self, index: usize, source: &str, stack: &StackReserve<'p>) -> Admitted<'p> {
        if self.options.hooks().admit_panic == Some(index) {
            std::panic::panic_any(ADMIT_PANIC_HOOK);
        }
        if self.options.isolation.refused.contains(&index) {
            stack.give_up(index);
            return Err(self.died_of(index));
        }
        if self.parse_exceeds_the_budget(source.len()) {
            stack.give_up(index);
            return Err(TOO_LARGE_FOR_THE_BUDGET);
        }
        if self.options.hooks().scan_off {
            return Ok((stack.reserve(index, MIB), None));
        }
        let limits = limits_for(room(), source.len());
        match scan(source, &limits) {
            Ok(found) => {
                let print_room = limits
                    .print_bytes
                    .saturating_sub(print_estimate(&found, source.len()));
                Ok((
                    stack.reserve(index, stack_request(found.nesting)),
                    Some(InputRoom::new(print_room, found.nesting)),
                ))
            }
            Err(refusal) => {
                stack.give_up(index);
                Err(refusal_note(refusal))
            }
        }
    }

    pub(super) fn unpack_budgeted(
        &self,
        parts: &mut OutputParts,
        limits: Limits,
    ) -> Option<BundleOutput> {
        let budget = untwist_core::budget::enter(limits);
        let ready = untwist_core::budget::check().is_ok();
        let bundle = ready.then(|| self.unpack_admitted(parts)).flatten();
        let spent = untwist_core::budget::hit();
        parts.memory_peak = parts.memory_peak.max(budget.peak());
        drop(budget);
        if spent.is_none() || !self.options.unpack {
            return bundle;
        }
        parts.notes.push(pipeline_refusal(BUDGET_UNPACK));
        None
    }

    pub(super) fn unpack_admitted(&self, parts: &mut OutputParts) -> Option<BundleOutput> {
        let admitted = parts.codes.len() - parts.depth_refused.len();
        if !self.options.unpack || admitted == 0 {
            return None;
        }
        if self.options.isolation.unpack_lost {
            parts.unpack_panicked = true;
            parts.notes.push(pipeline_refusal(UNPACK_PANICKED));
            return None;
        }
        self.options.progress(Progress::UnpackBegins);
        let (stages, notes, panicked) = (
            &mut parts.stages,
            &mut parts.notes,
            &mut parts.unpack_panicked,
        );
        let guard = (FinalTextGuard::PremiseP1, parts.unbundled);
        if parts.depth_refused.is_empty() {
            let prior = Prior {
                guards: &parts.guards,
                maps: &parts.source_maps,
            };
            return self.unpack_final_texts(&parts.codes, prior, guard, stages, notes, panicked);
        }
        let mut texts: Vec<&str> = Vec::with_capacity(admitted);
        texts.extend(
            parts
                .codes
                .iter()
                .enumerate()
                .filter(|(index, _)| !parts.depth_refused.contains(index))
                .map(|(_, code)| code.as_str()),
        );
        let maps: Vec<Option<OwnedSourceMap>> = parts
            .source_maps
            .iter()
            .enumerate()
            .filter(|(index, _)| !parts.depth_refused.contains(index))
            .map(|(_, map)| map.clone())
            .collect();
        let prior = Prior {
            guards: &parts.guards,
            maps: &maps,
        };
        self.unpack_final_texts(&texts, prior, guard, stages, notes, panicked)
    }

    #[doc(hidden)]
    pub fn run_with_final_text_guard_for_tests(&self, final_texts: &[&str]) -> Output {
        let ladder = Ladder::new(self.options.hooks());
        on_a_worker_stack(&ladder, |_| {
            let mut parts = OutputParts::from_results(
                final_texts
                    .iter()
                    .map(|text| InputResult {
                        code: (*text).to_string(),
                        ..InputResult::default()
                    })
                    .collect(),
            );
            let (stages, notes, panicked) = (
                &mut parts.stages,
                &mut parts.notes,
                &mut parts.unpack_panicked,
            );
            let earlier = &parts.guards;
            let scan = (FinalTextGuard::Scan, false);
            let prior = Prior::guards(earlier);
            let bundle = self.unpack_final_texts(final_texts, prior, scan, stages, notes, panicked);
            Output {
                wasm_check: self.options.wasm_check,
                ..parts.into_output(bundle, self.options.single_file)
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::{refusal_note, stack_request, MIB};
    use untwist_core::depth::{DepthRefusal, UNIT_STACK_BYTES};
    use untwist_eval::stack_reserve::{DECODED_TEXT_STACK_BYTES, STACK_RESERVE_BYTES};

    #[test]
    fn every_scan_refusal_has_its_own_note() {
        let notes = [
            DepthRefusal::Nesting,
            DepthRefusal::SpeculativeParens,
            DepthRefusal::PrintSize,
            DepthRefusal::Typescript,
        ]
        .map(refusal_note);
        for (index, note) in notes.iter().enumerate() {
            assert!(!notes[index + 1..].contains(note));
        }
    }

    #[test]
    fn a_stack_request_holds_the_decoded_text_headroom_and_at_most_the_reserve() {
        let headroom = DECODED_TEXT_STACK_BYTES;
        assert_eq!(stack_request(0), MIB + headroom);
        assert_eq!(
            stack_request(505),
            MIB.max(505 * UNIT_STACK_BYTES as u64) + headroom
        );
        assert_eq!(
            stack_request(100_000),
            100_000 * UNIT_STACK_BYTES as u64 + headroom
        );
        assert_eq!(stack_request(u32::MAX), STACK_RESERVE_BYTES);
    }
}
