use oxc_allocator::Allocator;
use oxc_ast::ast::*;
use oxc_ast_visit::{walk_js, VisitJs};
use oxc_span::GetSpan;

use crate::parse::{has_error, parse_program};

mod guard_key;
mod guards;
#[cfg(debug_assertions)]
mod rounds;

use crate::guard_shape::may_be_a_guard;
use guards::{splice_guards, Guard};
pub use guards::{GuardRecord, GuardTally};
#[cfg(debug_assertions)]
pub use rounds::{recording, Witnessed};

const KEYWORD: &str = "function";
const ASYNC: &str = "async";
const COMMENT_OPENERS: [&str; 2] = ["/*", "//"];

#[derive(Clone, Copy)]
enum Edit {
    Insert(u32),
    Collapse(u32, u32),
}

struct Scan<'c, const GUARDS: bool> {
    code: &'c str,
    cascade: bool,
    edits: Vec<Edit>,
    guards: Vec<Guard>,
    record: GuardRecord,
    searched_until: u32,
    candidates: Vec<u32>,
    broken: Vec<Broken>,
    removed_breaks: usize,
}

struct Broken {
    gaps: Vec<(u32, u32)>,
    elements: Vec<(u32, u32)>,
}

#[derive(Clone, Copy)]
struct Marks {
    removed_breaks: usize,
    broken: usize,
}

struct Found {
    edits: Vec<Edit>,
    guards: Vec<Guard>,
    record: GuardRecord,
}

#[inline(always)]
fn keyword_end(text: &str) -> Option<usize> {
    if let Some(rest) = text.strip_prefix(ASYNC) {
        let trimmed = rest.trim_start();
        let gap = rest.len() - trimmed.len();
        if gap == 0 || !trimmed.starts_with(KEYWORD) {
            return None;
        }
        return Some(ASYNC.len() + gap + KEYWORD.len());
    }
    text.starts_with(KEYWORD).then_some(KEYWORD.len())
}

fn holds_an_elision(array: &ArrayExpression<'_>) -> bool {
    array
        .elements
        .iter()
        .any(|element| matches!(element, ArrayExpressionElement::Elision(_)))
}

impl<const GUARDS: bool> Scan<'_, GUARDS> {
    #[inline(always)]
    fn keyword_gap(&mut self, function: &Function<'_>) {
        if function.id.is_some() {
            return;
        }
        let start = function.span.start as usize;
        let Some(after_keyword) = self.code.get(start..).and_then(keyword_end) else {
            return;
        };
        self.edits
            .push(Edit::Insert(function.span.start + after_keyword as u32));
    }

    fn gaps(&self, array: &ArrayExpression<'_>) -> Vec<(u32, u32)> {
        let mut gaps = Vec::with_capacity(array.elements.len() + 1);
        let mut previous = array.span.start + 1;
        for element in &array.elements {
            let span = element.span();
            gaps.push((previous, span.start));
            previous = span.end;
        }
        gaps.push((previous, array.span.end - 1));
        gaps
    }

    fn holds_a_comment(&self, (from, to): (u32, u32)) -> bool {
        self.code
            .get(from as usize..to as usize)
            .is_some_and(|text| COMMENT_OPENERS.iter().any(|opener| text.contains(opener)))
    }

    fn is_a_bare_separator(&self, (from, to): (u32, u32), commas: usize) -> bool {
        self.code
            .get(from as usize..to as usize)
            .is_some_and(|text| {
                text.bytes()
                    .all(|byte| byte == b',' || byte.is_ascii_whitespace())
                    && text.bytes().filter(|byte| *byte == b',').count() == commas
            })
    }

    fn are_bare_separators(&self, gaps: &[(u32, u32)]) -> bool {
        let last = gaps.len().saturating_sub(1);
        gaps.iter().enumerate().all(|(index, gap)| {
            let between_elements = index != 0 && index != last;
            self.is_a_bare_separator(*gap, usize::from(between_elements))
        })
    }

    fn collapsible_gaps(&self, array: &ArrayExpression<'_>) -> Option<Vec<(u32, u32)>> {
        if array.elements.len() <= 2 || holds_an_elision(array) {
            return None;
        }
        let gaps = self.gaps(array);
        let collapsible =
            !gaps.iter().any(|gap| self.holds_a_comment(*gap)) && self.are_bare_separators(&gaps);
        collapsible.then_some(gaps)
    }

    fn text(&self, (from, to): (u32, u32)) -> Option<&str> {
        self.code.get(from as usize..to as usize)
    }

    fn holds_a_break(&self, range: (u32, u32)) -> bool {
        self.text(range).is_some_and(|text| text.contains('\n'))
    }

    fn breaks_in(&self, range: (u32, u32)) -> usize {
        self.text(range)
            .map_or(0, |text| text.bytes().filter(|byte| *byte == b'\n').count())
    }

    fn marks(&self) -> Marks {
        Marks {
            removed_breaks: self.removed_breaks,
            broken: self.broken.len(),
        }
    }

    fn consider(&mut self, array: &ArrayExpression<'_>, marks: Marks) {
        if array.elements.len() <= 2 || holds_an_elision(array) {
            return;
        }
        let spans = array.elements.iter().map(|element| {
            let span = element.span();
            (span.start, span.end)
        });
        if spans.clone().any(|element| self.holds_a_break(element)) {
            if self.cascade {
                self.cascade_into(array, spans, marks);
            }
            return;
        }
        if let Some(gaps) = self.collapsible_gaps(array) {
            self.collapse(gaps);
        }
    }

    fn cascade_into(
        &mut self,
        array: &ArrayExpression<'_>,
        spans: impl Iterator<Item = (u32, u32)>,
        marks: Marks,
    ) {
        let undecided = self.broken.len() > marks.broken || (GUARDS && !self.candidates.is_empty());
        if undecided {
            if let Some(gaps) = self.collapsible_gaps(array) {
                let elements = spans.collect();
                self.broken.push(Broken { gaps, elements });
            }
            return;
        }
        let breaks: usize = spans.map(|element| self.breaks_in(element)).sum();
        if breaks != self.removed_breaks - marks.removed_breaks {
            return;
        }
        if let Some(gaps) = self.collapsible_gaps(array) {
            self.collapse(gaps);
        }
    }

    fn collapse(&mut self, gaps: Vec<(u32, u32)>) {
        for gap in gaps {
            self.removed_breaks += self.breaks_in(gap);
            self.edits.push(Edit::Collapse(gap.0, gap.1));
        }
    }

    fn collapsed_breaks(&self) -> Vec<Edit> {
        let mut removed: Vec<Edit> = self
            .edits
            .iter()
            .filter(
                |edit| matches!(edit, Edit::Collapse(from, to) if self.holds_a_break((*from, *to))),
            )
            .copied()
            .collect();
        removed.sort_by_key(Edit::start);
        removed
    }

    fn resolve(&mut self) {
        if self.broken.is_empty() {
            return;
        }
        let mut removed = self.collapsed_breaks();
        for array in std::mem::take(&mut self.broken) {
            let unbroken = array
                .elements
                .iter()
                .all(|element| self.loses_its_breaks(*element, &removed));
            if unbroken {
                for &(from, to) in &array.gaps {
                    if self.holds_a_break((from, to)) {
                        let at = removed.partition_point(|edit| edit.start() < from);
                        removed.insert(at, Edit::Collapse(from, to));
                    }
                }
                self.collapse(array.gaps);
            }
        }
    }

    fn loses_its_breaks(&self, (from, to): (u32, u32), removed: &[Edit]) -> bool {
        let Some(text) = self.text((from, to)) else {
            return true;
        };
        text.match_indices('\n').all(|(offset, _)| {
            u32::try_from(offset)
                .ok()
                .and_then(|offset| from.checked_add(offset))
                .is_some_and(|at| self.spliced_away(at) || collapsed_away(removed, at))
        })
    }

    fn spliced_away(&self, at: u32) -> bool {
        let after = self.guards.partition_point(|guard| guard.0 <= at);
        after
            .checked_sub(1)
            .and_then(|index| self.guards.get(index))
            .is_some_and(|guard| at < guard.1)
    }
}

fn collapsed_away(removed: &[Edit], at: u32) -> bool {
    let after = removed.partition_point(|edit| edit.start() <= at);
    match after.checked_sub(1).and_then(|index| removed.get(index)) {
        Some(Edit::Collapse(_, to)) => at < *to,
        _ => false,
    }
}

impl<'a, const GUARDS: bool> VisitJs<'a> for Scan<'_, GUARDS> {
    fn visit_function(&mut self, it: &Function<'a>, flags: oxc_syntax::scope::ScopeFlags) {
        self.keyword_gap(it);
        if GUARDS && may_be_a_guard(it) {
            self.note_candidate(it);
        }
        walk_js::walk_function(self, it, flags);
    }

    fn visit_array_expression(&mut self, it: &ArrayExpression<'a>) {
        let marks = self.marks();
        walk_js::walk_array_expression(self, it);
        self.consider(it, marks);
    }
}

impl Edit {
    fn start(&self) -> u32 {
        match self {
            Edit::Insert(at) => *at,
            Edit::Collapse(from, _) => *from,
        }
    }

    #[inline(always)]
    fn replacement(&self, bytes: &[u8], cursor: usize) -> Option<(usize, usize, &'static str)> {
        match *self {
            Edit::Insert(at) => {
                let at = at as usize;
                (at >= cursor && bytes.get(at) == Some(&b'(')).then_some((at, at, " "))
            }
            Edit::Collapse(from, to) => {
                let (from, to) = (from as usize, to as usize);
                if from < cursor || to <= from || to > bytes.len() {
                    return None;
                }
                let separator = if bytes[from..to].contains(&b',') {
                    ", "
                } else {
                    ""
                };
                Some((from, to, separator))
            }
        }
    }
}

fn applied_len(bytes: &[u8], edits: &[Edit]) -> usize {
    let mut cursor = 0;
    let mut len = bytes.len();
    for edit in edits {
        if let Some((from, to, replacement)) = edit.replacement(bytes, cursor) {
            len = len
                .saturating_sub(to - from)
                .saturating_add(replacement.len());
            cursor = to;
        }
    }
    len
}

fn apply(code: String, mut edits: Vec<Edit>) -> (String, bool) {
    edits.sort_by_key(Edit::start);
    let bytes = code.as_bytes();
    let mut out = String::with_capacity(applied_len(bytes, &edits));
    let mut cursor = 0;
    for edit in edits {
        let Some((from, to, replacement)) = edit.replacement(bytes, cursor) else {
            continue;
        };
        let Some(kept) = code.get(cursor..from) else {
            return (code, false);
        };
        out.push_str(kept);
        out.push_str(replacement);
        cursor = to;
    }
    let Some(rest) = code.get(cursor..) else {
        return (code, false);
    };
    out.push_str(rest);
    let changed = out.as_bytes() != bytes;
    (out, changed)
}

type Lend<'l> = Option<&'l mut dyn FnMut(&Program<'_>)>;

fn scan_text<const GUARDS: bool>(
    reused: Option<&mut Allocator>,
    code: &str,
    cascade: bool,
    lend: Lend<'_>,
) -> Option<Found> {
    let mut fresh = None;
    let alloc = match reused {
        Some(alloc) => {
            alloc.reset();
            alloc
        }
        None => fresh.insert(Allocator::default()),
    };
    let parsed = parse_program(alloc, code);
    if has_error(&parsed.diagnostics) {
        return None;
    }
    if let Some(lend) = lend {
        lend(&parsed.program);
    }
    let mut scan = Scan::<GUARDS> {
        code,
        cascade,
        edits: Vec::new(),
        guards: Vec::new(),
        record: GuardRecord::default(),
        searched_until: 0,
        candidates: Vec::new(),
        broken: Vec::new(),
        removed_breaks: 0,
    };
    scan.visit_program(&parsed.program);
    if GUARDS && !scan.candidates.is_empty() {
        scan.settle_slots(&parsed.program);
    }
    scan.resolve();
    Some(Found {
        edits: scan.edits,
        guards: scan.guards,
        record: scan.record,
    })
}

fn reflow_once<const GUARDS: bool>(
    code: String,
    reused: Option<&mut Allocator>,
    cascade: bool,
    lend: Lend<'_>,
) -> (String, bool, GuardRecord) {
    let found = match scan_text::<GUARDS>(reused, &code, cascade, lend) {
        Some(found) if !found.edits.is_empty() || !found.guards.is_empty() => found,
        Some(found) => return (code, false, found.record),
        None => return (code, false, GuardRecord::default()),
    };
    let (code, edits, spliced) = if found.guards.is_empty() {
        (code, found.edits, false)
    } else {
        splice_guards(code, found.edits, found.guards)
    };
    let (code, changed) = apply(code, edits);
    (code, changed || spliced, found.record)
}

fn reflow_recorded_in(
    code: String,
    reused: Option<&mut Allocator>,
    lend: Lend<'_>,
) -> (String, GuardRecord) {
    #[cfg(debug_assertions)]
    let witnessed = rounds::is_recording().then(|| code.clone());
    let (code, _, record) = reflow_once::<true>(code, reused, true, lend);
    #[cfg(debug_assertions)]
    if let Some(before) = witnessed {
        rounds::witness(before, &code, &record);
    }
    (code, record)
}

fn settle(code: String, cascade: bool) -> String {
    if cascade {
        return reflow_once::<false>(code, None, true, None).0;
    }
    let mut code = code;
    loop {
        let (next, changed, _) = reflow_once::<false>(code, None, false, None);
        code = next;
        if !changed {
            return code;
        }
    }
}

#[derive(Default)]
pub struct Reflower {
    alloc: Allocator,
}

impl Reflower {
    pub fn reflow_recorded(&mut self, code: String) -> (String, GuardRecord) {
        reflow_recorded_in(code, Some(&mut self.alloc), None)
    }
}

pub fn reflow_recorded(code: String) -> (String, GuardRecord) {
    reflow_recorded_in(code, None, None)
}

pub fn reflow_recorded_lending(
    code: String,
    lend: &mut dyn FnMut(&Program<'_>),
) -> (String, GuardRecord) {
    reflow_recorded_in(code, None, Some(lend))
}

pub fn reflow_tallied(code: String) -> (String, GuardTally) {
    let (code, record) = reflow_recorded(code);
    (code, record.tally())
}

pub fn reflow(code: String) -> String {
    reflow_tallied(code).0
}
