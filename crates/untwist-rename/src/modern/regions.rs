use std::collections::BTreeMap;

use oxc_span::Span;

#[derive(Default, Debug)]
pub struct Regions {
    spans: BTreeMap<u32, u32>,
}

impl Regions {
    fn enclosing(&self, span: Span) -> bool {
        self.spans
            .range(..=span.start)
            .next_back()
            .is_some_and(|(_, end)| span.end <= *end)
    }

    pub fn insert(&mut self, span: Span) -> Option<Vec<Span>> {
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

    pub fn len(&self) -> usize {
        self.spans.len()
    }
}
