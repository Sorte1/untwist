mod bits;
mod checksum;
mod eval;
mod guards;
mod identity;
mod inert;
mod length_reads;
mod registers;
mod sealed;
mod value;

#[cfg(test)]
mod tests;

use oxc_ast::ast::{
    Expression, IdentifierReference, Program, Statement, VariableDeclaration,
    VariableDeclarationKind,
};
use oxc_ast_visit::VisitJs;
use oxc_semantic::Scoping;
use oxc_span::Span;
use oxc_syntax::operator::{BinaryOperator, UnaryOperator};
use oxc_syntax::symbol::SymbolId;
use rustc_hash::{FxHashMap, FxHashSet};
use untwist_core::pass::Cx;
use untwist_core::semantic::builtins::{patched_by_another_input, scan, scan_outside, Reliance};
use untwist_core::semantic::{
    has_module_syntax, may_call_eval, other_inputs_given, top_level_is_local,
};

use crate::init_order::{
    read_before_declaration, read_before_local_declaration, read_during_initialiser,
};
pub use bits::Bits;
use eval::{abstract_value, concrete, low_determined, Leaves};
pub use value::{Kind, Val};

const ENUMERATED_BITS: u32 = 12;

static MATH_IMUL: Reliance = Reliance {
    roots: &["Math"],
    members: &[("Math", false, &["imul"])],
};

#[derive(Default)]
pub struct KnownBits {
    approved: FxHashMap<SymbolId, Val>,
    maybe: FxHashMap<SymbolId, Val>,
    imul: bool,
    wanted: bool,
    barren: bool,
    seedless: bool,
    plain: identity::PlainSymbols,
    plain_ready: bool,
    identities: bool,
    local: bool,
}

impl KnownBits {
    pub fn learn(
        &mut self,
        program: &Program<'_>,
        cx: &Cx<'_, '_>,
        has_with: impl FnOnce() -> bool,
    ) {
        self.approved.clear();
        self.maybe.clear();
        self.imul = false;
        self.plain.clear();
        self.plain_ready = false;
        self.local = top_level_is_local(program);
        let seeking = self.wanted && !self.seedless;
        if !(seeking || self.identities) || self.barren {
            return;
        }
        if has_with() || may_call_eval(cx.scoping()) {
            self.barren = true;
            return;
        }
        if seeking {
            self.learn_seeds(program, cx);
        }
        if self.identities || !self.approved.is_empty() {
            self.learn_plain(program, cx);
        }
        let known = !self.approved.is_empty() || !self.maybe.is_empty();
        self.imul = known && imul_holds(program, cx, &self.approved, &self.maybe, &self.plain);
        if self.plain_ready || known {
            cx.walk_whole_round();
        }
    }

    fn learn_seeds(&mut self, program: &Program<'_>, cx: &Cx<'_, '_>) {
        let scoping = cx.scoping();
        let lane = cx.opts.unsafe_transforms;
        let found = seeds(program, scoping, lane, false);
        let none = FxHashSet::default();
        let mut approved = self.approve(program, scoping, &found.computed, &none);
        let partial = approved.len() < found.computed.len();
        if self.local && (found.refused || partial) {
            let hypothesis = if found.refused {
                seeds(program, scoping, lane, true)
            } else {
                found.clone()
            };
            let settled = self.settle(program, cx, &hypothesis.computed, &hypothesis.assumed);
            if let Some(wider) = settled.filter(|wider| wider.len() > approved.len()) {
                approved = wider;
            }
        }
        if approved.is_empty() && found.empty {
            self.seedless = true;
        }
        self.maybe = maybe_of(&found.computed, &approved);
        self.approved = approved;
    }

    fn settle(
        &self,
        program: &Program<'_>,
        cx: &Cx<'_, '_>,
        computed: &FxHashMap<SymbolId, Seed>,
        assumed: &[&'static Reliance],
    ) -> Option<FxHashMap<SymbolId, Val>> {
        let scoping = cx.scoping();
        let none = FxHashSet::default();
        let base = self.approve(program, scoping, computed, &none);
        if base.is_empty() {
            return None;
        }
        let maybe = maybe_of(computed, &base);
        let (plain, known) = self.with_frames(program, scoping, base);
        let dead = dead_with(program, scoping, (&known, &maybe), &plain, self.local);
        let silent = guards::references_inside(program, &dead);
        let widened = self.approve(program, scoping, computed, &silent);
        let maybe = maybe_of(computed, &widened);
        let (plain, known) = self.with_frames(program, scoping, widened.clone());
        let dead = dead_with(program, scoping, (&known, &maybe), &plain, self.local);
        let mut relied: Vec<&'static Reliance> = assumed.to_vec();
        relied.push(&MATH_IMUL);
        relied_hold(program, cx, &relied, &dead).then_some(widened)
    }

    fn with_frames(
        &self,
        program: &Program<'_>,
        scoping: &Scoping,
        approved: FxHashMap<SymbolId, Val>,
    ) -> (identity::PlainSymbols, FxHashMap<SymbolId, Val>) {
        let frames = Frames {
            approved: &approved,
            scoping,
        };
        let evaluate = |init: &Expression<'_>| frames.value(init);
        let mut plain = identity::plain_symbols(program, scoping, self.local, Some(&evaluate));
        let mut known = approved.clone();
        known.extend(std::mem::take(&mut plain.frames));
        (plain, known)
    }

    fn approve(
        &self,
        program: &Program<'_>,
        scoping: &Scoping,
        computed: &FxHashMap<SymbolId, Seed>,
        silent: &FxHashSet<Span>,
    ) -> FxHashMap<SymbolId, Val> {
        if computed.is_empty() {
            return FxHashMap::default();
        }
        let bound: Vec<(Span, SymbolId)> = computed
            .iter()
            .map(|(symbol, seed)| (seed.statement, *symbol))
            .collect();
        let early = if self.local {
            let mut harmless = length_reads::function_length_reads(program, scoping);
            harmless.extend(silent.iter().copied());
            read_before_local_declaration(program, scoping, &bound, &harmless)
        } else {
            read_before_declaration(program, scoping, &bound)
        };
        let during: Vec<(Span, SymbolId, bool)> = computed
            .iter()
            .filter(|(_, seed)| !seed.closed)
            .map(|(symbol, seed)| (seed.statement, *symbol, seed.inert))
            .collect();
        let during = read_during_initialiser(program, scoping, &during);
        computed
            .iter()
            .filter(|(symbol, _)| !early.contains(symbol) && !during.contains(symbol))
            .map(|(symbol, seed)| (*symbol, seed.value))
            .collect()
    }

    fn learn_plain(&mut self, program: &Program<'_>, cx: &Cx<'_, '_>) {
        let scoping = cx.scoping();
        let frames = Frames {
            approved: &self.approved,
            scoping,
        };
        let evaluate = |init: &Expression<'_>| frames.value(init);
        let frame: Option<identity::Frame<'_>> =
            (!self.approved.is_empty()).then_some(&evaluate as identity::Frame<'_>);
        self.plain = identity::plain_symbols(program, scoping, self.local, frame);
        let found = std::mem::take(&mut self.plain.frames);
        self.approved.extend(found);
        self.plain_ready = true;
    }

    pub fn identity(&mut self, expr: &Expression<'_>, cx: &Cx<'_, '_>) -> Option<bool> {
        if self.plain_ready {
            if let Some(value) = self.plain.flag(expr, cx.scoping()) {
                return Some(value);
            }
        }
        let relation = identity::relation(expr)?;
        if self.plain_ready {
            return relation.decide(&self.plain, cx.scoping());
        }
        if !self.identities && !self.barren {
            self.identities = true;
            cx.opts.round_requested.set(true);
        }
        None
    }

    pub fn decide(&mut self, expr: &Expression<'_>, cx: &Cx<'_, '_>) -> Option<bool> {
        if let Some(value) = self.identity(expr, cx) {
            return Some(value);
        }
        if self.approved.is_empty() && self.maybe.is_empty() {
            if !self.wanted
                && !self.seedless
                && !self.barren
                && is_candidate(expr, cx.scoping(), self.local)
            {
                self.wanted = true;
                cx.opts.round_requested.set(true);
            }
            return None;
        }
        decide_strict(expr, &self.approved, &self.maybe, cx.scoping(), self.imul)
    }
}

fn imul_holds(
    program: &Program<'_>,
    cx: &Cx<'_, '_>,
    approved: &FxHashMap<SymbolId, Val>,
    maybe: &FxHashMap<SymbolId, Val>,
    plain: &identity::PlainSymbols,
) -> bool {
    if patched_by_another_input(&MATH_IMUL) {
        return false;
    }
    let module = has_module_syntax(program);
    let scoping = cx.scoping();
    let lane = cx.opts.unsafe_transforms;
    if scan(program, scoping, module, false, &MATH_IMUL)
        .verdict
        .allows_safe(lane)
    {
        return true;
    }
    let decide = |expr: &Expression<'_>| decide_strict(expr, approved, maybe, scoping, true);
    let dead = guards::dead_regions(
        program,
        scoping,
        &decide,
        plain,
        top_level_is_local(program),
    );
    !dead.is_empty() && scan_outside(program, scoping, module, &MATH_IMUL, &dead).allows_safe(lane)
}

fn decide_with(expr: &Expression<'_>, leaves: &dyn Leaves) -> Option<bool> {
    if let Some(known) = leaves.kind_of(expr) {
        return Some(known);
    }
    match expr.without_parentheses() {
        Expression::UnaryExpression(unary) if unary.operator == UnaryOperator::LogicalNot => {
            decide_with(&unary.argument, leaves).map(|value| !value)
        }
        Expression::BinaryExpression(binary) => {
            let negate = match binary.operator {
                BinaryOperator::StrictEquality | BinaryOperator::Equality => false,
                BinaryOperator::StrictInequality | BinaryOperator::Inequality => true,
                _ => return abstract_value(expr, leaves)?.truthy(),
            };
            equality(&binary.left, &binary.right, leaves).map(|equal| equal != negate)
        }
        other => abstract_value(other, leaves)?.truthy(),
    }
}

fn equality(left: &Expression<'_>, right: &Expression<'_>, leaves: &dyn Leaves) -> Option<bool> {
    let left_value = abstract_value(left, leaves)?;
    let right_value = abstract_value(right, leaves)?;
    if let Some(decided) = left_value.equals(right_value) {
        return Some(decided);
    }
    enumerate([(left, left_value), (right, right_value)], leaves)
}

fn enumerate(sides: [(&Expression<'_>, Val); 2], leaves: &dyn Leaves) -> Option<bool> {
    let width = sides
        .iter()
        .map(|(_, value)| 32 - value.bits.unknown().leading_zeros())
        .max()?;
    let window = bits::low_mask(width);
    let mut seen = Vec::new();
    if !sides
        .iter()
        .all(|(expr, _)| low_determined(expr, leaves, &mut seen))
    {
        return None;
    }
    let free: Vec<u32> = seen
        .iter()
        .map(|(_, known)| known.unknown() & window)
        .collect();
    let count: u32 = free.iter().map(|mask| mask.count_ones()).sum();
    if count > ENUMERATED_BITS {
        return None;
    }
    let mut outcome = None;
    for assignment in 0..(1u32 << count) {
        let env = |symbol: SymbolId| {
            let index = seen
                .iter()
                .position(|(candidate, _)| *candidate == symbol)?;
            let offset: u32 = free[..index].iter().map(|mask| mask.count_ones()).sum();
            Some(seen[index].1.ones | scatter(assignment >> offset, free[index]))
        };
        let [left, right] = sides.map(|(expr, value)| {
            let low = concrete(expr, leaves, &env)?;
            let full = (value.bits.ones & !window) | (low & window);
            value.bits.admits(full).then(|| Val {
                bits: Bits::exact(full),
                kind: value.kind,
            })
        });
        let decided = left?.equals(right?)?;
        if outcome.is_some_and(|previous| previous != decided) {
            return None;
        }
        outcome = Some(decided);
    }
    outcome
}

fn scatter(mut source: u32, mask: u32) -> u32 {
    let mut out = 0;
    let mut remaining = mask;
    while remaining != 0 {
        let bit = remaining.isolate_lowest_one();
        if source & 1 == 1 {
            out |= bit;
        }
        source >>= 1;
        remaining &= remaining - 1;
    }
    out
}

pub(super) fn resolved(id: &IdentifierReference<'_>, scoping: &Scoping) -> Option<SymbolId> {
    scoping.get_reference(id.reference_id.get()?).symbol_id()
}

fn is_global_math_imul(callee: &Expression<'_>, scoping: &Scoping) -> bool {
    let Expression::StaticMemberExpression(member) = callee.without_parentheses() else {
        return false;
    };
    if member.optional || member.property.name != "imul" {
        return false;
    }
    let Expression::Identifier(object) = &member.object else {
        return false;
    };
    object.name == "Math"
        && object.reference_id.get().is_some()
        && resolved(object, scoping).is_none()
}

struct Strict<'k, 's> {
    approved: &'k FxHashMap<SymbolId, Val>,
    maybe: &'k FxHashMap<SymbolId, Val>,
    absent: bool,
    scoping: &'s Scoping,
    imul: bool,
}

fn decide_strict(
    expr: &Expression<'_>,
    approved: &FxHashMap<SymbolId, Val>,
    maybe: &FxHashMap<SymbolId, Val>,
    scoping: &Scoping,
    imul: bool,
) -> Option<bool> {
    let present = Strict {
        approved,
        maybe,
        absent: false,
        scoping,
        imul,
    };
    let decided = decide_with(expr, &present)?;
    if maybe.is_empty() {
        return Some(decided);
    }
    let missing = Strict {
        absent: true,
        ..present
    };
    (decide_with(expr, &missing)? == decided).then_some(decided)
}

impl Leaves for Strict<'_, '_> {
    fn leaf(&self, id: &IdentifierReference<'_>) -> Option<(SymbolId, Val)> {
        let symbol = resolved(id, self.scoping)?;
        if let Some(value) = self.approved.get(&symbol) {
            return Some((symbol, *value));
        }
        let value = self.maybe.get(&symbol)?;
        Some((
            symbol,
            if self.absent {
                Val::undefined()
            } else {
                *value
            },
        ))
    }

    fn is_imul(&self, callee: &Expression<'_>) -> bool {
        self.imul && is_global_math_imul(callee, self.scoping)
    }

    fn lax(&self) -> bool {
        false
    }

    fn kind_of(&self, test: &Expression<'_>) -> Option<bool> {
        typeof_number(test, self.approved, self.scoping)
    }
}

struct Frames<'k, 's> {
    approved: &'k FxHashMap<SymbolId, Val>,
    scoping: &'s Scoping,
}

impl Frames<'_, '_> {
    fn value(&self, init: &Expression<'_>) -> Option<Val> {
        if !reads_any(init, self.approved, self.scoping) {
            return None;
        }
        abstract_value(init, self).filter(|value| value.bits.known() != 0)
    }
}

impl Leaves for Frames<'_, '_> {
    fn leaf(&self, id: &IdentifierReference<'_>) -> Option<(SymbolId, Val)> {
        let symbol = resolved(id, self.scoping)?;
        self.approved.get(&symbol).map(|value| (symbol, *value))
    }

    fn is_imul(&self, _callee: &Expression<'_>) -> bool {
        false
    }

    fn lax(&self) -> bool {
        true
    }

    fn decided(&self, test: &Expression<'_>) -> Option<bool> {
        typeof_number(test, self.approved, self.scoping)
    }
}

fn typeof_number(
    test: &Expression<'_>,
    approved: &FxHashMap<SymbolId, Val>,
    scoping: &Scoping,
) -> Option<bool> {
    let Expression::BinaryExpression(binary) = test.without_parentheses() else {
        return None;
    };
    let negate = match binary.operator {
        BinaryOperator::StrictEquality | BinaryOperator::Equality => false,
        BinaryOperator::StrictInequality | BinaryOperator::Inequality => true,
        _ => return None,
    };
    let (Expression::UnaryExpression(unary), Expression::StringLiteral(kind)) = (
        binary.left.without_parentheses(),
        binary.right.without_parentheses(),
    ) else {
        return None;
    };
    let Expression::Identifier(id) = unary.argument.without_parentheses() else {
        return None;
    };
    if unary.operator != UnaryOperator::Typeof || !approved.contains_key(&resolved(id, scoping)?) {
        return None;
    }
    Some((kind.value == "number") != negate)
}

fn reads_any(
    expr: &Expression<'_>,
    approved: &FxHashMap<SymbolId, Val>,
    scoping: &Scoping,
) -> bool {
    let mut scan = ReadsAny {
        approved,
        scoping,
        found: false,
    };
    scan.visit_expression(expr);
    scan.found
}

struct ReadsAny<'k, 's> {
    approved: &'k FxHashMap<SymbolId, Val>,
    scoping: &'s Scoping,
    found: bool,
}

impl<'a> VisitJs<'a> for ReadsAny<'_, '_> {
    fn visit_identifier_reference(&mut self, it: &IdentifierReference<'a>) {
        self.found |=
            resolved(it, self.scoping).is_some_and(|symbol| self.approved.contains_key(&symbol));
    }
}

#[derive(Clone)]
struct Seed {
    statement: Span,
    value: Val,
    inert: bool,
    closed: bool,
    var: bool,
}

struct Lax<'k, 's, 'r, 'p> {
    computed: &'k FxHashMap<SymbolId, Seed>,
    scoping: &'s Scoping,
    roots: &'r checksum::Roots<'p>,
    seed: Span,
    latches: &'k sealed::Latches,
    sealed: bool,
}

impl Lax<'_, '_, '_, '_> {
    fn is_function(&self, symbol: SymbolId) -> bool {
        self.roots.is_top_function(symbol)
    }

    fn sealed_part(&self, part: &Expression<'_>) -> Option<bool> {
        if sealed::length_guard(part, &|symbol| self.is_function(symbol), self.scoping) {
            return Some(true);
        }
        let Expression::BinaryExpression(binary) = part.without_parentheses() else {
            return None;
        };
        let negate = match binary.operator {
            BinaryOperator::StrictEquality | BinaryOperator::Equality => false,
            BinaryOperator::StrictInequality | BinaryOperator::Inequality => true,
            _ => return None,
        };
        equality(&binary.left, &binary.right, self).map(|equal| equal != negate)
    }
}

impl Leaves for Lax<'_, '_, '_, '_> {
    fn leaf(&self, id: &IdentifierReference<'_>) -> Option<(SymbolId, Val)> {
        let symbol = resolved(id, self.scoping)?;
        self.computed.get(&symbol).map(|seed| (symbol, seed.value))
    }

    fn is_imul(&self, _callee: &Expression<'_>) -> bool {
        false
    }

    fn lax(&self) -> bool {
        true
    }

    fn decided(&self, test: &Expression<'_>) -> Option<bool> {
        if let Some(known) = self.roots.decide(test, self.seed, self.scoping) {
            return Some(known);
        }
        if !self.sealed {
            return None;
        }
        sealed::all_of(test, &|part| self.sealed_part(part))
    }

    fn member(&self, expr: &Expression<'_>) -> Option<Val> {
        if !self.sealed || !self.roots.typed_hold(self.scoping) {
            return None;
        }
        let word = sealed::latch_read(expr, self.latches, self.scoping)?;
        Some(Val::uint32(Bits::exact(word)))
    }
}

#[derive(Clone)]
struct Found {
    computed: FxHashMap<SymbolId, Seed>,
    assumed: Vec<&'static Reliance>,
    refused: bool,
    empty: bool,
}

fn seeds<'p>(
    program: &'p Program<'p>,
    scoping: &Scoping,
    unsafe_lane: bool,
    assume: bool,
) -> Found {
    let local = top_level_is_local(program);
    let mut scan = SeedScan {
        roots: checksum::Roots::of(program, unsafe_lane, assume),
        scoping,
        alone: !other_inputs_given(),
        local,
        sealing: local && !has_module_syntax(program),
        computed: FxHashMap::default(),
        latches: sealed::Latches::default(),
        digests: FxHashSet::default(),
    };
    let mut sealed = scan.sealing;
    for statement in &program.body {
        if let Statement::VariableDeclaration(declaration) = statement {
            scan.declaration(declaration, sealed);
        }
        sealed = sealed && sealed::statement_is_sealed(statement, &scan.digests, scoping);
    }
    Found {
        empty: scan.computed.is_empty(),
        assumed: scan.roots.assumed(),
        refused: scan.roots.refused(),
        computed: scan.computed,
    }
}

fn dead_with(
    program: &Program<'_>,
    scoping: &Scoping,
    known: (&FxHashMap<SymbolId, Val>, &FxHashMap<SymbolId, Val>),
    plain: &identity::PlainSymbols,
    local: bool,
) -> FxHashSet<Span> {
    let (approved, maybe) = known;
    let decide = |expr: &Expression<'_>| decide_strict(expr, approved, maybe, scoping, true);
    guards::dead_regions(program, scoping, &decide, plain, local)
}

fn maybe_of(
    computed: &FxHashMap<SymbolId, Seed>,
    approved: &FxHashMap<SymbolId, Val>,
) -> FxHashMap<SymbolId, Val> {
    computed
        .iter()
        .filter(|(symbol, seed)| seed.var && seed.closed && !approved.contains_key(symbol))
        .map(|(symbol, seed)| (*symbol, seed.value))
        .collect()
}

fn relied_hold(
    program: &Program<'_>,
    cx: &Cx<'_, '_>,
    relied: &[&'static Reliance],
    dead: &FxHashSet<Span>,
) -> bool {
    let scoping = cx.scoping();
    let lane = cx.opts.unsafe_transforms;
    let module = has_module_syntax(program);
    relied.iter().all(|reliance| {
        !patched_by_another_input(reliance)
            && scan_outside(program, scoping, module, reliance, dead).allows_safe(lane)
    })
}

struct SeedScan<'p, 's> {
    roots: checksum::Roots<'p>,
    scoping: &'s Scoping,
    alone: bool,
    local: bool,
    sealing: bool,
    computed: FxHashMap<SymbolId, Seed>,
    latches: sealed::Latches,
    digests: FxHashSet<SymbolId>,
}

impl SeedScan<'_, '_> {
    fn declaration(&mut self, declaration: &VariableDeclaration<'_>, sealed: bool) {
        let kind_allowed = match declaration.kind {
            VariableDeclarationKind::Const => true,
            VariableDeclarationKind::Var => self.local,
            _ => false,
        };
        if !kind_allowed {
            return;
        }
        for declarator in &declaration.declarations {
            let (Some(id), Some(init)) = (declarator.id.get_binding_identifier(), &declarator.init)
            else {
                continue;
            };
            let Some(symbol) = id.symbol_id.get() else {
                continue;
            };
            if written(symbol, self.scoping) || self.digest(symbol, init, declaration.span) {
                continue;
            }
            if is_seed_shape(init, self.local) {
                self.seed(symbol, init, declaration, sealed);
            }
        }
        if self.sealing {
            sealed::record_latches(declaration, self.scoping, &mut self.latches);
        }
    }

    fn digest(&mut self, symbol: SymbolId, init: &Expression<'_>, statement: Span) -> bool {
        if !self.sealing {
            return false;
        }
        let Some(value) = self
            .roots
            .digest(init, statement, self.scoping)
            .and_then(Val::from_number)
        else {
            return false;
        };
        let seed = Seed {
            statement,
            value,
            inert: false,
            closed: false,
            var: false,
        };
        self.computed.insert(symbol, seed);
        self.digests.insert(symbol);
        true
    }

    fn seed(
        &mut self,
        symbol: SymbolId,
        init: &Expression<'_>,
        declaration: &VariableDeclaration<'_>,
        sealed: bool,
    ) {
        let lax = Lax {
            computed: &self.computed,
            scoping: self.scoping,
            roots: &self.roots,
            seed: declaration.span,
            latches: &self.latches,
            sealed,
        };
        let Some(value) = abstract_value(init, &lax).filter(|value| value.bits.known() != 0) else {
            return;
        };
        let inits = || {
            declaration
                .declarations
                .iter()
                .map(|declarator| declarator.init.as_ref())
        };
        let inert = self.alone
            && inits().all(|init| init.is_some_and(|init| inert::runs_no_code(init, &lax)));
        let functions = |symbol: SymbolId| self.roots.is_top_function(symbol);
        let closed = inits().all(|init| {
            init.is_some_and(|init| {
                is_closed_number(init, 0)
                    || (sealed && sealed::quiet(init, &self.latches, &functions, self.scoping))
            })
        });
        let seed = Seed {
            statement: declaration.span,
            value,
            inert,
            closed,
            var: declaration.kind == VariableDeclarationKind::Var,
        };
        self.computed.insert(symbol, seed);
    }
}

fn written(symbol: SymbolId, scoping: &Scoping) -> bool {
    let flags = scoping.symbol_flags(symbol);
    !(flags.is_const_variable() || flags.is_function_scoped_declaration())
        || !scoping.symbol_redeclarations(symbol).is_empty()
        || scoping
            .get_resolved_references(symbol)
            .any(|reference| reference.is_write())
}

fn is_seed_shape(init: &Expression<'_>, local: bool) -> bool {
    match init.without_parentheses() {
        Expression::NumericLiteral(_) => local,
        Expression::BinaryExpression(binary) => matches!(
            binary.operator,
            BinaryOperator::BitwiseAnd
                | BinaryOperator::BitwiseOR
                | BinaryOperator::BitwiseXOR
                | BinaryOperator::ShiftLeft
                | BinaryOperator::ShiftRight
                | BinaryOperator::ShiftRightZeroFill
        ),
        Expression::UnaryExpression(unary) => unary.operator == UnaryOperator::BitwiseNot,
        _ => false,
    }
}

fn is_candidate(expr: &Expression<'_>, scoping: &Scoping, local: bool) -> bool {
    match expr.without_parentheses() {
        Expression::UnaryExpression(unary) if unary.operator == UnaryOperator::LogicalNot => {
            is_candidate(&unary.argument, scoping, local)
        }
        Expression::BinaryExpression(binary) => {
            let mut reads = Vec::new();
            let imul = const_reads(&binary.left, scoping, local, 0, &mut reads)
                | const_reads(&binary.right, scoping, local, 0, &mut reads);
            imul || reads
                .iter()
                .enumerate()
                .any(|(index, symbol)| reads[..index].contains(symbol))
        }
        _ => false,
    }
}

fn const_reads(
    expr: &Expression<'_>,
    scoping: &Scoping,
    local: bool,
    depth: u32,
    reads: &mut Vec<SymbolId>,
) -> bool {
    if depth > 8 {
        return false;
    }
    match expr.without_parentheses() {
        Expression::Identifier(id) => {
            if let Some(symbol) = resolved(id, scoping)
                .filter(|symbol| depth > 0 && is_counter_like(scoping, *symbol, local))
            {
                reads.push(symbol);
            }
            false
        }
        Expression::BinaryExpression(binary) if is_int_operator(binary.operator) => {
            const_reads(&binary.left, scoping, local, depth + 1, reads)
                | const_reads(&binary.right, scoping, local, depth + 1, reads)
        }
        Expression::UnaryExpression(unary)
            if matches!(
                unary.operator,
                UnaryOperator::BitwiseNot | UnaryOperator::UnaryNegation
            ) =>
        {
            const_reads(&unary.argument, scoping, local, depth + 1, reads)
        }
        Expression::CallExpression(call) if is_global_math_imul(&call.callee, scoping) => {
            let before = reads.len();
            for argument in call
                .arguments
                .iter()
                .filter_map(|argument| argument.as_expression())
            {
                const_reads(argument, scoping, local, depth + 1, reads);
            }
            reads.len() > before
        }
        _ => false,
    }
}

fn is_closed_number(expr: &Expression<'_>, depth: u32) -> bool {
    if depth > 8 {
        return false;
    }
    match expr.without_parentheses() {
        Expression::NumericLiteral(_) => true,
        Expression::BinaryExpression(binary) => {
            (is_int_operator(binary.operator) || binary.operator == BinaryOperator::Multiplication)
                && is_closed_number(&binary.left, depth + 1)
                && is_closed_number(&binary.right, depth + 1)
        }
        Expression::UnaryExpression(unary) => {
            matches!(
                unary.operator,
                UnaryOperator::BitwiseNot | UnaryOperator::UnaryNegation | UnaryOperator::UnaryPlus
            ) && is_closed_number(&unary.argument, depth + 1)
        }
        _ => false,
    }
}

fn is_counter_like(scoping: &Scoping, symbol: SymbolId, local: bool) -> bool {
    let flags = scoping.symbol_flags(symbol);
    flags.is_const_variable() || (local && flags.is_function_scoped_declaration())
}

fn is_int_operator(operator: BinaryOperator) -> bool {
    matches!(
        operator,
        BinaryOperator::BitwiseAnd
            | BinaryOperator::BitwiseOR
            | BinaryOperator::BitwiseXOR
            | BinaryOperator::ShiftLeft
            | BinaryOperator::ShiftRight
            | BinaryOperator::ShiftRightZeroFill
            | BinaryOperator::Addition
            | BinaryOperator::Subtraction
    )
}
