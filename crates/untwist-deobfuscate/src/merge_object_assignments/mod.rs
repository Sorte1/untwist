use oxc_allocator::{GetAllocator, Vec as AVec};
use oxc_ast::ast::*;
use oxc_ast_visit::{walk_mut, VisitJs, VisitMut};
use oxc_semantic::Scoping;
use oxc_span::{GetSpan, Span};
use oxc_syntax::symbol::SymbolId;
use rustc_hash::{FxHashMap, FxHashSet};
use untwist_core::ast_util::edit::{ExprEdits, StmtEdits};
use untwist_core::ast_util::global::is_global_reference;
use untwist_core::ast_util::literal::is_safe_literal;
use untwist_core::ast_util::prop::prop_name;
use untwist_core::ast_util::readonly::{scan_symbol_uses, PropRead, SymbolUses};
use untwist_core::ast_util::respan::synthetic_spans_exhausted;
use untwist_core::pass::{Cx, Interest, Pass};
use untwist_core::semantic::{is_constant_binding, read_reference_count, scope_hazard};

use crate::bindings::{reference_symbol, resolved_symbol, unexposed_globals};
use crate::control_flow_object::single_declarator;
use crate::fused::{Fused, Member, Walk};
use crate::init_order::{
    lend_order, pass_done, reads_are_initialised, refuse_read_before_declaration, InitOrder,
};
use crate::notes;
use crate::string_array::bare_template_text;

mod fold;

use fold::{
    dynamic_key_symbol, empty_object_binding, fold_assignments, plain_assignment, property_target,
    MergedKey, PROTO_KEY,
};

pub const PASS: &str = "merge-object-assignments";

pub fn is_inlineable_object(expr: &Expression<'_>) -> bool {
    match expr {
        Expression::ArrayExpression(array) => array
            .elements
            .iter()
            .all(|element| element.as_expression().is_some_and(is_inlineable_object)),
        Expression::ObjectExpression(object) => object.properties.iter().all(|property| {
            let ObjectPropertyKind::ObjectProperty(property) = property else {
                return false;
            };
            !property.computed
                && prop_name(&property.key).is_some()
                && is_inlineable_object(&property.value)
        }),
        other => is_safe_literal(other),
    }
}

struct RepeatedScan<'s> {
    scoping: &'s Scoping,
    owners: &'s FxHashMap<Span, SymbolId>,
    open: FxHashMap<SymbolId, u32>,
    settled: FxHashSet<SymbolId>,
    depth: u32,
    repeated: FxHashSet<SymbolId>,
}

impl<'s> RepeatedScan<'s> {
    fn new(scoping: &'s Scoping, owners: &'s FxHashMap<Span, SymbolId>) -> Self {
        Self {
            scoping,
            owners,
            open: FxHashMap::default(),
            settled: FxHashSet::default(),
            depth: 0,
            repeated: FxHashSet::default(),
        }
    }

    fn enter_list(&mut self, stmts: &[Statement<'_>]) -> Vec<SymbolId> {
        let mut entered = Vec::new();
        for stmt in stmts {
            let Some(symbol) = self.owners.get(&stmt.span()).copied() else {
                continue;
            };
            if self.settled.insert(symbol) {
                self.open.insert(symbol, self.depth);
                entered.push(symbol);
            }
        }
        entered
    }

    fn deeper(&mut self) -> Vec<SymbolId> {
        self.depth += 1;
        Vec::new()
    }

    fn finish(self, candidates: &[(Span, SymbolId)]) -> FxHashSet<SymbolId> {
        let mut repeated = self.repeated;
        for (_, symbol) in candidates {
            if !self.settled.contains(symbol) {
                repeated.insert(*symbol);
            }
        }
        repeated
    }
}

impl<'a> Member<'a> for RepeatedScan<'_> {
    type Saved = Vec<SymbolId>;

    #[inline(always)]
    fn enter_statements(&mut self, it: &AVec<'a, Statement<'a>>) -> Vec<SymbolId> {
        self.enter_list(it)
    }

    #[inline(always)]
    fn leave_statements(&mut self, entered: Vec<SymbolId>) {
        for symbol in entered {
            self.open.remove(&symbol);
        }
    }

    #[inline(always)]
    fn enter_expression(&mut self, it: &Expression<'a>) -> Option<Vec<SymbolId>> {
        if let Expression::Identifier(id) = it {
            if let Some(symbol) = resolved_symbol(id, self.scoping) {
                if self
                    .open
                    .get(&symbol)
                    .is_some_and(|entry| *entry < self.depth)
                {
                    self.repeated.insert(symbol);
                }
            }
        }
        Some(Vec::new())
    }

    #[inline(always)]
    fn enter_function(&mut self, _it: &Function<'a>) -> Vec<SymbolId> {
        self.deeper()
    }

    #[inline(always)]
    fn leave_function(&mut self, _saved: Vec<SymbolId>) {
        self.depth -= 1;
    }

    #[inline(always)]
    fn enter_arrow(&mut self, _it: &ArrowFunctionExpression<'a>) -> Vec<SymbolId> {
        self.deeper()
    }

    #[inline(always)]
    fn leave_arrow(&mut self, _saved: Vec<SymbolId>) {
        self.depth -= 1;
    }

    #[inline(always)]
    fn enter_nested(&mut self) -> Vec<SymbolId> {
        self.deeper()
    }

    #[inline(always)]
    fn leave_nested(&mut self, _saved: Vec<SymbolId>) {
        self.depth -= 1;
    }
}

struct InlineFacts {
    uses: SymbolUses,
    repeated: FxHashSet<SymbolId>,
}

fn repeated_contexts(
    program: &Program<'_>,
    scoping: &Scoping,
    candidates: &[(Span, SymbolId)],
) -> FxHashSet<SymbolId> {
    let owners: FxHashMap<Span, SymbolId> = candidates.iter().copied().collect();
    let mut walk = Walk(RepeatedScan::new(scoping, &owners));
    walk.visit_program(program);
    walk.0.finish(candidates)
}

fn inline_facts(
    program: &Program<'_>,
    scoping: &Scoping,
    candidates: &[(Span, SymbolId)],
) -> InlineFacts {
    let symbols: FxHashSet<SymbolId> = candidates.iter().map(|(_, symbol)| *symbol).collect();
    InlineFacts {
        uses: scan_symbol_uses(program, scoping, &symbols),
        repeated: repeated_contexts(program, scoping, candidates),
    }
}

#[cfg(all(debug_assertions, feature = "test-hooks"))]
pub(crate) fn fused_and_separate_facts(
    program: &Program<'_>,
    scoping: &Scoping,
    candidates: &[(Span, SymbolId)],
) -> (u64, u64) {
    use crate::fused::check::{digest_of, sorted, symbol_uses_digest};
    use crate::init_order::try_scan_init_order_with;
    use untwist_core::ast_util::readonly::UsesScan;
    let symbols: FxHashSet<SymbolId> = candidates.iter().map(|(_, symbol)| *symbol).collect();
    let owners: FxHashMap<Span, SymbolId> = candidates.iter().copied().collect();
    let digest = |order: Option<InitOrder>, uses: &SymbolUses, repeated: FxHashSet<SymbolId>| {
        let repeated = sorted(repeated.into_iter().map(|symbol| (symbol.index(), ())));
        digest_of(&(
            order.map(|order| order.digest()),
            symbol_uses_digest(uses),
            repeated,
        ))
    };
    let member = Fused::new(
        UsesScan::new(scoping, &symbols),
        RepeatedScan::new(scoping, &owners),
    );
    let (order, member) = try_scan_init_order_with(program, scoping, member);
    let fused = digest(
        order,
        &member.first.finish(),
        member.second.finish(candidates),
    );
    let mut repeated = Walk(RepeatedScan::new(scoping, &owners));
    repeated.visit_program(program);
    let separate = digest(
        crate::init_order::try_scan_init_order(program, scoping),
        &scan_symbol_uses(program, scoping, &symbols),
        repeated.0.finish(candidates),
    );
    (fused, separate)
}

#[cfg(all(debug_assertions, feature = "test-hooks"))]
pub(crate) fn fused_and_separate_keys(program: &Program<'_>, scoping: &Scoping) -> (u64, u64) {
    use crate::fused::check::{digest_of, sorted};
    let digest = |keys: FxHashSet<SymbolId>, found: Vec<(Span, SymbolId)>| {
        let keys = sorted(keys.into_iter().map(|symbol| (symbol.index(), ())));
        let found: Vec<(Span, usize)> = found
            .into_iter()
            .map(|(span, symbol)| (span, symbol.index()))
            .collect();
        digest_of(&(keys, found))
    };
    let (keys, found) = keys_and_candidates(program, scoping, true);
    let fused = digest(keys.unwrap_or_default(), found);
    let mut keys = Walk(DynamicKeys {
        scoping,
        found: FxHashSet::default(),
    });
    keys.visit_program(program);
    let (_, found) = keys_and_candidates(program, scoping, false);
    (fused, digest(keys.0.found, found))
}

#[cfg(debug_assertions)]
fn record_keys_and_candidates(keys: Option<&FxHashSet<SymbolId>>, candidates: &[(Span, SymbolId)]) {
    use crate::fused::check;
    if !check::active() {
        return;
    }
    let keys = keys.map(|keys| check::sorted(keys.iter().map(|symbol| (symbol.index(), ()))));
    let candidates: Vec<(Span, usize)> = candidates
        .iter()
        .map(|(span, symbol)| (*span, symbol.index()))
        .collect();
    check::record("keys-and-candidates", check::digest_of(&(keys, candidates)));
}

#[cfg(debug_assertions)]
fn record_inline_facts(facts: &InlineFacts, order: &InitOrder) {
    use crate::fused::check;
    if !check::active() {
        return;
    }
    let repeated = check::sorted(facts.repeated.iter().map(|symbol| (symbol.index(), ())));
    let digest = check::digest_of(&(
        order.digest(),
        check::symbol_uses_digest(&facts.uses),
        repeated,
    ));
    check::record("inline-facts", digest);
}

fn inlineable_declaration(stmt: &Statement<'_>) -> Option<SymbolId> {
    let (id, init) = single_declarator(stmt)?;
    let init = init?;
    let Expression::ObjectExpression(object) = init else {
        return None;
    };
    if object.properties.is_empty() || !is_inlineable_object(init) {
        return None;
    }
    id.symbol_id.get()
}

#[derive(Default)]
struct Candidates {
    found: Vec<(Span, SymbolId)>,
}

impl<'a> Member<'a> for Candidates {
    type Saved = ();

    #[inline(always)]
    fn enter_statement(&mut self, it: &Statement<'a>) {
        if let Some(symbol) = inlineable_declaration(it) {
            self.found.push((it.span(), symbol));
        }
    }
}

struct InlinePlan {
    statement: Span,
    reference: Span,
}

const PROTO_SHORTHAND: &str = "__proto__";

fn is_a_candidate(statement: Span, symbol: SymbolId, scoping: &Scoping) -> bool {
    statement.start < statement.end
        && scoping.symbol_name(symbol) != PROTO_SHORTHAND
        && read_reference_count(scoping, symbol) == 1
        && is_constant_binding(scoping, symbol)
}

fn reachable_reference(facts: &InlineFacts, symbol: SymbolId) -> Option<Span> {
    if facts.uses.of(symbol).is_some_and(|uses| uses.writes != 0) {
        return None;
    }
    if facts.repeated.contains(&symbol) {
        return None;
    }
    let [reference] = facts.uses.references_of(symbol) else {
        return None;
    };
    (reference.start < reference.end).then_some(*reference)
}

fn inline_plan(
    order: &InitOrder,
    facts: &InlineFacts,
    statement: Span,
    symbol: SymbolId,
    cx: &mut Cx<'_, '_>,
) -> Option<InlinePlan> {
    let reference = reachable_reference(facts, symbol)?;
    let reads = [PropRead {
        member_span: reference,
        object_span: reference,
        name: cx.scoping().symbol_name(symbol).to_string(),
    }];
    if let Err(refused) = reads_are_initialised(&reads, statement, order) {
        refuse_read_before_declaration(
            refused,
            symbol,
            statement,
            notes::MERGE_USE_BEFORE_INIT,
            cx,
        );
        return None;
    }
    Some(InlinePlan {
        statement,
        reference,
    })
}

fn keys_and_candidates(
    program: &Program<'_>,
    scoping: &Scoping,
    extended: bool,
) -> (Option<FxHashSet<SymbolId>>, Vec<(Span, SymbolId)>) {
    if !extended {
        let mut walk = Walk(Candidates::default());
        walk.visit_program(program);
        return (None, walk.0.found);
    }
    let keys = DynamicKeys {
        scoping,
        found: FxHashSet::default(),
    };
    let mut walk = Walk(Fused::new(keys, Candidates::default()));
    walk.visit_program(program);
    let Fused { first, second, .. } = walk.0;
    (Some(first.found), second.found)
}

fn folded_only(
    found: Vec<(Span, SymbolId)>,
    folded: Option<&FxHashSet<Span>>,
) -> Vec<(Span, SymbolId)> {
    found
        .into_iter()
        .filter(|(statement, _)| folded.is_none_or(|merged| merged.contains(statement)))
        .collect()
}

fn plan_inline(
    program: &Program<'_>,
    candidates: Vec<(Span, SymbolId)>,
    cx: &mut Cx<'_, '_>,
    shares: bool,
) -> Vec<InlinePlan> {
    let reachable: Vec<(Span, SymbolId)> = candidates
        .into_iter()
        .filter(|(statement, symbol)| is_a_candidate(*statement, *symbol, cx.scoping()))
        .collect();
    let reachable = unexposed_globals(reachable, |(_, symbol)| *symbol, program, cx.scoping());
    if reachable.is_empty() {
        return Vec::new();
    }
    let facts = inline_facts(program, cx.scoping(), &reachable);
    let order = lend_order(shares, program, cx);
    #[cfg(debug_assertions)]
    record_inline_facts(&facts, &order);
    let plans: Vec<InlinePlan> = reachable
        .into_iter()
        .filter_map(|(statement, symbol)| inline_plan(&order, &facts, statement, symbol, cx))
        .collect();
    if plans.is_empty() {
        order.give_back(program, cx);
    }
    plans
}

struct TakeLiteral<'a> {
    statement: Span,
    literal: Option<Expression<'a>>,
}

impl<'a> VisitMut<'a> for TakeLiteral<'a> {
    fn visit_statement(&mut self, it: &mut Statement<'a>) {
        if self.literal.is_some() {
            return;
        }
        if it.span() != self.statement {
            walk_mut::walk_statement(self, it);
            return;
        }
        let Statement::VariableDeclaration(declaration) = it else {
            return;
        };
        let Some(declarator) = declaration.declarations.first_mut() else {
            return;
        };
        self.literal = declarator.init.take();
    }
}

fn take_literal<'a>(program: &mut Program<'a>, statement: Span) -> Option<Expression<'a>> {
    let mut take = TakeLiteral {
        statement,
        literal: None,
    };
    take.visit_program(program);
    take.literal
}

fn inline_literals<'a>(
    program: &mut Program<'a>,
    plans: &[InlinePlan],
    cx: &mut Cx<'a, '_>,
) -> usize {
    let mut edits = ExprEdits::new();
    let mut removals = StmtEdits::new();
    for plan in plans {
        let Some(literal) = take_literal(program, plan.statement) else {
            continue;
        };
        if edits.insert(plan.reference, literal) {
            removals.remove(plan.statement);
        }
    }
    let wanted = edits.len() + removals.len();
    let applied = edits.apply(program, cx.allocator(), cx.scoping_mut())
        + removals.apply(program, cx.allocator(), cx.scoping_mut());
    if applied != wanted {
        cx.note(
            notes::planned_and_applied_edits(PASS, wanted, applied),
            None,
        );
    }
    applied
}

struct DynamicKeys<'s> {
    scoping: &'s Scoping,
    found: FxHashSet<SymbolId>,
}

impl DynamicKeys<'_> {
    fn keys_of_the_run(&mut self, stmts: &[Statement<'_>], index: usize, object: SymbolId) {
        for stmt in stmts.iter().skip(index + 1) {
            let Some((target, key)) =
                plain_assignment(stmt).and_then(|assignment| property_target(&assignment.left))
            else {
                return;
            };
            if reference_symbol(target, self.scoping) != Some(object) {
                return;
            }
            if let MergedKey::Dynamic(expression) = key {
                if let Some(symbol) = dynamic_key_symbol(expression, self.scoping) {
                    self.found.insert(symbol);
                }
            }
        }
    }
}

impl<'a> Member<'a> for DynamicKeys<'_> {
    type Saved = ();

    #[inline(always)]
    fn enter_statements(&mut self, it: &AVec<'a, Statement<'a>>) {
        for (index, stmt) in it.iter().enumerate() {
            if let Some((object, _)) = empty_object_binding(stmt, self.scoping) {
                self.keys_of_the_run(it, index, object);
            }
        }
    }
}

fn resolvable_dynamic_keys(
    program: &Program<'_>,
    scoping: &Scoping,
    candidates: &FxHashSet<SymbolId>,
    (memo, pristine): (&mut FxHashMap<SymbolId, bool>, bool),
) -> FxHashSet<SymbolId> {
    if candidates.is_empty() {
        return FxHashSet::default();
    }
    let fresh: Vec<SymbolId> = candidates
        .iter()
        .copied()
        .filter(|symbol| !memo.contains_key(symbol))
        .collect();
    let inits = crate::calls::constant_initialisers(program, scoping, &fresh);
    for (symbol, init) in fresh.iter().zip(inits) {
        #[cfg(debug_assertions)]
        crate::fused::check::record(
            "constant-initialiser",
            crate::fused::check::digest_of(&(
                symbol.index(),
                init.map(|init| (init.span(), std::mem::discriminant(init))),
            )),
        );
        let safe = init.is_some_and(|init| {
            is_a_safe_literal_key(init) || (pristine && is_a_symbol(init, scoping))
        });
        memo.insert(*symbol, safe);
    }
    candidates
        .iter()
        .copied()
        .filter(|symbol| memo.get(symbol).copied().unwrap_or(false))
        .collect()
}

fn global_is_written(scoping: &Scoping, name: &str) -> bool {
    scoping
        .root_unresolved_references()
        .iter()
        .filter(|(unresolved, _)| unresolved.as_str() == name)
        .any(|(_, ids)| ids.iter().any(|id| scoping.get_reference(*id).is_write()))
}

fn is_a_symbol(init: &Expression<'_>, scoping: &Scoping) -> bool {
    let Expression::CallExpression(call) = init.without_parentheses() else {
        return false;
    };
    let root = match call.callee.without_parentheses() {
        Expression::Identifier(id) => id,
        Expression::StaticMemberExpression(member) if member.property.name == "for" => {
            let Expression::Identifier(id) = member.object.without_parentheses() else {
                return false;
            };
            id
        }
        _ => return false,
    };
    root.name == "Symbol"
        && is_global_reference(scoping, root)
        && !global_is_written(scoping, "Symbol")
}

fn is_a_safe_literal_key(init: &Expression<'_>) -> bool {
    match init {
        Expression::NumericLiteral(_) => true,
        Expression::StringLiteral(literal) => literal.value.as_str() != PROTO_KEY,
        other => bare_template_text(other).is_some_and(|text| text != PROTO_KEY),
    }
}

#[derive(Default)]
pub struct MergeObjectAssignments {
    folded: FxHashSet<Span>,
    resolved: FxHashSet<SymbolId>,
    memo: FxHashMap<SymbolId, bool>,
    hazard: bool,
}

pub fn boxed<'a>() -> Box<dyn Pass<'a> + 'a> {
    Box::new(MergeObjectAssignments::default())
}

impl<'a> Pass<'a> for MergeObjectAssignments {
    fn name(&self) -> &'static str {
        PASS
    }

    fn interest(&self) -> Interest {
        Interest::ENTER_PROGRAM | Interest::EXIT_STATEMENTS
    }

    fn reset(&mut self) {
        self.memo.clear();
        self.resolved.clear();
    }

    fn enter_program(&mut self, program: &mut Program<'a>, cx: &mut Cx<'a, '_>) {
        self.hazard = scope_hazard(cx.scoping()).is_some();
        if self.hazard {
            return;
        }
        let (keys, found) = keys_and_candidates(program, cx.scoping(), cx.opts.extended);
        if let Some(keys) = keys.as_ref() {
            let pristine = cx.opts.unsafe_transforms;
            self.resolved =
                resolvable_dynamic_keys(program, cx.scoping(), keys, (&mut self.memo, pristine));
        }
        let conservative = (!cx.opts.extended).then_some(&self.folded);
        let candidates = folded_only(found, conservative);
        #[cfg(debug_assertions)]
        record_keys_and_candidates(keys.as_ref(), &candidates);
        if candidates.is_empty() {
            return;
        }
        let plans = plan_inline(program, candidates, cx, cx.opts.shares_init_order);
        if plans.is_empty() {
            return;
        }
        if synthetic_spans_exhausted() {
            let span = plans.first().map(|plan| plan.statement);
            cx.note(notes::MERGE_SPANS_EXHAUSTED, span);
            return;
        }
        let applied = inline_literals(program, &plans, cx);
        cx.changed_by(applied);
        pass_done(cx.opts.shares_init_order, applied > 0);
    }

    fn exit_statements(&mut self, stmts: &mut AVec<'a, Statement<'a>>, cx: &mut Cx<'a, '_>) {
        if self.hazard {
            return;
        }
        let folded = fold_assignments(stmts, &mut self.folded, &self.resolved, cx);
        cx.changed_by(folded);
    }
}
