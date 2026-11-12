use oxc_ast::ast::*;
use oxc_ast_visit::{walk_js, VisitJs};
use oxc_semantic::Scoping;
use oxc_syntax::scope::{ScopeFlags, ScopeId};
use oxc_syntax::symbol::SymbolId;
use oxc_traverse::Ancestor;
use rustc_hash::FxHashSet;
use untwist_core::pass::Cx;
use untwist_core::semantic::is_constant_binding;

use super::fold::MergedKey;
use crate::bindings::resolved_symbol;
use crate::control_flow_object::single_declarator;

#[derive(Clone, Copy)]
pub(super) struct Site {
    function_body: bool,
    scope: ScopeId,
}

fn is_function_body(parent: &Ancestor<'_, '_>) -> Option<bool> {
    match parent {
        Ancestor::FunctionBodyStatements(_) => Some(true),
        Ancestor::BlockStatementBody(_) => Some(false),
        _ => None,
    }
}

pub(super) fn function_local_site(cx: &Cx<'_, '_>) -> Option<Site> {
    let mut ancestors = cx.tcx.ancestors();
    let function_body = is_function_body(&ancestors.next()?)?;
    for ancestor in ancestors {
        match ancestor {
            Ancestor::FunctionBody(_) | Ancestor::ArrowFunctionExpressionBody(_) => {
                return Some(Site {
                    function_body,
                    scope: cx.current_scope_id(),
                });
            }
            Ancestor::TryStatementBlock(_)
            | Ancestor::TryStatementHandler(_)
            | Ancestor::TryStatementFinalizer(_)
            | Ancestor::CatchClauseBody(_)
            | Ancestor::WithStatementBody(_)
            | Ancestor::StaticBlockBody(_)
            | Ancestor::ProgramBody(_) => return None,
            _ => {}
        }
    }
    None
}

fn empty_object_name<'k>(stmt: &'k Statement<'_>) -> Option<&'k str> {
    let (id, init) = single_declarator(stmt)?;
    match init? {
        Expression::ObjectExpression(object) if object.properties.is_empty() => {
            Some(id.name.as_str())
        }
        _ => None,
    }
}

#[derive(Default)]
pub(super) struct Captured {
    symbols: FxHashSet<SymbolId>,
    unresolved: FxHashSet<String>,
}

impl Captured {
    fn holds(&self, symbol: SymbolId, name: &str) -> bool {
        self.symbols.contains(&symbol) || self.unresolved.contains(name)
    }
}

struct CaptureScan<'k> {
    wanted: &'k FxHashSet<&'k str>,
    scoping: &'k Scoping,
    depth: u32,
    found: Captured,
}

impl<'a> VisitJs<'a> for CaptureScan<'_> {
    fn visit_identifier_reference(&mut self, it: &IdentifierReference<'a>) {
        if self.depth == 0 || !self.wanted.contains(it.name.as_str()) {
            return;
        }
        match resolved_symbol(it, self.scoping) {
            Some(symbol) => self.found.symbols.insert(symbol),
            None => self.found.unresolved.insert(it.name.to_string()),
        };
    }

    fn visit_function(&mut self, it: &Function<'a>, flags: ScopeFlags) {
        self.depth += 1;
        walk_js::walk_function(self, it, flags);
        self.depth -= 1;
    }

    fn visit_arrow_function_expression(&mut self, it: &ArrowFunctionExpression<'a>) {
        self.depth += 1;
        walk_js::walk_arrow_function_expression(self, it);
        self.depth -= 1;
    }

    fn visit_class(&mut self, it: &Class<'a>) {
        self.depth += 1;
        walk_js::walk_class(self, it);
        self.depth -= 1;
    }
}

fn captures(stmts: &[Statement<'_>], scoping: &Scoping) -> Captured {
    let wanted: FxHashSet<&str> = stmts.iter().filter_map(empty_object_name).collect();
    let mut scan = CaptureScan {
        wanted: &wanted,
        scoping,
        depth: 0,
        found: Captured::default(),
    };
    if !wanted.is_empty() {
        for stmt in stmts {
            scan.visit_statement(stmt);
        }
    }
    scan.found
}

fn reads_and_arithmetic(expr: &Expression<'_>, object: &str) -> bool {
    let reads = |inner: &Expression<'_>| reads_and_arithmetic(inner, object);
    match expr.without_parentheses() {
        Expression::Identifier(id) => id.name.as_str() != object,
        Expression::ThisExpression(_)
        | Expression::StringLiteral(_)
        | Expression::NumericLiteral(_)
        | Expression::BooleanLiteral(_)
        | Expression::NullLiteral(_)
        | Expression::BigIntLiteral(_)
        | Expression::RegExpLiteral(_)
        | Expression::FunctionExpression(_)
        | Expression::ArrowFunctionExpression(_) => true,
        Expression::TemplateLiteral(template) => template.expressions.iter().all(reads),
        Expression::StaticMemberExpression(member) => !member.optional && reads(&member.object),
        Expression::ComputedMemberExpression(member) => {
            !member.optional && reads(&member.object) && reads(&member.expression)
        }
        Expression::PrivateFieldExpression(member) => !member.optional && reads(&member.object),
        Expression::UnaryExpression(unary) => {
            unary.operator != UnaryOperator::Delete && reads(&unary.argument)
        }
        Expression::BinaryExpression(binary) => {
            !matches!(
                binary.operator,
                BinaryOperator::In | BinaryOperator::Instanceof
            ) && reads(&binary.left)
                && reads(&binary.right)
        }
        Expression::LogicalExpression(logical) => reads(&logical.left) && reads(&logical.right),
        Expression::ConditionalExpression(conditional) => {
            reads(&conditional.test)
                && reads(&conditional.consequent)
                && reads(&conditional.alternate)
        }
        Expression::ArrayExpression(array) => array.elements.iter().all(|element| match element {
            ArrayExpressionElement::Elision(_) => true,
            ArrayExpressionElement::SpreadElement(_) => false,
            other => other.as_expression().is_some_and(reads),
        }),
        Expression::ObjectExpression(literal) => {
            literal.properties.iter().all(|property| match property {
                ObjectPropertyKind::ObjectProperty(property) => {
                    property.kind == PropertyKind::Init
                        && (!property.computed
                            || matches!(
                                property.key,
                                PropertyKey::StringLiteral(_) | PropertyKey::NumericLiteral(_)
                            ))
                        && reads(&property.value)
                }
                ObjectPropertyKind::SpreadProperty(_) => false,
            })
        }
        _ => false,
    }
}

enum Region {
    List,
    Function,
}

fn region(declaration: &Statement<'_>) -> Option<Region> {
    let Statement::VariableDeclaration(declaration) = declaration else {
        return None;
    };
    match declaration.kind {
        VariableDeclarationKind::Let | VariableDeclarationKind::Const => Some(Region::List),
        VariableDeclarationKind::Var => Some(Region::Function),
        _ => None,
    }
}

fn every_reference_is_in(scoping: &Scoping, symbol: SymbolId, scope: ScopeId) -> bool {
    scoping.get_resolved_references(symbol).all(|reference| {
        scoping
            .scope_ancestors(reference.scope_id())
            .any(|ancestor| ancestor == scope)
    })
}

pub(super) struct Local<'k, 'a> {
    pub(super) stmts: &'k [Statement<'a>],
    pub(super) index: usize,
    pub(super) scoping: &'k Scoping,
    pub(super) site: Option<Site>,
    pub(super) captured: &'k mut Option<Captured>,
}

impl Local<'_, '_> {
    pub(super) fn admits(
        &mut self,
        symbol: SymbolId,
        key: &MergedKey<'_, '_>,
        value: &Expression<'_>,
    ) -> bool {
        let Some(name) = self.stmts.get(self.index).and_then(empty_object_name) else {
            return false;
        };
        !matches!(key, MergedKey::Dynamic(_))
            && reads_and_arithmetic(value, name)
            && self.is_uncaptured(symbol, name)
    }

    fn holds_every_reference(&self, symbol: SymbolId, site: Site) -> bool {
        match region(&self.stmts[self.index]) {
            Some(Region::List) => true,
            Some(Region::Function) => {
                site.function_body || every_reference_is_in(self.scoping, symbol, site.scope)
            }
            None => false,
        }
    }

    fn is_uncaptured(&mut self, symbol: SymbolId, name: &str) -> bool {
        let Some(site) = self.site else {
            return false;
        };
        if !is_constant_binding(self.scoping, symbol) || !self.holds_every_reference(symbol, site) {
            return false;
        }
        let (stmts, scoping) = (self.stmts, self.scoping);
        let captured = self
            .captured
            .get_or_insert_with(|| captures(stmts, scoping));
        !captured.holds(symbol, name)
    }
}
