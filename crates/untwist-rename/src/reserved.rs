use oxc_semantic::Scoping;
use rustc_hash::FxHashSet;

pub const RESERVED_WORDS: [&str; 44] = [
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
    "enum",
    "export",
    "extends",
    "false",
    "finally",
    "for",
    "function",
    "if",
    "implements",
    "import",
    "in",
    "instanceof",
    "interface",
    "let",
    "new",
    "null",
    "package",
    "private",
    "protected",
    "public",
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
    "void",
    "while",
];

pub const PSEUDO_GLOBALS: [&str; 8] = [
    "with",
    "yield",
    "arguments",
    "eval",
    "undefined",
    "NaN",
    "Infinity",
    "globalThis",
];

pub fn is_reserved(name: &str) -> bool {
    RESERVED_WORDS.contains(&name) || PSEUDO_GLOBALS.contains(&name)
}

#[derive(Default)]
pub struct Excluded {
    names: FxHashSet<String>,
}

impl Excluded {
    pub fn from_scoping(scoping: &Scoping) -> Self {
        let mut names = FxHashSet::default();
        for name in scoping.root_unresolved_references().keys() {
            names.insert(name.as_str().to_string());
        }
        Self { names }
    }

    pub fn contains(&self, name: &str) -> bool {
        is_reserved(name) || self.names.contains(name)
    }

    pub fn lists(&self, name: &str) -> bool {
        self.names.contains(name)
    }

    pub fn insert(&mut self, name: &str) {
        self.names.insert(name.to_string());
    }
}
