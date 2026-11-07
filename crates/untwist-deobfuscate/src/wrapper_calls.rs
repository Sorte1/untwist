use oxc_allocator::GetAllocator;
use oxc_allocator::Vec as AVec;
use oxc_ast::ast::*;
use oxc_ast_visit::{walk_js, VisitJs};
use oxc_semantic::Scoping;
use oxc_span::{GetSpan, Span};
use oxc_syntax::operator::UnaryOperator;
use oxc_syntax::scope::ScopeFlags;
use oxc_syntax::symbol::SymbolId;
use rustc_hash::{FxHashMap, FxHashSet};
use untwist_core::ast_util::edit::{ExprEdits, StmtEdits};
use untwist_core::ast_util::inline::{inline_function_call, param_names, returned_expression};
use untwist_core::ast_util::literal::is_undefined_like;
use untwist_core::ast_util::matcher::{call_parts, var_function_or_declaration};
use untwist_core::ast_util::prop::identifier_name;
use untwist_core::pass::Cx;
use untwist_core::semantic::is_constant_binding;

use crate::init_order::try_scan_init_order;
use crate::mutants::wrapper_guards_dropped;
use crate::notes;
use crate::probe::{expression_contains, holds_own_scope};
use crate::stamp::{declaration_symbol, decoder_index, is_machinery_name};
use crate::units::MachineryDepth;
use crate::wrapper_effects::inline_keeping_effects;

pub const WRAPPER_ROUND_CAP: usize = 16;

pub const WRAPPER_EFFECT_CHECKS: &str = "inline-decoder-wrappers/effect-checks";

struct WrapperShape {
    statement: Span,
    symbol: SymbolId,
    function: Span,
    bound_by_var: bool,
}

fn uses_any_param(arguments: &AVec<'_, Argument<'_>>, names: &[&str]) -> bool {
    arguments.iter().any(|argument| {
        argument.as_expression().is_some_and(|expr| {
            expression_contains(expr, |inner| {
                identifier_name(inner).is_some_and(|name| names.contains(&name))
            })
        })
    })
}

fn is_wrapper(stmt: &Statement<'_>) -> Option<WrapperShape> {
    let declaration = var_function_or_declaration(stmt)?;
    if decoder_index(declaration.name).is_some() {
        return None;
    }
    let names = param_names(declaration.function)?;
    if names.len() < 2 {
        return None;
    }
    let returned = returned_expression(declaration.function)?;
    let (callee, arguments) = call_parts(returned)?;
    if arguments.len() < 2 {
        return None;
    }
    decoder_index(identifier_name(callee)?)?;
    if !uses_any_param(arguments, &names) {
        return None;
    }
    Some(WrapperShape {
        statement: stmt.span(),
        symbol: declaration_symbol(stmt)?,
        function: declaration.function.span,
        bound_by_var: matches!(stmt, Statement::VariableDeclaration(_)),
    })
}

fn returns_nested_function(stmt: &Statement<'_>) -> bool {
    let Some(declaration) = var_function_or_declaration(stmt) else {
        return false;
    };
    let Some(returned) = returned_expression(declaration.function) else {
        return false;
    };
    expression_contains(returned, holds_own_scope)
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

fn body_is_self_contained(function: &Function<'_>, scoping: &Scoping) -> bool {
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
        escapes: false,
    };
    scan.visit_expression(returned);
    !scan.escapes
}

fn binding_is_stable(scoping: &Scoping, symbol: SymbolId) -> bool {
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

fn arguments_hold_a_scope(arguments: &AVec<'_, Argument<'_>>) -> bool {
    arguments.iter().any(|argument| {
        argument
            .as_expression()
            .is_some_and(|expr| expression_contains(expr, holds_own_scope))
    })
}

struct WrapperSearch<'s> {
    scoping: &'s Scoping,
    units: MachineryDepth,
    found: Vec<WrapperShape>,
    rejected: Vec<(Span, &'static str)>,
}

impl<'a> VisitJs<'a> for WrapperSearch<'_> {
    fn visit_statement(&mut self, it: &Statement<'a>) {
        let entered = self.units.enter(it);
        if !self.units.inside() {
            if let Some(shape) = is_wrapper(it) {
                if returns_nested_function(it) {
                    self.rejected
                        .push((shape.statement, notes::WRAPPER_NESTED_FUNCTION));
                } else if !binding_is_stable(self.scoping, shape.symbol) {
                    self.rejected
                        .push((shape.statement, notes::WRAPPER_BINDING_UNSTABLE));
                } else if !var_function_or_declaration(it).is_some_and(|declaration| {
                    body_is_self_contained(declaration.function, self.scoping)
                }) {
                    self.rejected
                        .push((shape.statement, notes::WRAPPER_BODY_NOT_SELF_CONTAINED));
                } else {
                    self.found.push(shape);
                }
            }
        }
        walk_js::walk_statement(self, it);
        self.units.leave(entered);
    }
}

fn called_expression<'k, 'a>(expr: &'k Expression<'a>) -> Option<&'k CallExpression<'a>> {
    match expr {
        Expression::CallExpression(call) => Some(call),
        Expression::ChainExpression(chain) => match &chain.expression {
            ChainElement::CallExpression(call) => Some(call),
            _ => None,
        },
        _ => None,
    }
}

struct CallSite<'p> {
    wrapper: SymbolId,
    span: Span,
    arguments: &'p AVec<'p, Argument<'p>>,
    ancestors: Vec<usize>,
    in_a_wrapper_body: bool,
}

struct SiteSearch<'p, 's> {
    scoping: &'s Scoping,
    wrappers: FxHashSet<SymbolId>,
    units: MachineryDepth,
    machinery_functions: usize,
    bodies: usize,
    stack: Vec<usize>,
    found: Vec<CallSite<'p>>,
}

impl SiteSearch<'_, '_> {
    fn called_wrapper(&self, expr: &Expression<'_>) -> Option<SymbolId> {
        let call = called_expression(expr)?;
        let Expression::Identifier(id) = &call.callee else {
            return None;
        };
        let symbol = id
            .reference_id
            .get()
            .and_then(|reference| self.scoping.get_reference(reference).symbol_id())?;
        self.wrappers.contains(&symbol).then_some(symbol)
    }

    fn declares_a_wrapper(&self, stmt: &Statement<'_>) -> bool {
        declaration_symbol(stmt).is_some_and(|symbol| self.wrappers.contains(&symbol))
    }
}

impl<'p> VisitJs<'p> for SiteSearch<'p, '_> {
    fn visit_statement(&mut self, it: &Statement<'p>) {
        let entered = self.units.enter(it);
        let function = entered && var_function_or_declaration(it).is_some();
        let body = self.declares_a_wrapper(it);
        self.machinery_functions += usize::from(function);
        self.bodies += usize::from(body);
        walk_js::walk_statement(self, it);
        self.bodies -= usize::from(body);
        self.machinery_functions -= usize::from(function);
        self.units.leave(entered);
    }

    fn visit_expression(&mut self, it: &Expression<'p>) {
        let wrapper = if self.machinery_functions > 0 {
            None
        } else {
            self.called_wrapper(it)
        };
        let Some(wrapper) = wrapper else {
            walk_js::walk_expression(self, it);
            return;
        };
        let expression: &'p Expression<'p> = self.alloc(it);
        let Some(call) = called_expression(expression) else {
            walk_js::walk_expression(self, it);
            return;
        };
        let id = self.found.len();
        self.found.push(CallSite {
            wrapper,
            span: expression.span(),
            arguments: &call.arguments,
            ancestors: self.stack.clone(),
            in_a_wrapper_body: self.bodies > 0,
        });
        self.stack.push(id);
        walk_js::walk_expression(self, it);
        self.stack.pop();
    }
}

struct Sites<'p> {
    all: Vec<CallSite<'p>>,
    by_wrapper: FxHashMap<SymbolId, Vec<usize>>,
    nested: FxHashSet<usize>,
}

impl Sites<'_> {
    fn of(&self, wrapper: SymbolId) -> &[usize] {
        self.by_wrapper
            .get(&wrapper)
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }

    fn holds_another_site(&self, id: usize) -> bool {
        self.nested.contains(&id)
    }
}

fn collect_sites<'p>(view: &Program<'p>, scoping: &Scoping, shapes: &[WrapperShape]) -> Sites<'p> {
    let mut search = SiteSearch {
        scoping,
        wrappers: shapes.iter().map(|shape| shape.symbol).collect(),
        units: MachineryDepth::every_unit(),
        machinery_functions: 0,
        bodies: 0,
        stack: Vec::new(),
        found: Vec::new(),
    };
    search.visit_program(view);
    let mut by_wrapper: FxHashMap<SymbolId, Vec<usize>> = FxHashMap::default();
    let mut nested: FxHashSet<usize> = FxHashSet::default();
    for (id, site) in search.found.iter().enumerate() {
        by_wrapper.entry(site.wrapper).or_default().push(id);
        nested.extend(site.ancestors.iter().copied());
    }
    Sites {
        all: search.found,
        by_wrapper,
        nested,
    }
}

struct FunctionAt<'p> {
    span: Span,
    found: Option<&'p Function<'p>>,
}

impl<'p> VisitJs<'p> for FunctionAt<'p> {
    fn visit_function(&mut self, it: &Function<'p>, flags: ScopeFlags) {
        if it.span == self.span {
            self.found = Some(self.alloc(it));
            return;
        }
        walk_js::walk_function(self, it, flags);
    }
}

#[derive(Default)]
struct RoundPlan<'a> {
    edits: ExprEdits<'a>,
    removals: StmtEdits<'a>,
    notes: Vec<(String, Option<Span>)>,
    claimed: FxHashSet<usize>,
    blocked: FxHashSet<usize>,
    kept: Vec<SymbolId>,
}

impl RoundPlan<'_> {
    fn defers(&self, id: usize, site: &CallSite<'_>) -> bool {
        site.in_a_wrapper_body
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

fn rewrite_sites<'p, 'a>(
    function: &Function<'p>,
    sites: &Sites<'p>,
    ids: &[usize],
    cx: &Cx<'a, '_>,
    plan: &mut RoundPlan<'a>,
) -> SiteCounts {
    let mut counts = SiteCounts::default();
    for id in ids.iter().copied() {
        let Some(site) = sites.all.get(id) else {
            continue;
        };
        if plan.defers(id, site) {
            counts.deferred += 1;
            plan.keep(site.span, notes::WRAPPER_SITE_KEPT);
            continue;
        }
        if arguments_hold_a_scope(site.arguments) {
            plan.keep(site.span, &notes::wrapper_site_scoped_argument());
            continue;
        }
        if !arguments_are_substitutable(site.arguments) {
            if sites.holds_another_site(id) {
                counts.deferred += 1;
                plan.keep(site.span, notes::WRAPPER_SITE_KEPT);
                continue;
            }
            let checked = !cx.opts.rule_disabled(WRAPPER_EFFECT_CHECKS);
            let inlined = inline_keeping_effects(function, site.arguments, site.span, checked, cx);
            if inlined.is_some_and(|inlined| plan.edits.insert(site.span, inlined)) {
                plan.claim(id, site);
                counts.rewritten += 1;
            } else {
                plan.keep(site.span, &notes::wrapper_site_impure_argument());
            }
            continue;
        }
        let Some(inlined) = inline_function_call(function, site.arguments, cx) else {
            plan.keep(site.span, notes::WRAPPER_SITE_KEPT);
            continue;
        };
        if plan.edits.insert(site.span, inlined) {
            plan.claim(id, site);
            counts.rewritten += 1;
        } else {
            plan.keep(site.span, notes::WRAPPER_SITE_KEPT);
        }
    }
    counts
}

fn rewrite_wrapper<'p, 'a>(
    view: &Program<'p>,
    shape: &WrapperShape,
    sites: &Sites<'p>,
    cx: &Cx<'a, '_>,
    plan: &mut RoundPlan<'a>,
    poisoned: &mut FxHashSet<SymbolId>,
) {
    let mut function_at = FunctionAt {
        span: shape.function,
        found: None,
    };
    function_at.visit_program(view);
    let Some(function) = function_at.found else {
        plan.notes.push((
            notes::WRAPPER_BODY_MISSING.to_string(),
            Some(shape.function),
        ));
        return;
    };
    let counts = rewrite_sites(function, sites, sites.of(shape.symbol), cx, plan);
    if counts.deferred > 0 {
        poisoned.insert(shape.symbol);
    }
    let references = cx.scoping().get_resolved_reference_ids(shape.symbol).len();
    settle_wrapper(shape, counts.rewritten, references, plan, poisoned);
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
        plan.removals.remove(shape.statement);
        return;
    }
    plan.kept.push(shape.symbol);
    plan.notes.push((
        notes::wrapper_partial(rewritten, references),
        Some(shape.statement),
    ));
}

fn refuse_var_wrappers_read_early(
    program: &Program<'_>,
    scoping: &Scoping,
    search: &mut WrapperSearch<'_>,
) {
    if wrapper_guards_dropped() || !search.found.iter().any(|shape| shape.bound_by_var) {
        return;
    }
    let order = try_scan_init_order(program, scoping);
    let ignored = FxHashSet::default();
    let (kept, early): (Vec<WrapperShape>, Vec<WrapperShape>) = std::mem::take(&mut search.found)
        .into_iter()
        .partition(|shape| {
            !shape.bound_by_var
                || order.as_ref().is_some_and(|order| {
                    order.declaration_slot(shape.statement).is_some_and(|slot| {
                        order.symbol_is_initialised_before_use(slot, shape.symbol, &ignored)
                    })
                })
        });
    search.found = kept;
    search.rejected.extend(
        early
            .into_iter()
            .map(|shape| (shape.statement, notes::WRAPPER_READ_BEFORE_DECLARATION)),
    );
}

fn plan_round<'p, 'a>(
    program: &Program<'p>,
    cx: &Cx<'a, '_>,
    poisoned: &mut FxHashSet<SymbolId>,
) -> Option<RoundPlan<'a>> {
    let mut search = WrapperSearch {
        scoping: cx.scoping(),
        units: MachineryDepth::every_unit(),
        found: Vec::new(),
        rejected: Vec::new(),
    };
    search.visit_program(program);
    if search.found.is_empty() && search.rejected.is_empty() {
        return None;
    }
    refuse_var_wrappers_read_early(program, cx.scoping(), &mut search);
    let sites = collect_sites(program, cx.scoping(), &search.found);
    let mut plan = RoundPlan {
        notes: search
            .rejected
            .iter()
            .map(|(span, reason)| (reason.to_string(), Some(*span)))
            .collect(),
        ..RoundPlan::default()
    };
    for shape in &search.found {
        rewrite_wrapper(program, shape, &sites, cx, &mut plan, poisoned);
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

fn immediate_wrapper<'k, 'p>(
    expr: &'k Expression<'p>,
    scoping: &Scoping,
) -> Option<(&'k Function<'p>, &'k AVec<'p, Argument<'p>>)> {
    let call = called_expression(expr)?;
    let Expression::FunctionExpression(function) = call.callee.without_parentheses() else {
        return None;
    };
    let names = param_names(function)?;
    let returned = returned_expression(function)?;
    let (callee, arguments) = call_parts(returned)?;
    let wraps = function.id.is_none()
        && !function.r#async
        && !function.generator
        && names.len() >= 2
        && arguments.len() >= 2
        && identifier_name(callee).and_then(decoder_index).is_some()
        && uses_any_param(arguments, &names)
        && body_is_self_contained(function, scoping)
        && arguments_are_substitutable(&call.arguments)
        && !arguments_hold_a_scope(&call.arguments);
    wraps.then_some((&**function, &call.arguments))
}

struct ImmediateSearch<'s, 'c, 'a, 'x> {
    scoping: &'s Scoping,
    cx: &'c Cx<'a, 'x>,
    edits: ExprEdits<'a>,
}

impl<'p> VisitJs<'p> for ImmediateSearch<'_, '_, '_, '_> {
    fn visit_expression(&mut self, it: &Expression<'p>) {
        if let Some((function, arguments)) = immediate_wrapper(it, self.scoping) {
            if let Some(inlined) = inline_function_call(function, arguments, self.cx) {
                if self.edits.insert(it.span(), inlined) {
                    return;
                }
            }
        }
        walk_js::walk_expression(self, it);
    }
}

pub fn inline_immediate_wrappers<'a>(program: &mut Program<'a>, cx: &mut Cx<'a, '_>) -> usize {
    let edits = {
        let view: &Program<'_> = program;
        let mut search = ImmediateSearch {
            scoping: cx.scoping(),
            cx: &*cx,
            edits: ExprEdits::new(),
        };
        search.visit_program(view);
        search.edits
    };
    if edits.is_empty() {
        return 0;
    }
    edits.apply(program, cx.allocator(), cx.scoping_mut())
}
