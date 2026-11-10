use oxc_allocator::GetAllocator;
use oxc_allocator::Vec as AVec;
use oxc_ast::ast::*;
use oxc_ast_visit::{walk_js, VisitJs};
use oxc_semantic::Scoping;
use oxc_span::{GetSpan, Span};
use oxc_syntax::operator::UnaryOperator;
use oxc_syntax::symbol::SymbolId;
use rustc_hash::FxHashSet;
use untwist_core::ast_util::edit::{ExprEdits, StmtEdits};
use untwist_core::ast_util::inline::{inline_function_call, param_names, returned_expression};
use untwist_core::ast_util::literal::is_undefined_like;
use untwist_core::ast_util::matcher::{call_parts, var_function_or_declaration};
use untwist_core::ast_util::prop::identifier_name;
use untwist_core::pass::Cx;
use untwist_core::semantic::{is_constant_binding, AnnexBWrites};

use crate::call_checks::{call_checks_apply, is_plain_function};
use crate::init_order::try_scan_init_order;
use crate::mutants::wrapper_guards_dropped;
use crate::notes;
use crate::primitive_names::PrimitiveNames;
use crate::probe::{expression_contains, holds_own_scope};
use crate::proof::refuse_annex_b_bindings;
use crate::stamp::{declaration_symbol, decoder_index, is_machinery_name};
use crate::units::MachineryDepth;
use crate::wrapper_args::SiteView;
use crate::wrapper_chain::{drop_broken_chains, promote_forwarders, Candidate, Chain, Fits};
use crate::wrapper_effects::{inline_keeping_effects, inline_local_writes};
use crate::wrapper_sites::{called_expression, collect_sites, CallSite, FunctionsAt, Sites};

pub const WRAPPER_ROUND_CAP: usize = 16;

pub const WRAPPER_EFFECT_CHECKS: &str = "inline-decoder-wrappers/effect-checks";

pub(crate) struct WrapperShape {
    pub(crate) statement: Span,
    pub(crate) symbol: SymbolId,
    pub(crate) function: Span,
    pub(crate) bound_by_var: bool,
    pub(crate) forwards_to: Option<SymbolId>,
}

pub(crate) fn uses_any_param(arguments: &AVec<'_, Argument<'_>>, names: &[&str]) -> bool {
    arguments.iter().any(|argument| {
        argument.as_expression().is_some_and(|expr| {
            expression_contains(expr, |inner| {
                identifier_name(inner).is_some_and(|name| names.contains(&name))
            })
        })
    })
}

fn wrapper_shape<'k, 'a>(
    stmt: &'k Statement<'a>,
    target: impl Fn(&IdentifierReference<'a>) -> Option<Option<SymbolId>>,
) -> Option<(WrapperShape, &'k Function<'a>)> {
    let declaration = var_function_or_declaration(stmt)?;
    if decoder_index(declaration.name).is_some() {
        return None;
    }
    let names = param_names(declaration.function)?;
    if names.is_empty() {
        return None;
    }
    let returned = returned_expression(declaration.function)?;
    let (callee, arguments) = call_parts(returned)?;
    if arguments.is_empty() {
        return None;
    }
    let Expression::Identifier(callee) = callee else {
        return None;
    };
    let forwards_to = target(callee)?;
    if !uses_any_param(arguments, &names) {
        return None;
    }
    let shape = WrapperShape {
        statement: stmt.span(),
        symbol: declaration_symbol(stmt)?,
        function: declaration.function.span,
        bound_by_var: matches!(stmt, Statement::VariableDeclaration(_)),
        forwards_to,
    };
    Some((shape, declaration.function))
}

pub(crate) fn returns_nested_function(function: &Function<'_>) -> bool {
    returned_expression(function)
        .is_some_and(|returned| expression_contains(returned, holds_own_scope))
}

pub(crate) fn is_substitutable_argument(expr: &Expression<'_>) -> bool {
    let inner = expr.without_parentheses();
    if is_undefined_like(inner) {
        return true;
    }
    match inner {
        Expression::Identifier(_)
        | Expression::NumericLiteral(_)
        | Expression::StringLiteral(_)
        | Expression::BooleanLiteral(_)
        | Expression::NullLiteral(_) => true,
        Expression::UnaryExpression(unary) => {
            matches!(
                unary.operator,
                UnaryOperator::UnaryNegation
                    | UnaryOperator::UnaryPlus
                    | UnaryOperator::LogicalNot
                    | UnaryOperator::BitwiseNot
            ) && is_substitutable_argument(&unary.argument)
        }
        Expression::BinaryExpression(binary) => {
            is_substitutable_argument(&binary.left) && is_substitutable_argument(&binary.right)
        }
        other => called_expression(other).is_some_and(is_substitutable_decoder_call),
    }
}

fn is_substitutable_decoder_call(call: &CallExpression<'_>) -> bool {
    if identifier_name(&call.callee)
        .and_then(decoder_index)
        .is_none()
    {
        return false;
    }
    call.arguments.iter().all(|argument| {
        argument
            .as_expression()
            .is_some_and(is_substitutable_argument)
    })
}

pub(crate) fn arguments_are_substitutable(arguments: &AVec<'_, Argument<'_>>) -> bool {
    arguments.iter().all(|argument| {
        argument
            .as_expression()
            .is_some_and(is_substitutable_argument)
    })
}

struct BodyScan<'s> {
    scoping: &'s Scoping,
    params: FxHashSet<SymbolId>,
    forwards_to: Option<SymbolId>,
    forwards: usize,
    escapes: bool,
}

impl<'a> VisitJs<'a> for BodyScan<'_> {
    fn visit_identifier_reference(&mut self, it: &IdentifierReference<'a>) {
        if is_machinery_name(it.name.as_str()) {
            return;
        }
        let symbol = it
            .reference_id
            .get()
            .and_then(|reference| self.scoping.get_reference(reference).symbol_id());
        if symbol.is_some() && symbol == self.forwards_to {
            self.forwards += 1;
            return;
        }
        if !symbol.is_some_and(|symbol| self.params.contains(&symbol)) {
            self.escapes = true;
        }
    }

    fn visit_expression(&mut self, it: &Expression<'a>) {
        if matches!(
            it,
            Expression::ThisExpression(_) | Expression::NewTarget(_) | Expression::Super(_)
        ) {
            self.escapes = true;
            return;
        }
        walk_js::walk_expression(self, it);
    }
}

pub(crate) fn body_is_self_contained(
    function: &Function<'_>,
    scoping: &Scoping,
    forwards_to: Option<SymbolId>,
) -> bool {
    let Some(returned) = returned_expression(function) else {
        return false;
    };
    let mut params = FxHashSet::default();
    for item in &function.params.items {
        let BindingPattern::BindingIdentifier(id) = &item.pattern else {
            return false;
        };
        let Some(symbol) = id.symbol_id.get() else {
            return false;
        };
        params.insert(symbol);
    }
    let mut scan = BodyScan {
        scoping,
        params,
        forwards_to,
        forwards: 0,
        escapes: false,
    };
    scan.visit_expression(returned);
    !scan.escapes && scan.forwards == usize::from(forwards_to.is_some())
}

pub(crate) fn binding_is_stable(scoping: &Scoping, symbol: SymbolId) -> bool {
    if wrapper_guards_dropped() {
        return true;
    }
    if !is_constant_binding(scoping, symbol) {
        return false;
    }
    if scoping
        .scope_flags(scoping.symbol_scope_id(symbol))
        .is_var()
    {
        return true;
    }
    let name = scoping.symbol_name(symbol);
    !scoping.root_unresolved_references().contains_key(name)
        && scoping
            .symbol_ids()
            .filter(|other| scoping.symbol_name(*other) == name)
            .count()
            == 1
}

pub(crate) fn arguments_hold_a_scope(arguments: &AVec<'_, Argument<'_>>) -> bool {
    arguments.iter().any(|argument| {
        argument
            .as_expression()
            .is_some_and(|expr| expression_contains(expr, holds_own_scope))
    })
}

struct WrapperSearch<'p, 's> {
    scoping: &'s Scoping,
    units: MachineryDepth,
    checked: bool,
    found: Vec<WrapperShape>,
    forwarders: Vec<Candidate<'p>>,
    rejected: Vec<(Span, &'static str)>,
}

impl<'p> WrapperSearch<'p, '_> {
    fn target(&self, callee: &IdentifierReference<'_>) -> Option<Option<SymbolId>> {
        let name = callee.name.as_str();
        if decoder_index(name).is_some() {
            return Some(None);
        }
        if !self.checked || is_machinery_name(name) {
            return None;
        }
        let reference = callee.reference_id.get()?;
        self.scoping.get_reference(reference).symbol_id().map(Some)
    }

    fn refusal(&self, shape: &WrapperShape, function: &Function<'_>) -> Option<&'static str> {
        if returns_nested_function(function) {
            return Some(notes::WRAPPER_NESTED_FUNCTION);
        }
        if !binding_is_stable(self.scoping, shape.symbol) {
            return Some(notes::WRAPPER_BINDING_UNSTABLE);
        }
        if !body_is_self_contained(function, self.scoping, None) {
            return Some(notes::WRAPPER_BODY_NOT_SELF_CONTAINED);
        }
        if self.checked && !is_plain_function(function) {
            return Some(notes::WRAPPER_SIGNATURE_NOT_PLAIN);
        }
        None
    }

    fn consider(&mut self, it: &Statement<'p>) {
        let statement: &'p Statement<'p> = self.alloc(it);
        let Some((shape, function)) = wrapper_shape(statement, |callee| self.target(callee)) else {
            return;
        };
        if shape.forwards_to.is_some() {
            self.forwarders.push(Candidate { shape, function });
            return;
        }
        match self.refusal(&shape, function) {
            Some(reason) => self.rejected.push((shape.statement, reason)),
            None => self.found.push(shape),
        }
    }
}

impl<'p> VisitJs<'p> for WrapperSearch<'p, '_> {
    fn visit_statement(&mut self, it: &Statement<'p>) {
        let entered = self.units.enter(it);
        if !self.units.inside() {
            self.consider(it);
        }
        walk_js::walk_statement(self, it);
        self.units.leave(entered);
    }
}

#[derive(Default)]
struct RoundPlan<'a> {
    edits: ExprEdits<'a>,
    removals: StmtEdits<'a>,
    notes: Vec<(String, Option<Span>)>,
    claimed: FxHashSet<usize>,
    blocked: FxHashSet<usize>,
    removed: FxHashSet<SymbolId>,
    kept: Vec<SymbolId>,
}

impl RoundPlan<'_> {
    fn defers(&self, id: usize, site: &CallSite<'_>, checked: bool) -> bool {
        (!checked && site.body_of.is_some())
            || self.blocked.contains(&id)
            || site
                .ancestors
                .iter()
                .any(|outer| self.claimed.contains(outer))
    }

    fn claim(&mut self, id: usize, site: &CallSite<'_>) {
        self.claimed.insert(id);
        self.blocked.insert(id);
        self.blocked.extend(site.ancestors.iter().copied());
    }

    fn keep(&mut self, span: Span, reason: &str) {
        self.notes.push((reason.to_string(), Some(span)));
    }
}

#[derive(Default)]
struct SiteCounts {
    rewritten: usize,
    deferred: usize,
}

enum Outcome {
    Rewritten,
    Deferred,
    Kept(String),
}

struct Wrapper<'r, 'p> {
    shape: &'r WrapperShape,
    function: &'r Function<'p>,
}

struct Round<'r, 'p, 'a, 'x> {
    sites: &'r Sites<'p>,
    chain: &'r Chain<'p>,
    view: SiteView<'r>,
    checked: bool,
    cx: &'r Cx<'a, 'x>,
}

type Attempt<'a> = (Option<Expression<'a>>, fn() -> String);

fn site_kept() -> String {
    notes::WRAPPER_SITE_KEPT.to_string()
}

impl<'p, 'a> Round<'_, 'p, 'a, '_> {
    fn plain(&self, wrapper: &Wrapper<'_, 'p>, site: &CallSite<'p>) -> Attempt<'a> {
        if !self.checked {
            let inlined = inline_function_call(wrapper.function, site.arguments, self.cx);
            return (inlined, site_kept);
        }
        let fits: Fits<'_> = &|uses, arguments, depth| {
            let in_parameters = depth == 0 && site.in_parameters;
            self.view.arguments_fit(uses, arguments, in_parameters)
        };
        let inlined = self
            .chain
            .inline_through(site.wrapper, site.arguments, fits, self.cx);
        (inlined, notes::wrapper_site_impure_argument)
    }

    fn keeping_effects(&self, wrapper: &Wrapper<'_, 'p>, site: &CallSite<'p>) -> Attempt<'a> {
        let effect_checks = !self.cx.opts.rule_disabled(WRAPPER_EFFECT_CHECKS);
        let inlined = if self.checked && effect_checks {
            inline_local_writes(
                self.chain,
                &self.view,
                site.wrapper,
                site.arguments,
                site.span,
                self.cx,
            )
        } else if self.checked && wrapper.shape.forwards_to.is_some() {
            None
        } else {
            let function = wrapper.function;
            inline_keeping_effects(function, site.arguments, site.span, effect_checks, self.cx)
        };
        (inlined, notes::wrapper_site_impure_argument)
    }

    fn rewrite_site(
        &self,
        wrapper: &Wrapper<'_, 'p>,
        id: usize,
        site: &CallSite<'p>,
        plan: &mut RoundPlan<'a>,
    ) -> Outcome {
        let body_is_gone = site
            .body_of
            .is_some_and(|body| plan.removed.contains(&body));
        if self.checked && body_is_gone {
            return Outcome::Rewritten;
        }
        if plan.defers(id, site, self.checked) {
            return Outcome::Deferred;
        }
        if arguments_hold_a_scope(site.arguments) {
            return Outcome::Kept(notes::wrapper_site_scoped_argument());
        }
        let (inlined, refusal) = if arguments_are_substitutable(site.arguments) {
            self.plain(wrapper, site)
        } else if self.sites.holds_another_site(id) {
            return Outcome::Deferred;
        } else {
            self.keeping_effects(wrapper, site)
        };
        if inlined.is_some_and(|inlined| plan.edits.insert(site.span, inlined)) {
            plan.claim(id, site);
            return Outcome::Rewritten;
        }
        Outcome::Kept(refusal())
    }

    fn rewrite_sites(&self, wrapper: &Wrapper<'_, 'p>, plan: &mut RoundPlan<'a>) -> SiteCounts {
        let mut counts = SiteCounts::default();
        for id in self.sites.of(wrapper.shape.symbol).iter().copied() {
            let Some(site) = self.sites.all.get(id) else {
                continue;
            };
            match self.rewrite_site(wrapper, id, site, plan) {
                Outcome::Rewritten => counts.rewritten += 1,
                Outcome::Deferred => {
                    counts.deferred += 1;
                    plan.keep(site.span, notes::WRAPPER_SITE_KEPT);
                }
                Outcome::Kept(reason) => plan.keep(site.span, &reason),
            }
        }
        counts
    }

    fn rewrite_wrapper(
        &self,
        functions: &FunctionsAt<'p>,
        shape: &WrapperShape,
        plan: &mut RoundPlan<'a>,
        poisoned: &mut FxHashSet<SymbolId>,
    ) {
        let Some(function) = functions.get(shape.function) else {
            plan.notes.push((
                notes::WRAPPER_BODY_MISSING.to_string(),
                Some(shape.function),
            ));
            return;
        };
        let counts = self.rewrite_sites(&Wrapper { shape, function }, plan);
        if counts.deferred > 0 {
            poisoned.insert(shape.symbol);
        }
        let references = self
            .cx
            .scoping()
            .get_resolved_reference_ids(shape.symbol)
            .len();
        settle_wrapper(shape, counts.rewritten, references, plan, poisoned);
    }
}

fn settle_wrapper(
    shape: &WrapperShape,
    rewritten: usize,
    references: usize,
    plan: &mut RoundPlan<'_>,
    poisoned: &FxHashSet<SymbolId>,
) {
    if poisoned.contains(&shape.symbol) {
        plan.notes
            .push((notes::WRAPPER_DEFERRED.to_string(), Some(shape.statement)));
        return;
    }
    if rewritten == references {
        if plan.removals.remove(shape.statement) {
            plan.removed.insert(shape.symbol);
        }
        return;
    }
    plan.kept.push(shape.symbol);
    plan.notes.push((
        notes::wrapper_partial(rewritten, references),
        Some(shape.statement),
    ));
}

type Rejected = Vec<(Span, &'static str)>;

fn refuse_var_wrappers_read_early(
    program: &Program<'_>,
    scoping: &Scoping,
    found: &mut Vec<WrapperShape>,
    rejected: &mut Rejected,
) {
    if wrapper_guards_dropped() || !found.iter().any(|shape| shape.bound_by_var) {
        return;
    }
    let order = try_scan_init_order(program, scoping);
    let ignored = FxHashSet::default();
    let (kept, early): (Vec<WrapperShape>, Vec<WrapperShape>) =
        std::mem::take(found).into_iter().partition(|shape| {
            !shape.bound_by_var
                || order.as_ref().is_some_and(|order| {
                    order.declaration_slot(shape.statement).is_some_and(|slot| {
                        order.symbol_is_initialised_before_use(slot, shape.symbol, &ignored)
                    })
                })
        });
    *found = kept;
    rejected.extend(
        early
            .into_iter()
            .map(|shape| (shape.statement, notes::WRAPPER_READ_BEFORE_DECLARATION)),
    );
}

fn refuse_annex_b_wrappers(
    program: &Program<'_>,
    scoping: &Scoping,
    found: &mut Vec<WrapperShape>,
    rejected: &mut Rejected,
) {
    if found.is_empty() || wrapper_guards_dropped() || crate::mutants::annex_b_writes_trusted() {
        return;
    }
    let writes = AnnexBWrites::of(program, scoping);
    if writes.is_empty() {
        return;
    }
    let (written, kept): (Vec<_>, Vec<_>) = std::mem::take(found)
        .into_iter()
        .partition(|shape| writes.may_write(shape.symbol));
    *found = kept;
    rejected.extend(
        written
            .into_iter()
            .map(|shape| (shape.statement, notes::WRAPPER_BINDING_UNSTABLE)),
    );
}

fn find_wrappers(
    program: &Program<'_>,
    cx: &mut Cx<'_, '_>,
) -> Option<(Vec<WrapperShape>, Rejected)> {
    let checked = call_checks_apply(cx.opts);
    let mut search = WrapperSearch {
        scoping: cx.scoping(),
        units: MachineryDepth::every_unit(),
        checked,
        found: Vec::new(),
        forwarders: Vec::new(),
        rejected: Vec::new(),
    };
    search.visit_program(program);
    let WrapperSearch {
        mut found,
        forwarders,
        mut rejected,
        ..
    } = search;
    if found.is_empty() && rejected.is_empty() {
        return None;
    }
    promote_forwarders(forwarders, &mut found, cx.scoping());
    refuse_annex_b_wrappers(program, cx.scoping(), &mut found, &mut rejected);
    refuse_var_wrappers_read_early(program, cx.scoping(), &mut found, &mut rejected);
    if checked {
        let site = |shape: &WrapperShape| (shape.statement, [shape.symbol]);
        refuse_annex_b_bindings(&mut found, site, program, cx);
        drop_broken_chains(&mut found);
    }
    Some((found, rejected))
}

fn plan_round<'p, 'a>(
    program: &Program<'p>,
    cx: &mut Cx<'a, '_>,
    poisoned: &mut FxHashSet<SymbolId>,
) -> Option<RoundPlan<'a>> {
    let (found, rejected) = find_wrappers(program, cx)?;
    let cx = &*cx;
    let checked = call_checks_apply(cx.opts);
    let sites = collect_sites(program, cx.scoping(), &found);
    let primitives = if checked {
        PrimitiveNames::collect(program, cx.scoping())
    } else {
        PrimitiveNames::default()
    };
    let functions = FunctionsAt::search(program, &found);
    let mut chain = Chain::default();
    for shape in found.iter().filter(|_| checked) {
        if let Some(function) = functions.get(shape.function) {
            chain.add(shape, function);
        }
    }
    let round = Round {
        sites: &sites,
        chain: &chain,
        view: SiteView::new(cx.scoping(), &primitives),
        checked,
        cx,
    };
    let mut plan = RoundPlan {
        notes: rejected
            .iter()
            .map(|(span, reason)| (reason.to_string(), Some(*span)))
            .collect(),
        ..RoundPlan::default()
    };
    let forwarders = found
        .iter()
        .rev()
        .filter(|shape| shape.forwards_to.is_some());
    let bases = found.iter().filter(|shape| shape.forwards_to.is_none());
    for shape in forwarders.chain(bases) {
        round.rewrite_wrapper(&functions, shape, &mut plan, poisoned);
    }
    Some(plan)
}

pub struct WrapperRounds {
    pub changes: usize,
    pub kept: Vec<SymbolId>,
}

pub fn inline_function_wrappers<'a>(
    program: &mut Program<'a>,
    cx: &mut Cx<'a, '_>,
) -> WrapperRounds {
    let mut rounds = WrapperRounds {
        changes: 0,
        kept: Vec::new(),
    };
    let mut poisoned: FxHashSet<SymbolId> = FxHashSet::default();
    for round in 0..WRAPPER_ROUND_CAP {
        let Some(plan) = plan_round(program, cx, &mut poisoned) else {
            return rounds;
        };
        let RoundPlan {
            edits,
            removals,
            notes,
            kept,
            ..
        } = plan;
        for (message, span) in notes {
            cx.note(message, span);
        }
        rounds.kept = kept;
        let applied = edits.apply(program, cx.allocator(), cx.scoping_mut())
            + removals.apply(program, cx.allocator(), cx.scoping_mut());
        rounds.changes += applied;
        if applied == 0 {
            return rounds;
        }
        if round + 1 == WRAPPER_ROUND_CAP {
            cx.note(notes::wrapper_rounds(WRAPPER_ROUND_CAP), None);
        }
    }
    rounds
}
