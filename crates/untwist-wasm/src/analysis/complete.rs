use rustc_hash::FxHashSet;
use serde::Serialize;

use super::index::{Index, Occurrence, NONE};

#[derive(Debug, Serialize)]
pub struct Completion<'a> {
    pub label: &'a str,
    pub kind: &'static str,
}

const KEYWORDS: [&str; 38] = [
    "async",
    "await",
    "break",
    "case",
    "catch",
    "class",
    "const",
    "continue",
    "debugger",
    "default",
    "delete",
    "do",
    "else",
    "export",
    "extends",
    "false",
    "finally",
    "for",
    "function",
    "if",
    "import",
    "in",
    "instanceof",
    "let",
    "new",
    "null",
    "of",
    "return",
    "static",
    "super",
    "switch",
    "this",
    "throw",
    "true",
    "try",
    "typeof",
    "var",
    "while",
];

pub fn completions(index: &Index, pos: u32) -> Vec<Completion<'_>> {
    let mut seen: FxHashSet<&str> = FxHashSet::default();
    let mut list = Vec::new();
    for scope in index.ancestors(index.scope_at(pos)) {
        for (_, name, symbol) in index.bindings_of(scope) {
            let label = index.names.get(*name);
            if seen.insert(label) {
                let kind = index.symbol(*symbol).map_or("binding", |s| s.kind.label());
                list.push(Completion { label, kind });
            }
        }
    }
    let typed = |occ: &&Occurrence| occ.start <= pos && pos <= occ.end;
    for occ in index
        .occurrences
        .iter()
        .filter(|occ| occ.symbol == NONE && !typed(occ))
    {
        let label = index.names.get(occ.name);
        if seen.insert(label) {
            list.push(Completion {
                label,
                kind: "global",
            });
        }
    }
    for label in KEYWORDS {
        if seen.insert(label) {
            list.push(Completion {
                label,
                kind: "keyword",
            });
        }
    }
    list
}
