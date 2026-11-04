use std::collections::HashSet;
use std::fmt;

use untwist_core::pass::Note;
use untwist_deobfuscate::notes;
use untwist_deobfuscate::refusal_spans::RefusalSpan;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct SiteAt {
    pub start: u32,
    pub end: u32,
    pub line: usize,
    pub column: usize,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct RefusalSite {
    pub input: usize,
    pub layer: u32,
    pub at: Option<SiteAt>,
    pub what: String,
}

impl fmt::Display for RefusalSite {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.at {
            Some(at) => write!(formatter, "{}:{} {}", at.line, at.column, self.what),
            None => write!(formatter, "synthetic {}", self.what),
        }
    }
}

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

fn site_at(lines: &Lines<'_>, span: &RefusalSpan) -> Option<SiteAt> {
    let (line, column) = lines.locate(span.span.start as usize)?;
    let end = span.span.end.max(span.span.start);
    let end = if end as usize <= lines.text.len() {
        end
    } else {
        span.span.start
    };
    Some(SiteAt {
        start: span.span.start,
        end,
        line,
        column,
    })
}

pub(super) fn located(text: &str, layer: u32, spans: Vec<RefusalSpan>) -> Vec<RefusalSite> {
    if spans.is_empty() {
        return Vec::new();
    }
    let lines = Lines::of(text);
    spans
        .into_iter()
        .map(|span| RefusalSite {
            input: 0,
            layer,
            at: site_at(&lines, &span),
            what: span.what,
        })
        .collect()
}

fn refused_here(notes: &[Note]) -> bool {
    notes.iter().any(|note| {
        note.message.starts_with(notes::BUILTIN_PATCHED)
            || note.message == notes::STRING_ARRAY_GLOBAL_REACH
    })
}

fn unique<T: Clone + Eq + std::hash::Hash>(items: impl IntoIterator<Item = T>) -> Vec<T> {
    let mut seen = HashSet::new();
    items
        .into_iter()
        .filter(|item| seen.insert(item.clone()))
        .collect()
}

pub(super) fn kept(notes: &[Note], sites: Vec<RefusalSite>) -> Vec<RefusalSite> {
    if !refused_here(notes) {
        return Vec::new();
    }
    unique(sites)
}

pub(super) fn spans_of(sites: &[RefusalSite]) -> Vec<String> {
    unique(sites.iter().map(RefusalSite::to_string))
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

    fn strings(text: &str, spans: Vec<RefusalSpan>) -> Vec<String> {
        spans_of(&located(text, 0, spans))
    }

    #[test]
    fn spans_are_located_by_line_and_character_column() {
        let text = "a;\nb\u{e9}c[k];\nd";
        let found = strings(text, vec![span("x", 0), span("y", 7), span("z", 12)]);
        assert_eq!(found, ["1:1 x", "2:4 y", "3:1 z"]);
    }

    #[test]
    fn a_span_past_the_text_is_synthetic() {
        assert_eq!(strings("a;", vec![span("x", 9)]), ["synthetic x"]);
        assert_eq!(located("a;", 0, vec![span("x", 9)])[0].at, None);
    }

    #[test]
    fn a_site_keeps_its_byte_span_and_the_layer_it_was_found_in() {
        let sites = located("ab;\ncd[k];", 2, vec![span("x", 6)]);
        assert_eq!(
            sites,
            [RefusalSite {
                input: 0,
                layer: 2,
                at: Some(SiteAt {
                    start: 6,
                    end: 7,
                    line: 2,
                    column: 3
                }),
                what: "x".to_string(),
            }]
        );
        let clipped = located("a;", 0, vec![span("y", 2)]);
        assert_eq!(clipped[0].at.map(|at| (at.start, at.end)), Some((2, 2)));
    }

    #[test]
    fn spans_are_kept_once_and_only_beside_a_refusal() {
        let sites = located("a;\nb;", 0, vec![span("x", 0), span("x", 0), span("y", 3)]);
        assert!(kept(&[note("decoded")], sites.clone()).is_empty());
        let refused = format!("{}: a prototype[a computed key]", notes::BUILTIN_PATCHED);
        let shown = |kept: Vec<RefusalSite>| spans_of(&kept);
        assert_eq!(
            shown(kept(&[note(&refused)], sites.clone())),
            ["1:1 x", "2:1 y"]
        );
        let reached = note(notes::STRING_ARRAY_GLOBAL_REACH);
        assert_eq!(kept(&[reached], sites).len(), 2);
    }
}
