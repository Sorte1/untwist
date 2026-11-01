use std::hash::{Hash, Hasher};

use crate::value::{JsString, ReservedString};

const SLOTS: usize = 32;

fn key_hash(pattern: &str, flags: &str) -> u64 {
    let mut hasher = rustc_hash::FxHasher::default();
    pattern.hash(&mut hasher);
    flags.hash(&mut hasher);
    hasher.finish()
}

#[derive(Default)]
pub(super) struct SharedRegexes {
    seen: [u64; SLOTS],
    seen_len: usize,
    seen_next: usize,
    kept: Vec<(Box<str>, Box<str>, regress::Regex)>,
}

impl SharedRegexes {
    pub(super) fn get(&self, pattern: &str, flags: &str) -> Option<regress::Regex> {
        self.kept
            .iter()
            .find(|(kept_pattern, kept_flags, _)| {
                **kept_pattern == *pattern && **kept_flags == *flags
            })
            .map(|(_, _, regex)| regex.clone())
    }

    pub(super) fn offer(&mut self, pattern: &str, flags: &str, regex: &regress::Regex) {
        let hash = key_hash(pattern, flags);
        let seen_before = self.seen[..self.seen_len].contains(&hash);
        if !seen_before {
            self.seen[self.seen_next] = hash;
            self.seen_next = (self.seen_next + 1) % SLOTS;
            self.seen_len = (self.seen_len + 1).min(SLOTS);
            return;
        }
        if self.kept.len() < SLOTS {
            self.kept
                .push((pattern.into(), flags.into(), regex.clone()));
        }
    }
}

#[derive(Default)]
pub(super) struct SharedSources {
    slots: [Option<(usize, ReservedString)>; SLOTS],
    next: usize,
}

impl SharedSources {
    pub(super) fn get(&self, node: usize) -> Option<JsString> {
        self.slots
            .iter()
            .flatten()
            .find(|(address, _)| *address == node)
            .and_then(|(_, text)| text.upgrade())
    }

    pub(super) fn keep(&mut self, node: usize, text: &JsString) {
        let reused = self
            .slots
            .iter()
            .position(|slot| slot.as_ref().is_some_and(|(address, _)| *address == node));
        let index = reused.unwrap_or(self.next);
        if reused.is_none() {
            self.next = (self.next + 1) % SLOTS;
        }
        self.slots[index] = Some((node, text.reserve()));
    }
}
