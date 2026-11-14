use std::collections::BTreeMap;

use oxc_span::Span;

#[derive(Default, Debug)]
pub struct Regions {
    spans: BTreeMap<u32, u32>,
    empty: bool,
}

impl Regions {
    fn enclosing(&self, span: Span) -> bool {
        self.enclosing_start(span).is_some()
    }

    pub fn enclosing_start(&self, span: Span) -> Option<u32> {
        self.spans
            .range(..=span.start)
            .next_back()
            .filter(|(_, end)| span.end <= **end)
            .map(|(start, _)| *start)
    }

    pub fn insert(&mut self, span: Span) -> Option<Vec<Span>> {
        if span.is_empty() {
            self.empty = true;
            return None;
        }
        if self.enclosing(span) {
            return None;
        }
        let subsumed: Vec<Span> = self
            .spans
            .range(span.start..span.end)
            .filter(|(_, end)| **end <= span.end)
            .map(|(start, end)| Span::new(*start, *end))
            .collect();
        for inner in &subsumed {
            self.spans.remove(&inner.start);
        }
        self.spans.insert(span.start, span.end);
        Some(subsumed)
    }

    pub fn covers(&self, span: Span) -> bool {
        self.enclosing(span)
    }

    pub fn contains(&self, span: Span) -> bool {
        self.spans.get(&span.start) == Some(&span.end)
    }

    pub fn is_empty(&self) -> bool {
        self.spans.is_empty()
    }

    pub fn has_unplaced(&self) -> bool {
        self.empty
    }

    pub fn len(&self) -> usize {
        self.spans.len()
    }
}
