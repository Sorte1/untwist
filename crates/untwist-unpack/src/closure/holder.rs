use oxc_allocator::{Allocator, Vec as AVec};
use oxc_ast::ast::*;
use oxc_ast::builder::AstBuilder;
use oxc_ast_visit::{walk_js, walk_mut, VisitJs, VisitMut};
use oxc_semantic::Scoping;
use oxc_span::Span;
use oxc_syntax::number::NumberBase;
use oxc_syntax::scope::{ScopeFlags, ScopeId};
use oxc_syntax::symbol::{SymbolFlags, SymbolId};
use rustc_hash::{FxHashMap, FxHashSet};

use crate::closure::roles::Role;

const HOLDER: &str = "__closure_vars";
const NAMESPACE: &str = "__closure_namespace";

fn fresh_name(base: &str, source_text: &str) -> String {
    (1..)
        .map(|index| match index {
            1 => base.to_string(),
            index => format!("{base}{index}"),
        })
        .find(|name| !source_text.contains(name.as_str()))
        .unwrap_or_default()
}

pub fn holder_name(source_text: &str) -> String {
    fresh_name(HOLDER, source_text)
}

pub fn namespace_name(source_text: &str) -> String {
    fresh_name(NAMESPACE, source_text)
}

struct Seen<'s, 'a> {
    scoping: &'s Scoping,
    wrapper: ScopeId,
    segment: usize,
    depth: u32,
    patterns: u32,
    uses: FxHashMap<SymbolId, Vec<usize>>,
    bad: FxHashSet<SymbolId>,
    unresolved: FxHashSet<&'a str>,
    block_functions: FxHashSet<&'a str>,
    with: bool,
}

impl<'a> Seen<'_, 'a> {
    fn ours(&self, symbol: SymbolId) -> bool {
        self.scoping.symbol_scope_id(symbol) == self.wrapper
    }

    fn note(&mut self, symbol: SymbolId) {
        if !self.ours(symbol) {
            return;
        }
        let segments = self.uses.entry(symbol).or_default();
        if segments.last() != Some(&self.segment) {
            segments.push(self.segment);
        }
    }

    fn mark(&mut self, symbol: Option<SymbolId>) {
        if let Some(symbol) = symbol.filter(|&symbol| self.ours(symbol)) {
            self.bad.insert(symbol);
        }
    }

    fn mark_pattern(&mut self, pattern: &BindingPattern<'a>) {
        for id in pattern.get_binding_identifiers() {
            self.mark(id.symbol_id.get());
        }
    }

    fn mark_declaration(&mut self, declaration: &VariableDeclaration<'a>) {
        for declarator in &declaration.declarations {
            self.mark_pattern(&declarator.id);
        }
    }

    fn reference(&self, id: &IdentifierReference<'_>) -> Option<SymbolId> {
        let reference = id.reference_id.get()?;
        self.scoping.get_reference(reference).symbol_id()
    }
}

impl<'a> VisitJs<'a> for Seen<'_, 'a> {
    fn visit_identifier_reference(&mut self, it: &IdentifierReference<'a>) {
        let symbol = self.reference(it);
        match symbol {
            Some(symbol) => self.note(symbol),
            None => {
                self.unresolved.insert(it.name.as_str());
            }
        }
        let written = it
            .reference_id
            .get()
            .is_some_and(|reference| self.scoping.get_reference(reference).is_write());
        if self.patterns > 0 && written {
            self.mark(symbol);
        }
    }

    fn visit_binding_identifier(&mut self, it: &BindingIdentifier<'a>) {
        if let Some(symbol) = it.symbol_id.get() {
            self.note(symbol);
        }
    }

    fn visit_function(&mut self, it: &Function<'a>, flags: ScopeFlags) {
        if self.depth == 0 && it.is_declaration() {
            if let Some(id) = &it.id {
                self.block_functions.insert(id.name.as_str());
            }
        }
        self.depth += 1;
        walk_js::walk_function(self, it, flags);
        self.depth -= 1;
    }

    fn visit_arrow_function_expression(&mut self, it: &ArrowFunctionExpression<'a>) {
        self.depth += 1;
        walk_js::walk_arrow_function_expression(self, it);
        self.depth -= 1;
    }

    fn visit_with_statement(&mut self, it: &WithStatement<'a>) {
        self.with = true;
        walk_js::walk_with_statement(self, it);
    }

    fn visit_for_statement(&mut self, it: &ForStatement<'a>) {
        if let Some(ForStatementInit::VariableDeclaration(declaration)) = &it.init {
            self.mark_declaration(declaration);
        }
        walk_js::walk_for_statement(self, it);
    }

    fn visit_for_in_statement(&mut self, it: &ForInStatement<'a>) {
        if let ForStatementLeft::VariableDeclaration(declaration) = &it.left {
            self.mark_declaration(declaration);
        }
        walk_js::walk_for_in_statement(self, it);
    }

    fn visit_for_of_statement(&mut self, it: &ForOfStatement<'a>) {
        if let ForStatementLeft::VariableDeclaration(declaration) = &it.left {
            self.mark_declaration(declaration);
        }
        walk_js::walk_for_of_statement(self, it);
    }

    fn visit_variable_declaration(&mut self, it: &VariableDeclaration<'a>) {
        let patterned = it
            .declarations
            .iter()
            .any(|declarator| !matches!(declarator.id, BindingPattern::BindingIdentifier(_)));
        if patterned || it.kind != VariableDeclarationKind::Var {
            self.mark_declaration(it);
        }
        walk_js::walk_variable_declaration(self, it);
    }

    fn visit_object_property(&mut self, it: &ObjectProperty<'a>) {
        if it.shorthand {
            if let Expression::Identifier(id) = &it.value {
                self.mark(self.reference(id));
            }
        }
        walk_js::walk_object_property(self, it);
    }

    fn visit_array_assignment_target(&mut self, it: &ArrayAssignmentTarget<'a>) {
        self.patterns += 1;
        walk_js::walk_array_assignment_target(self, it);
        self.patterns -= 1;
    }

    fn visit_object_assignment_target(&mut self, it: &ObjectAssignmentTarget<'a>) {
        self.patterns += 1;
        walk_js::walk_object_assignment_target(self, it);
        self.patterns -= 1;
    }

    fn visit_unary_expression(&mut self, it: &UnaryExpression<'a>) {
        if it.operator == UnaryOperator::Delete {
            if let Expression::Identifier(id) = it.argument.without_parentheses() {
                self.mark(self.reference(id));
            }
        }
        walk_js::walk_unary_expression(self, it);
    }
}

pub fn shared_vars(
    scoping: &Scoping,
    wrapper: ScopeId,
    (statements, roles): (&[Statement<'_>], &[Role]),
    excluded: &[&str],
) -> Option<FxHashSet<String>> {
    let mut seen = Seen {
        scoping,
        wrapper,
        segment: 0,
        depth: 0,
        patterns: 0,
        uses: FxHashMap::default(),
        bad: FxHashSet::default(),
        unresolved: FxHashSet::default(),
        block_functions: FxHashSet::default(),
        with: false,
    };
    for (index, (statement, role)) in statements.iter().zip(roles).enumerate() {
        if *role != Role::Prelude {
            seen.segment = index;
            seen.visit_statement(statement);
        }
    }
    let annex_b = seen
        .block_functions
        .iter()
        .any(|name| seen.unresolved.contains(name));
    if seen.with || annex_b {
        return None;
    }
    let mut names = FxHashSet::default();
    for (symbol, segments) in &seen.uses {
        let name = scoping.symbol_name(*symbol);
        if segments.len() < 2 || excluded.contains(&name) {
            continue;
        }
        let var_only = scoping.symbol_flags(*symbol) == SymbolFlags::FunctionScopedVariable;
        if !var_only || seen.bad.contains(symbol) {
            return None;
        }
        names.insert(name.to_string());
    }
    Some(names)
}

struct Rewrite<'s, 'a> {
    scoping: &'s Scoping,
    targets: FxHashSet<SymbolId>,
    holder: &'a str,
    builder: AstBuilder<'a>,
}

impl<'a> Rewrite<'_, 'a> {
    fn target(&self, id: &IdentifierReference<'_>) -> bool {
        id.reference_id
            .get()
            .and_then(|reference| self.scoping.get_reference(reference).symbol_id())
            .is_some_and(|symbol| self.targets.contains(&symbol))
    }

    fn holder(&self, span: Span) -> Expression<'a> {
        Expression::new_identifier(span, self.holder, &self.builder)
    }

    fn property(&self, span: Span, name: &'a str) -> IdentifierName<'a> {
        IdentifierName::new(span, name, &self.builder)
    }

    fn member(&self, span: Span, name: &'a str) -> Expression<'a> {
        Expression::new_static_member_expression(
            span,
            self.holder(span),
            self.property(span, name),
            false,
            &self.builder,
        )
    }

    fn unbound(&self, span: Span, name: &'a str) -> Expression<'a> {
        let zero =
            Expression::new_numeric_literal(span, 0.0, None, NumberBase::Decimal, &self.builder);
        let mut list = AVec::with_capacity_in(2, &self.builder);
        list.push(zero);
        list.push(self.member(span, name));
        Expression::new_sequence_expression(span, list, &self.builder)
    }

    fn callee(&self, callee: &mut Expression<'a>) {
        if let Expression::Identifier(id) = callee {
            if self.target(id) {
                let (span, name) = (id.span, id.name.as_str());
                *callee = self.unbound(span, name);
            }
        }
    }

    fn declaration(&self, declaration: &mut VariableDeclaration<'a>) -> Option<Statement<'a>> {
        let shared = |declarator: &VariableDeclarator<'_>| match &declarator.id {
            BindingPattern::BindingIdentifier(id) => id
                .symbol_id
                .get()
                .is_some_and(|symbol| self.targets.contains(&symbol)),
            _ => false,
        };
        if !declaration.declarations.iter().any(shared) {
            return None;
        }
        let span = declaration.span;
        let mut private = AVec::new_in(&self.builder);
        let mut assignments = AVec::new_in(&self.builder);
        for declarator in declaration.declarations.drain(..) {
            let is_shared = shared(&declarator);
            let BindingPattern::BindingIdentifier(id) = declarator.id else {
                return None;
            };
            let name = id.name.as_str();
            if !is_shared {
                private.push(VariableDeclarator::new(
                    declarator.span,
                    BindingPattern::BindingIdentifier(id),
                    None,
                    None,
                    false,
                    &self.builder,
                ));
            }
            let Some(init) = declarator.init else {
                continue;
            };
            let target = match is_shared {
                true => SimpleAssignmentTarget::new_static_member_expression(
                    declarator.span,
                    self.holder(declarator.span),
                    self.property(declarator.span, name),
                    false,
                    &self.builder,
                ),
                false => SimpleAssignmentTarget::new_assignment_target_identifier(
                    declarator.span,
                    name,
                    &self.builder,
                ),
            };
            assignments.push(Expression::new_assignment_expression(
                declarator.span,
                AssignmentOperator::Assign,
                AssignmentTarget::from(target),
                init,
                &self.builder,
            ));
        }
        let mut statements = AVec::new_in(&self.builder);
        if !private.is_empty() {
            statements.push(Statement::new_variable_declaration(
                span,
                VariableDeclarationKind::Var,
                private,
                false,
                &self.builder,
            ));
        }
        match assignments.len() {
            0 => {}
            1 => statements.push(Statement::new_expression_statement(
                span,
                assignments.pop()?,
                &self.builder,
            )),
            _ => statements.push(Statement::new_expression_statement(
                span,
                Expression::new_sequence_expression(span, assignments, &self.builder),
                &self.builder,
            )),
        }
        Some(match statements.len() {
            0 => Statement::new_empty_statement(span, &self.builder),
            1 => statements.pop()?,
            _ => Statement::new_block_statement(span, statements, &self.builder),
        })
    }
}

impl<'a> VisitMut<'a> for Rewrite<'_, 'a> {
    fn visit_statement(&mut self, it: &mut Statement<'a>) {
        if let Statement::VariableDeclaration(declaration) = it {
            if let Some(replacement) = self.declaration(declaration) {
                *it = replacement;
            }
        }
        walk_mut::walk_statement(self, it);
    }

    fn visit_expression(&mut self, it: &mut Expression<'a>) {
        if let Expression::Identifier(id) = it {
            if self.target(id) {
                let (span, name) = (id.span, id.name.as_str());
                *it = self.member(span, name);
                return;
            }
        }
        walk_mut::walk_expression(self, it);
    }

    fn visit_call_expression(&mut self, it: &mut CallExpression<'a>) {
        self.callee(&mut it.callee);
        walk_mut::walk_call_expression(self, it);
    }

    fn visit_tagged_template_expression(&mut self, it: &mut TaggedTemplateExpression<'a>) {
        self.callee(&mut it.tag);
        walk_mut::walk_tagged_template_expression(self, it);
    }

    fn visit_simple_assignment_target(&mut self, it: &mut SimpleAssignmentTarget<'a>) {
        if let SimpleAssignmentTarget::AssignmentTargetIdentifier(id) = it {
            if self.target(id) {
                let (span, name) = (id.span, id.name.as_str());
                *it = SimpleAssignmentTarget::new_static_member_expression(
                    span,
                    self.holder(span),
                    self.property(span, name),
                    false,
                    &self.builder,
                );
                return;
            }
        }
        walk_mut::walk_simple_assignment_target(self, it);
    }
}

pub fn rewrite_shared<'a>(
    alloc: &'a Allocator,
    scoping: &Scoping,
    statements: &mut [Statement<'a>],
    (names, holder): (&FxHashSet<String>, &str),
) {
    let root = scoping.root_scope_id();
    let targets = scoping
        .get_bindings(root)
        .iter()
        .filter(|(name, _)| names.contains(name.as_str()))
        .map(|(_, &symbol)| symbol)
        .collect();
    let mut rewrite = Rewrite {
        scoping,
        targets,
        holder: alloc.alloc_str(holder),
        builder: AstBuilder::new(alloc),
    };
    for statement in statements {
        rewrite.visit_statement(statement);
    }
}
