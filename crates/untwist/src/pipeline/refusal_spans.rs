use std::collections::HashSet;

use untwist_core::pass::Note;
use untwist_deobfuscate::notes;
use untwist_deobfuscate::refusal_spans::RefusalSpan;

struct Lines<'t> {
    text: &'t str,
    starts: Vec<usize>,
}

impl<'t> Lines<'t> {
    fn of(text: &'t str) -> Self {
        let starts = std::iter::once(0)
            .chain(text.match_indices('\n').map(|(at, _)| at + 1))
            .collect();
        Self { text, starts }
    }

    fn locate(&self, offset: usize) -> Option<(usize, usize)> {
        if offset > self.text.len() {
            return None;
        }
        let line = self.starts.partition_point(|start| *start <= offset);
        let start = self.starts[line - 1];
        let column = self.text.get(start..offset)?.chars().count() + 1;
        Some((line, column))
    }
}

pub(super) fn located(text: &str, spans: Vec<RefusalSpan>) -> Vec<String> {
    if spans.is_empty() {
        return Vec::new();
    }
    let lines = Lines::of(text);
    spans
        .into_iter()
        .map(|span| match lines.locate(span.span.start as usize) {
            Some((line, column)) => format!("{line}:{column} {}", span.what),
            None => format!("synthetic {}", span.what),
        })
        .collect()
}

fn refused_here(notes: &[Note]) -> bool {
    notes.iter().any(|note| {
        note.message.starts_with(notes::BUILTIN_PATCHED)
            || note.message == notes::STRING_ARRAY_GLOBAL_REACH
    })
}

pub(super) fn kept(notes: &[Note], spans: Vec<String>) -> Vec<String> {
    if !refused_here(notes) {
        return Vec::new();
    }
    let mut seen = HashSet::new();
    spans
        .into_iter()
        .filter(|span| seen.insert(span.clone()))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use oxc_span::Span;
    use untwist_core::pass::NoteKind;

    fn span(what: &str, start: u32) -> RefusalSpan {
        RefusalSpan {
            what: what.to_string(),
            span: Span::new(start, start + 1),
        }
    }

    fn note(message: &str) -> Note {
        Note {
            pass: "inline-decoded-strings",
            kind: NoteKind::Refusal,
            message: message.to_string(),
            span: None,
        }
    }

    #[test]
    fn spans_are_located_by_line_and_character_column() {
        let text = "a;\nb\u{e9}c[k];\nd";
        let found = located(text, vec![span("x", 0), span("y", 7), span("z", 12)]);
        assert_eq!(found, ["1:1 x", "2:4 y", "3:1 z"]);
    }

    #[test]
    fn a_span_past_the_text_is_synthetic() {
        assert_eq!(located("a;", vec![span("x", 9)]), ["synthetic x"]);
    }

    #[test]
    fn spans_are_kept_once_and_only_beside_a_refusal() {
        let spans = vec![
            "1:1 x".to_string(),
            "1:1 x".to_string(),
            "2:1 y".to_string(),
        ];
        assert!(kept(&[note("decoded")], spans.clone()).is_empty());
        let refused = format!("{}: a prototype[a computed key]", notes::BUILTIN_PATCHED);
        assert_eq!(kept(&[note(&refused)], spans.clone()), ["1:1 x", "2:1 y"]);
        let reached = note(notes::STRING_ARRAY_GLOBAL_REACH);
        assert_eq!(kept(&[reached], spans), ["1:1 x", "2:1 y"]);
    }
}
