mod kept;
mod scan;
mod tables;

use std::cell::Cell;

use oxc_ast::ast::{ModuleDeclaration, Program, Statement};
use oxc_semantic::Scoping;
use oxc_span::Span;
use oxc_syntax::symbol::SymbolId;
use rustc_hash::FxHashSet;
use untwist_core::ast_util::readonly::PropRead;
use untwist_core::pass::Cx;
use untwist_core::semantic::with_observes;

use crate::fused::Member;

use crate::mutants::early_read_guards_dropped;
use crate::notes;
use tables::{at, id, some, Site, Sites, SpanSlots};

pub use kept::{clear_kept_order, pass_done};
pub(crate) use kept::{
    lend_order, lend_order_and_readonly_uses, lend_order_with, note_overflow, sharing_mark,
    LentOrder,
};

const REACHABILITY_BUDGET: usize = 4096;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct DeclarationSlot {
    list: u32,
    index: u32,
}

#[derive(Hash)]
struct Frame {
    list: u32,
    index: u32,
    hoisted: Option<SymbolId>,
    functions: u32,
    parent: u32,
}

#[derive(Clone, Copy, Hash)]
enum ValueOwner {
    Anonymous,
    Binding { symbol: SymbolId, hoisted: bool },
    Opaque,
}

#[derive(Hash)]
struct FunctionValue {
    owner: ValueOwner,
    outer: u32,
    frame: u32,
}

#[derive(Hash)]
struct StatementList {
    start: u32,
    leading_inert: u32,
    functions: u32,
}

#[derive(Default)]
pub struct InitOrder {
    frames: Vec<Frame>,
    lists: Vec<StatementList>,
    slots: SpanSlots,
    sites: Sites,
    values: Vec<FunctionValue>,
    hazard: bool,
    reentrant: bool,
    exhausted: Cell<usize>,
    requested: ScanLengths,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ScanLengths {
    pub frames: usize,
    pub lists: usize,
    pub slots: usize,
    pub values: usize,
}

const SLACK_DIVISOR: usize = 64;

thread_local! {
    static LAST_SCAN: Cell<ScanLengths> = const {
        Cell::new(ScanLengths { frames: 0, lists: 0, slots: 0, values: 0 })
    };
}

pub fn capacity_hint() -> ScanLengths {
    LAST_SCAN.with(Cell::get)
}

pub fn reset_capacity_hint() {
    LAST_SCAN.with(|last| last.set(ScanLengths::default()));
}

fn record_lengths(lengths: ScanLengths) {
    LAST_SCAN.with(|last| last.set(lengths));
}

pub fn with_slack(len: usize) -> usize {
    len.saturating_add(len / SLACK_DIVISOR)
}

enum Step {
    Runs,
    Value(u32),
    Skip,
}

pub struct ReadRefusal<'r> {
    read: Option<&'r PropRead>,
    exhausted: bool,
}

impl DeclarationSlot {
    pub fn index(self) -> usize {
        at(self.index)
    }

    pub fn same_list(self, other: Self) -> bool {
        self.list == other.list
    }
}

impl InitOrder {
    fn with_reentry(mut self, program: &Program<'_>) -> Self {
        let declarations = || {
            program
                .body
                .iter()
                .filter_map(Statement::as_module_declaration)
        };
        let imports = declarations().any(|declaration| {
            matches!(
                declaration,
                ModuleDeclaration::ImportDeclaration(_)
                    | ModuleDeclaration::ExportAllDeclaration(_)
                    | ModuleDeclaration::ExportFromDeclaration(_)
            )
        });
        self.reentrant = imports
            && declarations().any(|declaration| {
                matches!(
                    declaration,
                    ModuleDeclaration::ExportDefaultDeclaration(_)
                        | ModuleDeclaration::ExportDeclaration(_)
                        | ModuleDeclaration::ExportNamedDeclaration(_)
                )
            });
        self
    }

    pub fn budget_exhausted(&self) -> usize {
        self.exhausted.get()
    }

    pub(crate) fn reusable(&mut self) -> bool {
        self.sites.forget_span_index();
        true
    }

    pub(crate) fn lean_for_reuse(&mut self) -> bool {
        let lengths = self.lengths();
        let fits = |requested: usize, len: usize| len <= requested && requested <= with_slack(len);
        let lean = fits(self.requested.frames, lengths.frames)
            && fits(self.requested.lists, lengths.lists)
            && fits(self.requested.slots, lengths.slots)
            && fits(self.requested.values, lengths.values);
        self.sites.forget_span_index();
        lean
    }

    fn lengths(&self) -> ScanLengths {
        ScanLengths {
            frames: self.frames.len(),
            lists: self.lists.len(),
            slots: self.slots.len(),
            values: self.values.len(),
        }
    }

    pub fn declaration_slot(&self, statement: Span) -> Option<DeclarationSlot> {
        self.slot_of_ordinal(self.slots.get(statement)?)
    }

    fn slot_of_ordinal(&self, ordinal: u32) -> Option<DeclarationSlot> {
        let list = self
            .lists
            .partition_point(|entry| entry.start <= ordinal)
            .checked_sub(1)?;
        let index = ordinal.checked_sub(self.lists.get(list)?.start)?;
        Some(DeclarationSlot {
            list: id(list)?,
            index,
        })
    }

    pub fn reference_is_initialised_before_use(
        &self,
        declaration: DeclarationSlot,
        reference: Span,
    ) -> bool {
        let Some(site) = self.sites.at_span(reference) else {
            return false;
        };
        self.site_is_initialised_before_use(declaration, site)
    }

    pub fn symbol_is_initialised_before_use(
        &self,
        declaration: DeclarationSlot,
        symbol: SymbolId,
        ignored: &FxHashSet<usize>,
    ) -> bool {
        self.sites.of_symbol(symbol).all(|site| {
            self.site_frame(site, declaration.list)
                .is_some_and(|entry| {
                    entry.index == declaration.index || ignored.contains(&at(entry.index))
                })
                || self.site_is_initialised_before_use(declaration, site)
        })
    }

    fn site_is_initialised_before_use(&self, declaration: DeclarationSlot, site: u32) -> bool {
        let Some(record) = self.sites.get(site) else {
            return false;
        };
        if self.reentrant && with_observes() && some(record.value).is_some() {
            return false;
        }
        let Some(entry) = self.frame_in_list(record.frame, declaration.list) else {
            return false;
        };
        let statement_is_safe = match entry.hoisted {
            None => entry.index > declaration.index,
            Some(hoisted) => {
                self.prefix_cannot_call(declaration)
                    || (entry.index > declaration.index
                        && !self.list_is_global(declaration.list)
                        && self.is_unreachable_before(declaration, hoisted))
            }
        };
        statement_is_safe || self.value_cannot_run_before(declaration, record)
    }

    pub fn slot_of_reference(
        &self,
        declaration: DeclarationSlot,
        reference: Span,
    ) -> Option<DeclarationSlot> {
        let record = self.sites.get(self.sites.at_span(reference)?)?;
        let entry = self.frame_in_list(record.frame, declaration.list)?;
        self.value_depth_is(record.value, entry.functions)
            .then_some(DeclarationSlot {
                list: entry.list,
                index: entry.index,
            })
    }

    fn value_depth_is(&self, value: u32, depth: u32) -> bool {
        let mut steps = 0u32;
        let mut current = some(value);
        while let Some(value) = current {
            if steps == depth {
                return false;
            }
            steps += 1;
            current = self
                .values
                .get(at(value))
                .and_then(|record| some(record.outer));
        }
        steps == depth
    }

    fn frame_in_list(&self, start: u32, list: u32) -> Option<&Frame> {
        let mut current = start;
        loop {
            let entry = self.frames.get(at(current))?;
            if entry.list == list {
                return Some(entry);
            }
            current = some(entry.parent)?;
        }
    }

    fn site_frame(&self, site: u32, list: u32) -> Option<&Frame> {
        let record = self.sites.get(site)?;
        self.frame_in_list(record.frame, list)
    }

    fn prefix_is_inert(&self, list: u32, count: usize) -> bool {
        let Some(entry) = self.lists.get(at(list)) else {
            return false;
        };
        let end = self
            .lists
            .get(at(list).saturating_add(1))
            .map_or(self.slots.len(), |next| at(next.start));
        let len = end.saturating_sub(at(entry.start));
        count.min(len) <= at(entry.leading_inert)
    }

    fn prefix_cannot_call(&self, declaration: DeclarationSlot) -> bool {
        self.prefix_is_inert(declaration.list, declaration.index())
    }

    fn roots_cannot_call(&self, declaration: DeclarationSlot) -> bool {
        self.prefix_is_inert(declaration.list, declaration.index().saturating_add(1))
    }

    fn list_is_global(&self, list: u32) -> bool {
        self.lists
            .get(at(list))
            .is_none_or(|entry| entry.functions == 0)
    }

    fn is_unreachable_before(&self, declaration: DeclarationSlot, hoisted: SymbolId) -> bool {
        let mut seen: FxHashSet<SymbolId> = FxHashSet::default();
        let mut pending = vec![hoisted];
        while let Some(current) = pending.pop() {
            if !seen.insert(current) {
                continue;
            }
            for site in self.sites.of_symbol(current) {
                let Some(entry) = self.site_frame(site, declaration.list) else {
                    return false;
                };
                if entry.index <= declaration.index {
                    return false;
                }
                if let Some(symbol) = entry.hoisted {
                    pending.push(symbol);
                }
            }
        }
        true
    }

    fn value_cannot_run_before(&self, declaration: DeclarationSlot, site: Site) -> bool {
        if self.hazard {
            return false;
        }
        let Some(value) = self.outermost_value(site.value, declaration.list) else {
            return false;
        };
        if self.roots_cannot_call(declaration) {
            return true;
        }
        if self.list_is_global(declaration.list) {
            return false;
        }
        !self.value_runs_before(value, declaration)
    }

    fn outermost_value(&self, start: u32, list: u32) -> Option<u32> {
        let mut current = some(start)?;
        self.value_index(current, list)?;
        while let Some(outer) = self
            .values
            .get(at(current))
            .and_then(|value| some(value.outer))
        {
            if self.value_index(outer, list).is_none() {
                break;
            }
            current = outer;
        }
        Some(current)
    }

    fn value_index(&self, value: u32, list: u32) -> Option<u32> {
        let frame = some(self.values.get(at(value))?.frame)?;
        self.frame_in_list(frame, list).map(|entry| entry.index)
    }

    fn value_runs_before(&self, root: u32, declaration: DeclarationSlot) -> bool {
        let mut seen: FxHashSet<u32> = FxHashSet::default();
        let mut pending = vec![root];
        while let Some(value) = pending.pop() {
            if !seen.insert(value) {
                continue;
            }
            if seen.len() > REACHABILITY_BUDGET {
                self.exhausted.set(self.exhausted.get().saturating_add(1));
                return true;
            }
            if self.value_expands(value, declaration, &mut pending) {
                return true;
            }
        }
        false
    }

    fn value_expands(
        &self,
        value: u32,
        declaration: DeclarationSlot,
        pending: &mut Vec<u32>,
    ) -> bool {
        let Some(record) = self.values.get(at(value)) else {
            return true;
        };
        if matches!(record.owner, ValueOwner::Opaque) {
            return true;
        }
        if !self.value_is_created(value, record, declaration) {
            return false;
        }
        let ValueOwner::Binding { symbol, .. } = record.owner else {
            return true;
        };
        for site in self.sites.of_symbol(symbol) {
            match self.site_step(site, declaration) {
                Step::Runs => return true,
                Step::Value(next) => pending.push(next),
                Step::Skip => {}
            }
        }
        false
    }

    fn value_is_created(
        &self,
        value: u32,
        record: &FunctionValue,
        declaration: DeclarationSlot,
    ) -> bool {
        let Some(index) = self.value_index(value, declaration.list) else {
            return true;
        };
        matches!(record.owner, ValueOwner::Binding { hoisted: true, .. })
            || index <= declaration.index
    }

    fn site_step(&self, site: u32, declaration: DeclarationSlot) -> Step {
        let Some(record) = self.sites.get(site) else {
            return Step::Runs;
        };
        let Some(entry) = self.frame_in_list(record.frame, declaration.list) else {
            return Step::Runs;
        };
        match self.outermost_value(record.value, declaration.list) {
            Some(value) => Step::Value(value),
            None if entry.index <= declaration.index => Step::Runs,
            None => Step::Skip,
        }
    }
}

#[cfg(debug_assertions)]
impl InitOrder {
    pub(crate) fn digest(&self) -> u64 {
        crate::fused::check::digest_of(&(
            &self.frames,
            &self.lists,
            self.slots.spans(),
            self.sites.content(),
            &self.values,
            self.hazard,
        ))
    }
}

pub fn reads_are_initialised<'r>(
    reads: &'r [PropRead],
    declaration: Span,
    order: &InitOrder,
) -> Result<DeclarationSlot, ReadRefusal<'r>> {
    let Some(slot) = order.declaration_slot(declaration) else {
        return Err(ReadRefusal {
            read: None,
            exhausted: false,
        });
    };
    let mark = order.exhausted.get();
    match reads
        .iter()
        .find(|read| !order.reference_is_initialised_before_use(slot, read.object_span))
    {
        Some(read) => Err(ReadRefusal {
            read: Some(read),
            exhausted: order.exhausted.get() > mark,
        }),
        None => Ok(slot),
    }
}

pub fn refuse_read_before_declaration(
    refused: ReadRefusal<'_>,
    binding: SymbolId,
    declaration: Span,
    message: &'static str,
    cx: &mut Cx<'_, '_>,
) {
    let span = refused.read.map_or(declaration, |read| read.member_span);
    cx.note(message, Some(span));
    if !refused.exhausted {
        return;
    }
    let name = cx.scoping().symbol_name(binding).to_string();
    cx.note(notes::init_order_budget(&name), Some(span));
}

fn constant_written(scoping: &Scoping, symbol: SymbolId) -> bool {
    scoping.symbol_flags(symbol).is_const_variable()
        && scoping
            .get_resolved_references(symbol)
            .any(|reference| reference.is_write())
}

pub(crate) fn early_or_written(
    scoping: &Scoping,
    early: &FxHashSet<SymbolId>,
    symbol: SymbolId,
) -> Option<&'static str> {
    if early.contains(&symbol) {
        return Some(notes::MACHINERY_READ_BEFORE_DECLARATION);
    }
    (!early_read_guards_dropped() && constant_written(scoping, symbol))
        .then_some(notes::MACHINERY_CONSTANT_WRITTEN)
}

pub(crate) fn read_before_declaration(
    program: &Program<'_>,
    scoping: &Scoping,
    bound: &[(Span, SymbolId)],
) -> FxHashSet<SymbolId> {
    if bound.is_empty() || early_read_guards_dropped() {
        return FxHashSet::default();
    }
    let order = try_scan_init_order(program, scoping);
    let ignored = FxHashSet::default();
    bound
        .iter()
        .filter(|(statement, symbol)| {
            !order.as_ref().is_some_and(|order| {
                order.declaration_slot(*statement).is_some_and(|slot| {
                    order.symbol_is_initialised_before_use(slot, *symbol, &ignored)
                })
            })
        })
        .map(|(_, symbol)| *symbol)
        .collect()
}

pub fn try_scan_init_order(program: &Program<'_>, scoping: &Scoping) -> Option<InitOrder> {
    scan::Scan::new(scoping).run(program)
}

pub(crate) fn order_or_note(order: Option<InitOrder>, cx: &mut Cx<'_, '_>) -> InitOrder {
    order.unwrap_or_else(|| {
        cx.note(notes::INIT_ORDER_OVERFLOW, None);
        InitOrder::default()
    })
}

pub fn scan_init_order(program: &Program<'_>, cx: &mut Cx<'_, '_>) -> InitOrder {
    let order = try_scan_init_order(program, cx.scoping());
    order_or_note(order, cx)
}

pub(crate) fn try_scan_init_order_with<'a, M: Member<'a>>(
    program: &Program<'a>,
    scoping: &Scoping,
    member: M,
) -> (Option<InitOrder>, M) {
    scan::Scan::new(scoping).run_with(program, member)
}

#[cfg(all(debug_assertions, feature = "test-hooks"))]
pub(crate) fn fused_and_separate_uses(
    program: &Program<'_>,
    scoping: &Scoping,
    symbols: &FxHashSet<SymbolId>,
) -> (u64, u64) {
    use crate::fused::check::{digest_of, symbol_uses_digest};
    use untwist_core::ast_util::readonly::{scan_symbol_uses, SymbolUses, UsesScan};
    let digest = |order: Option<InitOrder>, uses: &SymbolUses| {
        digest_of(&(order.map(|order| order.digest()), symbol_uses_digest(uses)))
    };
    let (order, uses) = try_scan_init_order_with(program, scoping, UsesScan::new(scoping, symbols));
    let fused = digest(order, &uses.finish());
    let order = try_scan_init_order(program, scoping);
    let separate = digest(order, &scan_symbol_uses(program, scoping, symbols));
    (fused, separate)
}

#[cfg(test)]
mod tests {
    use super::*;
    use oxc_allocator::Allocator;
    use untwist_core::parse::parse_program;
    use untwist_core::semantic::build_scoping;

    const HINTED: &str = "var a = 1;\nfunction f() { var b = a; if (b) { g(); } }\nf();\nvar c = function () { return a; };";

    fn capacities(order: &InitOrder) -> ScanLengths {
        ScanLengths {
            frames: order.frames.capacity(),
            lists: order.lists.capacity(),
            slots: order.slots.capacity(),
            values: order.values.capacity(),
        }
    }

    const NESTED_KEYS: &[&str] = &[
        "class A { @(class { static { ({ [function () { return z; }()]: 1 }); } }) m() {} }",
        "class B extends (class { static { var q = function () { return r; }; } }) { [k] = () => { s; }; static { t; } }",
        "function f() { class C { [g()]() { return h; } static #p = () => i; } return function () { j; }; }",
        "var o = { [a]: function () { class D extends E { [b] = c; } } }; label: for (;;) { d; break label; }",
    ];

    fn assert_site_depths_match(source: &str) {
        let alloc = Allocator::default();
        let parsed = parse_program(&alloc, source);
        let sem = build_scoping(&parsed.program);
        let Some(order) = try_scan_init_order(&parsed.program, &sem.scoping) else {
            return;
        };
        for site in 0..order.sites.len() {
            let record = order.sites.get(site as u32).unwrap();
            assert!(
                order.value_depth_is(record.value, record.functions),
                "site {site} of {source:.80}"
            );
            assert!(
                !order.value_depth_is(record.value, record.functions + 1),
                "site {site} at depth + 1 of {source:.80}"
            );
            if let Some(shallower) = record.functions.checked_sub(1) {
                assert!(
                    !order.value_depth_is(record.value, shallower),
                    "site {site} at depth - 1 of {source:.80}"
                );
            }
        }
    }

    fn fixture_sources(dir: &std::path::Path, out: &mut Vec<std::path::PathBuf>) {
        for entry in std::fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            let skipped = ["generated", "corpus-npm", "real-world"]
                .iter()
                .any(|name| path.ends_with(name));
            if path.is_dir() && !skipped {
                fixture_sources(&path, out);
            } else if path.extension().is_some_and(|ext| ext == "js")
                && std::fs::metadata(&path).unwrap().len() < 400_000
            {
                out.push(path);
            }
        }
    }

    #[test]
    fn a_sites_value_depth_is_the_function_count_the_walk_recorded() {
        for source in NESTED_KEYS {
            assert_site_depths_match(source);
        }
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures");
        let mut paths = Vec::new();
        fixture_sources(&root, &mut paths);
        assert!(paths.len() > 200, "{}", paths.len());
        for path in paths {
            assert_site_depths_match(&std::fs::read_to_string(&path).unwrap());
        }
    }

    #[test]
    fn a_scan_leaves_its_lengths_as_the_next_scans_capacity() {
        reset_capacity_hint();
        assert_eq!(capacity_hint(), ScanLengths::default());
        let alloc = Allocator::default();
        let parsed = parse_program(&alloc, HINTED);
        let sem = build_scoping(&parsed.program);
        let first = try_scan_init_order(&parsed.program, &sem.scoping).unwrap();
        assert_eq!(capacity_hint(), first.lengths());
        assert!(capacity_hint().slots >= 5 && capacity_hint().lists >= 3);
        let second = try_scan_init_order(&parsed.program, &sem.scoping).unwrap();
        let wanted = second.lengths();
        let got = capacities(&second);
        assert!(got.frames >= wanted.frames && got.lists >= wanted.lists);
        assert!(got.slots >= wanted.slots && got.values >= wanted.values);
        reset_capacity_hint();
        assert_eq!(capacity_hint(), ScanLengths::default());
    }
}
