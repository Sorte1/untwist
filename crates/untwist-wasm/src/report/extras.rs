#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Extras {
    pub refusals: bool,
}

impl Extras {
    pub fn from_words(words: &str) -> Self {
        words
            .split_whitespace()
            .fold(Self::default(), |extras, word| match word {
                "refusals" => Self { refusals: true },
                _ => extras,
            })
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
    }
}
