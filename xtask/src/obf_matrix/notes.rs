use std::collections::BTreeMap;

use sha2::{Digest, Sha256};

const INFO: [&str; 7] = [
    "string array found",
    "string array rotator found",
    "string array rotator: none",
    "string array decoders found",
    "no string array found; deobfuscation skipped",
    "shell around the whole program was removed",
    "was peeled",
];

const MASKED_AFTER: [&str; 7] = [
    "defines", "property", "access", "name", "named", "binding", "for",
];
const CLASS_WORDS: usize = 14;

pub type Classes = BTreeMap<String, usize>;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Warnings {
    pub classes: Classes,
    pub errors: usize,
}

impl Warnings {
    pub fn total(&self) -> usize {
        self.classes.values().sum()
    }
}

pub fn is_info(message: &str) -> bool {
    INFO.iter().any(|info| message.contains(info))
}

fn mask(word: &str) -> String {
    if word.chars().any(|c| c.is_ascii_digit()) {
        return "N".to_string();
    }
    let plain = word
        .chars()
        .all(|c| c.is_ascii_lowercase() || matches!(c, '\'' | '-' | '(' | ')'));
    if plain {
        word.to_string()
    } else {
        "_".to_string()
    }
}

pub fn class_of(line: &str) -> String {
    let body = line
        .strip_prefix("note: ")
        .or_else(|| line.strip_prefix("warning: "))
        .unwrap_or(line);
    let (pass, message) = match body.split_once(": ") {
        Some((pass, rest)) if !pass.contains(' ') => (pass, rest),
        _ => ("cli", body),
    };
    let message = message
        .strip_prefix("find-string-array: ")
        .unwrap_or(message);
    let head = message
        .split([':', ',', ';', '('])
        .next()
        .unwrap_or(message);
    let mut words = Vec::new();
    let mut masked_next = false;
    for word in head.split_whitespace().take(CLASS_WORDS) {
        words.push(if masked_next {
            "_".to_string()
        } else {
            mask(word)
        });
        masked_next = MASKED_AFTER.contains(&word);
    }
    format!("{pass}: {}", words.join(" "))
}

pub fn class_id(class: &str) -> String {
    let digest = Sha256::digest(class.as_bytes());
    digest[..4].iter().map(|b| format!("{b:02x}")).collect()
}

pub fn warnings_of(stderr: &str, exit: Option<i32>) -> Warnings {
    let mut warnings = Warnings::default();
    for line in stderr.lines() {
        if line.contains("panicked") || line.starts_with("error:") || line.starts_with("Error:") {
            warnings.errors += 1;
            continue;
        }
        let is_note = line.starts_with("note: ");
        if !is_note && !line.starts_with("warning: ") {
            continue;
        }
        if is_note && is_info(line) {
            continue;
        }
        *warnings.classes.entry(class_of(line)).or_default() += 1;
    }
    if exit != Some(0) {
        warnings.errors += 1;
    }
    warnings
}

pub fn render_ids(classes: &Classes) -> String {
    if classes.is_empty() {
        return "-".to_string();
    }
    let mut ids: Vec<String> = classes
        .iter()
        .map(|(class, n)| format!("{}={n}", class_id(class)))
        .collect();
    ids.sort();
    ids.join(",")
}

pub fn parse_ids(text: &str) -> BTreeMap<String, usize> {
    text.split(',')
        .filter(|part| !part.is_empty() && *part != "-")
        .filter_map(|part| {
            let (id, n) = part.split_once('=')?;
            Some((id.to_string(), n.parse().ok()?))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn found_notes_are_not_warnings() {
        let w = warnings_of(
            "note: find-string-array: string array found: 19749 entries\nnote: find-decoders: string array decoders found: 1\n",
            Some(0),
        );
        assert_eq!(w.total(), 0);
        assert_eq!(w.errors, 0);
    }

    #[test]
    fn a_refusal_is_one_warning_of_its_class() {
        let w = warnings_of(
            "note: remove-decoder-machinery: decoder machinery kept: 108223 reference(s) to the string array did not decode\n",
            Some(0),
        );
        assert_eq!(
            w.classes.keys().collect::<Vec<_>>(),
            ["remove-decoder-machinery: decoder machinery kept"]
        );
    }

    #[test]
    fn names_from_the_input_do_not_reach_the_class() {
        assert_eq!(
            class_of(
                "note: inline-object-props: object not inlined: property TWOFA is not defined"
            ),
            "inline-object-props: object not inlined"
        );
        assert_eq!(
            class_of("note: inline-object-props: the object literal defines teams more than once or through __proto__, so"),
            "inline-object-props: the object literal defines _ more than once or through _"
        );
    }

    #[test]
    fn a_nonzero_exit_and_a_panic_are_errors() {
        let w = warnings_of("thread 'main' panicked at x\n", Some(101));
        assert_eq!(w.errors, 2);
    }

    #[test]
    fn class_ids_round_trip() {
        let mut classes = Classes::new();
        classes.insert("a: b".to_string(), 3);
        let ids = parse_ids(&render_ids(&classes));
        assert_eq!(ids.get(&class_id("a: b")), Some(&3));
        assert!(parse_ids("-").is_empty());
    }
}
