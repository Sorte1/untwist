use super::{Cx, NoteKind};
use oxc_span::Span;

pub const SPANNED_NOTE_CAP: usize = 3;

#[derive(Default)]
pub struct RepeatedSites {
    sites: Vec<(Span, String)>,
}

impl RepeatedSites {
    pub fn record(&mut self, span: Span, detail: &str) {
        self.sites.push((span, detail.to_string()));
    }

    pub fn len(&self) -> usize {
        self.sites.len()
    }

    pub fn is_empty(&self) -> bool {
        self.sites.is_empty()
    }

    pub fn breakdown(&self) -> Vec<(String, usize)> {
        let mut counted: Vec<(String, usize)> = Vec::new();
        for (_, detail) in &self.sites {
            match counted.iter_mut().find(|(name, _)| name == detail) {
                Some((_, count)) => *count += 1,
                None => counted.push((detail.clone(), 1)),
            }
        }
        counted.sort_by(|left, right| right.1.cmp(&left.1).then_with(|| left.0.cmp(&right.0)));
        counted
    }

    pub fn emit(
        &self,
        kind: NoteKind,
        spanned: impl Fn(&str) -> String,
        summary: impl Fn(usize, &[(String, usize)]) -> String,
        cx: &mut Cx<'_, '_>,
    ) {
        for (span, detail) in self.sites.iter().take(SPANNED_NOTE_CAP) {
            cx.note_kind(kind, spanned(detail), Some(*span));
        }
        if self.sites.len() > SPANNED_NOTE_CAP {
            cx.note_kind(kind, summary(self.sites.len(), &self.breakdown()), None);
        }
    }
}
