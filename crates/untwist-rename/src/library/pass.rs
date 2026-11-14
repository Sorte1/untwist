use oxc_allocator::GetAllocator;
use oxc_ast::ast::Program;
use rustc_hash::{FxHashMap, FxHashSet};
use untwist_core::pass::{Cx, Interest, Pass};

use super::annotate::annotate;
use super::annotate::one_line;
use super::{
    intersect_ranges, match_program, parse_ranges, ranges_text, union_ranges, Hit, Ranges,
};

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
    common: Option<Ranges>,
    all: Ranges,
}

impl<'h> Summary<'h> {
    fn add(&mut self, hit: &'h Hit) {
        self.modules.insert(hit.modules.as_slice());
        self.common = Some(match self.common.take() {
            None => hit.ranges.clone(),
            Some(known) => intersect_ranges(&known, &hit.ranges),
        });
        self.all = union_ranges(&self.all, &hit.ranges);
    }

    fn window(&self) -> Window {
        window(self.common.as_deref().unwrap_or_default(), &self.all)
    }
}

struct Window {
    range: String,
    disagree: bool,
}

fn window(common: &[(String, String)], all: &[(String, String)]) -> Window {
    match (common.is_empty(), all.is_empty()) {
        (false, _) => Window {
            range: ranges_text(common),
            disagree: false,
        },
        (true, false) => Window {
            range: ranges_text(all),
            disagree: true,
        },
        (true, true) => Window {
            range: String::new(),
            disagree: false,
        },
    }
}

const NOTE_PREFIX: &str = "library: ";
const NOTE_SUFFIX: &str = " recognised";
const DISAGREE: &str = " (matches disagree)";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LibrarySummary {
    pub library: String,
    pub range: String,
    pub disagree: bool,
    pub modules: usize,
}

fn note_text(library: &str, window: &Window, count: usize) -> String {
    let noun = if count == 1 { "module" } else { "modules" };
    let range = match (window.range.is_empty(), window.disagree) {
        (true, _) => String::new(),
        (false, false) => format!(" {}", window.range),
        (false, true) => format!(" {}{DISAGREE}", window.range),
    };
    format!("{NOTE_PREFIX}{library}{range}, {count} {noun}{NOTE_SUFFIX}")
}

pub fn parse_note(message: &str) -> Option<LibrarySummary> {
    let body = message
        .strip_prefix(NOTE_PREFIX)?
        .strip_suffix(NOTE_SUFFIX)?;
    let (name, count) = body.rsplit_once(", ")?;
    let modules = count.split(' ').next()?.parse().ok()?;
    let (library, range) = name.split_once(' ').unwrap_or((name, ""));
    let (range, disagree) = match range.strip_suffix(DISAGREE) {
        Some(range) => (range, true),
        None => (range, false),
    };
    Some(LibrarySummary {
        library: library.to_string(),
        range: range.to_string(),
        disagree,
        modules,
    })
}

#[derive(Default)]
struct Merged {
    common: Option<Ranges>,
    all: Ranges,
    disagree: bool,
    modules: usize,
}

impl Merged {
    fn add(&mut self, summary: &LibrarySummary) {
        let parsed = parse_ranges(&summary.range);
        self.disagree |= summary.disagree;
        self.common = Some(match self.common.take() {
            None => parsed.clone(),
            Some(known) => intersect_ranges(&known, &parsed),
        });
        self.all = union_ranges(&self.all, &parsed);
        self.modules += summary.modules;
    }

    fn window(&self) -> Window {
        match self.disagree {
            true => window(&[], &self.all),
            false => window(self.common.as_deref().unwrap_or_default(), &self.all),
        }
    }
}

pub fn summaries<'n>(messages: impl IntoIterator<Item = &'n str>) -> Vec<LibrarySummary> {
    let mut order: Vec<String> = Vec::new();
    let mut grouped: FxHashMap<String, Merged> = FxHashMap::default();
    for summary in messages.into_iter().filter_map(parse_note) {
        if !grouped.contains_key(&summary.library) {
            order.push(summary.library.clone());
        }
        grouped
            .entry(summary.library.clone())
            .or_default()
            .add(&summary);
    }
    order
        .into_iter()
        .map(|library| {
            let merged = &grouped[&library];
            let window = merged.window();
            LibrarySummary {
                range: window.range,
                disagree: window.disagree,
                modules: merged.modules,
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
            note_text(&one_line(label), &summary.window(), summary.modules.len())
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
            cx.progress(note, None);
        }
    }
}
