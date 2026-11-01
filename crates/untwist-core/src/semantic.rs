use oxc_ast::ast::{
    AccessorProperty, ArrowFunctionBody, ArrowFunctionExpression, AssignmentTarget, BindingPattern,
    Expression, FormalParameter, Function, FunctionBody, IdentifierReference, Program,
    PropertyDefinition, Statement, StaticBlock,
};
use oxc_ast_visit::{walk_js, VisitJs};
use oxc_semantic::{Scoping, SemanticBuilder, Stats};
use oxc_span::{GetSpan, Span};
use oxc_syntax::operator::AssignmentOperator;
use oxc_syntax::scope::{ScopeFlags, ScopeId};
use oxc_syntax::symbol::{SymbolFlags, SymbolId};
use rustc_hash::{FxHashMap, FxHashSet};

#[cfg(feature = "test-hooks")]
pub mod compare;
mod observers;
mod refresh;
mod strictness;

pub use observers::{
    assume_observers, assumed_hazard, closed_eval_names, eval_payload_names, observers,
    with_observes, Observers, ObserversGuard,
};
#[cfg(feature = "test-hooks")]
pub use refresh::{
    check_patches_for_tests, last_stale_for_tests, refresh_counts_for_tests,
    skip_one_deletion_for_tests, RefreshCounts, STALE_PATCH,
};
pub use refresh::{forget_failures, refresh_scoping, Round, Shape};
pub use strictness::Strictness;

pub fn has_module_syntax(program: &Program<'_>) -> bool {
    program.body.iter().any(Statement::is_module_declaration)
}

pub fn program_is_strict(program: &Program<'_>) -> bool {
    program.has_use_strict_directive() || has_module_syntax(program)
}

pub fn scope_hazard(scoping: &Scoping) -> Option<&'static str> {
    for scope in scoping.scope_descendants_from_root() {
        let flags = scoping.scope_flags(scope);
        if flags.contains(ScopeFlags::DirectEval) {
            return Some("direct eval");
        }
        if flags.contains(ScopeFlags::With) && with_observes() {
            return Some("with statement");
        }
    }
    None
}

pub struct Semantics {
    pub scoping: Scoping,
    pub stats: Stats,
    pub patched: bool,
}

pub fn build_scoping(program: &Program<'_>) -> Semantics {
    settled(program, finish(SemanticBuilder::new().build(program)))
}

pub fn rebuild_scoping(program: &Program<'_>, previous: Stats) -> Semantics {
    settled(
        program,
        finish(SemanticBuilder::new().with_stats(previous).build(program)),
    )
}

fn settled(program: &Program<'_>, mut sem: Semantics) -> Semantics {
    observers::settle_eval_scopes(program, &mut sem.scoping);
    sem
}

fn finish(ret: oxc_semantic::SemanticBuilderReturn<'_>) -> Semantics {
    let stats = ret.semantic.stats();
    Semantics {
        scoping: ret.semantic.into_scoping(),
        stats,
        patched: false,
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
    !scoping.symbol_is_mutated(symbol) && scoping.symbol_redeclarations(symbol).is_empty()
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

    fn visit_identifier_reference(&mut self, it: &IdentifierReference<'a>) {
        if self.remaining == 0 {
            return;
        }
        let Some(reference_id) = it.reference_id.get() else {
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
        if is_nested || it.span.start < state.boundary.end {
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

struct OwnerBodyScan {
    owner_scope_id: ScopeId,
    boundary: Span,
    found: bool,
}

impl<'a> VisitJs<'a> for OwnerBodyScan {
    fn visit_function(&mut self, it: &Function<'a>, flags: ScopeFlags) {
        if it.scope_id.get() == Some(self.owner_scope_id) {
            if let Some(body) = it.body.as_deref() {
                self.found = body_has_the_write(body, self.boundary);
            }
            return;
        }
        walk_js::walk_function(self, it, flags);
    }

    fn visit_arrow_function_expression(&mut self, it: &ArrowFunctionExpression<'a>) {
        if it.scope_id.get() == Some(self.owner_scope_id) {
            if let ArrowFunctionBody::FunctionBody(body) = &it.body {
                self.found = body_has_the_write(body, self.boundary);
            }
            return;
        }
        walk_js::walk_arrow_function_expression(self, it);
    }
}

fn body_has_the_write(body: &FunctionBody<'_>, boundary: Span) -> bool {
    body.statements.iter().any(|statement| {
        let Statement::ExpressionStatement(inner) = statement else {
            return false;
        };
        let Expression::AssignmentExpression(assignment) = &inner.expression else {
            return false;
        };
        assignment.span == boundary
    })
}

fn is_write_at_owner_body_level(
    program: &Program<'_>,
    scoping: &Scoping,
    symbol: SymbolId,
    boundary: Span,
) -> bool {
    let Some(owner_scope_id) = owner_scope_of(scoping, symbol) else {
        return false;
    };
    let mut scan = OwnerBodyScan {
        owner_scope_id,
        boundary,
        found: false,
    };
    scan.visit_program(program);
    scan.found
}

fn may_be_parameter(scoping: &Scoping, symbol: SymbolId) -> bool {
    scoping
        .symbol_flags(symbol)
        .contains(SymbolFlags::FunctionScopedVariable)
}

pub fn effectively_constant_bindings(
    program: &Program<'_>,
    scoping: &Scoping,
    candidates: &[SymbolId],
) -> FxHashSet<SymbolId> {
    let mut constant = FxHashSet::default();
    let mut rest = FxHashSet::default();
    for symbol in candidates {
        if is_constant_binding(scoping, *symbol) {
            constant.insert(*symbol);
        } else if write_reference_count(scoping, *symbol) == 1 {
            rest.insert(*symbol);
        }
    }
    rest.retain(|symbol| may_be_parameter(scoping, *symbol));
    if rest.is_empty() {
        return constant;
    }
    let parameters = parameter_symbols(program);
    rest.retain(|symbol| parameters.contains(symbol));
    if rest.is_empty() {
        return constant;
    }
    let spans = write_spans(program, scoping, &rest);
    let pairs: Vec<(SymbolId, Span)> = rest
        .iter()
        .filter_map(|symbol| match spans.get(symbol).map(Vec::as_slice) {
            Some([only]) => Some((*symbol, *only)),
            _ => None,
        })
        .filter(|(symbol, span)| is_write_at_owner_body_level(program, scoping, *symbol, *span))
        .collect();
    let used_before = bindings_possibly_used_before(program, scoping, &pairs);
    constant.extend(
        pairs
            .into_iter()
            .map(|(symbol, _)| symbol)
            .filter(|symbol| !used_before.contains(symbol)),
    );
    constant
}

pub fn is_effectively_constant_binding(
    program: &Program<'_>,
    scoping: &Scoping,
    symbol: SymbolId,
) -> bool {
    effectively_constant_bindings(program, scoping, &[symbol]).contains(&symbol)
}
