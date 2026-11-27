use oxc_semantic::Scoping;
use oxc_span::Span;
use oxc_syntax::symbol::SymbolId;
use rustc_hash::FxHashSet;
use untwist_eval::fold::Const;

use super::super::base91;
use super::super::refs::{inside, Refs};

const EVAL_FUEL: u64 = 1_000_000;
const EVAL_HEAP: u64 = 1 << 22;
const BYTE_SINK: &str = "(a){var s=\"\";for(var i=0;i<a.length;i++){var v=a[i];if(typeof v!==\"number\"||v!==(v&255))throw 0;s+=v+\",\"}return s}";

pub(super) struct Sandbox<'k> {
    pub(super) source: &'k str,
    pub(super) scoping: &'k Scoping,
    pub(super) strict: bool,
    pub(super) array: Option<(SymbolId, Span)>,
    pub(super) helpers: &'k FxHashSet<SymbolId>,
    pub(super) bytes: Option<SymbolId>,
}

pub(super) fn js_string_literal(units: &[u16]) -> String {
    let mut out = String::from("\"");
    for unit in units {
        match char::from_u32(u32::from(*unit)) {
            Some(c) if c.is_ascii_alphanumeric() || c == ' ' => out.push(c),
            _ => out.push_str(&format!("\\u{unit:04x}")),
        }
    }
    out.push('"');
    out
}

fn run(text: &str) -> Option<Const> {
    let alloc = oxc_allocator::Allocator::default();
    let parsed = untwist_core::parse::parse_program(&alloc, text);
    if !parsed.diagnostics.is_empty() {
        return None;
    }
    untwist_eval::closed_script_value(&parsed.program, EVAL_FUEL, EVAL_HEAP)
}

fn text_of(source: &str, span: Span) -> &str {
    source
        .get(span.start as usize..span.end as usize)
        .unwrap_or("")
}

pub(super) fn outside_symbols(refs: &Refs<'_>, scoping: &Scoping, region: Span) -> Vec<SymbolId> {
    let mut found: Vec<SymbolId> = Vec::new();
    for (r, _) in refs.all.iter().filter(|(r, _)| inside(region, r.span)) {
        let Some(symbol) = r.symbol else {
            continue;
        };
        if !inside(region, scoping.symbol_span(symbol)) && !found.contains(&symbol) {
            found.push(symbol);
        }
    }
    found
}

impl Sandbox<'_> {
    fn definition(&self, symbol: SymbolId) -> Option<String> {
        let name = self.scoping.symbol_name(symbol);
        match self.array {
            Some((array, init)) if array == symbol => {
                return Some(format!("var {name} = {};", text_of(self.source, init)))
            }
            _ => {}
        }
        if self.bytes == Some(symbol) {
            return Some(format!("function {name}{BYTE_SINK}"));
        }
        self.helpers
            .contains(&symbol)
            .then(|| format!("function {name}(){{}}"))
    }

    pub(super) fn script(&self, outside: &[SymbolId], own: &[&str], body: &str) -> Option<String> {
        let mut seen: Vec<&str> = own.to_vec();
        let mut out = String::from(if self.strict { "\"use strict\";\n" } else { "" });
        for symbol in outside {
            let name = self.scoping.symbol_name(*symbol);
            if seen.contains(&name) {
                return None;
            }
            seen.push(name);
            out.push_str(&self.definition(*symbol)?);
            out.push('\n');
        }
        out.push_str(body);
        Some(out)
    }

    pub(super) fn decode(
        &self,
        outside: &[SymbolId],
        (decoder, name): (&str, &str),
        raw: &[u16],
    ) -> Option<String> {
        let body = format!("{decoder}\n{name}({});", js_string_literal(raw));
        let Const::Str(value) = run(&self.script(outside, &[name], &body)?)? else {
            return None;
        };
        match self.bytes {
            None => Some(value),
            Some(_) => {
                let bytes: Vec<u8> = value.strip_suffix(',').map_or(Some(Vec::new()), |list| {
                    list.split(',').map(|b| b.parse::<u8>().ok()).collect()
                })?;
                base91::utf8(&bytes)
            }
        }
    }
}
