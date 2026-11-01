use oxc_allocator::Allocator;
use oxc_ast::ast::Program;
use oxc_span::Span;
use rustc_hash::FxHashSet;
use untwist_core::depth::{limits_for, scan_as, Goal, Limits, UNIT_STACK_BYTES};
use untwist_core::parse::{parse_with, SourceType};
use untwist_eval::stack::room;
use untwist_eval::stack_reserve::DECODED_TEXT_STACK_BYTES;

use super::escape::{escape_in_decoded_program, MAX_SINK_DEPTH};

const TEXT_ARENA_BYTES_PER_UNIT: usize = 48;
const TEXT_ARENA_MIN: usize = 256;
const TEXT_ARENA_MAX: usize = 16 * 1024 - 64;

fn text_arena(text: &str) -> Allocator {
    Allocator::with_capacity(
        text.len()
            .saturating_mul(TEXT_ARENA_BYTES_PER_UNIT)
            .clamp(TEXT_ARENA_MIN, TEXT_ARENA_MAX),
    )
}

fn decoded_text_nesting() -> u32 {
    let per_text = DECODED_TEXT_STACK_BYTES / u64::from(MAX_SINK_DEPTH);
    let units = per_text / u64::try_from(UNIT_STACK_BYTES).unwrap_or(u64::MAX);
    u32::try_from(units).unwrap_or(u32::MAX)
}

fn decoded_text_limits(len: usize) -> Limits {
    let limits = limits_for(room(), len);
    #[cfg(target_arch = "wasm32")]
    let limits = Limits {
        nesting: untwist_core::depth::capped_above_the_input(limits.nesting, MAX_SINK_DEPTH),
        ..limits
    };
    Limits {
        nesting: limits.nesting.min(decoded_text_nesting()),
        ..limits
    }
}

pub(super) fn scan_decoded_text(
    text: &str,
    globals: &FxHashSet<&'static str>,
    resolve: Option<&dyn Fn(Span) -> Option<String>>,
    depth: u32,
) -> Option<(Option<Span>, FxHashSet<&'static str>)> {
    scan_as(text, &decoded_text_limits(text.len()), Goal::Script).ok()?;
    let alloc = text_arena(text);
    let parsed = parse_with(&alloc, text, SourceType::cjs());
    if !parsed.diagnostics.is_empty() {
        return None;
    }
    Some(escape_in_decoded_program(
        &parsed.program,
        globals,
        resolve,
        depth,
    ))
}

pub(crate) fn with_parsed_text<R>(
    text: &str,
    judge: impl FnOnce(&Program<'_>) -> R,
) -> Option<Option<R>> {
    scan_as(text, &decoded_text_limits(text.len()), Goal::Script).ok()?;
    let alloc = text_arena(text);
    let parsed = parse_with(&alloc, text, SourceType::cjs());
    Some(
        parsed
            .diagnostics
            .is_empty()
            .then(|| judge(&parsed.program)),
    )
}

pub(crate) fn literal_text_writes(text: &str) -> bool {
    if scan_as(text, &decoded_text_limits(text.len()), Goal::Script).is_err() {
        return true;
    }
    let alloc = text_arena(text);
    let parsed = parse_with(&alloc, text, SourceType::cjs());
    parsed.diagnostics.is_empty() && super::text_writes::writes_in_text(&parsed.program)
}

fn clean_text_reaches(text: &str) -> Option<bool> {
    scan_as(text, &decoded_text_limits(text.len()), Goal::Script).ok()?;
    let alloc = text_arena(text);
    let parsed = parse_with(&alloc, text, SourceType::cjs());
    parsed.diagnostics.is_empty().then(|| {
        super::text_writes::writes_in_text(&parsed.program)
            || super::text_writes::runs_in_text(&parsed.program)
    })
}

pub(super) fn literal_text_reaches_in(text: &str) -> bool {
    literal_text_reaches(text)
}

pub(crate) fn literal_text_reaches(text: &str) -> bool {
    clean_text_reaches(text)
        .or_else(|| clean_text_reaches(&format!("(function () {{\n{text}\n}});")))
        .unwrap_or(false)
}

pub(crate) fn literal_text_binds(text: &str) -> bool {
    if scan_as(text, &decoded_text_limits(text.len()), Goal::Script).is_err() {
        return true;
    }
    let alloc = text_arena(text);
    let parsed = parse_with(&alloc, text, SourceType::cjs());
    parsed.diagnostics.is_empty() && super::text_writes::binds_in_text(&parsed.program)
}

pub(crate) fn literal_text_runs(text: &str) -> bool {
    if scan_as(text, &decoded_text_limits(text.len()), Goal::Script).is_err() {
        return true;
    }
    let alloc = text_arena(text);
    let parsed = parse_with(&alloc, text, SourceType::cjs());
    parsed.diagnostics.is_empty() && super::text_writes::runs_in_text(&parsed.program)
}
