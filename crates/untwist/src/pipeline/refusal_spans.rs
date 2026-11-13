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

    fn locate(&self, offset: usize, after: Option<Located>) -> Option<Located> {
        if offset > self.text.len() {
            return None;
        }
        let line = self.starts.partition_point(|start| *start <= offset);
        let (from, column) = match after {
            Some(known) if known.line == line && known.offset <= offset => {
                (known.offset, known.column)
            }
            _ => (self.starts[line - 1], 1),
        };
        let column = column + self.text.get(from..offset)?.chars().count();
        Some(Located {
            offset,
            line,
            column,
        })
    }

    fn locate_all(&self, offsets: &[usize]) -> Vec<Option<Located>> {
        let mut order: Vec<usize> = (0..offsets.len()).collect();
        order.sort_by_key(|&index| offsets[index]);
        let mut found = vec![None; offsets.len()];
        let mut last = None;
        for index in order {
            found[index] = self.locate(offsets[index], last);
            last = found[index].or(last);
        }
        found
    }
}

#[derive(Clone, Copy)]
struct Located {
    offset: usize,
    line: usize,
    column: usize,
}

fn site_at(lines: &Lines<'_>, span: &RefusalSpan, located: Option<Located>) -> Option<SiteAt> {
    let Located { line, column, .. } = located?;
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
    let mut seen = HashSet::new();
    let spans: Vec<RefusalSpan> = spans
        .into_iter()
        .filter(|span| seen.insert((span.span, span.what.clone())))
        .collect();
    let lines = Lines::of(text);
    let offsets: Vec<usize> = spans.iter().map(|span| span.span.start as usize).collect();
    spans
        .into_iter()
        .zip(lines.locate_all(&offsets))
        .map(|(span, located)| RefusalSite {
            input: 0,
            layer,
            at: site_at(&lines, &span, located),
            what: span.what,
        })
        .collect()
}

fn refused_here(notes: &[Note]) -> bool {
    notes.iter().any(|note| {
        note.message.starts_with(notes::BUILTIN_PATCHED)
            || note.message == notes::STRING_ARRAY_GLOBAL_REACH
            || note.message == notes::MACHINERY_NAME_OBSERVED
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
    fn spans_in_any_order_get_the_columns_of_a_fresh_count() {
        let text = "\u{e9}a\u{e9}b;\nc\u{e9}d;\n\u{e9}";
        let starts: [u32; 11] = [9, 0, 3, 13, 2, 6, 99, 1, 4, 10, 6];
        let found = located(text, 0, starts.iter().map(|&at| span("x", at)).collect());
        let fresh = |offset: usize| {
            let before = text.get(..offset)?;
            let start = before.rfind('\n').map_or(0, |at| at + 1);
            let line = before[..start].matches('\n').count() + 1;
            Some((line, before[start..].chars().count() + 1))
        };
        for (site, &start) in found.iter().zip(&starts) {
            let offset = start as usize;
            let expected = (offset <= text.len()).then(|| fresh(offset)).flatten();
            assert_eq!(
                site.at.map(|at| (at.line, at.column)),
                expected,
                "offset {offset}"
            );
        }
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
        assert_eq!(kept(&[reached], sites.clone()).len(), 2);
        let observed = note(notes::MACHINERY_NAME_OBSERVED);
        assert_eq!(kept(&[observed], sites).len(), 2);
    }
}
