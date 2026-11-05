use std::cell::Cell;

thread_local! {
    static MISPLACE_AN_EDIT: Cell<bool> = const { Cell::new(false) };
    static FLIP_BUNDLE_ANSWER: Cell<bool> = const { Cell::new(false) };
}

#[doc(hidden)]
pub fn misplace_an_edit_for_tests(on: bool) {
    MISPLACE_AN_EDIT.with(|slot| slot.set(on));
}

#[doc(hidden)]
pub fn flip_bundle_answer_for_tests(on: bool) {
    FLIP_BUNDLE_ANSWER.with(|slot| slot.set(on));
}

pub(super) fn misplaced() -> bool {
    MISPLACE_AN_EDIT.with(Cell::get)
}

pub(super) fn flipped() -> bool {
    FLIP_BUNDLE_ANSWER.with(Cell::get)
}
