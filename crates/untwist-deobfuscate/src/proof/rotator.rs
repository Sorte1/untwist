use oxc_ast::ast::*;
use oxc_semantic::Scoping;
use oxc_span::{GetSpan, Span};
use oxc_syntax::symbol::SymbolId;
use rustc_hash::FxHashSet;
use untwist_core::semantic::with_observes;

use super::foreign::ForeignUnits;
use super::seal::sealed;
use super::Refusal;
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

    pub(super) fn statement<'s, 'a>(&self, program: &'s Program<'a>) -> Option<&'s Statement<'a>> {
        self.top_level_index(program)
            .and_then(|index| program.body.get(index))
    }

    fn top_level_index(&self, program: &Program<'_>) -> Option<usize> {
        program
            .body
            .iter()
            .position(|statement| std::ptr::from_ref(statement) as usize == self.address)
    }
}

pub struct LiteralBindings {
    symbols: FxHashSet<SymbolId>,
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
        let symbols = program
            .body
            .iter()
            .filter_map(|statement| match statement {
                Statement::VariableDeclaration(declaration) => Some(&declaration.declarations),
                _ => None,
            })
            .flatten()
            .filter_map(|declarator| literal_binding(declarator, scoping))
            .collect();
        Self { symbols }
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
                    && self.runs_no_code(scoping, &property.value)
            }),
            other => self.primitive(scoping, other),
        }
    }

    pub fn inert_statement(&self, scoping: &Scoping, stmt: &Statement<'_>) -> bool {
        match stmt {
            Statement::FunctionDeclaration(_) | Statement::EmptyStatement(_) => true,
            Statement::VariableDeclaration(declaration) => {
                !declaration.kind.is_using()
                    && declaration.declarations.iter().all(|d| {
                        matches!(d.id, BindingPattern::BindingIdentifier(_))
                            && d.init.as_ref().is_none_or(|init| {
                                self.runs_no_code(scoping, init) || hoisted_function(scoping, init)
                            })
                    })
            }
            _ => false,
        }
    }
}

fn hoisted_function(scoping: &Scoping, expr: &Expression<'_>) -> bool {
    let Expression::Identifier(id) = expr.without_parentheses() else {
        return false;
    };
    id.reference_id
        .get()
        .and_then(|reference| scoping.get_reference(reference).symbol_id())
        .is_some_and(|symbol| {
            scoping.symbol_flags(symbol).is_function()
                && scoping.symbol_scope_id(symbol) == scoping.root_scope_id()
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
    prefix: &'s [Statement<'a>],
    foreign: Option<(&ForeignUnits<'_, 'a>, usize)>,
) -> Option<&'s Statement<'a>> {
    if prefix.iter().all(trivially_inert) {
        return None;
    }
    let literals = LiteralBindings::new(program, scoping);
    prefix.iter().find(|statement| {
        !literals.inert_statement(scoping, statement)
            && !foreign.is_some_and(|(units, judged)| {
                units.admits(program, scoping, &literals, statement, judged)
            })
    })
}

fn imports(statement: &Statement<'_>) -> bool {
    matches!(
        statement,
        Statement::ImportDeclaration(_)
            | Statement::ExportAllDeclaration(_)
            | Statement::ExportFromDeclaration(_)
    )
}

pub(super) fn uninitialised(scoping: &Scoping, allowed: &FxHashSet<SymbolId>) -> Option<Span> {
    allowed
        .iter()
        .filter(|symbol| !scoping.symbol_flags(**symbol).is_function())
        .map(|symbol| scoping.symbol_span(*symbol))
        .min_by_key(|span| span.start)
}

pub fn rotator_refusal_at<'a>(
    program: &Program<'a>,
    scoping: &Scoping,
    sites: &[RotatorSite],
    allowed: &FxHashSet<SymbolId>,
    foreign: Option<(&ForeignUnits<'_, 'a>, usize)>,
) -> Option<Refusal> {
    let mut first = None;
    for site in sites {
        match site.top_level_index(program) {
            Some(index) => {
                first.get_or_insert(index);
            }
            None => return Some(Refusal::at(notes::ROTATOR_NOT_TOP_LEVEL, site.span)),
        }
    }
    let index = first?;
    let prefix = program.body.get(..index)?;
    if let Some(statement) = first_active(program, scoping, prefix, foreign) {
        return Some(Refusal::at(notes::CODE_BEFORE_ROTATOR, statement.span()));
    }
    let later = program.body.get(index..)?;
    if let Some(statement) = later.iter().find(|s| with_observes() && imports(s)) {
        return Some(Refusal::at(notes::CODE_BEFORE_ROTATOR, statement.span()));
    }
    if let Some(span) = uninitialised(scoping, allowed) {
        return Some(Refusal::at(notes::MACHINERY_NOT_INITIALISED, span));
    }
    let statement = program.body.get(index)?;
    sealed(statement, scoping, allowed).map(|span| Refusal::at(notes::ROTATOR_NOT_SEALED, span))
}
