use std::cell::OnceCell;

use oxc_ast::ast::*;
use oxc_ast_visit::{walk, Visit};
use oxc_semantic::Scoping;
use oxc_span::{GetSpan, Span};
use oxc_str::Ident;
use oxc_syntax::operator::AssignmentOperator;
use oxc_syntax::reference::ReferenceId;
use oxc_syntax::scope::{ScopeFlags, ScopeId};
use oxc_syntax::symbol::{SymbolFlags, SymbolId};
use rustc_hash::FxHashSet;
use untwist_core::pass::Cx;

pub(crate) fn hoist_needs_braces(
    branch: &Statement<'_>,
    renamed: &[SymbolId],
    scoping: &Scoping,
) -> bool {
    if renamed.is_empty() {
        return false;
    }
    let names: FxHashSet<&str> = renamed
        .iter()
        .map(|symbol| scoping.symbol_name(*symbol))
        .collect();
    let mut finder = BlockFunctions {
        names: &names,
        found: false,
    };
    finder.visit_statement(branch);
    finder.found
}

pub(crate) struct DroppedBranch {
    span: Span,
    references: FxHashSet<ReferenceId>,
}

impl DroppedBranch {
    pub(crate) fn of(statement: Option<&Statement<'_>>) -> Option<Self> {
        let statement = statement?;
        let mut references = ReferencesIn::default();
        references.visit_statement(statement);
        Some(Self {
            span: statement.span(),
            references: references.ids,
        })
    }

    fn holds(&self, span: Span) -> bool {
        !span.is_empty() && self.span.start <= span.start && span.end <= self.span.end
    }
}

#[derive(Default)]
struct ReferencesIn {
    ids: FxHashSet<ReferenceId>,
}

impl<'a> Visit<'a> for ReferencesIn {
    fn visit_identifier_reference(&mut self, it: &IdentifierReference<'a>) {
        if let Some(id) = it.reference_id.get() {
            self.ids.insert(id);
        }
    }
}

pub(crate) fn shadows_dropped(
    name: &str,
    outer: ScopeId,
    (dropped, branch): (&FxHashSet<ReferenceId>, Option<&DroppedBranch>),
    scoping: &Scoping,
) -> bool {
    let ident = Ident::from(name);
    let Some((scope, symbol)) = scoping.scope_ancestors(outer).find_map(|scope| {
        scoping
            .get_binding(scope, ident)
            .map(|symbol| (scope, symbol))
    }) else {
        return false;
    };
    let flags = scoping.symbol_flags(symbol);
    let references = scoping.get_resolved_reference_ids(symbol);
    if flags.contains(SymbolFlags::BlockScopedVariable) && !flags.is_function() && !flags.is_class()
    {
        return scope != outer
            && references
                .iter()
                .all(|reference| dropped.contains(reference));
    }
    let plain_var = flags.is_function_scoped_declaration()
        && !flags.is_function()
        && !flags.is_class()
        && !flags.is_catch_variable();
    let Some(branch) = branch.filter(|_| plain_var) else {
        return false;
    };
    let declared_there = branch.holds(scoping.symbol_span(symbol))
        && scoping
            .symbol_redeclarations(symbol)
            .iter()
            .all(|redeclaration| branch.holds(redeclaration.span));
    declared_there
        && references
            .iter()
            .all(|reference| branch.references.contains(reference))
}

pub(crate) struct Givers<'b, 'a> {
    branch: &'b Statement<'a>,
    scoping: &'b Scoping,
    observed: bool,
    found: OnceCell<FxHashSet<SymbolId>>,
}

impl<'b, 'a> Givers<'b, 'a> {
    pub(crate) fn new(branch: &'b Statement<'a>, cx: &'b Cx<'a, '_>) -> Self {
        Self {
            branch,
            scoping: cx.scoping(),
            observed: cx.names_observed(),
            found: OnceCell::new(),
        }
    }

    pub(crate) fn gives(&self, symbol: SymbolId) -> bool {
        if !self.observed {
            return false;
        }
        self.scoping.symbol_flags(symbol).is_class()
            || self
                .found
                .get_or_init(|| named_values(self.branch, self.scoping))
                .contains(&symbol)
    }
}

fn named_values(branch: &Statement<'_>, scoping: &Scoping) -> FxHashSet<SymbolId> {
    let mut finder = NamedEvaluation {
        scoping,
        found: FxHashSet::default(),
    };
    finder.visit_statement(branch);
    finder.found
}

struct NamedEvaluation<'n> {
    scoping: &'n Scoping,
    found: FxHashSet<SymbolId>,
}

impl NamedEvaluation<'_> {
    fn note(&mut self, symbol: Option<SymbolId>, value: Option<&Expression<'_>>) {
        if let (Some(symbol), true) = (
            symbol,
            value.is_some_and(Expression::is_anonymous_function_definition),
        ) {
            self.found.insert(symbol);
        }
    }

    fn target(&self, id: &IdentifierReference<'_>) -> Option<SymbolId> {
        id.reference_id
            .get()
            .and_then(|reference| self.scoping.get_reference(reference).symbol_id())
    }
}

impl<'a> Visit<'a> for NamedEvaluation<'_> {
    fn visit_variable_declarator(&mut self, it: &VariableDeclarator<'a>) {
        if let Some(id) = it.id.get_binding_identifier() {
            self.note(id.symbol_id.get(), it.init.as_ref());
        }
        walk::walk_variable_declarator(self, it);
    }

    fn visit_assignment_pattern(&mut self, it: &AssignmentPattern<'a>) {
        if let Some(id) = it.left.get_binding_identifier() {
            self.note(id.symbol_id.get(), Some(&it.right));
        }
        walk::walk_assignment_pattern(self, it);
    }

    fn visit_assignment_expression(&mut self, it: &AssignmentExpression<'a>) {
        let naming = matches!(
            it.operator,
            AssignmentOperator::Assign
                | AssignmentOperator::LogicalAnd
                | AssignmentOperator::LogicalOr
                | AssignmentOperator::LogicalNullish
        );
        if let (true, AssignmentTarget::AssignmentTargetIdentifier(id)) = (naming, &it.left) {
            let symbol = self.target(id);
            self.note(symbol, Some(&it.right));
        }
        walk::walk_assignment_expression(self, it);
    }

    fn visit_assignment_target_with_default(&mut self, it: &AssignmentTargetWithDefault<'a>) {
        if let AssignmentTarget::AssignmentTargetIdentifier(id) = &it.binding {
            let symbol = self.target(id);
            self.note(symbol, Some(&it.init));
        }
        walk::walk_assignment_target_with_default(self, it);
    }

    fn visit_assignment_target_property_identifier(
        &mut self,
        it: &AssignmentTargetPropertyIdentifier<'a>,
    ) {
        let symbol = self.target(&it.binding);
        self.note(symbol, it.init.as_ref());
        walk::walk_assignment_target_property_identifier(self, it);
    }
}

struct BlockFunctions<'n> {
    names: &'n FxHashSet<&'n str>,
    found: bool,
}

impl<'a> Visit<'a> for BlockFunctions<'_> {
    fn visit_function(&mut self, it: &Function<'a>, _flags: ScopeFlags) {
        let named = it
            .id
            .as_ref()
            .is_some_and(|id| self.names.contains(id.name.as_str()));
        self.found |= it.is_declaration() && named;
    }

    fn visit_arrow_function_expression(&mut self, _it: &ArrowFunctionExpression<'a>) {}

    fn visit_class(&mut self, _it: &Class<'a>) {}
}
