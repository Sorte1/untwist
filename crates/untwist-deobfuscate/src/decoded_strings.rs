use std::cell::RefCell;

use oxc_span::Span;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DecodedString {
    pub span: Span,
    pub value: String,
}

#[derive(Default)]
struct Found {
    rows: Vec<(Span, usize, usize)>,
    text: String,
}

impl Found {
    fn into_strings(self) -> Vec<DecodedString> {
        self.rows
            .into_iter()
            .map(|(span, start, end)| DecodedString {
                span,
                value: self.text[start..end].to_string(),
            })
            .collect()
    }
}

thread_local! {
    static FOUND: RefCell<Option<Found>> = const { RefCell::new(None) };
}

pub struct Recorder {
    outer: Option<Option<Found>>,
}

impl Recorder {
    pub fn start() -> Self {
        let outer = FOUND.with(|found| found.replace(Some(Found::default())));
        Self { outer: Some(outer) }
    }

    pub fn finish(mut self) -> Vec<DecodedString> {
        self.restore().map(Found::into_strings).unwrap_or_default()
    }

    fn restore(&mut self) -> Option<Found> {
        let outer = self.outer.take()?;
        FOUND.with(|found| found.replace(outer))
    }
}

impl Drop for Recorder {
    fn drop(&mut self) {
        self.restore();
    }
}

pub(crate) fn record(span: Span, value: impl FnOnce() -> Option<String>) {
    FOUND.with(|found| {
        if let Some(found) = found.borrow_mut().as_mut() {
            if let Some(value) = value() {
                let start = found.text.len();
                found.text.push_str(&value);
                found.rows.push((span, start, found.text.len()));
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn text(value: &str) -> impl FnOnce() -> Option<String> + '_ {
        move || Some(value.to_string())
    }

    #[test]
    fn strings_are_kept_only_inside_a_recorder() {
        record(Span::new(0, 1), text("outside"));
        let recorder = Recorder::start();
        record(Span::new(2, 3), text("inside"));
        record(Span::new(4, 5), || None);
        assert_eq!(
            recorder.finish(),
            [DecodedString {
                span: Span::new(2, 3),
                value: "inside".to_string()
            }]
        );
        drop(Recorder::start());
        record(Span::new(6, 7), text("lost"));
        assert!(Recorder::start().finish().is_empty());
    }

    #[test]
    fn a_nested_recorder_hands_the_outer_one_back() {
        let outer = Recorder::start();
        record(Span::new(0, 1), text("first"));
        let inner = Recorder::start();
        record(Span::new(1, 2), text("second"));
        assert_eq!(inner.finish().len(), 1);
        record(Span::new(2, 3), text("third"));
        let values: Vec<String> = outer
            .finish()
            .into_iter()
            .map(|found| found.value)
            .collect();
        assert_eq!(values, ["first", "third"]);
    }
}
