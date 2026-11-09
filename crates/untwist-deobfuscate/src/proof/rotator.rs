use oxc_ast::ast::*;
use oxc_ast_visit::{walk_js, VisitJs};
use oxc_semantic::Scoping;
use oxc_span::{GetSpan, Span};
use oxc_syntax::scope::{ScopeFlags, ScopeId};
use oxc_syntax::symbol::SymbolId;
use rustc_hash::FxHashSet;
use std::borrow::Cow;
use untwist_core::ast_util::js_text::js_text;
use untwist_core::semantic::with_observes;

use super::foreign::ForeignUnits;
use super::seal::sealed;
use super::Refusal;
use crate::mutants::{
    builtin_imports_dropped, declared_machinery_trusted, inert_prefix_dropped,
    object_values_unchecked, rotator_placement_dropped,
};
use crate::notes;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RotatorSite {
    pub span: Span,
    address: usize,
}

impl RotatorSite {
    pub fn of(statement: &Statement<'_>) -> Self {
        Self {
            span: statement.span(),
            address: std::ptr::from_ref(statement) as usize,
        }
    }

    pub fn holds(&self, program: &Program<'_>, rotator: &Expression<'_>) -> bool {
        let Some(Statement::ExpressionStatement(statement)) = self.statement(program) else {
            return false;
        };
        std::ptr::eq(&statement.expression, rotator)
    }

    pub(super) fn statement<'s, 'p>(&self, program: &'s Program<'p>) -> Option<&'s Statement<'p>> {
        self.locate(program)
            .and_then(|list| list.statements.get(list.index))
    }

    fn position(&self, statements: &[Statement<'_>]) -> Option<usize> {
        statements
            .iter()
            .position(|statement| std::ptr::from_ref(statement) as usize == self.address)
    }

    fn locate<'s, 'p>(&self, program: &'s Program<'p>) -> Option<SiteList<'s, 'p>> {
        if let Some(index) = self.position(&program.body) {
            return Some(SiteList {
                statements: &program.body,
                index,
                scope: None,
            });
        }
        let mut search = BodySearch {
            site: *self,
            found: None,
        };
        search.visit_program(program);
        search.found
    }
}

struct SiteList<'s, 'p> {
    statements: &'s [Statement<'p>],
    index: usize,
    scope: Option<ScopeId>,
}

impl SiteList<'_, '_> {
    fn same_list(&self, other: &SiteList<'_, '_>) -> bool {
        std::ptr::eq(
            self.statements.as_ptr().cast::<u8>(),
            other.statements.as_ptr().cast(),
        )
    }

    fn binds_all(&self, scoping: &Scoping, allowed: &FxHashSet<SymbolId>) -> bool {
        self.scope.is_none_or(|scope| {
            allowed
                .iter()
                .all(|symbol| scoping.symbol_scope_id(*symbol) == scope)
        })
    }
}

struct BodySearch<'s, 'p> {
    site: RotatorSite,
    found: Option<SiteList<'s, 'p>>,
}

impl<'s, 'p: 's> BodySearch<'s, 'p> {
    fn holds(&mut self, body: &'p FunctionBody<'p>, scope: Option<ScopeId>) -> bool {
        let (Some(index), Some(scope)) = (self.site.position(&body.statements), scope) else {
            return false;
        };
        self.found = Some(SiteList {
            statements: &body.statements,
            index,
            scope: Some(scope),
        });
        true
    }
}

impl<'s, 'p: 's> VisitJs<'p> for BodySearch<'s, 'p> {
    fn visit_function(&mut self, it: &Function<'p>, flags: ScopeFlags) {
        if self.found.is_some() {
            return;
        }
        let function: &'p Function<'p> = self.alloc(it);
        if let Some(body) = &function.body {
            if self.holds(body, function.scope_id.get()) {
                return;
            }
        }
        walk_js::walk_function(self, it, flags);
    }

    fn visit_arrow_function_expression(&mut self, it: &ArrowFunctionExpression<'p>) {
        if self.found.is_some() {
            return;
        }
        let arrow: &'p ArrowFunctionExpression<'p> = self.alloc(it);
        if let ArrowFunctionBody::FunctionBody(body) = &arrow.body {
            if self.holds(body, arrow.scope_id.get()) {
                return;
            }
        }
        walk_js::walk_arrow_function_expression(self, it);
    }
}

pub struct LiteralBindings {
    symbols: FxHashSet<SymbolId>,
    scope: Option<ScopeId>,
}

fn is_primitive_literal(expr: &Expression<'_>) -> bool {
    match expr.without_parentheses() {
        Expression::NumericLiteral(_)
        | Expression::StringLiteral(_)
        | Expression::BooleanLiteral(_)
        | Expression::NullLiteral(_)
        | Expression::BigIntLiteral(_) => true,
        Expression::TemplateLiteral(template) => template.expressions.is_empty(),
        _ => false,
    }
}

fn literal_binding(declarator: &VariableDeclarator<'_>, scoping: &Scoping) -> Option<SymbolId> {
    let (BindingPattern::BindingIdentifier(id), Some(init)) = (&declarator.id, &declarator.init)
    else {
        return None;
    };
    let symbol = id.symbol_id.get()?;
    let written = scoping
        .get_resolved_reference_ids(symbol)
        .iter()
        .any(|reference| scoping.get_reference(*reference).is_write());
    (is_primitive_literal(init) && !written).then_some(symbol)
}

impl LiteralBindings {
    pub fn new(program: &Program<'_>, scoping: &Scoping) -> Self {
        Self::of(&program.body, scoping, None)
    }

    fn of(statements: &[Statement<'_>], scoping: &Scoping, scope: Option<ScopeId>) -> Self {
        let symbols = statements
            .iter()
            .filter_map(|statement| match statement {
                Statement::VariableDeclaration(declaration) => Some(&declaration.declarations),
                _ => None,
            })
            .flatten()
            .filter_map(|declarator| literal_binding(declarator, scoping))
            .collect();
        Self { symbols, scope }
    }

    fn primitive_identifier(&self, scoping: &Scoping, id: &IdentifierReference<'_>) -> bool {
        match id
            .reference_id
            .get()
            .map(|r| scoping.get_reference(r).symbol_id())
        {
            Some(Some(symbol)) => self.symbols.contains(&symbol),
            Some(None) => matches!(id.name.as_str(), "undefined" | "NaN" | "Infinity"),
            None => false,
        }
    }

    fn primitive(&self, scoping: &Scoping, expr: &Expression<'_>) -> bool {
        match expr.without_parentheses() {
            Expression::Identifier(id) => self.primitive_identifier(scoping, id),
            Expression::UnaryExpression(unary) => {
                unary.operator != UnaryOperator::Delete && self.primitive(scoping, &unary.argument)
            }
            Expression::BinaryExpression(binary) => {
                !matches!(
                    binary.operator,
                    BinaryOperator::In | BinaryOperator::Instanceof
                ) && self.primitive(scoping, &binary.left)
                    && self.primitive(scoping, &binary.right)
            }
            Expression::LogicalExpression(logical) => {
                self.primitive(scoping, &logical.left) && self.primitive(scoping, &logical.right)
            }
            other => is_primitive_literal(other),
        }
    }

    pub fn runs_no_code(&self, scoping: &Scoping, expr: &Expression<'_>) -> bool {
        match expr.without_parentheses() {
            Expression::RegExpLiteral(_) => true,
            Expression::ArrayExpression(array) => array.elements.iter().all(|element| {
                matches!(element, ArrayExpressionElement::Elision(_))
                    || element
                        .as_expression()
                        .is_some_and(|e| self.runs_no_code(scoping, e))
            }),
            Expression::ObjectExpression(object) => object.properties.iter().all(|property| {
                let ObjectPropertyKind::ObjectProperty(property) = property else {
                    return false;
                };
                !property.computed
                    && !property.method
                    && property.kind == PropertyKind::Init
                    && (object_values_unchecked()
                        || self.runs_no_code(scoping, &property.value)
                        || hoisted_function(scoping, &property.value, self.scope))
            }),
            other => self.primitive(scoping, other),
        }
    }

    pub fn inert_statement(&self, scoping: &Scoping, stmt: &Statement<'_>) -> bool {
        match stmt {
            Statement::FunctionDeclaration(_) | Statement::EmptyStatement(_) => true,
            Statement::ImportDeclaration(_)
            | Statement::ExportAllDeclaration(_)
            | Statement::ExportFromDeclaration(_) => loads_a_codeless_builtin(stmt),
            Statement::ExpressionStatement(expression) => {
                inert_prefix_dropped() || self.runs_no_code(scoping, &expression.expression)
            }
            Statement::BlockStatement(block) => {
                inert_prefix_dropped()
                    || block.body.iter().all(|inner| {
                        !matches!(inner, Statement::FunctionDeclaration(_))
                            && self.inert_statement(scoping, inner)
                    })
            }
            Statement::VariableDeclaration(declaration) => {
                !declaration.kind.is_using()
                    && declaration.declarations.iter().all(|d| {
                        matches!(d.id, BindingPattern::BindingIdentifier(_))
                            && d.init.as_ref().is_none_or(|init| {
                                self.runs_no_code(scoping, init)
                                    || hoisted_function(scoping, init, self.scope)
                                    || creates_a_function(init)
                            })
                    })
            }
            _ => false,
        }
    }
}

fn creates_a_function(init: &Expression<'_>) -> bool {
    declared_machinery_trusted()
        || matches!(
            init.without_parentheses(),
            Expression::FunctionExpression(_)
        )
}

fn declared_before(prefix: &[Statement<'_>]) -> FxHashSet<SymbolId> {
    prefix
        .iter()
        .filter_map(|statement| match statement {
            Statement::VariableDeclaration(declaration) if !declaration.kind.is_using() => {
                Some(&declaration.declarations)
            }
            _ => None,
        })
        .flatten()
        .filter(|declarator| declarator.init.as_ref().is_some_and(creates_a_function))
        .filter_map(|declarator| match &declarator.id {
            BindingPattern::BindingIdentifier(id) => id.symbol_id.get(),
            _ => None,
        })
        .collect()
}

fn hoisted_function(scoping: &Scoping, expr: &Expression<'_>, scope: Option<ScopeId>) -> bool {
    let Expression::Identifier(id) = expr.without_parentheses() else {
        return false;
    };
    id.reference_id
        .get()
        .and_then(|reference| scoping.get_reference(reference).symbol_id())
        .is_some_and(|symbol| {
            scoping.symbol_flags(symbol).is_function()
                && scoping.symbol_scope_id(symbol) == scope.unwrap_or(scoping.root_scope_id())
        })
}

fn trivially_inert(stmt: &Statement<'_>) -> bool {
    matches!(
        stmt,
        Statement::FunctionDeclaration(_) | Statement::EmptyStatement(_)
    )
}

fn first_active<'s, 'a>(
    program: &Program<'a>,
    scoping: &Scoping,
    (prefix, scope): (&'s [Statement<'a>], Option<ScopeId>),
    foreign: Option<(&ForeignUnits<'_, 'a>, usize)>,
) -> Option<&'s Statement<'a>> {
    if prefix.iter().all(trivially_inert) {
        return None;
    }
    let literals = match scope {
        None => LiteralBindings::new(program, scoping),
        Some(_) => LiteralBindings::of(prefix, scoping, scope),
    };
    prefix.iter().find(|statement| {
        !literals.inert_statement(scoping, statement)
            && !foreign.is_some_and(|(units, judged)| {
                units.admits(program, scoping, &literals, statement, judged)
            })
    })
}

const CODELESS_BUILTINS: [&str; 2] = ["node:path", "node:url"];

fn loaded_module<'s>(statement: &'s Statement<'_>) -> Option<(Cow<'s, str>, bool)> {
    match statement {
        Statement::ImportDeclaration(import) => Some((
            js_text(import.source.value),
            import.with_clause.is_none() && import.phase.is_none(),
        )),
        Statement::ExportAllDeclaration(export) => {
            Some((js_text(export.source.value), export.with_clause.is_none()))
        }
        Statement::ExportFromDeclaration(export) => {
            Some((js_text(export.source.value), export.with_clause.is_none()))
        }
        _ => None,
    }
}

fn loads_a_codeless_builtin(statement: &Statement<'_>) -> bool {
    loaded_module(statement).is_some_and(|(source, plain)| {
        plain && (builtin_imports_dropped() || CODELESS_BUILTINS.contains(&source.as_ref()))
    })
}

fn imports(statement: &Statement<'_>) -> bool {
    loaded_module(statement).is_some() && !loads_a_codeless_builtin(statement)
}

pub(super) fn uninitialised(
    scoping: &Scoping,
    allowed: &FxHashSet<SymbolId>,
    initialised: &FxHashSet<SymbolId>,
) -> Option<Span> {
    allowed
        .iter()
        .filter(|symbol| !scoping.symbol_flags(**symbol).is_function())
        .filter(|symbol| !declared_machinery_trusted() && !initialised.contains(*symbol))
        .map(|symbol| scoping.symbol_span(*symbol))
        .min_by_key(|span| span.start)
}

pub fn rotator_sealed(
    statement: &Statement<'_>,
    scoping: &Scoping,
    allowed: &FxHashSet<SymbolId>,
) -> bool {
    sealed(statement, scoping, allowed).is_none()
}

pub fn rotator_refusal_at<'a>(
    program: &Program<'a>,
    scoping: &Scoping,
    sites: &[RotatorSite],
    allowed: &FxHashSet<SymbolId>,
    foreign: Option<(&ForeignUnits<'_, 'a>, usize)>,
) -> Option<Refusal> {
    let mut first: Option<SiteList<'_, 'a>> = None;
    for site in sites {
        let Some(list) = site.locate(program) else {
            return Some(Refusal::at(notes::ROTATOR_NOT_TOP_LEVEL, site.span));
        };
        let placed = rotator_placement_dropped()
            || (list.binds_all(scoping, allowed)
                && first.as_ref().is_none_or(|first| first.same_list(&list)));
        if !placed {
            return Some(Refusal::at(notes::ROTATOR_NOT_TOP_LEVEL, site.span));
        }
        first.get_or_insert(list);
    }
    let list = first?;
    let prefix = list.statements.get(..list.index)?;
    if let Some(statement) = first_active(program, scoping, (prefix, list.scope), foreign) {
        return Some(Refusal::at(notes::CODE_BEFORE_ROTATOR, statement.span()));
    }
    let later = program.body.get(index_in_program(program, &list)..)?;
    if let Some(statement) = later.iter().find(|s| with_observes() && imports(s)) {
        return Some(Refusal::at(notes::CODE_BEFORE_ROTATOR, statement.span()));
    }
    if let Some(span) = uninitialised(scoping, allowed, &declared_before(prefix)) {
        return Some(Refusal::at(notes::MACHINERY_NOT_INITIALISED, span));
    }
    let statement = list.statements.get(list.index)?;
    sealed(statement, scoping, allowed).map(|span| Refusal::at(notes::ROTATOR_NOT_SEALED, span))
}

fn index_in_program(program: &Program<'_>, list: &SiteList<'_, '_>) -> usize {
    match list.scope {
        None => list.index,
        Some(_) => program.body.len(),
    }
}
