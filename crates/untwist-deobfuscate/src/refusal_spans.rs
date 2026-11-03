use std::cell::RefCell;

use oxc_span::Span;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RefusalSpan {
    pub what: String,
    pub span: Span,
}

thread_local! {
    static SPANS: RefCell<Option<Vec<RefusalSpan>>> = const { RefCell::new(None) };
}

pub struct Recorder {
    outer: Option<Option<Vec<RefusalSpan>>>,
}

impl Recorder {
    pub fn start() -> Self {
        let outer = SPANS.with(|spans| spans.replace(Some(Vec::new())));
        Self { outer: Some(outer) }
    }

    pub fn finish(mut self) -> Vec<RefusalSpan> {
        self.restore().unwrap_or_default()
    }

    fn restore(&mut self) -> Option<Vec<RefusalSpan>> {
        let outer = self.outer.take()?;
        SPANS.with(|spans| spans.replace(outer))
    }
}

impl Drop for Recorder {
    fn drop(&mut self) {
        self.restore();
    }
}

pub(crate) fn active() -> bool {
    SPANS.with(|spans| spans.borrow().is_some())
}

pub(crate) fn record(what: impl FnOnce() -> String, span: Span) {
    SPANS.with(|spans| {
        if let Some(spans) = spans.borrow_mut().as_mut() {
            spans.push(RefusalSpan { what: what(), span });
        }
    });
}

pub(crate) fn extend(trail: &[RefusalSpan]) {
    SPANS.with(|spans| {
        if let Some(spans) = spans.borrow_mut().as_mut() {
            spans.extend_from_slice(trail);
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nothing_is_recorded_outside_a_recorder() {
        record(|| "outside".to_string(), Span::new(0, 1));
        let recorder = Recorder::start();
        record(|| "inside".to_string(), Span::new(2, 3));
        let spans = recorder.finish();
        assert_eq!(
            spans,
            vec![RefusalSpan {
                what: "inside".to_string(),
                span: Span::new(2, 3)
            }]
        );
        assert!(!active());
    }

    #[test]
    fn a_nested_recorder_hands_the_outer_one_back() {
        let outer = Recorder::start();
        record(|| "first".to_string(), Span::new(0, 1));
        let inner = Recorder::start();
        record(|| "second".to_string(), Span::new(1, 2));
        assert_eq!(inner.finish().len(), 1);
        record(|| "third".to_string(), Span::new(2, 3));
        let whats: Vec<String> = outer.finish().into_iter().map(|span| span.what).collect();
        assert_eq!(whats, ["first", "third"]);
    }

    #[test]
    fn a_dropped_recorder_stops_recording() {
        drop(Recorder::start());
        assert!(!active());
    }
}
