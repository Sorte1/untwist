use std::cell::RefCell;
use std::hash::{Hash, Hasher};

use oxc_syntax::symbol::SymbolId;
use rustc_hash::FxHashMap;
use untwist_core::ast_util::readonly::{ObjectUses, SymbolUses};

const FNV_OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;

thread_local! {
    static RECORDS: RefCell<Option<Vec<String>>> = const { RefCell::new(None) };
}

struct Digest(u64);

impl Default for Digest {
    fn default() -> Self {
        Self(FNV_OFFSET)
    }
}

impl Hasher for Digest {
    fn finish(&self) -> u64 {
        self.0
    }

    fn write(&mut self, bytes: &[u8]) {
        for byte in bytes {
            self.0 ^= u64::from(*byte);
            self.0 = self.0.wrapping_mul(FNV_PRIME);
        }
    }
}

pub fn digest_of(value: &impl Hash) -> u64 {
    let mut digest = Digest::default();
    value.hash(&mut digest);
    digest.finish()
}

#[cfg(feature = "test-hooks")]
pub fn recording<R>(run: impl FnOnce() -> R) -> (R, Vec<String>) {
    RECORDS.with(|records| records.replace(Some(Vec::new())));
    let result = run();
    let taken = RECORDS.with(|records| records.take()).unwrap_or_default();
    (result, taken)
}

pub(crate) fn active() -> bool {
    RECORDS.with(|records| records.borrow().is_some())
}

pub(crate) fn record(site: &str, digest: u64) {
    RECORDS.with(|records| {
        if let Some(records) = records.borrow_mut().as_mut() {
            records.push(format!("{site} {digest:016x}"));
        }
    });
}

pub(crate) fn sorted<K: Ord + Copy, V>(entries: impl IntoIterator<Item = (K, V)>) -> Vec<(K, V)> {
    let mut entries: Vec<(K, V)> = entries.into_iter().collect();
    entries.sort_by_key(|(key, _)| *key);
    entries
}

pub(crate) fn uses_digest(uses: &FxHashMap<SymbolId, ObjectUses>) -> u64 {
    digest_of(&sorted(
        uses.iter().map(|(symbol, uses)| (symbol.index(), uses)),
    ))
}

pub(crate) fn symbol_uses_digest(uses: &SymbolUses) -> u64 {
    let objects = sorted(
        uses.uses
            .iter()
            .map(|(symbol, uses)| (symbol.index(), uses)),
    );
    let references = sorted(
        uses.references
            .iter()
            .map(|(symbol, spans)| (symbol.index(), spans)),
    );
    digest_of(&(objects, references))
}

pub(crate) fn record_order_and_uses(
    site: &str,
    order: &crate::init_order::InitOrder,
    uses: &FxHashMap<SymbolId, ObjectUses>,
) {
    if active() {
        record(site, digest_of(&(order.digest(), uses_digest(uses))));
    }
}

#[cfg(feature = "test-hooks")]
const SAMPLED_SYMBOLS: usize = 256;

#[cfg(feature = "test-hooks")]
#[derive(Default)]
struct Declarations {
    statements: Vec<(oxc_span::Span, SymbolId)>,
}

#[cfg(feature = "test-hooks")]
impl<'a> oxc_ast_visit::VisitJs<'a> for Declarations {
    fn visit_statement(&mut self, it: &oxc_ast::ast::Statement<'a>) {
        if let Some(symbol) = crate::stamp::declaration_symbol(it) {
            self.statements.push((oxc_span::GetSpan::span(it), symbol));
        }
        oxc_ast_visit::walk_js::walk_statement(self, it);
    }
}

#[cfg(feature = "test-hooks")]
pub fn fused_and_separate(source: &str) -> Vec<(&'static str, u64, u64)> {
    use oxc_ast_visit::VisitJs;
    let alloc = oxc_allocator::Allocator::default();
    let parsed = untwist_core::parse::parse_program(&alloc, source);
    let program = &parsed.program;
    let scoping = untwist_core::semantic::build_scoping(program).scoping;
    let ids: Vec<SymbolId> = scoping.symbol_ids().collect();
    let stride = ids.len().div_ceil(SAMPLED_SYMBOLS).max(1);
    let all: Vec<SymbolId> = ids.iter().copied().step_by(stride).collect();
    let set: rustc_hash::FxHashSet<SymbolId> = all.iter().copied().collect();
    let mut declarations = Declarations::default();
    declarations.visit_program(program);
    let (uses_fused, uses_separate) =
        crate::init_order::fused_and_separate_uses(program, &scoping, &set);
    let (facts_fused, facts_separate) = crate::merge_object_assignments::fused_and_separate_facts(
        program,
        &scoping,
        &declarations.statements,
    );
    let (vars_fused, vars_separate) =
        crate::calls::fused_and_separate_variables(program, &scoping, &all);
    let (inits_fused, inits_separate) =
        crate::calls::fused_and_separate_initialisers(program, &scoping, &all);
    let (keys_fused, keys_separate) =
        crate::merge_object_assignments::fused_and_separate_keys(program, &scoping);
    let (alias_fused, alias_separate) =
        crate::wrappers::fused_and_separate_alias_facts(program, &scoping);
    vec![
        ("keys and candidates", keys_fused, keys_separate),
        ("alias facts", alias_fused, alias_separate),
        ("order and uses", uses_fused, uses_separate),
        ("order, uses and repeated", facts_fused, facts_separate),
        ("variable facts", vars_fused, vars_separate),
        ("constant initialisers", inits_fused, inits_separate),
    ]
}
