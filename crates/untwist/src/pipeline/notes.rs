use std::collections::HashSet;

use untwist_core::budget::Hit;
use untwist_core::catalogue;
use untwist_core::pass::{note_survives_retraction, Note, NoteKind};
use untwist_core::reflow::GuardTally;
use untwist_deobfuscate::notes;
use untwist_deobfuscate::PROTECTIONS_ORDER;

pub use super::refusal::ADMIT_PANICKED;
pub use super::refusal::NESTED_TOO_DEEP;
pub use super::refusal::OVER_MEMORY_REFUSED;
pub use super::refusal::OVER_TIME_REFUSED;
pub use super::refusal::PARENTHESES_TOO_DEEP;
pub use super::refusal::PRINT_TOO_LARGE;
pub use super::refusal::TOO_LARGE_FOR_THE_BUDGET;
pub use super::refusal::TYPESCRIPT_IN_JAVASCRIPT;
pub use super::refusal::UNPACK_TEXT_TOO_DEEP;
pub use super::refusal::WORKER_DIED;

pub const PANICKED: &str = "this input panicked part-way through a pass; its output is the tree \
     as far as the passes got and may not run; the other inputs continued";

pub const UNPACK_PANICKED: &str =
    "panicked while unpacking; the inputs are written without a bundle";

pub const UNPACK_TEXT_UNPARSABLE: &str = "a final text failed to parse while unpacking; the \
     inputs are written without a bundle";

pub const SLOT_NOT_RESERVED: &str = "unique rename could not reserve its counter block for this \
     input, so the rename stage was skipped";

pub const ANONYMOUS_SHELL_STRIPPED: &str = "a function anonymous(...) shell around the whole \
     program was removed; the program is its body";

pub const ANONYMOUS_SHELL_SCOPED: &str = "the body of a stripped function anonymous(...) shell \
     declares names at its own level, which would become globals at the top level, so it is kept \
     inside a called function";

pub const ANONYMOUS_SHELL_KEPT: &str = "a function anonymous(...) shell around the whole program \
     was recognised but left in place because";

pub const GUARD_ON_ONE_LINE: &str = "a self-defending check that reads a function's own source \
     survived; the function it reads was printed on one line, as the obfuscator emitted it:";

pub const GUARD_KEPT_ACROSS_LINES: &str = "a self-defending check that reads a function's own \
     source survived, but that function holds a line break no one-line print can remove, so the \
     output may not run as the input did:";

pub const BUDGET_MEMORY_REACHED: &str = "budget-memory: this input reached the memory budget \
     (--max-memory); the work after that point was cut short or refused, so the output is the \
     tree as far as the passes got";

pub const BUDGET_TIME_REACHED: &str = "budget-time: this input reached the time budget \
     (--timeout); the work after that point was cut short or refused, so the output is the tree \
     as far as the passes got";

pub const BUDGET_UNPACK: &str = "the run reached its budget before or while unpacking; the \
     inputs are written without a bundle";

pub(super) fn budget_note(hit: Hit) -> &'static str {
    match hit {
        Hit::Memory => BUDGET_MEMORY_REACHED,
        Hit::Time => BUDGET_TIME_REACHED,
    }
}

pub const UNSAFE_LANE_DECODES: &str = "references to the string array or its decoders are still \
     in the output: the default lane left the string array undecoded because the program may \
     replace a standard built-in the decoder uses; rerun with --unsafe to decode it (that lane \
     assumes the program patches no standard built-in)";

pub const REACH_REASON: &str = ": code that may hold the global object, or code from text, may \
     reach the string array's function by a key that may be its name or by an enumeration that \
     yields values, so the decoder calls were left alone; the report lists the details";

catalogue!(
    PIPELINE_NOTES,
    REACH_REASON,
    UNSAFE_LANE_DECODES,
    BUDGET_MEMORY_REACHED,
    BUDGET_TIME_REACHED,
    BUDGET_UNPACK,
    PANICKED,
    UNPACK_PANICKED,
    UNPACK_TEXT_UNPARSABLE,
    SLOT_NOT_RESERVED,
    ANONYMOUS_SHELL_STRIPPED,
    ANONYMOUS_SHELL_KEPT,
    ANONYMOUS_SHELL_SCOPED,
    GUARD_ON_ONE_LINE,
    GUARD_KEPT_ACROSS_LINES,
    NESTED_TOO_DEEP,
    PARENTHESES_TOO_DEEP,
    PRINT_TOO_LARGE,
    UNPACK_TEXT_TOO_DEEP,
    TYPESCRIPT_IN_JAVASCRIPT,
    WORKER_DIED,
    OVER_MEMORY_REFUSED,
    OVER_TIME_REFUSED,
    TOO_LARGE_FOR_THE_BUDGET,
    ADMIT_PANICKED
);

pub(super) type StageRun = (&'static str, usize, usize);

fn superseded_run(runs: &[StageRun], index: usize) -> bool {
    let (stage, _, _) = runs[index];
    runs[index + 1..]
        .iter()
        .any(|(later, _, _)| *later == stage)
}

pub(super) fn retain_last_spanned(notes: Vec<Note>, runs: &[StageRun]) -> Vec<Note> {
    let mut superseded = vec![false; notes.len()];
    for index in 0..runs.len() {
        if !superseded_run(runs, index) {
            continue;
        }
        let (_, start, end) = runs[index];
        for slot in superseded.iter_mut().take(end).skip(start) {
            *slot = true;
        }
    }
    notes
        .into_iter()
        .enumerate()
        .filter(|(index, note)| !superseded[*index] || note_survives_retraction(note))
        .map(|(_, note)| note)
        .collect()
}

pub(super) fn retain_latest_protection_notes(notes: &mut Vec<Note>, latest: usize) {
    let is_protection = |note: &Note| PROTECTIONS_ORDER.contains(&note.pass);
    let total = notes.iter().filter(|note| is_protection(note)).count();
    let mut superseded = total.saturating_sub(latest);
    notes.retain(|note| {
        if superseded == 0 || !is_protection(note) {
            return true;
        }
        superseded -= 1;
        false
    });
}

fn guard_note(kind: NoteKind, message: &str, count: u32) -> Option<Note> {
    (count > 0).then(|| Note {
        pass: "pipeline",
        kind,
        message: format!("{message} {count}"),
        span: None,
    })
}

pub(super) fn guard_notes(tally: GuardTally) -> impl Iterator<Item = Note> {
    let compacted = guard_note(NoteKind::Progress, GUARD_ON_ONE_LINE, tally.compacted);
    let kept = guard_note(NoteKind::Refusal, GUARD_KEPT_ACROSS_LINES, tally.kept);
    compacted.into_iter().chain(kept)
}

const MACHINERY_PASS: &str = "remove-decoder-machinery";
const KEPT_PREFIX: &str = "decoder machinery kept: ";

fn kept_count(note: &Note) -> Option<u64> {
    if note.pass != MACHINERY_PASS || note.kind != NoteKind::Refusal {
        return None;
    }
    let rest = note.message.strip_prefix(KEPT_PREFIX)?;
    let digits = rest.bytes().take_while(u8::is_ascii_digit).count();
    let tail = rest.get(digits..)?;
    (tail == notes::MACHINERY_REFERENCES_KEPT)
        .then(|| rest.get(..digits)?.parse().ok())
        .flatten()
}

fn reach_detail(note: &Note) -> bool {
    note.kind == NoteKind::Refusal
        && (note.message == notes::STRING_ARRAY_GLOBAL_REACH
            || note.message.starts_with(notes::WEBPACK_BOOTSTRAP_UNPROVEN))
}

pub(super) fn unsafe_lane_hint(notes: &[Note]) -> Option<Note> {
    let aliased = notes.iter().any(|note| {
        note.kind == NoteKind::Refusal && note.message.starts_with(notes::BUILTIN_ALIAS_PATCHED)
    });
    if !aliased {
        return None;
    }
    let kept = notes.iter().filter_map(kept_count).max()?;
    Some(Note {
        pass: "pipeline",
        kind: NoteKind::Refusal,
        message: format!("{kept} {UNSAFE_LANE_DECODES}"),
        span: None,
    })
}

pub(super) fn consolidate_reach(mut notes: Vec<Note>) -> Vec<Note> {
    let refused = notes.iter().any(|note| {
        note.kind == NoteKind::Refusal && note.message == notes::STRING_ARRAY_GLOBAL_REACH
    });
    let head = notes
        .iter()
        .enumerate()
        .filter_map(|(index, note)| kept_count(note).map(|count| (count, index)))
        .max_by_key(|(count, index)| (*count, std::cmp::Reverse(*index)))
        .map(|(_, index)| index);
    let (true, Some(head)) = (refused, head) else {
        return notes;
    };
    for (index, note) in notes.iter_mut().enumerate() {
        if index == head {
            note.message.push_str(REACH_REASON);
        } else if reach_detail(note) || kept_count(note).is_some() {
            note.kind = NoteKind::Progress;
        }
    }
    notes
}

pub(super) fn dedupe_spanned(notes: Vec<Note>) -> Vec<Note> {
    let mut seen: HashSet<Note> = HashSet::new();
    notes
        .into_iter()
        .filter(|note| note.span.is_none() || seen.insert(note.clone()))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::dedupe_spanned;
    use oxc_span::Span;
    use untwist_core::pass::{Note, NoteKind};

    fn note(kind: NoteKind, message: &str, span: Option<Span>) -> Note {
        Note {
            pass: "t",
            kind,
            message: message.to_string(),
            span,
        }
    }

    #[test]
    fn a_spanned_duplicate_keeps_its_first_occurrence() {
        let span = Some(Span::new(1, 2));
        let kept = dedupe_spanned(vec![
            note(NoteKind::Refusal, "x", span),
            note(NoteKind::Refusal, "y", None),
            note(NoteKind::Refusal, "x", span),
        ]);
        assert_eq!(kept.len(), 2);
        assert_eq!(kept[0].message, "x");
        assert_eq!(kept[1].message, "y");
    }

    #[test]
    fn spanless_duplicates_and_different_kinds_are_all_kept() {
        let span = Some(Span::new(1, 2));
        let kept = dedupe_spanned(vec![
            note(NoteKind::Refusal, "x", None),
            note(NoteKind::Refusal, "x", None),
            note(NoteKind::Progress, "x", span),
            note(NoteKind::Refusal, "x", span),
        ]);
        assert_eq!(kept.len(), 4);
    }
}
