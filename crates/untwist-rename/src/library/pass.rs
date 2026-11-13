use oxc_allocator::GetAllocator;
use oxc_ast::ast::Program;
use rustc_hash::FxHashMap;
use untwist_core::pass::{Cx, Interest, Pass};

use super::annotate::annotate;
use super::{match_program, version_key, version_range, Hit};

pub const NAME: &str = "library-names";

pub struct LibraryNamesPass {
    done: bool,
}

pub fn boxed<'a>() -> Box<dyn Pass<'a> + 'a> {
    Box::new(LibraryNamesPass { done: false })
}

pub fn reported(hits: &[Hit]) -> Vec<&Hit> {
    hits.iter()
        .filter(|hit| hit.outermost && hit.reported)
        .collect()
}

pub fn commented<'h>(hits: &[&'h Hit]) -> Vec<&'h Hit> {
    let mut seen: Vec<(&str, &[String])> = Vec::new();
    hits.iter()
        .copied()
        .filter(|hit| {
            if hit.role != super::table::Role::Function {
                return true;
            }
            let key = (hit.label.as_str(), hit.modules.as_slice());
            if seen.contains(&key) {
                return false;
            }
            seen.push(key);
            true
        })
        .collect()
}

#[derive(Default)]
struct Summary {
    modules: Vec<String>,
    low: Option<String>,
    high: Option<String>,
    union: Option<(String, String)>,
}

impl Summary {
    fn add(&mut self, hit: &Hit) {
        for module in &hit.modules {
            if !self.modules.contains(module) {
                self.modules.push(module.clone());
            }
        }
        let (low, high) = &hit.versions;
        if self
            .low
            .as_ref()
            .is_none_or(|known| version_key(low) > version_key(known))
        {
            self.low = Some(low.clone());
        }
        if self
            .high
            .as_ref()
            .is_none_or(|known| version_key(high) < version_key(known))
        {
            self.high = Some(high.clone());
        }
        let union = self
            .union
            .get_or_insert_with(|| (low.clone(), high.clone()));
        if version_key(low) < version_key(&union.0) {
            union.0 = low.clone();
        }
        if version_key(high) > version_key(&union.1) {
            union.1 = high.clone();
        }
    }

    fn range(&self) -> String {
        match (&self.low, &self.high) {
            (Some(low), Some(high)) if version_key(low) <= version_key(high) => {
                version_range(low, high)
            }
            _ => self
                .union
                .as_ref()
                .map(|(low, high)| version_range(low, high))
                .unwrap_or_default(),
        }
    }
}

const NOTE_PREFIX: &str = "library: ";
const NOTE_SUFFIX: &str = " recognised";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LibrarySummary {
    pub library: String,
    pub range: String,
    pub modules: usize,
}

pub fn parse_note(message: &str) -> Option<LibrarySummary> {
    let body = message
        .strip_prefix(NOTE_PREFIX)?
        .strip_suffix(NOTE_SUFFIX)?;
    let (name, count) = body.rsplit_once(", ")?;
    let (library, range) = name.rsplit_once('@')?;
    let modules = count.split(' ').next()?.parse().ok()?;
    Some(LibrarySummary {
        library: library.to_string(),
        range: range.to_string(),
        modules,
    })
}

fn range_bounds(range: &str) -> (String, String) {
    match range.split_once('\u{2013}') {
        Some((low, high)) => (low.to_string(), high.to_string()),
        None => (range.to_string(), range.to_string()),
    }
}

fn combined_range(ranges: &[String]) -> String {
    let mut window: Option<(String, String)> = None;
    for range in ranges {
        let (low, high) = range_bounds(range);
        window = Some(match window {
            None => (low, high),
            Some((known_low, known_high)) => {
                let low = if version_key(&low) > version_key(&known_low) {
                    low
                } else {
                    known_low
                };
                let high = if version_key(&high) < version_key(&known_high) {
                    high
                } else {
                    known_high
                };
                (low, high)
            }
        });
    }
    match window {
        Some((low, high)) if version_key(&low) <= version_key(&high) => version_range(&low, &high),
        _ => ranges.join(", "),
    }
}

pub fn summaries<'n>(messages: impl IntoIterator<Item = &'n str>) -> Vec<LibrarySummary> {
    let mut order: Vec<String> = Vec::new();
    let mut grouped: FxHashMap<String, (Vec<String>, usize)> = FxHashMap::default();
    for summary in messages.into_iter().filter_map(parse_note) {
        if !grouped.contains_key(&summary.library) {
            order.push(summary.library.clone());
        }
        let entry = grouped.entry(summary.library).or_default();
        if !entry.0.contains(&summary.range) {
            entry.0.push(summary.range);
        }
        entry.1 += summary.modules;
    }
    order
        .into_iter()
        .map(|library| {
            let (ranges, modules) = &grouped[&library];
            LibrarySummary {
                range: combined_range(ranges),
                modules: *modules,
                library,
            }
        })
        .collect()
}

pub fn summary_notes(hits: &[&Hit]) -> Vec<String> {
    let mut libraries: FxHashMap<&str, Summary> = FxHashMap::default();
    let mut order: Vec<&str> = Vec::new();
    for hit in hits {
        if !libraries.contains_key(hit.label.as_str()) {
            order.push(&hit.label);
        }
        libraries.entry(&hit.label).or_default().add(hit);
    }
    order
        .into_iter()
        .map(|label| {
            let summary = &libraries[label];
            let count = summary.modules.len();
            let noun = if count == 1 { "module" } else { "modules" };
            format!(
                "{NOTE_PREFIX}{label}@{}, {count} {noun}{NOTE_SUFFIX}",
                summary.range()
            )
        })
        .collect()
}

impl<'a> Pass<'a> for LibraryNamesPass {
    fn name(&self) -> &'static str {
        NAME
    }

    fn interest(&self) -> Interest {
        Interest::EXIT_PROGRAM
    }

    fn exit_program(&mut self, program: &mut Program<'a>, cx: &mut Cx<'a, '_>) {
        if std::mem::replace(&mut self.done, true) {
            return;
        }
        let found = match_program(program, cx.scoping());
        let hits = reported(&found.hits);
        if hits.is_empty() {
            return;
        }
        let alloc = cx.allocator();
        annotate(program, &commented(&hits), alloc);
        for note in summary_notes(&hits) {
            cx.note(note, None);
        }
    }
}
