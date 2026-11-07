use oxc_allocator::GetAllocator;
use oxc_allocator::Vec as AVec;
use oxc_ast::ast::*;
use oxc_ast_visit::{walk_js, VisitJs};
use oxc_semantic::Scoping;
use oxc_span::{GetSpan, Span};
use oxc_str::Ident;
use oxc_syntax::operator::AssignmentOperator;
use oxc_syntax::scope::ScopeId;
use oxc_syntax::symbol::SymbolId;
use rustc_hash::{FxHashMap, FxHashSet};
use untwist_core::ast_util::edit::{ExprEdits, StmtEdits};
use untwist_core::ast_util::prop::identifier_name;
use untwist_core::ast_util::rename::Renames;
use untwist_core::ast_util::scope::binding_symbol;
use untwist_core::ast_util::search::{expression_at, statement_at};
use untwist_core::pass::{Cx, Interest, Pass};
use untwist_core::semantic::{is_constant_binding, scope_hazard, write_reference_count};

use crate::alias_forms::{
    alias_source, decoders_ignore_this, deleted_span, literal_headed_identifier, member_reads,
    single_property_source, ObjectAlias,
};
use crate::bindings::{reference_symbol, resolved_symbol};
use crate::fused::{Fused, Member, Walk};
use crate::init_order::{lend_order_with, note_overflow, sharing_mark, DeclarationSlot, LentOrder};
use crate::mutants::wrapper_guards_dropped;
use crate::notes;
use crate::proof::has_with_statement;
use crate::stamp::{find_stamped_named, is_unedited};
use crate::units::MachineryDepth;
use crate::wrapper_calls::{inline_function_wrappers, inline_immediate_wrappers};

pub const ALIAS_FORM_CHECKS: &str = "inline-decoder-wrappers/alias-form-checks";

#[derive(Hash)]
pub struct AliasSite {
    pub symbol: SymbolId,
    pub source: SymbolId,
    pub statement: Span,
    pub expression: Option<Span>,
}

struct AliasSearch<'s> {
    scoping: &'s Scoping,
    keeps_machinery: bool,
    statement: Span,
    units: MachineryDepth,
    machinery: FxHashSet<SymbolId>,
    found: Vec<AliasSite>,
    forms: AliasForms,
}

#[derive(Default)]
struct AliasForms {
    objects: Vec<ObjectAlias>,
    sequences: Vec<(Span, SymbolId)>,
    deleted: FxHashSet<Span>,
}

impl AliasSearch<'_> {
    fn reference_symbol(&self, expr: &Expression<'_>) -> Option<SymbolId> {
        let Expression::Identifier(id) = expr else {
            return None;
        };
        let reference_id = id.reference_id.get()?;
        self.scoping.get_reference(reference_id).symbol_id()
    }

    fn claim(&mut self, symbol: Option<SymbolId>) {
        if let (true, Some(symbol)) = (self.units.inside(), symbol) {
            self.machinery.insert(symbol);
        }
    }
}

impl<'s> AliasSearch<'s> {
    fn new(scoping: &'s Scoping) -> Self {
        Self {
            scoping,
            keeps_machinery: false,
            statement: Span::empty(0),
            units: MachineryDepth::every_unit(),
            machinery: FxHashSet::default(),
            found: Vec::new(),
            forms: AliasForms::default(),
        }
    }

    fn finish(self) -> Vec<AliasSite> {
        self.finish_with_forms().0
    }

    fn finish_with_forms(self) -> (Vec<AliasSite>, AliasForms) {
        let Self {
            machinery,
            mut found,
            keeps_machinery,
            mut forms,
            ..
        } = self;
        if !keeps_machinery {
            found.retain(|site| !machinery.contains(&site.symbol));
            forms
                .objects
                .retain(|site| !machinery.contains(&site.symbol));
        }
        (found, forms)
    }
}

impl<'a> Member<'a> for AliasSearch<'_> {
    type Saved = (bool, Span);

    #[inline(always)]
    fn enter_statement(&mut self, it: &Statement<'a>) -> (bool, Span) {
        let entered = self.units.enter(it);
        let outer = std::mem::replace(&mut self.statement, it.span());
        (entered, outer)
    }

    #[inline(always)]
    fn leave_statement(&mut self, (entered, outer): (bool, Span)) {
        self.statement = outer;
        self.units.leave(entered);
    }

    #[inline(always)]
    fn enter_expression(&mut self, it: &Expression<'a>) -> Option<(bool, Span)> {
        if let Some(span) = deleted_span(it) {
            self.forms.deleted.insert(span);
        }
        if let (Expression::SequenceExpression(sequence), Some(id)) =
            (it, literal_headed_identifier(it))
        {
            if let Some(symbol) = id
                .reference_id
                .get()
                .and_then(|id| self.scoping.get_reference(id).symbol_id())
            {
                self.forms.sequences.push((sequence.span, symbol));
            }
        }
        Some((false, Span::default()))
    }

    #[inline(always)]
    fn binding_identifier(&mut self, it: &BindingIdentifier<'a>) {
        self.claim(it.symbol_id.get());
    }

    #[inline(always)]
    fn identifier_reference(&mut self, it: &IdentifierReference<'a>) {
        let Some(reference) = it
            .reference_id
            .get()
            .map(|id| self.scoping.get_reference(id))
        else {
            return;
        };
        if reference.is_write() {
            self.claim(reference.symbol_id());
        }
    }

    #[inline(always)]
    fn enter_variable_declarator(&mut self, it: &VariableDeclarator<'a>) -> (bool, Span) {
        let (BindingPattern::BindingIdentifier(id), Some(init)) = (&it.id, &it.init) else {
            return (false, Span::default());
        };
        let Some(symbol) = id.symbol_id.get() else {
            return (false, Span::default());
        };
        if let Some(source) = alias_source(init, self.scoping) {
            self.found.push(AliasSite {
                symbol,
                source,
                statement: self.statement,
                expression: None,
            });
        } else if let Some((source, key)) = single_property_source(init, self.scoping) {
            self.forms.objects.push(ObjectAlias {
                symbol,
                source,
                statement: self.statement,
                key,
            });
        }
        (false, Span::default())
    }

    #[inline(always)]
    fn assignment_expression(&mut self, it: &AssignmentExpression<'a>) {
        if it.operator != AssignmentOperator::Assign {
            return;
        }
        let AssignmentTarget::AssignmentTargetIdentifier(target) = &it.left else {
            return;
        };
        let target_symbol = target
            .reference_id
            .get()
            .and_then(|id| self.scoping.get_reference(id).symbol_id());
        if let (Some(symbol), Some(source)) = (target_symbol, self.reference_symbol(&it.right)) {
            self.found.push(AliasSite {
                symbol,
                source,
                statement: self.statement,
                expression: Some(it.span),
            });
        }
    }
}

pub fn collect_alias_sites(program: &Program<'_>, scoping: &Scoping) -> Vec<AliasSite> {
    let mut walk = Walk(AliasSearch::new(scoping));
    walk.visit_program(program);
    walk.0.finish()
}

fn single_declarator_statement(stmt: &Statement<'_>) -> Option<(SymbolId, Span)> {
    let Statement::VariableDeclaration(declaration) = stmt else {
        return None;
    };
    if declaration.declarations.len() != 1 {
        return None;
    }
    let declarator = declaration.declarations.first()?;
    Some((binding_symbol(&declarator.id)?, stmt.span()))
}

#[derive(Default)]
struct DeclarationSearch {
    found: FxHashMap<SymbolId, Span>,
    counts: FxHashMap<SymbolId, usize>,
    initialised: FxHashSet<SymbolId>,
    members: FxHashSet<Span>,
}

impl<'a> Member<'a> for DeclarationSearch {
    type Saved = ();

    #[inline(always)]
    fn enter_statements(&mut self, it: &AVec<'a, Statement<'a>>) {
        for statement in it {
            if single_declarator_statement(statement).is_some() {
                self.members.insert(statement.span());
            }
        }
    }

    #[inline(always)]
    fn enter_statement(&mut self, it: &Statement<'a>) {
        if let Some((symbol, span)) = single_declarator_statement(it) {
            self.found.insert(symbol, span);
        }
    }

    #[inline(always)]
    fn enter_variable_declarator(&mut self, it: &VariableDeclarator<'a>) {
        if let Some(symbol) = binding_symbol(&it.id) {
            *self.counts.entry(symbol).or_default() += 1;
            if it.init.is_some() {
                self.initialised.insert(symbol);
            }
        }
    }
}

fn alias_scans(
    program: &Program<'_>,
    cx: &mut Cx<'_, '_>,
    (shares, keeps_machinery): (bool, bool),
) -> (Vec<AliasSite>, AliasForms, DeclarationSearch, LentOrder) {
    let mark = sharing_mark(shares, program, cx);
    let (order, (sites, forms), declarations) = {
        let scoping = cx.scoping();
        let search = AliasSearch {
            keeps_machinery,
            ..AliasSearch::new(scoping)
        };
        let member = Fused::new(search, DeclarationSearch::default());
        let found = lend_order_with(mark, program, scoping, member);
        let member = found.member;
        (found.order, member.first.finish_with_forms(), member.second)
    };
    note_overflow(&order, cx);
    (sites, forms, declarations, order)
}

fn is_bare_statement(program: &Program<'_>, statement: Span, expression: Span) -> bool {
    let Some(Statement::ExpressionStatement(stmt)) = statement_at(program, statement) else {
        return false;
    };
    stmt.expression.span() == expression
}

#[derive(Default)]
struct VariablePlan<'a> {
    aliases: FxHashMap<SymbolId, Ident<'a>>,
    chosen: Vec<(SymbolId, Span, Option<Span>)>,
    self_assignments: Vec<(Span, Span)>,
    decoders: FxHashSet<SymbolId>,
    declarations: FxHashMap<SymbolId, Span>,
    rejected: Vec<(Span, &'static str)>,
    budget: Vec<(Span, SymbolId)>,
    objects: Vec<(Span, Vec<Span>, Ident<'a>)>,
    collapses: Vec<(Span, Ident<'a>)>,
}

fn decoder_targets<'a>(program: &Program<'_>, cx: &Cx<'a, '_>) -> FxHashMap<SymbolId, Ident<'a>> {
    let stamped = find_stamped_named(program, cx.scoping(), is_unedited(cx));
    let mut targets: FxHashMap<SymbolId, Ident<'a>> = FxHashMap::default();
    for decoder in &stamped.decoders {
        if let Some(symbol) = decoder.symbol {
            targets.insert(symbol, Ident::from_str_in(decoder.name, cx));
        }
    }
    targets
}

fn alias_target<'a>(
    site: &AliasSite,
    targets: &FxHashMap<SymbolId, Ident<'a>>,
    plan: &VariablePlan<'a>,
) -> Option<Ident<'a>> {
    if site.symbol == site.source || plan.aliases.contains_key(&site.symbol) {
        return None;
    }
    targets
        .get(&site.source)
        .or_else(|| plan.aliases.get(&site.source))
        .copied()
}

struct AliasFacts {
    declarations: DeclarationSearch,
    self_writes: FxHashMap<SymbolId, Vec<Span>>,
    order: LentOrder,
}

#[cfg(debug_assertions)]
fn alias_facts_digest(sites: &[AliasSite], declarations: &DeclarationSearch, order: u64) -> u64 {
    use crate::fused::check;
    let found = check::sorted(
        declarations
            .found
            .iter()
            .map(|(symbol, span)| (symbol.index(), *span)),
    );
    let counts = check::sorted(
        declarations
            .counts
            .iter()
            .map(|(symbol, count)| (symbol.index(), *count)),
    );
    let initialised = check::sorted(
        declarations
            .initialised
            .iter()
            .map(|symbol| (symbol.index(), ())),
    );
    let members = check::sorted(
        declarations
            .members
            .iter()
            .map(|span| ((span.start, span.end), ())),
    );
    check::digest_of(&(sites, (found, counts, initialised, members), order))
}

#[cfg(debug_assertions)]
fn record_alias_facts(sites: &[AliasSite], facts: &AliasFacts) {
    use crate::fused::check;
    if check::active() {
        let digest = alias_facts_digest(sites, &facts.declarations, facts.order.digest());
        check::record("alias-facts", digest);
    }
}

#[cfg(all(debug_assertions, feature = "test-hooks"))]
pub(crate) fn fused_and_separate_alias_facts(
    program: &Program<'_>,
    scoping: &Scoping,
) -> (u64, u64) {
    let member = Fused::new(AliasSearch::new(scoping), DeclarationSearch::default());
    let (order, member) = crate::init_order::try_scan_init_order_with(program, scoping, member);
    let order = order.map_or(0, |order| order.digest());
    let fused = alias_facts_digest(&member.first.finish(), &member.second, order);
    let mut declarations = Walk(DeclarationSearch::default());
    declarations.visit_program(program);
    let order =
        crate::init_order::try_scan_init_order(program, scoping).map_or(0, |order| order.digest());
    let separate = alias_facts_digest(
        &collect_alias_sites(program, scoping),
        &declarations.0,
        order,
    );
    (fused, separate)
}

fn self_write_counts(program: &Program<'_>, sites: &[AliasSite]) -> FxHashMap<SymbolId, Vec<Span>> {
    let mut counts: FxHashMap<SymbolId, Vec<Span>> = FxHashMap::default();
    for site in sites {
        let Some(expression) = site.expression else {
            continue;
        };
        if site.symbol != site.source {
            continue;
        }
        let entry = counts.entry(site.symbol).or_default();
        if is_bare_statement(program, site.statement, expression) {
            entry.push(site.statement);
        } else {
            entry.push(Span::empty(0));
        }
    }
    counts
}

fn removed_self_assignments(
    site: &AliasSite,
    facts: &AliasFacts,
    slot: DeclarationSlot,
) -> FxHashSet<usize> {
    facts
        .self_writes
        .get(&site.symbol)
        .into_iter()
        .flatten()
        .filter_map(|statement| facts.order.declaration_slot(*statement))
        .filter(|other| other.same_list(slot))
        .map(DeclarationSlot::index)
        .collect()
}

fn reads_before_the_alias(site: &AliasSite, facts: &AliasFacts) -> bool {
    let Some(slot) = facts.order.declaration_slot(site.statement) else {
        return true;
    };
    let ignored = removed_self_assignments(site, facts, slot);
    !facts
        .order
        .symbol_is_initialised_before_use(slot, site.symbol, &ignored)
}

fn rejection(site: &AliasSite, facts: &AliasFacts, scoping: &Scoping) -> Option<&'static str> {
    if reads_before_the_alias(site, facts) {
        return Some(notes::WRAPPER_ALIAS_READ_BEFORE_THE_ALIAS);
    }
    let declarations = &facts.declarations;
    if !scoping.symbol_redeclarations(site.symbol).is_empty() {
        return Some(notes::WRAPPER_ALIAS_NOT_REMOVABLE);
    }
    let Some(declaration) = declarations.found.get(&site.symbol) else {
        return Some(notes::WRAPPER_ALIAS_NOT_REMOVABLE);
    };
    if declarations.counts.get(&site.symbol) != Some(&1)
        || !declarations.members.contains(declaration)
    {
        return Some(notes::WRAPPER_ALIAS_NOT_REMOVABLE);
    }
    if site.expression.is_none() {
        if *declaration != site.statement {
            return Some(notes::WRAPPER_ALIAS_NOT_REMOVABLE);
        }
        return (!is_constant_binding(scoping, site.symbol))
            .then_some(notes::WRAPPER_ALIAS_NOT_CONSTANT);
    }
    if declarations.initialised.contains(&site.symbol) {
        return Some(notes::WRAPPER_ALIAS_ALREADY_INITIALISED);
    }
    let self_writes = facts.self_writes.get(&site.symbol).map_or(0, Vec::len);
    (write_reference_count(scoping, site.symbol) != self_writes + 1)
        .then_some(notes::WRAPPER_ALIAS_EXTRA_WRITES)
}

fn collect_leftovers<'a>(
    sites: &[AliasSite],
    targets: &FxHashMap<SymbolId, Ident<'a>>,
    facts: &AliasFacts,
    scoping: &Scoping,
    plan: &mut VariablePlan<'a>,
) {
    let mut noted: FxHashSet<SymbolId> = FxHashSet::default();
    for site in sites {
        if site.symbol == site.source {
            let renamed = plan.aliases.contains_key(&site.symbol);
            if let (true, Some(expression)) = (renamed, site.expression) {
                plan.self_assignments.push((site.statement, expression));
            }
            continue;
        }
        let known = targets.contains_key(&site.source) || plan.aliases.contains_key(&site.source);
        if !known || plan.aliases.contains_key(&site.symbol) {
            continue;
        }
        let mark = facts.order.budget_exhausted();
        if let Some(reason) = rejection(site, facts, scoping) {
            if noted.insert(site.symbol) {
                let span = site.expression.unwrap_or(site.statement);
                plan.rejected.push((span, reason));
                if facts.order.budget_exhausted() > mark {
                    plan.budget.push((span, site.symbol));
                }
            }
        }
    }
}

fn wrapper_targets<'a>(cx: &Cx<'a, '_>, kept: &[SymbolId]) -> FxHashMap<SymbolId, Ident<'a>> {
    let scoping = cx.scoping();
    let guarded = !wrapper_guards_dropped();
    if guarded && scope_hazard(scoping).is_some() {
        return FxHashMap::default();
    }
    kept.iter()
        .copied()
        .filter(|symbol| !guarded || bound_once_in_a_var_scope(scoping, *symbol))
        .map(|symbol| (symbol, Ident::from_str_in(scoping.symbol_name(symbol), cx)))
        .collect()
}

fn bound_once_in_a_var_scope(scoping: &Scoping, symbol: SymbolId) -> bool {
    is_constant_binding(scoping, symbol)
        && scoping
            .scope_flags(scoping.symbol_scope_id(symbol))
            .is_var()
}

fn name_reaches(scoping: &Scoping, from: ScopeId, target: SymbolId) -> bool {
    let home = scoping.symbol_scope_id(target);
    let name = scoping.symbol_ident(target);
    for scope in scoping.scope_ancestors(from) {
        if scope == home {
            return true;
        }
        if scoping.get_binding(scope, name).is_some() {
            return false;
        }
    }
    false
}

fn in_target_scope(
    scoping: &Scoping,
    site: &AliasSite,
    targets: &FxHashMap<SymbolId, Ident<'_>>,
) -> bool {
    if wrapper_guards_dropped() || !targets.contains_key(&site.source) {
        return true;
    }
    scoping
        .get_resolved_reference_ids(site.symbol)
        .iter()
        .all(|reference| {
            let from = scoping.get_reference(*reference).scope_id();
            name_reaches(scoping, from, site.source)
        })
}

enum AliasTargets<'k> {
    Decoders,
    Wrappers(&'k [SymbolId]),
}

fn variable_alias_plan<'a>(
    program: &Program<'_>,
    cx: &mut Cx<'a, '_>,
    shares: bool,
    wanted: AliasTargets<'_>,
) -> VariablePlan<'a> {
    let mut plan = VariablePlan::default();
    let (targets, keeps_machinery) = match wanted {
        AliasTargets::Decoders => (decoder_targets(program, cx), false),
        AliasTargets::Wrappers(kept) => (wrapper_targets(cx, kept), true),
    };
    if targets.is_empty() {
        return plan;
    }
    if !keeps_machinery {
        plan.decoders = targets.keys().copied().collect();
    }

    let (mut sites, forms, declarations, order) =
        alias_scans(program, cx, (shares && !keeps_machinery, keeps_machinery));
    if keeps_machinery {
        let scoping = cx.scoping();
        sites.retain(|site| in_target_scope(scoping, site, &targets));
    }
    let facts = AliasFacts {
        declarations,
        self_writes: self_write_counts(program, &sites),
        order,
    };
    #[cfg(debug_assertions)]
    record_alias_facts(&sites, &facts);
    let mut progressed = true;
    while progressed {
        progressed = false;
        for site in &sites {
            let Some(name) = alias_target(site, &targets, &plan) else {
                continue;
            };
            if rejection(site, &facts, cx.scoping()).is_some() {
                continue;
            }
            plan.aliases.insert(site.symbol, name);
            plan.chosen
                .push((site.symbol, site.statement, site.expression));
            progressed = true;
        }
    }

    collect_leftovers(&sites, &targets, &facts, cx.scoping(), &mut plan);
    let checked = !cx.opts.rule_disabled(ALIAS_FORM_CHECKS);
    if !keeps_machinery {
        plan.objects = object_rewrites(program, &forms.objects, &targets, &facts, cx, &mut plan);
    }
    plan.collapses = sequence_collapses(&forms, &targets, &plan, cx.scoping(), checked);
    plan.declarations = facts.declarations.found;
    plan
}

fn known_target<'a>(
    symbol: SymbolId,
    targets: &FxHashMap<SymbolId, Ident<'a>>,
    plan: &VariablePlan<'a>,
) -> Option<Ident<'a>> {
    targets
        .get(&symbol)
        .or_else(|| plan.aliases.get(&symbol))
        .copied()
}

fn object_rewrites<'a>(
    program: &Program<'_>,
    objects: &[ObjectAlias],
    targets: &FxHashMap<SymbolId, Ident<'a>>,
    facts: &AliasFacts,
    cx: &Cx<'a, '_>,
    plan: &mut VariablePlan<'a>,
) -> Vec<(Span, Vec<Span>, Ident<'a>)> {
    let scoping = cx.scoping();
    let checked = !cx.opts.rule_disabled(ALIAS_FORM_CHECKS);
    let mut candidates = Vec::new();
    for object in objects {
        let Some(name) = known_target(object.source, targets, plan) else {
            continue;
        };
        let site = AliasSite {
            symbol: object.symbol,
            source: object.source,
            statement: object.statement,
            expression: None,
        };
        match rejection(&site, facts, scoping).filter(|_| checked) {
            Some(reason) => plan.rejected.push((object.statement, reason)),
            None => candidates.push((object, name)),
        }
    }
    if candidates.is_empty() {
        return Vec::new();
    }
    if checked && !decoders_ignore_this(program, &plan.decoders) {
        for (object, _) in &candidates {
            plan.rejected
                .push((object.statement, notes::WRAPPER_OBJECT_ALIAS_THIS));
        }
        return Vec::new();
    }
    let wanted: FxHashMap<SymbolId, &str> = candidates
        .iter()
        .map(|(object, _)| (object.symbol, object.key.as_str()))
        .collect();
    let reads = member_reads(program, scoping, &wanted, checked);
    let mut rewrites = Vec::new();
    for (object, name) in candidates {
        match reads.get(&object.symbol) {
            Some(members) => rewrites.push((object.statement, members.clone(), name)),
            None => plan
                .rejected
                .push((object.statement, notes::WRAPPER_OBJECT_ALIAS_OTHER_USE)),
        }
    }
    rewrites
}

fn sequence_collapses<'a>(
    forms: &AliasForms,
    targets: &FxHashMap<SymbolId, Ident<'a>>,
    plan: &VariablePlan<'a>,
    scoping: &Scoping,
    checked: bool,
) -> Vec<(Span, Ident<'a>)> {
    if forms.sequences.is_empty() || (checked && has_with_statement(scoping)) {
        return Vec::new();
    }
    forms
        .sequences
        .iter()
        .filter(|(span, _)| !checked || !forms.deleted.contains(span))
        .filter_map(|(span, symbol)| Some((*span, known_target(*symbol, targets, plan)?)))
        .collect()
}

fn apply_alias_site<'a>(
    program: &Program<'a>,
    plan: &VariablePlan<'a>,
    site: (SymbolId, Span, Option<Span>),
    cx: &mut Cx<'a, '_>,
    edits: &mut ExprEdits<'a>,
    removals: &mut StmtEdits<'a>,
) {
    let (symbol, statement, expression) = site;
    let Some(expression) = expression else {
        removals.remove(statement);
        return;
    };
    if let Some(declaration) = plan.declarations.get(&symbol) {
        removals.remove(*declaration);
    }
    if is_bare_statement(program, statement, expression) {
        removals.remove(statement);
        return;
    }
    let Some(name) = decoder_name_at(program, expression, &*cx) else {
        cx.note(notes::WRAPPER_ALIAS_NOT_AN_IDENTIFIER, Some(expression));
        return;
    };
    if !edits.insert(
        expression,
        Expression::new_identifier(expression, name, &*cx),
    ) {
        cx.note(notes::WRAPPER_ALIAS_NO_SPAN, Some(expression));
    }
}

#[derive(Default)]
pub struct InlineDecoderWrappers;

pub fn boxed<'a>() -> Box<dyn Pass<'a> + 'a> {
    Box::new(InlineDecoderWrappers)
}

impl<'a> Pass<'a> for InlineDecoderWrappers {
    fn name(&self) -> &'static str {
        "inline-decoder-wrappers"
    }

    fn interest(&self) -> Interest {
        Interest::ENTER_PROGRAM
    }

    fn enter_program(&mut self, program: &mut Program<'a>, cx: &mut Cx<'a, '_>) {
        let shares = cx.opts.shares_init_order;
        let immediate = if crate::string_array::legacy::lifted_any() {
            inline_immediate_wrappers(program, cx)
        } else {
            0
        };
        if immediate > 0 {
            cx.changed_by(immediate);
            return;
        }
        let variable_changes = inline_variable_aliases(program, cx, shares, AliasTargets::Decoders);
        let mut rounds = inline_function_wrappers(program, cx);
        let mut changes = variable_changes + rounds.changes;
        if !rounds.kept.is_empty() {
            let kept = std::mem::take(&mut rounds.kept);
            let renamed =
                inline_variable_aliases(program, cx, false, AliasTargets::Wrappers(&kept));
            changes += renamed;
            if renamed > 0 {
                changes += inline_function_wrappers(program, cx).changes;
            }
        }
        cx.changed_by(changes);
    }
}

fn inline_variable_aliases<'a>(
    program: &mut Program<'a>,
    cx: &mut Cx<'a, '_>,
    shares: bool,
    wanted: AliasTargets<'_>,
) -> usize {
    let plan = variable_alias_plan(program, cx, shares, wanted);
    for (span, reason) in plan.rejected.iter().copied() {
        cx.note(reason, Some(span));
    }
    for (span, symbol) in plan.budget.iter().copied() {
        let name = cx.scoping().symbol_name(symbol).to_string();
        cx.note(notes::init_order_budget(&name), Some(span));
    }
    note_kept_self_assignments(program, &plan, cx);
    if plan.aliases.is_empty() && plan.objects.is_empty() && plan.collapses.is_empty() {
        return 0;
    }

    let mut renames = Renames::new();
    for (symbol, name) in &plan.aliases {
        renames.insert(*symbol, *name);
    }
    let mut renamed = renames.apply(program, cx.scoping());

    let mut removals = StmtEdits::new();
    let mut edits = ExprEdits::new();
    for site in plan.chosen.iter().copied() {
        apply_alias_site(program, &plan, site, cx, &mut edits, &mut removals);
    }
    for (statement, members, name) in &plan.objects {
        removals.remove(*statement);
        for member in members {
            edits.insert(*member, Expression::new_identifier(*member, *name, &*cx));
        }
    }
    for (span, name) in plan.collapses.iter().copied() {
        edits.insert(span, Expression::new_identifier(span, name, &*cx));
    }
    for (statement, expression) in plan.self_assignments.iter().copied() {
        if is_bare_statement(program, statement, expression) {
            removals.remove(statement);
        }
    }
    renamed += edits.apply(program, cx.allocator(), cx.scoping_mut());
    renamed += removals.apply(program, cx.allocator(), cx.scoping_mut());
    renamed
}

struct SelfAssignments<'s> {
    scoping: &'s Scoping,
    decoders: &'s FxHashSet<SymbolId>,
    statement: Span,
    found: Vec<(Span, Span)>,
}

impl<'a> VisitJs<'a> for SelfAssignments<'_> {
    fn visit_statement(&mut self, it: &Statement<'a>) {
        let outer = std::mem::replace(&mut self.statement, it.span());
        walk_js::walk_statement(self, it);
        self.statement = outer;
    }

    fn visit_assignment_expression(&mut self, it: &AssignmentExpression<'a>) {
        if it.operator == AssignmentOperator::Assign {
            if let AssignmentTarget::AssignmentTargetIdentifier(target) = &it.left {
                let written = resolved_symbol(target, self.scoping);
                if written.is_some_and(|symbol| self.decoders.contains(&symbol))
                    && written == reference_symbol(&it.right, self.scoping)
                {
                    self.found.push((self.statement, it.span));
                }
            }
        }
        walk_js::walk_assignment_expression(self, it);
    }
}

fn note_kept_self_assignments(program: &Program<'_>, plan: &VariablePlan<'_>, cx: &mut Cx<'_, '_>) {
    if plan.decoders.is_empty() {
        return;
    }
    let mut search = SelfAssignments {
        scoping: cx.scoping(),
        decoders: &plan.decoders,
        statement: Span::empty(0),
        found: Vec::new(),
    };
    search.visit_program(program);
    for (statement, expression) in search.found {
        if !is_bare_statement(program, statement, expression) {
            cx.note(notes::WRAPPER_ALIAS_SELF_ASSIGNMENT, Some(expression));
        }
    }
}

fn decoder_name_at<'a>(
    program: &Program<'_>,
    expression: Span,
    cx: &Cx<'a, '_>,
) -> Option<Ident<'a>> {
    let Expression::AssignmentExpression(assignment) = expression_at(program, expression)? else {
        return None;
    };
    Some(Ident::from_str_in(identifier_name(&assignment.right)?, cx))
}
