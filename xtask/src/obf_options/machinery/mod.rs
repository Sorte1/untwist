mod names;
mod scan;
mod shapes;

use std::collections::BTreeMap;

use oxc_allocator::Allocator;
use oxc_ast_visit::Visit;
use untwist_core::parse::{has_error, parse_program};

use names::Names;
use scan::Scan;

const MIN_LOOSE_ARRAY_STRINGS: usize = 10;
const DEBUG_STRINGS: [&str; 5] = ["debu", "gger", "debugger", "while (true) {}", "stateObject"];
const DEBUG_PATTERNS: [&str; 2] = ["function *\\( *\\)", "\\+\\+ *(?:[a-zA-Z_$]"];
const SELF_DEFENDING: &str = "(((.+)+)+)+$";
const GLOBAL_GETTER: &str = "return this";
const REDIRECT: &str = "about:blank";
const CONSOLE_METHODS: [&str; 2] = ["exception", "table"];

pub const MACHINERY: [&str; 14] = [
    "string_arrays",
    "decoders",
    "decoder_calls",
    "rotators",
    "dispatchers",
    "proxy_objects",
    "opaque_predicates",
    "debug_protection",
    "self_defending",
    "console_override",
    "domain_lock",
    "call_controllers",
    "global_getters",
    "dead_declarations",
];

pub const SIMPLIFY: [&str; 8] = [
    "number_expressions",
    "hex_numbers",
    "split_strings",
    "escapes",
    "object_key_assignments",
    "bool_arrays",
    "computed_members",
    "quoted_keys",
];

pub const OUTPUT_ONLY: [&str; 1] = ["dead_declarations"];

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Found {
    pub parses: bool,
    pub counts: BTreeMap<&'static str, usize>,
}

impl Found {
    pub fn get(&self, class: &str) -> usize {
        self.counts.get(class).copied().unwrap_or(0)
    }

    fn bump(&mut self, class: &'static str) {
        self.add(class, 1);
    }

    fn add(&mut self, class: &'static str, n: usize) {
        if n > 0 {
            *self.counts.entry(class).or_default() += n;
        }
    }

    pub fn present(&self, classes: &[&'static str]) -> Vec<&'static str> {
        classes
            .iter()
            .copied()
            .filter(|c| self.get(c) > 0)
            .collect()
    }

    pub fn render(&self, classes: &[&'static str]) -> String {
        let parts: Vec<String> = self
            .present(classes)
            .into_iter()
            .map(|c| format!("{c}={}", self.get(c)))
            .collect();
        if parts.is_empty() {
            "-".to_string()
        } else {
            parts.join(",")
        }
    }
}

pub fn measure(source: &str) -> Found {
    let alloc = Allocator::default();
    let parsed = parse_program(&alloc, source);
    let mut names = Names::default();
    names.visit_program(&parsed.program);
    let closure = names.decoder_closure();
    let mut scan = Scan {
        source,
        closure: &closure,
        found: Found::default(),
        folded_depth: 0,
    };
    scan.visit_program(&parsed.program);
    let mut found = scan.found;
    found.add(
        "string_arrays",
        names.array_fns.len() + names.legacy_arrays().len(),
    );
    found.add("decoders", names.decoders().len());
    found.add(
        "dead_declarations",
        names
            .bare_declarations
            .iter()
            .filter(|name| !names.referenced.contains(*name))
            .count(),
    );
    found.parses = !has_error(&parsed.diagnostics);
    found
}

#[cfg(test)]
mod tests;
