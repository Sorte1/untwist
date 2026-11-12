use oxc_ast::ast::ArrowFunctionBody;
use oxc_ast::AstKind;
use oxc_semantic::{AstNodes, Scoping, Semantic};
use oxc_span::{GetSpan, Span};
use oxc_syntax::node::NodeId;
use oxc_syntax::scope::{ScopeFlags, ScopeId};
use oxc_syntax::symbol::SymbolId;
use rustc_hash::{FxHashMap, FxHashSet};
use untwist_core::semantic::pins::script_global_symbols;
use untwist_core::semantic::{AnnexBFunction, AnnexBWrites};

const MAX_RENAMED_LEN: usize = 3;

type Edits = Vec<(Span, String)>;
type Pairs = Vec<(String, String)>;

pub(super) struct Renamer<'s> {
    semantic: &'s Semantic<'s>,
    pinned: FxHashSet<SymbolId>,
}

impl<'s> Renamer<'s> {
    pub(super) fn new(semantic: &'s Semantic<'s>, script: bool) -> Self {
        let scoping = semantic.scoping();
        let mut pinned = annex_b_pins(semantic);
        if script {
            pinned.extend(script_global_symbols(scoping));
        }
        Self { semantic, pinned }
    }

    fn is_pinned(&self, symbol: SymbolId) -> bool {
        self.pinned.contains(&symbol)
    }

    fn nodes(&self) -> &'s AstNodes<'s> {
        self.semantic.nodes()
    }

    fn scoping(&self) -> &'s Scoping {
        self.semantic.scoping()
    }

    fn is_exported_declaration(&self, id: NodeId) -> bool {
        self.nodes().ancestor_kinds(id).any(|kind| {
            matches!(
                kind,
                AstKind::ExportNamedDeclaration(_)
                    | AstKind::ExportDefaultDeclaration(_)
                    | AstKind::ExportDeclaration(_)
            )
        })
    }

    fn renamable(&self, symbol: SymbolId) -> bool {
        let scoping = self.scoping();
        let nodes = self.nodes();
        if scoping.symbol_name(symbol).len() > MAX_RENAMED_LEN
            || !scoping.symbol_redeclarations(symbol).is_empty()
            || self.is_pinned(symbol)
        {
            return false;
        }
        let declaration = scoping.symbol_declaration(symbol);
        let plain = match nodes.kind(declaration) {
            AstKind::VariableDeclarator(declarator) => {
                declarator.id.get_binding_identifier().is_some()
            }
            AstKind::Function(function) => function.is_declaration(),
            _ => false,
        };
        if !plain || self.is_exported_declaration(declaration) {
            return false;
        }
        scoping.get_resolved_references(symbol).all(|reference| {
            !matches!(
                nodes.parent_kind(reference.node_id()),
                AstKind::ExportDefaultDeclaration(_)
            )
        })
    }

    fn has_hazard(&self) -> bool {
        let scoping = self.scoping();
        scoping.scope_descendants_from_root().any(|scope| {
            scoping
                .scope_flags(scope)
                .intersects(ScopeFlags::DirectEval | ScopeFlags::With)
        })
    }

    fn taken(&self) -> FxHashSet<String> {
        let scoping = self.scoping();
        scoping
            .symbol_names()
            .map(str::to_string)
            .chain(
                scoping
                    .root_unresolved_references()
                    .keys()
                    .map(|name| name.to_string()),
            )
            .collect()
    }

    fn reference_text(&self, id: NodeId, old: &str, name: &str) -> String {
        match self.nodes().parent_kind(id) {
            AstKind::ObjectProperty(property) if property.shorthand => format!("{old}: {name}"),
            AstKind::AssignmentTargetPropertyIdentifier(_) => format!("{old}: {name}"),
            AstKind::ExportSpecifier(specifier) if specifier.exported.name() == old => {
                format!("{name} as {old}")
            }
            _ => name.to_string(),
        }
    }

    fn rename_edits(&self, symbol: SymbolId, name: &str, edits: &mut Vec<(Span, String)>) {
        let scoping = self.scoping();
        let old = scoping.symbol_name(symbol);
        edits.push((scoping.symbol_span(symbol), name.to_string()));
        for reference in scoping.get_resolved_references(symbol) {
            let id = reference.node_id();
            if let AstKind::ExportSpecifier(specifier) = self.nodes().parent_kind(id) {
                if specifier.exported.name() == name {
                    edits.push((specifier.span, name.to_string()));
                    continue;
                }
            }
            let span = self.nodes().kind(id).span();
            edits.push((span, self.reference_text(id, old, name)));
        }
    }

    pub(super) fn apply(
        &self,
        wanted: Vec<(SymbolId, String)>,
        claimed: &mut Claimed,
    ) -> (Edits, Pairs) {
        if self.has_hazard() {
            return (Vec::new(), Vec::new());
        }
        let mut taken = self.taken();
        taken.extend(claimed.names.iter().map(|(_, name)| name.clone()));
        let mut edits = Vec::new();
        let mut applied = Vec::new();
        for (symbol, name) in wanted {
            if claimed.symbols.contains(&symbol)
                || taken.contains(&name)
                || self.scoping().symbol_name(symbol) == name
                || !self.renamable(symbol)
            {
                continue;
            }
            self.rename_edits(symbol, &name, &mut edits);
            applied.push((self.scoping().symbol_name(symbol).to_string(), name.clone()));
            claimed.claim(symbol, None, &name);
            taken.insert(name);
        }
        (edits, applied)
    }

    pub(super) fn apply_by_name(&self, wanted: &[(String, String)]) -> (Edits, Pairs) {
        let scoping = self.scoping();
        let symbols = wanted
            .iter()
            .filter_map(|(old, new)| {
                let mut matching = scoping
                    .symbol_ids()
                    .filter(|symbol| scoping.symbol_name(*symbol) == old);
                match (matching.next(), matching.next()) {
                    (Some(only), None) => Some((only, new.clone())),
                    _ => None,
                }
            })
            .collect();
        self.apply(symbols, &mut Claimed::default())
    }

    fn function_with_body(&self, body: Span) -> Option<Span> {
        self.nodes().iter().find_map(|node| match node.kind() {
            AstKind::Function(function) => function
                .body
                .as_ref()
                .is_some_and(|own| own.span == body)
                .then_some(function.span),
            AstKind::ArrowFunctionExpression(arrow) => match &arrow.body {
                ArrowFunctionBody::FunctionBody(own) => (own.span == body).then_some(arrow.span),
                _ => None,
            },
            _ => None,
        })
    }

    fn has_hazard_within(&self, within: Span) -> bool {
        let scoping = self.scoping();
        scoping.scope_descendants_from_root().any(|scope| {
            scoping
                .scope_flags(scope)
                .intersects(ScopeFlags::DirectEval | ScopeFlags::With)
                && contains(within, self.nodes().kind(scoping.get_node_id(scope)).span())
        })
    }

    fn taken_within(&self, within: Span, declared: &[SymbolId]) -> FxHashSet<String> {
        let scoping = self.scoping();
        let mut taken: FxHashSet<String> = declared
            .iter()
            .map(|symbol| scoping.symbol_name(*symbol).to_string())
            .chain(
                scoping
                    .root_unresolved_references()
                    .keys()
                    .map(|name| name.to_string()),
            )
            .collect();
        for node in self.nodes().iter() {
            let AstKind::IdentifierReference(reference) = node.kind() else {
                continue;
            };
            if !contains(within, reference.span) {
                continue;
            }
            let outside = scoping
                .get_reference(reference.reference_id())
                .symbol_id()
                .is_none_or(|symbol| !contains(within, scoping.symbol_span(symbol)));
            if outside {
                taken.insert(reference.name.to_string());
            }
        }
        taken
    }

    pub(super) fn plan_within(
        &self,
        body: Span,
        pairs: &[(String, String)],
        claimed: &Claimed,
    ) -> Result<Planned, String> {
        let within = self
            .function_with_body(body)
            .ok_or_else(|| "its module function was not found".to_string())?;
        if self.has_hazard_within(within) {
            return Err("its module function holds a direct eval or with".to_string());
        }
        let scoping = self.scoping();
        let declared: Vec<SymbolId> = scoping
            .symbol_ids()
            .filter(|symbol| contains(within, scoping.symbol_span(*symbol)))
            .collect();
        let mut taken = self.taken_within(within, &declared);
        let mut plan = Vec::with_capacity(pairs.len());
        for (old, new) in pairs {
            let mut matching = declared
                .iter()
                .copied()
                .filter(|symbol| scoping.symbol_name(*symbol) == old);
            let symbol = match (matching.next(), matching.next()) {
                (Some(only), None) => only,
                _ => return Err(format!("{old} is not declared once in its module function")),
            };
            if claimed.symbols.contains(&symbol)
                || taken.contains(new)
                || claimed.conflicts(within, new)
                || !self.renamable(symbol)
            {
                return Err(format!("{old} cannot become {new} there"));
            }
            taken.insert(new.clone());
            plan.push((symbol, new.clone()));
        }
        Ok(Planned {
            within,
            renames: plan,
        })
    }

    pub(super) fn commit_within(
        &self,
        planned: Planned,
        claimed: &mut Claimed,
        edits: &mut Vec<(Span, String)>,
    ) -> usize {
        let count = planned.renames.len();
        for (symbol, name) in planned.renames {
            self.rename_edits(symbol, &name, edits);
            claimed.claim(symbol, Some(planned.within), &name);
        }
        count
    }
}

fn annex_b_pins(semantic: &Semantic<'_>) -> FxHashSet<SymbolId> {
    let scoping = semantic.scoping();
    let writes = AnnexBWrites::of(semantic.nodes().program(), scoping);
    let mut pinned = FxHashSet::default();
    let mut lone: FxHashMap<&str, Vec<AnnexBFunction>> = FxHashMap::default();
    for function in writes.functions() {
        match function
            .target
            .filter(|target| *target != function.function)
        {
            Some(target) => {
                pinned.insert(function.function);
                pinned.insert(target);
            }
            None => lone
                .entry(scoping.symbol_name(function.function))
                .or_default()
                .push(*function),
        }
    }
    if lone.is_empty() {
        return pinned;
    }
    for node in semantic.nodes().iter() {
        let AstKind::IdentifierReference(reference) = node.kind() else {
            continue;
        };
        let Some(functions) = lone.get(reference.name.as_str()) else {
            continue;
        };
        let reference = scoping.get_reference(reference.reference_id());
        for function in functions {
            if reference.symbol_id() != Some(function.function)
                && within(scoping, reference.scope_id(), function.var_scope)
            {
                pinned.insert(function.function);
            }
        }
    }
    pinned
}

fn within(scoping: &Scoping, scope: ScopeId, outer: ScopeId) -> bool {
    scoping
        .scope_ancestors(scope)
        .any(|ancestor| ancestor == outer)
}

fn contains(outer: Span, inner: Span) -> bool {
    outer.start <= inner.start && inner.end <= outer.end
}

fn overlaps(a: Span, b: Span) -> bool {
    a.start < b.end && b.start < a.end
}

pub(super) struct Planned {
    within: Span,
    renames: Vec<(SymbolId, String)>,
}

#[derive(Debug, Default)]
pub(super) struct Claimed {
    symbols: FxHashSet<SymbolId>,
    names: Vec<(Option<Span>, String)>,
}

impl Claimed {
    fn claim(&mut self, symbol: SymbolId, within: Option<Span>, name: &str) {
        self.symbols.insert(symbol);
        self.names.push((within, name.to_string()));
    }

    fn conflicts(&self, within: Span, name: &str) -> bool {
        self.names.iter().any(|(scope, claimed)| {
            claimed == name && scope.is_none_or(|scope| overlaps(scope, within))
        })
    }
}
