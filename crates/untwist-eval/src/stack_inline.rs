use crate::budget::CALL_STACK_GROWTH;

pub(crate) const SPAN_BYTES: usize = CALL_STACK_GROWTH / 2;

const NATIVE_WORKER_STACK_BYTES: usize = 1 << 30;

#[inline(always)]
pub fn has_room() -> bool {
    true
}

#[inline(always)]
pub fn has_room_for(_bytes: usize) -> bool {
    true
}

pub fn grow<R>(_stack_size: usize, callback: impl FnOnce() -> R) -> R {
    callback()
}

pub fn on_this_threads_stack<R>(body: impl FnOnce() -> R) -> R {
    body()
}

pub fn room() -> usize {
    NATIVE_WORKER_STACK_BYTES - SPAN_BYTES
}

#[inline(never)]
pub fn maybe_grow_dyn(callback: &mut dyn FnMut()) {
    callback();
}

#[inline(always)]
pub fn maybe_grow<R>(callback: impl FnOnce() -> R) -> R {
    callback()
}
