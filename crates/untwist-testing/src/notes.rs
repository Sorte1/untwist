use std::collections::HashSet;

use untwist_core::pass::{Note, NoteKind};

pub type Table = &'static [(&'static str, &'static str)];

pub fn declared_message_constants(source: &str) -> Vec<String> {
    let mut names = Vec::new();
    for line in source.lines() {
        if let Some(rest) = line.strip_prefix("pub const ") {
            if let Some(name) = rest.split(':').next() {
                names.push(name.trim().to_string());
            }
        } else if let Some(rest) = line.strip_prefix("pub use ") {
            let path = rest.trim_end_matches(';');
            let name = match path.split(" as ").nth(1) {
                Some(alias) => alias.trim().to_string(),
                None => path.rsplit("::").next().unwrap_or(path).trim().to_string(),
            };
            names.push(name);
        }
    }
    names
}

fn union(tables: &[Table]) -> Vec<(&'static str, &'static str)> {
    tables
        .iter()
        .flat_map(|table| table.iter().copied())
        .collect()
}

pub fn no_note_is_a_substring_of_another(tables: &[Table]) {
    let messages = union(tables);
    for (name, message) in &messages {
        for (other_name, other) in &messages {
            if name == other_name {
                continue;
            }
            assert!(
                !message.contains(other),
                "{name} contains {other_name}, so a test that pins {other_name} by substring \
                 cannot tell them apart"
            );
        }
    }
}

pub fn every_constant_is_catalogued_exactly_once(source: &str, tables: &[Table]) {
    let catalogued: Vec<String> = union(tables)
        .iter()
        .map(|(name, _)| (*name).to_string())
        .collect();
    let unique: HashSet<&String> = catalogued.iter().collect();
    assert_eq!(
        catalogued.len(),
        unique.len(),
        "a constant is catalogued more than once: {catalogued:?}"
    );
    let declared: HashSet<String> = declared_message_constants(source).into_iter().collect();
    let catalogued: HashSet<String> = catalogued.into_iter().collect();
    let missing: Vec<&String> = declared.difference(&catalogued).collect();
    let extra: Vec<&String> = catalogued.difference(&declared).collect();
    assert!(
        missing.is_empty(),
        "declared in notes.rs but in no table: {missing:?}"
    );
    assert!(
        extra.is_empty(),
        "catalogued but not declared in notes.rs: {extra:?}"
    );
}

pub fn every_catalogued_note_is_reachable(table: Table, pinned: &[&str]) {
    for (name, message) in table {
        assert!(
            pinned.contains(message),
            "{name} is in the catalogue with no case that fires it"
        );
    }
}

pub fn every_case_names_a_catalogued_note(table: Table, pinned: &[&str]) {
    for expects in pinned {
        assert!(
            table.iter().any(|(_, message)| message == expects),
            "{expects} is pinned by a case but is not in the catalogue"
        );
    }
}

pub fn every_note_carries_the_kind_its_text_claims(notes: &[Note], progress: &[&str]) {
    for note in notes {
        let expected = match progress.iter().any(|text| note.message.contains(text)) {
            true => NoteKind::Progress,
            false => NoteKind::Refusal,
        };
        assert_eq!(
            note.kind, expected,
            "{} was emitted as {:?}",
            note.message, note.kind
        );
    }
}
