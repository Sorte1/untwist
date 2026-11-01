use crate::budget::{CALL_STACK_GROWTH, NESTING_DEPTH_CAP};
use crate::compile::{Compiled, Evaluate};
use crate::frame::FrameRef;
use crate::interp::Interp;
use crate::stack;
use crate::value::{Completion, Eval, Value};

pub const NODE_FRAME_BYTES: usize = 8192;
pub const UNCHECKED_BYTES: usize = stack::SPAN_BYTES / 2;
pub const UNCHECKED_LEVELS: u32 = (UNCHECKED_BYTES / NODE_FRAME_BYTES) as u32;
const ALWAYS_CHECKED: u32 = NESTING_DEPTH_CAP;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) struct Reach(u32);

impl Reach {
    pub(crate) const CHECKED: Reach = Reach(0);
    #[cfg(feature = "test-hooks")]
    pub(crate) const UNBOUNDED: Reach = Reach(u32::MAX);

    #[inline(always)]
    pub(crate) fn checks(self) -> bool {
        self.0 == 0
    }

    #[inline(always)]
    pub(crate) fn covers(self, nesting: u32, levels: u32) -> bool {
        nesting.wrapping_add(levels) <= self.0
    }
}

pub(crate) fn unit_levels(deepest: u32) -> u32 {
    if deepest <= UNCHECKED_LEVELS {
        deepest
    } else {
        ALWAYS_CHECKED
    }
}

#[inline(always)]
pub(crate) fn frame_reach(interp: &Interp<'_>) -> Reach {
    #[cfg(feature = "test-hooks")]
    if let Some(forced) = crate::test_hooks::forced_reach() {
        return forced;
    }
    generous_reach(interp)
}

#[inline(always)]
fn generous_reach(interp: &Interp<'_>) -> Reach {
    let ceiling = interp
        .budget
        .nesting_depth()
        .saturating_add(UNCHECKED_LEVELS);
    if ceiling < NESTING_DEPTH_CAP && stack::has_room_for(UNCHECKED_BYTES) {
        return Reach(ceiling);
    }
    Reach::CHECKED
}

#[inline(always)]
pub(crate) fn unit_reach(interp: &Interp<'_>, given: Reach, levels: u32) -> Reach {
    let nesting = interp.budget.nesting_depth();
    if given.covers(nesting, levels) {
        return given;
    }
    fresh_reach(interp, nesting, levels)
}

#[inline(never)]
fn fresh_reach(interp: &Interp<'_>, nesting: u32, levels: u32) -> Reach {
    #[cfg(feature = "test-hooks")]
    crate::test_hooks::note_fresh_reach();
    #[cfg(feature = "test-hooks")]
    if let Some(forced) = crate::test_hooks::forced_reach() {
        return forced;
    }
    let generous = generous_reach(interp);
    if generous.covers(nesting, levels) {
        return generous;
    }
    let ceiling = nesting.saturating_add(levels);
    if ceiling < NESTING_DEPTH_CAP && stack::has_room_for(levels as usize * NODE_FRAME_BYTES) {
        return Reach(ceiling);
    }
    Reach::CHECKED
}

#[inline(always)]
pub(crate) fn level<'a, T>(
    interp: &mut Interp<'a>,
    reach: Reach,
    body: impl FnOnce(&mut Interp<'a>) -> Eval<'a, T>,
) -> Eval<'a, T> {
    let outer = interp.budget.nesting_depth();
    if reach.checks() {
        interp.budget.enter_nesting()?;
    } else {
        interp.budget.deepen();
        #[cfg(feature = "test-hooks")]
        crate::test_hooks::note_unchecked_level(interp.budget.nesting_depth(), reach.0);
    }
    let result = body(interp);
    interp.budget.restore_nesting(outer);
    result
}

#[cold]
#[inline(never)]
pub(crate) fn grown_eval<'a>(
    code: &dyn Evaluate<'a>,
    interp: &mut Interp<'a>,
    frame: &FrameRef<'a>,
    reach: Reach,
) -> Eval<'a, Value<'a>> {
    stack::grow(CALL_STACK_GROWTH, || code.eval(interp, frame, reach))
}

#[cold]
#[inline(never)]
pub(crate) fn grown_run<'a>(
    code: &dyn Compiled<'a>,
    interp: &mut Interp<'a>,
    frame: &FrameRef<'a>,
    reach: Reach,
) -> Eval<'a, Completion<'a>> {
    stack::grow(CALL_STACK_GROWTH, || code.run(interp, frame, reach))
}
