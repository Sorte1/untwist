use oxc_ast::ast::{
    AccessorProperty, ArrowFunctionExpression, AssignmentExpression, AssignmentTarget,
    BindingPattern, Expression, FormalParameter, Function, IdentifierReference, Program,
    PropertyDefinition, Statement, StaticBlock,
};
use oxc_ast_visit::{walk_js, VisitJs};
use oxc_semantic::{Scoping, SemanticBuilder, Stats};
use oxc_span::{GetSpan, Span};
use oxc_syntax::operator::AssignmentOperator;
use oxc_syntax::scope::{ScopeFlags, ScopeId};
use oxc_syntax::symbol::SymbolId;
use rustc_hash::{FxHashMap, FxHashSet};

mod annex_b;
mod blocked_hoists;
pub mod builtins;
#[cfg(feature = "test-hooks")]
pub mod compare;
mod observers;
pub mod pins;
mod refresh;
mod renamed;
mod shared;
mod strictness;
pub mod unbound;

pub use annex_b::{
    block_function_symbols, declared_in_block, declaring_block, is_constant_binding_in,
    is_hoisted_block_function, AnnexBFunction, AnnexBWrites, ScopeOwners,
};
pub use blocked_hoists::{hoist_blockers, repair_blocked_hoists};
pub use observers::{
    assume_observers, assumed_hazard, closed_eval_names, closed_evals_kept, eval_payload_names,
    is_closed_eval_at, observers, observing_open, open_eval_sites, with_observes, Observers,
    ObserversGuard,
};
#[cfg(feature = "test-hooks")]
pub use refresh::{
    check_patches_for_tests, force_rebuilds_for_tests, keep_one_dead_binding_for_tests,
    last_stale_for_tests, misplace_one_scope_for_tests, refresh_counts_for_tests,
    skip_one_deletion_for_tests, trust_id_order_for_tests, RefreshCounts, STALE_PATCH,
};
pub use refresh::{forget_failures, refresh_scoping, Round, Shape};
pub(crate) use renamed::forget_program as forget_renames;
pub(crate) use renamed::note_renames;
pub use renamed::renamed_since_refresh;
pub use shared::{
    assume_shared_globals, is_foreign_global, is_shared_global, SharedGlobals, SharedGlobalsGuard,
};
pub use strictness::Strictness;

pub fn has_module_syntax(program: &Program<'_>) -> bool {
    program.body.iter().any(Statement::is_module_declaration)
}

pub fn program_is_strict(program: &Program<'_>) -> bool {
    program.has_use_strict_directive() || has_module_syntax(program)
}

pub fn scope_hazard(scoping: &Scoping) -> Option<&'static str> {
    let mut with = false;
    for scope in scoping.scope_descendants_from_root() {
        let flags = scoping.scope_flags(scope);
        if flags.contains(ScopeFlags::DirectEval) {
            return Some("direct eval");
        }
        with |= flags.contains(ScopeFlags::With);
    }
    with.then_some("with statement")
}

pub struct Semantics {
    pub scoping: Scoping,
    pub stats: Stats,
    pub patched: bool,
    pub canonical: bool,
}

pub fn build_scoping(program: &Program<'_>) -> Semantics {
    refresh::count_build();
    unbound::forget();
    settled(program, finish(SemanticBuilder::new().build(program)))
}

pub fn rebuild_scoping(program: &Program<'_>, previous: Stats) -> Semantics {
    unbound::forget();
    settled(
        program,
        finish(SemanticBuilder::new().with_stats(previous).build(program)),
    )
}

fn settled(program: &Program<'_>, mut sem: Semantics) -> Semantics {
    blocked_hoists::repair_blocked_hoists(program, &mut sem.scoping);
    observers::settle_eval_scopes(program, &mut sem.scoping);
    sem
}

fn finish(ret: oxc_semantic::SemanticBuilderReturn<'_>) -> Semantics {
    let stats = ret.semantic.stats();
    Semantics {
        scoping: ret.semantic.into_scoping(),
        stats,
        patched: false,
        canonical: true,
    }
}

pub fn read_reference_count(scoping: &Scoping, symbol: SymbolId) -> usize {
    scoping
        .get_resolved_references(symbol)
        .filter(|r| r.is_read())
        .count()
}

pub fn write_reference_count(scoping: &Scoping, symbol: SymbolId) -> usize {
    scoping
        .get_resolved_references(symbol)
        .filter(|r| r.is_write())
        .count()
}

pub fn is_constant_binding(scoping: &Scoping, symbol: SymbolId) -> bool {
    is_constant_binding_by(scoping, symbol, |symbol| {
        is_hoisted_block_function(scoping, symbol)
    })
}

fn is_constant_binding_by(
    scoping: &Scoping,
    symbol: SymbolId,
    hoisted_from_a_block: impl Fn(SymbolId) -> bool,
) -> bool {
    !scoping.symbol_is_mutated(symbol)
        && scoping.symbol_redeclarations(symbol).is_empty()
        && !hoisted_from_a_block(symbol)
        && !(scoping.symbol_scope_id(symbol) == scoping.root_scope_id()
            && is_shared_global(scoping.symbol_name(symbol)))
}

#[derive(Default)]
struct ParameterScan {
    found: FxHashSet<SymbolId>,
}

impl<'a> VisitJs<'a> for ParameterScan {
    fn visit_formal_parameter(&mut self, it: &FormalParameter<'a>) {
        if let BindingPattern::BindingIdentifier(id) = &it.pattern {
            if let Some(symbol) = id.symbol_id.get() {
                self.found.insert(symbol);
            }
        }
        walk_js::walk_formal_parameter(self, it);
    }
}

pub fn parameter_symbols(program: &Program<'_>) -> FxHashSet<SymbolId> {
    let mut scan = ParameterScan::default();
    scan.visit_program(program);
    scan.found
}

struct BatchWriteScan<'s> {
    scoping: &'s Scoping,
    symbols: &'s FxHashSet<SymbolId>,
    spans: FxHashMap<SymbolId, Vec<Span>>,
}

impl<'a> VisitJs<'a> for BatchWriteScan<'_> {
    fn visit_expression(&mut self, it: &Expression<'a>) {
        if let Expression::AssignmentExpression(assignment) = it {
            if assignment.operator == AssignmentOperator::Assign {
                if let AssignmentTarget::AssignmentTargetIdentifier(target) = &assignment.left {
                    let resolved = target
                        .reference_id
                        .get()
                        .and_then(|id| self.scoping.get_reference(id).symbol_id());
                    if let Some(symbol) = resolved.filter(|symbol| self.symbols.contains(symbol)) {
                        self.spans.entry(symbol).or_default().push(it.span());
                    }
                }
            }
        }
        walk_js::walk_expression(self, it);
    }
}

pub fn write_spans(
    program: &Program<'_>,
    scoping: &Scoping,
    symbols: &FxHashSet<SymbolId>,
) -> FxHashMap<SymbolId, Vec<Span>> {
    let mut scan = BatchWriteScan {
        scoping,
        symbols,
        spans: FxHashMap::default(),
    };
    scan.visit_program(program);
    scan.spans
}

pub fn single_write_span(
    program: &Program<'_>,
    scoping: &Scoping,
    symbol: SymbolId,
) -> Option<Span> {
    let symbols = FxHashSet::from_iter([symbol]);
    let spans = write_spans(program, scoping, &symbols);
    match spans.get(&symbol).map(Vec::as_slice) {
        Some([only]) => Some(*only),
        _ => None,
    }
}

fn owner_scope_of(scoping: &Scoping, symbol: SymbolId) -> Option<ScopeId> {
    let declaring = scoping.symbol_scope_id(symbol);
    scoping
        .scope_ancestors(declaring)
        .find(|&scope_id| scoping.scope_flags(scope_id).is_function())
}

struct CandidateState {
    owner_depth: Option<u32>,
    boundary: Span,
    passed: bool,
    found: bool,
}

struct UseScan<'s> {
    scoping: &'s Scoping,
    depth: u32,
    owners: FxHashMap<ScopeId, Vec<SymbolId>>,
    states: FxHashMap<SymbolId, CandidateState>,
    remaining: usize,
}

impl UseScan<'_> {
    fn unresolved_use(&mut self, name: &str) {
        let scoping = self.scoping;
        for (symbol, state) in &mut self.states {
            if !state.found && scoping.symbol_name(*symbol) == name {
                state.found = true;
                self.remaining -= 1;
            }
        }
    }

    fn enter_owner_scope(&mut self, scope_id: Option<ScopeId>) -> Vec<SymbolId> {
        self.depth += 1;
        let Some(owned) = scope_id.and_then(|scope_id| self.owners.get(&scope_id)) else {
            return Vec::new();
        };
        let owned = owned.clone();
        for symbol in &owned {
            if let Some(state) = self.states.get_mut(symbol) {
                state.owner_depth = Some(self.depth);
            }
        }
        owned
    }

    fn leave_owner_scope(&mut self, owned: Vec<SymbolId>) {
        for symbol in &owned {
            if let Some(state) = self.states.get_mut(symbol) {
                state.owner_depth = None;
            }
        }
        self.depth -= 1;
    }

    fn pass_the_write(&mut self, assignment: &AssignmentExpression<'_>) {
        if assignment.operator != AssignmentOperator::Assign {
            return;
        }
        let AssignmentTarget::AssignmentTargetIdentifier(target) = &assignment.left else {
            return;
        };
        let symbol = target
            .reference_id
            .get()
            .and_then(|id| self.scoping.get_reference(id).symbol_id());
        let Some(state) = symbol.and_then(|symbol| self.states.get_mut(&symbol)) else {
            return;
        };
        if state.boundary.start <= assignment.span.start
            && assignment.span.end <= state.boundary.end
        {
            state.passed = true;
        }
    }
}

impl<'a> VisitJs<'a> for UseScan<'_> {
    fn visit_function(&mut self, it: &Function<'a>, flags: ScopeFlags) {
        if self.remaining == 0 {
            return;
        }
        let owned = self.enter_owner_scope(it.scope_id.get());
        walk_js::walk_function(self, it, flags);
        self.leave_owner_scope(owned);
    }

    fn visit_arrow_function_expression(&mut self, it: &ArrowFunctionExpression<'a>) {
        if self.remaining == 0 {
            return;
        }
        let owned = self.enter_owner_scope(it.scope_id.get());
        walk_js::walk_arrow_function_expression(self, it);
        self.leave_owner_scope(owned);
    }

    fn visit_static_block(&mut self, it: &StaticBlock<'a>) {
        if self.remaining == 0 {
            return;
        }
        self.depth += 1;
        self.visit_statements(&it.body);
        self.depth -= 1;
    }

    fn visit_property_definition(&mut self, it: &PropertyDefinition<'a>) {
        if self.remaining == 0 {
            return;
        }
        self.visit_decorators(&it.decorators);
        self.visit_property_key(&it.key);
        if let Some(value) = &it.value {
            self.depth += 1;
            self.visit_expression(value);
            self.depth -= 1;
        }
    }

    fn visit_accessor_property(&mut self, it: &AccessorProperty<'a>) {
        if self.remaining == 0 {
            return;
        }
        self.visit_decorators(&it.decorators);
        self.visit_property_key(&it.key);
        if let Some(value) = &it.value {
            self.depth += 1;
            self.visit_expression(value);
            self.depth -= 1;
        }
    }

    fn visit_statement(&mut self, it: &Statement<'a>) {
        if self.remaining == 0 {
            return;
        }
        walk_js::walk_statement(self, it);
    }

    fn visit_expression(&mut self, it: &Expression<'a>) {
        if self.remaining == 0 {
            return;
        }
        walk_js::walk_expression(self, it);
    }

    fn visit_assignment_expression(&mut self, it: &AssignmentExpression<'a>) {
        walk_js::walk_assignment_expression(self, it);
        self.pass_the_write(it);
    }

    fn visit_identifier_reference(&mut self, it: &IdentifierReference<'a>) {
        if self.remaining == 0 {
            return;
        }
        let Some(reference_id) = it.reference_id.get() else {
            self.unresolved_use(it.name.as_str());
            return;
        };
        let reference = self.scoping.get_reference(reference_id);
        let Some(symbol) = reference.symbol_id() else {
            return;
        };
        let Some(state) = self.states.get_mut(&symbol) else {
            return;
        };
        if state.found {
            return;
        }
        let is_nested = state
            .owner_depth
            .is_some_and(|owner_depth| self.depth > owner_depth);
        if !is_nested && reference.flags().is_write_only() {
            return;
        }
        if is_nested || !state.passed {
            state.found = true;
            self.remaining -= 1;
        }
    }
}

pub fn bindings_possibly_used_before(
    program: &Program<'_>,
    scoping: &Scoping,
    candidates: &[(SymbolId, Span)],
) -> FxHashSet<SymbolId> {
    let mut states = FxHashMap::default();
    let mut owners: FxHashMap<ScopeId, Vec<SymbolId>> = FxHashMap::default();
    for &(symbol, boundary) in candidates {
        let owner_scope_id = owner_scope_of(scoping, symbol);
        if let Some(scope_id) = owner_scope_id {
            owners.entry(scope_id).or_default().push(symbol);
        }
        let owner_depth = owner_scope_id.is_none().then_some(0);
        states.insert(
            symbol,
            CandidateState {
                owner_depth,
                boundary,
                passed: false,
                found: false,
            },
        );
    }
    let mut scan = UseScan {
        scoping,
        depth: 0,
        remaining: states.len(),
        owners,
        states,
    };
    scan.visit_program(program);
    scan.states
        .into_iter()
        .filter(|(_, state)| state.found)
        .map(|(symbol, _)| symbol)
        .collect()
}

pub fn is_binding_possibly_used_before(
    program: &Program<'_>,
    scoping: &Scoping,
    symbol: SymbolId,
    boundary: Span,
) -> bool {
    bindings_possibly_used_before(program, scoping, &[(symbol, boundary)]).contains(&symbol)
}

pub fn effectively_constant_bindings(
    scoping: &Scoping,
    candidates: &[SymbolId],
) -> FxHashSet<SymbolId> {
    candidates
        .iter()
        .copied()
        .filter(|symbol| is_constant_binding(scoping, *symbol))
        .collect()
}
