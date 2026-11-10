use super::*;
use untwist_core::ast_util::readonly::{scan_symbol_uses, SymbolUses};
use untwist_core::semantic::declaring_block;

pub struct ArrayAlias {
    pub symbol: SymbolId,
    pub source: SymbolId,
    pub statement: Span,
    pub init: Span,
    pub scope: ScopeId,
    pub index: usize,
}

#[derive(Default)]
pub struct ArrayAliasSites {
    pub aliases: Vec<ArrayAlias>,
    pub arrays: FxHashMap<SymbolId, (ScopeId, usize)>,
}

struct AliasSearch<'s> {
    scoping: &'s Scoping,
    sites: ArrayAliasSites,
    simple: Vec<SimpleArray>,
}

impl AliasSearch<'_> {
    fn scan_body(&mut self, scope: Option<ScopeId>, statements: &[Statement<'_>]) {
        let Some(scope) = scope else {
            return;
        };
        for (index, statement) in statements.iter().enumerate() {
            if array_declaration(statement).is_some() {
                if let Some(symbol) = declaration_symbol(statement) {
                    self.sites.arrays.insert(symbol, (scope, index));
                }
            }
            let Some((symbol, source, init)) = alias_of(statement, self.scoping) else {
                continue;
            };
            if self.scoping.symbol_scope_id(symbol) == scope {
                self.sites.aliases.push(ArrayAlias {
                    symbol,
                    source,
                    statement: statement.span(),
                    init,
                    scope,
                    index,
                });
            }
        }
    }
}

impl<'a> VisitJs<'a> for AliasSearch<'_> {
    fn visit_program(&mut self, it: &Program<'a>) {
        self.scan_body(it.scope_id.get(), &it.body);
        walk_js::walk_program(self, it);
    }

    fn visit_block_statement(&mut self, it: &BlockStatement<'a>) {
        self.scan_body(it.scope_id.get(), &it.body);
        walk_js::walk_block_statement(self, it);
    }

    fn visit_static_block(&mut self, it: &StaticBlock<'a>) {
        self.scan_body(it.scope_id.get(), &it.body);
        walk_js::walk_static_block(self, it);
    }

    fn visit_function(&mut self, it: &Function<'a>, flags: ScopeFlags) {
        if let Some(body) = it.body.as_ref() {
            self.scan_body(it.scope_id.get(), &body.statements);
        }
        walk_js::walk_function(self, it, flags);
    }

    fn visit_arrow_function_expression(&mut self, it: &ArrowFunctionExpression<'a>) {
        if let ArrowFunctionBody::FunctionBody(body) = &it.body {
            self.scan_body(it.scope_id.get(), &body.statements);
        }
        walk_js::walk_arrow_function_expression(self, it);
    }

    fn visit_statement(&mut self, it: &Statement<'a>) {
        if let Some(simple) = simple_array(it) {
            self.simple.push(simple);
        }
        walk_js::walk_statement(self, it);
    }
}

fn alias_of(stmt: &Statement<'_>, scoping: &Scoping) -> Option<(SymbolId, SymbolId, Span)> {
    let Statement::VariableDeclaration(declaration) = stmt else {
        return None;
    };
    if declaration.declarations.len() != 1 {
        return None;
    }
    let declarator = declaration.declarations.first()?;
    let BindingPattern::BindingIdentifier(id) = &declarator.id else {
        return None;
    };
    let Expression::Identifier(init) = declarator.init.as_ref()? else {
        return None;
    };
    let symbol = id.symbol_id.get()?;
    let source = scoping
        .get_reference(init.reference_id.get()?)
        .symbol_id()?;
    if symbol == source {
        return None;
    }
    Some((symbol, source, init.span))
}

pub(super) fn search_aliases(
    program: &Program<'_>,
    scoping: &Scoping,
) -> (ArrayAliasSites, Vec<SimpleArray>) {
    let mut search = AliasSearch {
        scoping,
        sites: ArrayAliasSites::default(),
        simple: Vec::new(),
    };
    search.visit_program(program);
    (search.sites, search.simple)
}

fn name_counts(
    scoping: &Scoping,
    roots: impl Iterator<Item = SymbolId>,
) -> FxHashMap<String, usize> {
    let mut counts: FxHashMap<String, usize> = roots
        .map(|root| (scoping.symbol_name(root).to_owned(), 0))
        .collect();
    for symbol in scoping.symbol_ids() {
        if let Some(count) = counts.get_mut(scoping.symbol_name(symbol)) {
            *count += 1;
        }
    }
    counts
}

fn root_name_reaches(scoping: &Scoping, alias: SymbolId, root: SymbolId) -> bool {
    if crate::mutants::alias_root_names_trusted() {
        return true;
    }
    let name = scoping.symbol_name(root);
    let home = scoping.symbol_scope_id(root);
    let block_function_alike = scoping.symbol_ids().any(|symbol| {
        symbol != root
            && scoping.symbol_name(symbol) == name
            && declaring_block(scoping, symbol).is_some()
    });
    if block_function_alike {
        return false;
    }
    scoping.get_resolved_references(alias).all(|reference| {
        let mut seen_home = false;
        let shadowed = scoping
            .scope_ancestors(reference.scope_id())
            .take_while(|scope| {
                let more = !seen_home;
                seen_home |= *scope == home;
                more
            })
            .any(|scope| {
                scoping
                    .get_binding(scope, name.into())
                    .is_some_and(|symbol| symbol != root)
            });
        seen_home && !shadowed
    })
}

struct Stability<'p, 'a> {
    program: &'p Program<'a>,
    symbols: FxHashSet<SymbolId>,
    uses: Option<SymbolUses>,
    known: FxHashMap<SymbolId, bool>,
}

impl<'p, 'a> Stability<'p, 'a> {
    fn new(program: &'p Program<'a>, sites: &ArrayAliasSites) -> Self {
        let symbols = sites
            .aliases
            .iter()
            .flat_map(|alias| [alias.symbol, alias.source])
            .chain(sites.arrays.keys().copied())
            .collect();
        Self {
            program,
            symbols,
            uses: None,
            known: FxHashMap::default(),
        }
    }

    fn uses(&mut self, scoping: &Scoping) -> &SymbolUses {
        let (program, symbols) = (self.program, &self.symbols);
        self.uses
            .get_or_insert_with(|| scan_symbol_uses(program, scoping, symbols))
    }

    fn of(&mut self, scoping: &Scoping, symbol: SymbolId) -> bool {
        if let Some(known) = self.known.get(&symbol) {
            return *known;
        }
        let stable = scoping.symbol_redeclarations(symbol).is_empty()
            && is_constant_binding(scoping, symbol)
            && self
                .uses(scoping)
                .of(symbol)
                .map_or(0, |found| found.writes)
                == 0;
        self.known.insert(symbol, stable);
        stable
    }
}

fn declaration_slots(sites: &ArrayAliasSites) -> FxHashMap<SymbolId, (ScopeId, usize)> {
    let mut slots = sites.arrays.clone();
    for alias in &sites.aliases {
        slots
            .entry(alias.symbol)
            .or_insert((alias.scope, alias.index));
    }
    slots
}

fn precedes(
    slots: &FxHashMap<SymbolId, (ScopeId, usize)>,
    source: SymbolId,
    alias: &ArrayAlias,
) -> bool {
    slots
        .get(&source)
        .is_some_and(|(scope, index)| *scope == alias.scope && *index < alias.index)
}

fn alias_roots(
    sites: &ArrayAliasSites,
    slots: &FxHashMap<SymbolId, (ScopeId, usize)>,
    stability: &mut Stability<'_, '_>,
    scoping: &Scoping,
) -> FxHashMap<SymbolId, SymbolId> {
    let mut roots: FxHashMap<SymbolId, SymbolId> = FxHashMap::default();
    for symbol in sites.arrays.keys() {
        roots.insert(*symbol, *symbol);
    }
    let mut progressed = true;
    while progressed {
        progressed = false;
        for alias in &sites.aliases {
            let Some(root) = roots.get(&alias.source).copied() else {
                continue;
            };
            if !precedes(slots, alias.source, alias) {
                continue;
            }
            if stability.of(scoping, alias.source) && !roots.contains_key(&alias.symbol) {
                roots.insert(alias.symbol, root);
                progressed = true;
            }
        }
    }
    roots
}

fn resolved_root(
    sites: &ArrayAliasSites,
    roots: &FxHashMap<SymbolId, SymbolId>,
    scoping: &Scoping,
    stability: &mut Stability<'_, '_>,
    alias: &ArrayAlias,
) -> Option<SymbolId> {
    let root = roots.get(&alias.symbol).copied()?;
    if root == alias.symbol {
        return None;
    }
    let (scope, index) = sites.arrays.get(&root).copied()?;
    if scope != alias.scope || index >= alias.index {
        return None;
    }
    let stable = stability.of(scoping, alias.symbol) && stability.of(scoping, root);
    stable.then_some(root)
}

const ROTATION_METHODS: [&str; 2] = ["push", "shift"];

fn alias_never_escapes(uses: &ObjectUses, root: SymbolId, feeds: &Feeds) -> bool {
    uses.writes == 0
        && uses.opaque_spans.iter().all(|span| {
            feeds
                .get(span)
                .is_some_and(|(fed_root, _)| *fed_root == root)
        })
        && uses.reads.iter().all(|read| {
            super::canonical_index(&read.name).is_some()
                || ROTATION_METHODS.contains(&read.name.as_str())
        })
}

type Feeds = FxHashMap<Span, (SymbolId, SymbolId)>;

struct AliasPlan<'k> {
    alias: &'k ArrayAlias,
    root: SymbolId,
    reads: Vec<Span>,
    feeds: Vec<SymbolId>,
}

struct Checking<'c> {
    uses: &'c SymbolUses,
    order: &'c InitOrder,
    feeds: &'c Feeds,
    names: &'c FxHashMap<String, usize>,
}

fn check_one_alias<'k>(
    (alias, root): (&'k ArrayAlias, SymbolId),
    checking: &Checking<'_>,
    cx: &mut Cx<'_, '_>,
) -> Option<AliasPlan<'k>> {
    let default = ObjectUses::default();
    let uses = checking.uses.of(alias.symbol).unwrap_or(&default);
    if !alias_never_escapes(uses, root, checking.feeds) {
        cx.note(notes::ARRAY_ALIAS_ESCAPES, Some(alias.statement));
        return None;
    }
    if let Err(refused) = reads_are_initialised(&uses.reads, alias.statement, checking.order) {
        refuse_read_before_declaration(
            refused,
            alias.symbol,
            alias.statement,
            notes::ARRAY_ALIAS_READ_BEFORE_DECLARATION,
            cx,
        );
        return None;
    }
    let name = cx.scoping().symbol_name(root).to_string();
    let unique = checking.names.get(name.as_str()) == Some(&1);
    if !unique && !root_name_reaches(cx.scoping(), alias.symbol, root) {
        cx.note(
            notes::array_alias_name_not_unique(&name),
            Some(alias.statement),
        );
        return None;
    }
    let reads = uses
        .reads
        .iter()
        .filter(|read| !ROTATION_METHODS.contains(&read.name.as_str()))
        .map(|read| read.member_span)
        .collect();
    let feeds = uses
        .opaque_spans
        .iter()
        .filter_map(|span| checking.feeds.get(span).map(|(_, fed)| *fed))
        .collect();
    Some(AliasPlan {
        alias,
        root,
        reads,
        feeds,
    })
}

fn keep_fed_aliases(plans: &mut Vec<AliasPlan<'_>>, cx: &mut Cx<'_, '_>) {
    let mut planned: FxHashSet<SymbolId> = plans.iter().map(|plan| plan.alias.symbol).collect();
    let mut feeders: FxHashMap<SymbolId, Vec<SymbolId>> = FxHashMap::default();
    let mut lost = Vec::new();
    for plan in plans.iter() {
        for fed in &plan.feeds {
            feeders.entry(*fed).or_default().push(plan.alias.symbol);
            if !planned.contains(fed) {
                lost.push(plan.alias.symbol);
            }
        }
    }
    for symbol in &lost {
        planned.remove(symbol);
    }
    while let Some(symbol) = lost.pop() {
        for feeder in feeders.get(&symbol).into_iter().flatten() {
            if planned.remove(feeder) {
                lost.push(*feeder);
            }
        }
    }
    plans.retain(|plan| {
        let kept = planned.contains(&plan.alias.symbol);
        if !kept {
            cx.note(notes::ARRAY_ALIAS_ESCAPES, Some(plan.alias.statement));
        }
        kept
    });
}

fn keep_value_aliases(program: &Program<'_>, plans: &mut Vec<AliasPlan<'_>>, cx: &mut Cx<'_, '_>) {
    let reads: Vec<Span> = plans
        .iter()
        .flat_map(|plan| plan.reads.iter().copied())
        .collect();
    let consumed = super::positions::reads_not_values(program, &reads);
    if consumed.is_empty() {
        return;
    }
    plans.retain(|plan| {
        let held = plan.reads.iter().any(|span| consumed.contains(span));
        if held {
            cx.note(notes::ARRAY_ALIAS_ESCAPES, Some(plan.alias.statement));
        }
        !held
    });
}

fn keep_unreached_aliases(
    program: &Program<'_>,
    plans: &mut Vec<AliasPlan<'_>>,
    cx: &mut Cx<'_, '_>,
) {
    if plans.is_empty() {
        return;
    }
    let candidates: Vec<(SymbolId, Span)> = plans
        .iter()
        .map(|plan| (plan.alias.symbol, plan.alias.statement))
        .collect();
    let kept = super::reach::reached(program, cx, &candidates);
    plans.retain(|plan| {
        !kept.symbols.contains(&plan.alias.symbol) && !kept.declared.contains(&plan.alias.symbol)
    });
    for (note, span) in kept.notes {
        cx.note(note, Some(span));
    }
    if let Some(span) = kept.observed {
        cx.note(notes::ARRAY_ALIAS_NAME_OBSERVED, Some(span));
    }
}

#[cfg(debug_assertions)]
fn record_unscanned_order(program: &Program<'_>, scoping: &Scoping) {
    if crate::fused::check::active() {
        let digest = crate::init_order::try_scan_init_order(program, scoping)
            .map_or(0, |order| order.digest());
        crate::fused::check::record("alias-order", digest);
    }
}

fn checked_aliases<'k>(
    program: &Program<'_>,
    resolved: &[(&'k ArrayAlias, SymbolId)],
    stability: &mut Stability<'_, '_>,
    cx: &mut Cx<'_, '_>,
) -> Vec<AliasPlan<'k>> {
    if resolved.is_empty() {
        #[cfg(debug_assertions)]
        record_unscanned_order(program, cx.scoping());
        return Vec::new();
    }
    let order = crate::init_order::scan_init_order(program, cx);
    #[cfg(debug_assertions)]
    crate::fused::check::record("alias-order", order.digest());
    let feeds: Feeds = resolved
        .iter()
        .map(|(alias, root)| (alias.init, (*root, alias.symbol)))
        .collect();
    let names = name_counts(cx.scoping(), resolved.iter().map(|(_, root)| *root));
    let checking = Checking {
        uses: stability.uses(cx.scoping()),
        order: &order,
        feeds: &feeds,
        names: &names,
    };
    resolved
        .iter()
        .filter_map(|(alias, root)| check_one_alias((alias, *root), &checking, cx))
        .collect()
}

pub(super) fn resolve_array_aliases<'a>(
    program: &mut Program<'a>,
    cx: &mut Cx<'a, '_>,
    sites: ArrayAliasSites,
) -> FxHashSet<SymbolId> {
    if sites.aliases.is_empty() {
        return FxHashSet::default();
    }
    let slots = declaration_slots(&sites);
    let mut stability = Stability::new(program, &sites);
    let roots = alias_roots(&sites, &slots, &mut stability, cx.scoping());
    let resolved: Vec<(&ArrayAlias, SymbolId)> = sites
        .aliases
        .iter()
        .filter_map(|alias| {
            resolved_root(&sites, &roots, cx.scoping(), &mut stability, alias)
                .map(|root| (alias, root))
        })
        .collect();
    let mut plans = checked_aliases(program, &resolved, &mut stability, cx);
    crate::proof::refuse_annex_b_bindings(
        &mut plans,
        |plan| (plan.alias.statement, [plan.alias.symbol, plan.root]),
        program,
        cx,
    );
    keep_value_aliases(program, &mut plans, cx);
    keep_unreached_aliases(program, &mut plans, cx);
    keep_fed_aliases(&mut plans, cx);
    let mut renames = Renames::new();
    let mut removals = StmtEdits::new();
    let mut renamed_into = FxHashSet::default();
    for plan in &plans {
        let name = cx.scoping().symbol_name(plan.root).to_string();
        renames.insert(plan.alias.symbol, Ident::from_str_in(&name, &*cx));
        removals.remove(plan.alias.statement);
        renamed_into.insert(plan.root);
    }
    if renames.meets_an_unbound_reference(cx.scoping()) {
        return FxHashSet::default();
    }
    let wanted = removals.len();
    let renamed = renames.apply(program, cx.scoping());
    let removed = removals.apply(program, cx.allocator(), cx.scoping_mut());
    cx.changed_by(renamed + removed);
    if removed != wanted {
        cx.note(
            notes::planned_and_applied_alias_removals(wanted, removed),
            None,
        );
    }
    renamed_into
}
