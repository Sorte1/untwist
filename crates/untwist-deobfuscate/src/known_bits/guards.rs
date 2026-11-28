use oxc_ast::ast::{
    ArrayExpressionElement, BindingPattern, BlockStatement, Expression, FunctionBody,
    IdentifierReference, IfStatement, ObjectPropertyKind, Program, Statement,
    VariableDeclarationKind, WhileStatement,
};
use oxc_ast_visit::{walk_js, VisitJs};
use oxc_semantic::Scoping;
use oxc_span::{GetSpan, Span};
use oxc_syntax::operator::LogicalOperator;
use oxc_syntax::symbol::SymbolId;
use rustc_hash::{FxHashMap, FxHashSet};

use super::identity::{relation, PlainSymbols};
use super::resolved;
use crate::exits::always_exits;

const MAX_DEPTH: u32 = 16;

pub(super) type Decide<'d> = &'d dyn Fn(&Expression<'_>) -> Option<bool>;

pub(super) fn dead_regions(
    program: &Program<'_>,
    scoping: &Scoping,
    decide: Decide<'_>,
    plain: &PlainSymbols,
    local: bool,
) -> FxHashSet<Span> {
    let mut scan = DeadBranches {
        decide,
        plain,
        scoping,
        local,
        found: Vec::new(),
        holders: Vec::new(),
        references: FxHashMap::default(),
    };
    scan.visit_program(program);
    if scan.found.is_empty() {
        return FxHashSet::default();
    }
    scan.settle_holders();
    scan.found.into_iter().collect()
}

pub(super) fn references_inside(program: &Program<'_>, dead: &FxHashSet<Span>) -> FxHashSet<Span> {
    let mut spans: Vec<Span> = dead.iter().copied().collect();
    spans.sort_unstable_by_key(|span| (span.start, std::cmp::Reverse(span.end)));
    let mut merged: Vec<Span> = Vec::with_capacity(spans.len());
    for span in spans {
        match merged.last_mut() {
            Some(last) if span.start <= last.end => last.end = last.end.max(span.end),
            _ => merged.push(span),
        }
    }
    let mut scan = Inside {
        merged,
        found: FxHashSet::default(),
    };
    scan.visit_program(program);
    scan.found
}

struct Inside {
    merged: Vec<Span>,
    found: FxHashSet<Span>,
}

impl<'a> VisitJs<'a> for Inside {
    fn visit_identifier_reference(&mut self, it: &IdentifierReference<'a>) {
        let at = self
            .merged
            .partition_point(|span| span.start <= it.span.start);
        let inside = at
            .checked_sub(1)
            .and_then(|index| self.merged.get(index))
            .is_some_and(|span| it.span.end <= span.end);
        if inside {
            self.found.insert(it.span);
        }
    }
}

struct Holder {
    span: Span,
    symbols: Vec<SymbolId>,
}

struct DeadBranches<'l, 's> {
    decide: Decide<'l>,
    plain: &'l PlainSymbols,
    scoping: &'s Scoping,
    local: bool,
    found: Vec<Span>,
    holders: Vec<Holder>,
    references: FxHashMap<SymbolId, Vec<Span>>,
}

impl DeadBranches<'_, '_> {
    fn test(&self, expr: &Expression<'_>) -> Option<bool> {
        if let Expression::LogicalExpression(logical) = expr.without_parentheses() {
            let left = self.test(&logical.left);
            let right = self.test(&logical.right);
            return match logical.operator {
                LogicalOperator::And if left == Some(false) || right == Some(false) => Some(false),
                LogicalOperator::Or if left == Some(true) || right == Some(true) => Some(true),
                _ => None,
            };
        }
        if let Some(flag) = self.plain.flag(expr, self.scoping) {
            return Some(flag);
        }
        if let Some(holds) = relation(expr).and_then(|found| found.decide(self.plain, self.scoping))
        {
            return Some(holds);
        }
        (self.decide)(expr)
    }

    fn dead_tail(&mut self, statements: &[Statement<'_>]) {
        let mut exited = false;
        for statement in statements {
            if exited && !is_declaration(statement) {
                self.found.push(statement.span());
                continue;
            }
            let Statement::IfStatement(branch) = statement else {
                continue;
            };
            let taken = match self.test(&branch.test) {
                Some(true) => Some(&branch.consequent),
                Some(false) => branch.alternate.as_ref(),
                None => None,
            };
            exited |= taken.is_some_and(always_exits);
        }
    }

    fn movable(&self, symbol: SymbolId) -> bool {
        self.local || self.scoping.symbol_scope_id(symbol) != self.scoping.root_scope_id()
    }

    fn holder(&self, statement: &Statement<'_>) -> Option<Holder> {
        let symbols = match statement {
            Statement::FunctionDeclaration(function) => {
                vec![function.id.as_ref()?.symbol_id.get()?]
            }
            Statement::VariableDeclaration(declaration)
                if declaration.kind != VariableDeclarationKind::Var =>
            {
                let mut symbols = Vec::new();
                for declarator in &declaration.declarations {
                    let BindingPattern::BindingIdentifier(id) = &declarator.id else {
                        return None;
                    };
                    if !declarator.init.as_ref().is_none_or(|init| inert(init, 0)) {
                        return None;
                    }
                    symbols.push(id.symbol_id.get()?);
                }
                symbols
            }
            _ => return None,
        };
        symbols
            .iter()
            .all(|symbol| self.movable(*symbol))
            .then_some(Holder {
                span: statement.span(),
                symbols,
            })
    }

    fn inside_dead(&self, span: Span) -> bool {
        self.found
            .iter()
            .any(|dead| dead.start <= span.start && span.end <= dead.end)
    }

    fn settle_holders(&mut self) {
        let mut grew = true;
        while grew {
            grew = false;
            let mut index = 0;
            while index < self.holders.len() {
                let holder = &self.holders[index];
                let span = holder.span;
                let unread = !self.inside_dead(span)
                    && holder.symbols.iter().all(|symbol| {
                        self.references.get(symbol).is_none_or(|spans| {
                            spans.iter().all(|read| {
                                self.inside_dead(*read) || span.contains_inclusive(*read)
                            })
                        })
                    });
                if unread {
                    self.holders.swap_remove(index);
                    self.found.push(span);
                    grew = true;
                } else {
                    index += 1;
                }
            }
        }
    }
}

fn is_declaration(statement: &Statement<'_>) -> bool {
    matches!(
        statement,
        Statement::VariableDeclaration(_)
            | Statement::FunctionDeclaration(_)
            | Statement::ClassDeclaration(_)
    )
}

fn inert(expr: &Expression<'_>, depth: u32) -> bool {
    if depth > MAX_DEPTH {
        return false;
    }
    let depth = depth + 1;
    match expr.without_parentheses() {
        Expression::NumericLiteral(_)
        | Expression::StringLiteral(_)
        | Expression::BooleanLiteral(_)
        | Expression::NullLiteral(_)
        | Expression::Identifier(_)
        | Expression::FunctionExpression(_)
        | Expression::ArrowFunctionExpression(_) => true,
        Expression::ObjectExpression(object) => {
            object.properties.iter().all(|property| match property {
                ObjectPropertyKind::ObjectProperty(property) => {
                    !property.computed
                        && !property.kind.is_accessor()
                        && inert(&property.value, depth)
                }
                ObjectPropertyKind::SpreadProperty(_) => false,
            })
        }
        Expression::ArrayExpression(array) => array.elements.iter().all(|element| match element {
            ArrayExpressionElement::SpreadElement(_) => false,
            ArrayExpressionElement::Elision(_) => true,
            other => other
                .as_expression()
                .is_some_and(|value| inert(value, depth)),
        }),
        _ => false,
    }
}

impl<'a> VisitJs<'a> for DeadBranches<'_, '_> {
    fn visit_function_body(&mut self, it: &FunctionBody<'a>) {
        self.dead_tail(&it.statements);
        walk_js::walk_function_body(self, it);
    }

    fn visit_block_statement(&mut self, it: &BlockStatement<'a>) {
        self.dead_tail(&it.body);
        walk_js::walk_block_statement(self, it);
    }

    fn visit_statement(&mut self, it: &Statement<'a>) {
        if let Some(holder) = self.holder(it) {
            self.holders.push(holder);
        }
        walk_js::walk_statement(self, it);
    }

    fn visit_identifier_reference(&mut self, it: &IdentifierReference<'a>) {
        if let Some(symbol) = resolved(it, self.scoping) {
            self.references.entry(symbol).or_default().push(it.span);
        }
    }

    fn visit_if_statement(&mut self, it: &IfStatement<'a>) {
        match self.test(&it.test) {
            Some(true) => {
                if let Some(alternate) = &it.alternate {
                    self.found.push(alternate.span());
                }
            }
            Some(false) => self.found.push(it.consequent.span()),
            None => {}
        }
        walk_js::walk_if_statement(self, it);
    }

    fn visit_while_statement(&mut self, it: &WhileStatement<'a>) {
        if self.test(&it.test) == Some(false) {
            self.found.push(it.body.span());
        }
        walk_js::walk_while_statement(self, it);
    }
}
