use std::hash::BuildHasher;

use hashbrown::hash_table::{Entry, HashTable};
use rustc_hash::FxBuildHasher;
use untwist_core::pass::{Note, NoteKind};

fn key_hash(note: &Note) -> u64 {
    FxBuildHasher.hash_one((note.pass, note.message.as_str(), note.span))
}

fn same_key(left: &Note, right: &Note) -> bool {
    left.pass == right.pass && left.span == right.span && left.message == right.message
}

#[derive(Default)]
pub(crate) struct NoteSet {
    notes: Vec<Note>,
    rounds: Vec<u32>,
    table: HashTable<u32>,
}

impl NoteSet {
    pub(crate) fn push(&mut self, note: Note, round: u32) {
        let Ok(ordinal) = u32::try_from(self.notes.len()) else {
            return;
        };
        let Self {
            notes,
            rounds,
            table,
        } = self;
        let same = |stored: &u32| {
            notes
                .get(*stored as usize)
                .is_some_and(|seen| same_key(seen, &note))
        };
        let rehash = |stored: &u32| notes.get(*stored as usize).map_or(0, key_hash);
        match table.entry(key_hash(&note), same, rehash) {
            Entry::Occupied(entry) => {
                if let Some(seen) = rounds.get_mut(*entry.get() as usize) {
                    *seen = round;
                }
            }
            Entry::Vacant(entry) => {
                entry.insert(ordinal);
                notes.push(note);
                rounds.push(round);
            }
        }
    }

    pub(crate) fn into_current(self, settled: u32) -> Vec<Note> {
        self.notes
            .into_iter()
            .zip(self.rounds)
            .filter(|(note, round)| {
                note.span.is_none() || note.kind == NoteKind::Progress || *round == settled
            })
            .map(|(note, _)| note)
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use oxc_span::Span;

    fn note(message: &str, span: Option<Span>, kind: NoteKind) -> Note {
        Note {
            pass: "p",
            kind,
            message: message.to_string(),
            span,
        }
    }

    #[test]
    fn a_repeated_note_is_kept_once_and_takes_the_latest_round() {
        let at = Some(Span::new(1, 2));
        let mut set = NoteSet::default();
        set.push(note("a", at, NoteKind::Refusal), 0);
        set.push(note("b", at, NoteKind::Refusal), 0);
        set.push(note("a", at, NoteKind::Refusal), 1);
        let current = set.into_current(1);
        assert_eq!(current.len(), 1);
        assert_eq!(current[0].message, "a");
    }

    #[test]
    fn spanless_and_progress_notes_survive_whatever_their_round() {
        let mut set = NoteSet::default();
        set.push(note("a", None, NoteKind::Refusal), 0);
        set.push(note("b", Some(Span::new(3, 4)), NoteKind::Progress), 0);
        set.push(note("c", Some(Span::new(3, 4)), NoteKind::Refusal), 0);
        let kept: Vec<String> = set
            .into_current(2)
            .into_iter()
            .map(|note| note.message)
            .collect();
        assert_eq!(kept, ["a", "b"]);
    }

    #[test]
    fn notes_differing_only_in_span_or_pass_stay_distinct() {
        let mut set = NoteSet::default();
        set.push(note("a", Some(Span::new(1, 2)), NoteKind::Refusal), 0);
        set.push(note("a", Some(Span::new(1, 3)), NoteKind::Refusal), 0);
        let mut other = note("a", Some(Span::new(1, 2)), NoteKind::Refusal);
        other.pass = "q";
        set.push(other, 0);
        assert_eq!(set.into_current(0).len(), 3);
    }
}
