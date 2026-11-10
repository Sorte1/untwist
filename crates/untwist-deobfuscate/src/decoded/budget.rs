use std::borrow::Cow;
use std::cell::RefCell;
use std::rc::Rc;
use untwist_core::ast_util::js_text::js_text;

use oxc_ast::ast::Expression;
use oxc_span::GetSpan;

use rustc_hash::FxHashSet;
use untwist_core::pass::InputRoom;
use untwist_eval::bail::{Bail, EvalResult};
use untwist_eval::budget::{
    default_fuel, heap_budget, input_fuel, safe_unit_fuel, Budget, UNSAFE_STEP_BUDGET,
};
use untwist_eval::decode::{Decoded, InterpEvaluator};
use untwist_eval::heap::HeapLedger;

use crate::bridge::DecodeRequest;

pub(crate) type SharedHeap = Rc<RefCell<InputHeap>>;

const NON_STRING_PRINT_BYTES: u64 = 32;
const ESCAPED_UNIT_BYTES: u64 = 6;
const INLINED_COPIES: u64 = 4;
const SCANNED_FROM_UNITS: usize = 32;
const SCAN_CHUNK: usize = 1024;
const ESCAPE_EXTRA: u16 = 5;

const SPENT: u64 = 1 << 63;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum UnitFuel {
    Fixed(u64),
    PerDistinctCall,
}

impl UnitFuel {
    pub(crate) fn of(forced: Option<u64>, unsafe_transforms: bool) -> Self {
        match (forced, unsafe_transforms) {
            (Some(fuel), _) => Self::Fixed(fuel),
            (None, true) => Self::Fixed(UNSAFE_STEP_BUDGET),
            (None, false) => Self::PerDistinctCall,
        }
    }

    fn base(self) -> u64 {
        match self {
            Self::Fixed(fuel) => fuel,
            Self::PerDistinctCall => default_fuel(false),
        }
    }

    pub(crate) fn for_unit(self, request: &DecodeRequest<'_>) -> u64 {
        match self {
            Self::Fixed(fuel) => fuel,
            Self::PerDistinctCall => safe_unit_fuel(distinct_calls(request)),
        }
    }
}

#[derive(PartialEq, Eq, Hash)]
enum ArgumentKey<'a> {
    Number(u64),
    Text(Cow<'a, str>),
    Site(u32, u32),
}

fn argument_key<'a>(argument: &'a Expression<'a>) -> ArgumentKey<'a> {
    match argument {
        Expression::NumericLiteral(number) => ArgumentKey::Number(number.value.to_bits()),
        Expression::StringLiteral(text) => ArgumentKey::Text(js_text(text.value)),
        other => {
            let span = other.span();
            ArgumentKey::Site(span.start, span.end)
        }
    }
}

fn distinct_calls(request: &DecodeRequest<'_>) -> usize {
    request
        .calls
        .iter()
        .map(|call| {
            let arguments: Vec<ArgumentKey<'_>> = call
                .arguments
                .iter()
                .map(|argument| argument_key(argument))
                .collect();
            (call.decoder, arguments)
        })
        .collect::<FxHashSet<_>>()
        .len()
}

#[derive(Default)]
pub(crate) struct InputHeap {
    ledger: Option<Rc<HeapLedger>>,
    inline: u64,
    room: Option<InputRoom>,
    run_clamped: bool,
    fuel_left: Option<u64>,
}

pub(crate) fn printed_bytes(value: &Decoded) -> u64 {
    let Decoded::Str(text) = value else {
        return NON_STRING_PRINT_BYTES;
    };
    let len = text.len() as u64;
    if text.len() <= SCANNED_FROM_UNITS {
        return len.saturating_mul(ESCAPED_UNIT_BYTES).saturating_add(2);
    }
    let extra: u64 = text.units().chunks(SCAN_CHUNK).map(chunk_extra).sum();
    len.saturating_add(2).saturating_add(extra)
}

fn chunk_extra(chunk: &[u16]) -> u64 {
    let mut extra: u16 = 0;
    for &unit in chunk {
        let quoted = u16::from(unit == 0x22) | u16::from(unit == 0x27) | u16::from(unit == 0x5C);
        let escaped = u16::from(unit.wrapping_sub(0x20) > 0x5E);
        extra += quoted + escaped * ESCAPE_EXTRA;
    }
    u64::from(extra)
}

impl InputHeap {
    pub(crate) fn start(
        &mut self,
        input_len: usize,
        forced: Option<u64>,
        room: Option<&InputRoom>,
    ) {
        if self.ledger.is_none() {
            let run_room = untwist_core::budget::room().unwrap_or(u64::MAX);
            let wanted = forced.unwrap_or_else(|| heap_budget(input_len));
            self.run_clamped = run_room < wanted;
            self.ledger = Some(HeapLedger::new(wanted.min(run_room)));
            self.inline = (heap_budget(input_len).min(run_room) / INLINED_COPIES) & !SPENT;
            self.room = room.cloned();
        }
    }

    pub(crate) fn inline(&mut self, value: &Decoded) -> bool {
        let bytes = printed_bytes(value);
        let fits = bytes <= self.inline & !SPENT
            && self
                .room
                .as_ref()
                .is_none_or(|room| room.spend_print(bytes));
        if !fits {
            self.inline = SPENT;
            return false;
        }
        self.inline -= bytes;
        true
    }

    pub(crate) fn is_spent(&self) -> bool {
        self.inline & SPENT != 0
    }

    pub(crate) fn start_fuel(&mut self, input_len: usize, fuel: UnitFuel) {
        self.fuel_left
            .get_or_insert_with(|| input_fuel(input_len, fuel.base()));
    }

    pub(crate) fn unit_fuel(&self, wanted: u64) -> Option<u64> {
        match self.fuel_left {
            Some(0) => None,
            Some(left) => Some(wanted.min(left)),
            None => Some(wanted),
        }
    }

    pub(crate) fn spend_fuel(&mut self, steps: u64) {
        if let Some(left) = self.fuel_left.as_mut() {
            *left = left.saturating_sub(steps);
        }
    }

    pub(crate) fn evaluator(&self, fuel: u64) -> InterpEvaluator {
        let mut evaluator = InterpEvaluator::new(Budget::new(fuel));
        evaluator.heap = self.ledger.clone();
        evaluator
    }

    pub(crate) fn settle(&mut self, result: &EvalResult<Vec<Option<Decoded>>>) {
        let ended_early = match result {
            Ok(values) => values.iter().any(Option::is_none),
            Err(bail) => *bail == Bail::Budget,
        };
        let over = self.ledger.as_ref().is_some_and(|ledger| ledger.is_over());
        if ended_early || over {
            self.inline |= SPENT;
            if self.run_clamped {
                untwist_core::budget::trip(untwist_core::budget::Hit::Memory);
            }
        }
    }

    pub(crate) fn peak(&self) -> u64 {
        self.ledger.as_ref().map_or(0, |ledger| ledger.peak())
    }
}

#[cfg(test)]
mod tests {
    use super::InputHeap;
    use untwist_core::pass::InputRoom;
    use untwist_eval::bail::Bail;
    use untwist_eval::heap::reserve;

    #[test]
    fn the_ledger_starts_once_and_bounds_live_bytes() {
        let mut heap = InputHeap::default();
        heap.start(0, Some(1000), None);
        heap.start(0, Some(5), None);
        let ledger = heap.evaluator(10).heap.expect("a ledger");
        let held = reserve(&ledger, 600).expect("fits");
        assert!(reserve(&ledger, 500).is_err());
        drop(held);
        assert!(reserve(&ledger, 900).is_ok());
        assert_eq!(heap.peak(), 900);
    }

    #[test]
    fn inlined_text_stops_at_the_smaller_of_the_budget_and_the_print_room() {
        let mut heap = InputHeap::default();
        let room = InputRoom::new(20, 0);
        heap.start(0, None, Some(&room));
        let text = untwist_eval::decode::Decoded::Str("ab".into());
        assert_eq!(super::printed_bytes(&text), 2 * 6 + 2);
        heap.settle(&Err(untwist_eval::bail::Bail::Budget));
        assert!(heap.inline(&text));
        assert_eq!(room.print(), 6);
        assert!(!heap.inline(&text));
        assert!(heap.is_spent());
        let long = format!("{}a\"\u{e9}", "x".repeat(40));
        let quoted = untwist_eval::decode::Decoded::Str(long.as_str().into());
        assert_eq!(super::printed_bytes(&quoted), 2 + 40 + 1 + 2 + 6);
    }

    #[test]
    fn a_unit_that_ends_on_the_budget_spends_the_input() {
        let mut heap = InputHeap::default();
        heap.settle(&Ok(vec![Some(untwist_eval::decode::Decoded::Null)]));
        assert!(!heap.is_spent());
        heap.settle(&Err(Bail::Budget));
        assert!(heap.is_spent());
    }
}
