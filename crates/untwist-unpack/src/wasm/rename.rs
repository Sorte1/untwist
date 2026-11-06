use oxc_ast::AstKind;
use oxc_semantic::{AstNodes, Scoping, Semantic};
use oxc_span::{GetSpan, Span};
use oxc_syntax::node::NodeId;
use oxc_syntax::scope::ScopeFlags;
use oxc_syntax::symbol::SymbolId;
use rustc_hash::FxHashSet;

const MAX_RENAMED_LEN: usize = 3;

pub(super) struct Renamer<'s> {
    semantic: &'s Semantic<'s>,
}

impl<'s> Renamer<'s> {
    pub(super) fn new(semantic: &'s Semantic<'s>) -> Self {
        Self { semantic }
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
            let span = self.nodes().kind(id).span();
            edits.push((span, self.reference_text(id, old, name)));
        }
    }

    pub(super) fn apply(&self, wanted: Vec<(SymbolId, String)>) -> (Vec<(Span, String)>, usize) {
        if self.has_hazard() {
            return (Vec::new(), 0);
        }
        let mut taken = self.taken();
        let mut done = FxHashSet::default();
        let mut edits = Vec::new();
        let mut renamed = 0;
        for (symbol, name) in wanted {
            if done.contains(&symbol)
                || taken.contains(&name)
                || self.scoping().symbol_name(symbol) == name
                || !self.renamable(symbol)
            {
                continue;
            }
            done.insert(symbol);
            self.rename_edits(symbol, &name, &mut edits);
            taken.insert(name);
            renamed += 1;
        }
        (edits, renamed)
    }

    pub(super) fn apply_by_name(
        &self,
        wanted: &[(String, String)],
    ) -> (Vec<(Span, String)>, usize) {
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
        self.apply(symbols)
    }
}
