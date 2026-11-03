pub const SNAPSHOT_CAP_BYTES: usize = 24 * 1024 * 1024;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Extras {
    pub refusals: bool,
    pub snapshots: bool,
}

impl Extras {
    pub fn from_words(words: &str) -> Self {
        words
            .split_whitespace()
            .fold(Self::default(), |extras, word| match word {
                "refusals" => Self {
                    refusals: true,
                    ..extras
                },
                "snapshots" => Self {
                    snapshots: true,
                    ..extras
                },
                _ => extras,
            })
    }

    pub fn snapshot_cap(self) -> Option<usize> {
        self.snapshots.then_some(SNAPSHOT_CAP_BYTES)
    }
}

#[cfg(test)]
mod tests {
    use super::Extras;

    #[test]
    fn extras_are_named_by_words_and_unknown_words_ask_for_nothing() {
        assert_eq!(Extras::from_words(""), Extras::default());
        assert!(Extras::from_words("refusals").refusals);
        assert!(Extras::from_words("  other refusals ").refusals);
        assert_eq!(Extras::from_words("other"), Extras::default());
        let both = Extras::from_words("snapshots refusals");
        assert!(both.refusals && both.snapshots);
        assert_eq!(both.snapshot_cap(), Some(super::SNAPSHOT_CAP_BYTES));
        assert_eq!(Extras::from_words("refusals").snapshot_cap(), None);
    }
}
