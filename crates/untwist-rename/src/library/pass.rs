use oxc_allocator::GetAllocator;
use oxc_ast::ast::Program;
use rustc_hash::{FxHashMap, FxHashSet};
use untwist_core::pass::{Cx, Interest, Pass};

use super::annotate::annotate;
use super::annotate::one_line;
use super::{intersect_ranges, match_program, parse_ranges, ranges_text, Hit, Ranges, VERSIONS_DISAGREE};

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
    let mut seen: FxHashSet<(&str, &[String])> = FxHashSet::default();
    hits.iter()
        .copied()
        .filter(|hit| {
            hit.role != super::table::Role::Function
                || seen.insert((hit.label.as_str(), hit.modules.as_slice()))
        })
        .collect()
}

#[derive(Default)]
struct Summary<'h> {
    modules: FxHashSet<&'h [String]>,
    ranges: Option<Ranges>,
}

impl<'h> Summary<'h> {
    fn add(&mut self, hit: &'h Hit) {
        self.modules.insert(hit.modules.as_slice());
        self.ranges = Some(match self.ranges.take() {
            None => hit.ranges.clone(),
            Some(known) => intersect_ranges(&known, &hit.ranges),
        });
    }

    fn range(&self) -> String {
        ranges_text(self.ranges.as_deref().unwrap_or_default())
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

fn combined_range(ranges: &[String]) -> String {
    let mut window: Option<Ranges> = None;
    for range in ranges {
        if range == VERSIONS_DISAGREE {
            return VERSIONS_DISAGREE.to_string();
        }
        let parsed = parse_ranges(range);
        window = Some(match window {
            None => parsed,
            Some(known) => intersect_ranges(&known, &parsed),
        });
    }
    ranges_text(window.as_deref().unwrap_or_default())
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
    for hit in hits.iter().filter(|hit| !hit.shared) {
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
                "{NOTE_PREFIX}{}@{}, {count} {noun}{NOTE_SUFFIX}",
                one_line(label),
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
