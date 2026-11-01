use untwist::{Output, UNPACK_TEXT_TOO_DEEP};
use untwist_core::depth::{limits_for, WASM_SHADOW_STACK_BYTES};

pub fn nesting_budget() -> u32 {
    limits_for(usize::MAX, 0).nesting
}

pub fn shadow_stack_bytes() -> u32 {
    WASM_SHADOW_STACK_BYTES
}

pub fn call_depth_cap() -> u32 {
    untwist_eval::budget::CALL_DEPTH_CAP
}

pub fn evaluator_nesting_cap() -> u32 {
    untwist_eval::budget::NESTING_DEPTH_CAP
}

#[derive(Clone, Copy, Default)]
pub struct Fired {
    cap_bails: u32,
    budget_refusals: u32,
}

#[cfg(target_arch = "wasm32")]
pub fn fired() -> Fired {
    Fired {
        cap_bails: untwist_eval::budget::wasm_cap_bails(),
        budget_refusals: untwist_core::depth::wasm_budget_refusals(),
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub fn fired() -> Fired {
    Fired::default()
}

impl Fired {
    pub fn since(self, before: Fired) -> Fired {
        Fired {
            cap_bails: self.cap_bails.saturating_sub(before.cap_bails),
            budget_refusals: self.budget_refusals.saturating_sub(before.budget_refusals),
        }
    }
}

fn unpack_text_refused(result: &Output) -> bool {
    result
        .notes
        .iter()
        .any(|note| note.pass == "pipeline" && note.message == UNPACK_TEXT_TOO_DEEP)
}

pub fn limits_of(result: &Output, fired: Fired) -> String {
    let named = [
        (!result.depth_refused.is_empty(), "refused"),
        (fired.budget_refusals > 0, "budget"),
        (unpack_text_refused(result), "unpack-text"),
        (fired.cap_bails > 0, "eval-cap"),
    ];
    named
        .iter()
        .filter(|(on, _)| *on)
        .map(|(_, name)| *name)
        .collect::<Vec<_>>()
        .join(",")
}
