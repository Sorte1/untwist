use super::*;

pub struct ArrayAlias {
    pub symbol: SymbolId,
    pub source: SymbolId,
    pub statement: Span,
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
            let Some((symbol, source)) = alias_of(statement, self.scoping) else {
                continue;
            };
            if self.scoping.symbol_scope_id(symbol) == scope {
                self.sites.aliases.push(ArrayAlias {
                    symbol,
                    source,
                    statement: statement.span(),
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

fn alias_of(stmt: &Statement<'_>, scoping: &Scoping) -> Option<(SymbolId, SymbolId)> {
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
    Some((symbol, source))
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

fn name_is_unique(scoping: &Scoping, name: &str) -> bool {
    scoping
        .symbol_ids()
        .filter(|symbol| scoping.symbol_name(*symbol) == name)
        .count()
        == 1
}

fn binding_is_never_rewritten(program: &Program<'_>, scoping: &Scoping, symbol: SymbolId) -> bool {
    scoping.symbol_redeclarations(symbol).is_empty()
        && is_constant_binding(scoping, symbol)
        && scan_object_uses(program, scoping, symbol).writes == 0
}

#[derive(Default)]
struct Stability {
    known: FxHashMap<SymbolId, bool>,
}

impl Stability {
    fn of(&mut self, program: &Program<'_>, scoping: &Scoping, symbol: SymbolId) -> bool {
        if let Some(known) = self.known.get(&symbol) {
            return *known;
        }
        let stable = binding_is_never_rewritten(program, scoping, symbol);
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
    program: &Program<'_>,
    stability: &mut Stability,
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
            if stability.of(program, scoping, alias.source) && !roots.contains_key(&alias.symbol) {
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
    program: &Program<'_>,
    scoping: &Scoping,
    stability: &mut Stability,
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
    let stable =
        stability.of(program, scoping, alias.symbol) && stability.of(program, scoping, root);
    stable.then_some(root)
}

fn alias_never_escapes(uses: &ObjectUses) -> bool {
    uses.writes == 0
        && uses.opaque == 0
        && uses
            .reads
            .iter()
            .all(|read| read.name.parse::<usize>().is_ok())
}

struct AliasPlan<'k> {
    alias: &'k ArrayAlias,
    root: SymbolId,
    reads: Vec<Span>,
}

fn check_one_alias<'k>(
    program: &Program<'_>,
    (alias, root): (&'k ArrayAlias, SymbolId),
    order: &InitOrder,
    cx: &mut Cx<'_, '_>,
) -> Option<AliasPlan<'k>> {
    let uses = scan_object_uses(program, cx.scoping(), alias.symbol);
    if !alias_never_escapes(&uses) {
        cx.note(notes::ARRAY_ALIAS_ESCAPES, Some(alias.statement));
        return None;
    }
    if let Err(refused) = reads_are_initialised(&uses.reads, alias.statement, order) {
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
    if !name_is_unique(cx.scoping(), &name) {
        cx.note(
            notes::array_alias_name_not_unique(&name),
            Some(alias.statement),
        );
        return None;
    }
    let reads = uses.reads.iter().map(|read| read.member_span).collect();
    Some(AliasPlan { alias, root, reads })
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
    resolved
        .iter()
        .filter_map(|(alias, root)| check_one_alias(program, (alias, *root), &order, cx))
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
    let mut stability = Stability::default();
    let roots = alias_roots(&sites, &slots, program, &mut stability, cx.scoping());
    let resolved: Vec<(&ArrayAlias, SymbolId)> = sites
        .aliases
        .iter()
        .filter_map(|alias| {
            resolved_root(&sites, &roots, program, cx.scoping(), &mut stability, alias)
                .map(|root| (alias, root))
        })
        .collect();
    let mut plans = checked_aliases(program, &resolved, cx);
    crate::proof::refuse_annex_b_bindings(
        &mut plans,
        |plan| (plan.alias.statement, [plan.alias.symbol, plan.root]),
        program,
        cx,
    );
    keep_value_aliases(program, &mut plans, cx);
    keep_unreached_aliases(program, &mut plans, cx);
    let mut renames = Renames::new();
    let mut removals = StmtEdits::new();
    let mut renamed_into = FxHashSet::default();
    for plan in &plans {
        let name = cx.scoping().symbol_name(plan.root).to_string();
        renames.insert(plan.alias.symbol, Ident::from_str_in(&name, &*cx));
        removals.remove(plan.alias.statement);
        renamed_into.insert(plan.root);
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
