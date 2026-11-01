use std::hash::BuildHasher;

use hashbrown::HashTable;
use rustc_hash::{FxBuildHasher, FxHashMap};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Name(u64);

pub const STATIC_NAMES: [&str; 38] = [
    "arguments",
    "undefined",
    "NaN",
    "Infinity",
    "globalThis",
    "eval",
    "Function",
    "setTimeout",
    "setInterval",
    "setImmediate",
    "requestAnimationFrame",
    "Date",
    "fetch",
    "XMLHttpRequest",
    "require",
    "process",
    "document",
    "window",
    "navigator",
    "parseInt",
    "parseFloat",
    "Boolean",
    "isNaN",
    "isFinite",
    "decodeURIComponent",
    "decodeURI",
    "encodeURIComponent",
    "encodeURI",
    "escape",
    "unescape",
    "atob",
    "btoa",
    "String",
    "Number",
    "Math",
    "Object",
    "JSON",
    "RegExp",
];

const STATIC_TAG: u64 = 1 << 62;
const OTHER_TAG: u64 = 2 << 62;

pub struct Names<'a> {
    source: &'a str,
    in_source: HashTable<u32>,
    others: FxHashMap<&'a str, Name>,
}

impl Default for Names<'_> {
    fn default() -> Self {
        Names::for_source("")
    }
}

fn is_identifier_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_' || byte == b'$'
}

fn is_ascii_identifier(text: &str) -> bool {
    !text.is_empty() && text.bytes().all(is_identifier_byte)
}

fn text_hash(text: &str) -> u64 {
    FxBuildHasher.hash_one(text)
}

fn run_at(source: &str, offset: u32) -> &str {
    let start = offset as usize;
    let rest = source.as_bytes().get(start..).unwrap_or_default();
    let length = rest
        .iter()
        .position(|byte| !is_identifier_byte(*byte))
        .unwrap_or(rest.len());
    source.get(start..start + length).unwrap_or_default()
}

fn run_matches(source: &str, offset: u32, text: &str) -> bool {
    let start = offset as usize;
    let Some(end) = start.checked_add(text.len()) else {
        return false;
    };
    let bytes = source.as_bytes();
    bytes.get(start..end) == Some(text.as_bytes())
        && !bytes.get(end).is_some_and(|byte| is_identifier_byte(*byte))
}

impl<'a> Names<'a> {
    pub fn for_source(source: &'a str) -> Self {
        Self {
            source,
            in_source: HashTable::new(),
            others: FxHashMap::default(),
        }
    }

    pub fn len(&self) -> usize {
        self.in_source.len() + self.others.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn intern(&mut self, text: &'a str) -> Name {
        let hash = text_hash(text);
        let identifier = is_ascii_identifier(text);
        if identifier {
            if let Some(offset) = self.find_in_source(text, hash) {
                return Name(u64::from(offset));
            }
        }
        if !self.others.is_empty() {
            if let Some(name) = self.others.get(text) {
                return *name;
            }
        }
        if let Some(index) = STATIC_NAMES.iter().position(|known| *known == text) {
            return Name(STATIC_TAG | index as u64);
        }
        match self.offset_in_source(text).filter(|_| identifier) {
            Some(offset) => self.insert_in_source(hash, offset),
            None => self.insert_other(text),
        }
    }

    fn find_in_source(&self, text: &str, hash: u64) -> Option<u32> {
        let source = self.source;
        self.in_source
            .find(hash, |offset| run_matches(source, *offset, text))
            .copied()
    }

    fn offset_in_source(&self, text: &str) -> Option<u32> {
        let offset = (text.as_ptr() as usize).checked_sub(self.source.as_ptr() as usize)?;
        let end = offset.checked_add(text.len())?;
        let bytes = self.source.as_bytes();
        let within = bytes.get(offset..end)?;
        if !std::ptr::eq(within.as_ptr(), text.as_ptr()) {
            return None;
        }
        if bytes.get(end).is_some_and(|byte| is_identifier_byte(*byte)) {
            return None;
        }
        u32::try_from(offset).ok()
    }

    fn insert_in_source(&mut self, hash: u64, offset: u32) -> Name {
        let source = self.source;
        self.in_source
            .insert_unique(hash, offset, |known| text_hash(run_at(source, *known)));
        Name(u64::from(offset))
    }

    fn insert_other(&mut self, text: &'a str) -> Name {
        let name = Name(OTHER_TAG | self.others.len() as u64);
        self.others.insert(text, name);
        name
    }
}
