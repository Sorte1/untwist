use oxc_allocator::{TakeIn, Vec as AVec};
use oxc_ast::ast::*;
use oxc_ast_visit::{walk_js, walk_mut, VisitJs, VisitMut};
use oxc_semantic::Scoping;
use oxc_span::{GetSpan, Span};
use oxc_syntax::scope::ScopeFlags;
use oxc_syntax::symbol::SymbolId;
use untwist_core::pass::Cx;

use super::prologue::{assigned_grab, inline_grab, polyfills, single};

struct Reference {
    span: Span,
}

struct References<'s> {
    scoping: &'s Scoping,
    symbol: SymbolId,
    depth: usize,
    found: Vec<Reference>,
}

impl<'a> VisitJs<'a> for References<'_> {
    fn visit_identifier_reference(&mut self, it: &IdentifierReference<'a>) {
        let Some(reference) = it.reference_id.get() else {
            return;
        };
        let reference = self.scoping.get_reference(reference);
        if reference.symbol_id() == Some(self.symbol) {
            self.found.push(Reference { span: it.span });
        }
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
}

fn references(stmt: &Statement<'_>, scoping: &Scoping, symbol: SymbolId) -> Vec<Reference> {
    let mut scan = References {
        scoping,
        symbol,
        depth: 0,
        found: Vec::new(),
    };
    scan.visit_statement(stmt);
    scan.found
}

fn overwrites_first(stmt: &Statement<'_>, scoping: &Scoping, symbol: SymbolId) -> bool {
    let (init, bound) = match stmt {
        Statement::VariableDeclaration(declaration) => {
            let [declarator] = declaration.declarations.as_slice() else {
                return false;
            };
            let BindingPattern::BindingIdentifier(id) = &declarator.id else {
                return false;
            };
            (declarator.init.as_ref(), id.symbol_id.get() == Some(symbol))
        }
        Statement::ExpressionStatement(node) => match &node.expression {
            Expression::AssignmentExpression(assigned)
                if assigned.operator == AssignmentOperator::Assign =>
            {
                let target = match &assigned.left {
                    AssignmentTarget::AssignmentTargetIdentifier(id) => id
                        .reference_id
                        .get()
                        .and_then(|reference| scoping.get_reference(reference).symbol_id()),
                    _ => None,
                };
                (Some(&assigned.right), target == Some(symbol))
            }
            _ => return false,
        },
        _ => return false,
    };
    let Some(init) = init else {
        return false;
    };
    let mut scan = References {
        scoping,
        symbol,
        depth: 0,
        found: Vec::new(),
    };
    scan.visit_expression(init);
    bound && scan.found.is_empty()
}

fn dead_after(
    list: &[Statement<'_>],
    index: usize,
    local: usize,
    scoping: &Scoping,
    symbol: SymbolId,
) -> bool {
    let total = scoping.get_resolved_reference_ids(symbol).len();
    let mut seen = local;
    let mut first_later = true;
    for stmt in &list[index + 1..] {
        let found = references(stmt, scoping, symbol);
        if first_later && overwrites_first(stmt, scoping, symbol) {
            first_later = false;
            seen += found.len();
            continue;
        }
        if found.is_empty() {
            continue;
        }
        if first_later {
            return false;
        }
        seen += found.len();
    }
    seen == total
}

fn grab_symbol(stmt: &Statement<'_>, scoping: &Scoping) -> Option<(SymbolId, bool)> {
    let Statement::TryStatement(attempt) = stmt else {
        return None;
    };
    match single(&attempt.block)? {
        Statement::VariableDeclaration(declaration) => {
            let BindingPattern::BindingIdentifier(id) = &declaration.declarations.first()?.id
            else {
                return None;
            };
            Some((id.symbol_id.get()?, true))
        }
        Statement::ExpressionStatement(node) => {
            let Expression::AssignmentExpression(assigned) = &node.expression else {
                return None;
            };
            let AssignmentTarget::AssignmentTargetIdentifier(id) = &assigned.left else {
                return None;
            };
            let symbol = scoping.get_reference(id.reference_id.get()?).symbol_id()?;
            Some((symbol, false))
        }
        _ => None,
    }
}

struct PairFinder<'s> {
    scoping: &'s Scoping,
    depth: usize,
    found: Vec<(Span, Span, SymbolId, bool)>,
}

impl<'a> VisitJs<'a> for PairFinder<'_> {
    fn visit_statements(&mut self, it: &oxc_allocator::Vec<'a, Statement<'a>>) {
        if self.depth == 0 {
            for pair in it.windows(2) {
                let Some(name) = inline_grab(&pair[0]) else {
                    continue;
                };
                if !polyfills(&pair[1], &[name]) {
                    continue;
                }
                if let Some((symbol, declared)) = grab_symbol(&pair[0], self.scoping) {
                    self.found
                        .push((pair[0].span(), pair[1].span(), symbol, declared));
                }
            }
        }
        walk_js::walk_statements(self, it);
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
}

fn is_loop(stmt: &Statement<'_>) -> bool {
    matches!(
        stmt,
        Statement::ForStatement(_)
            | Statement::ForInStatement(_)
            | Statement::ForOfStatement(_)
            | Statement::WhileStatement(_)
            | Statement::DoWhileStatement(_)
            | Statement::LabeledStatement(_)
    )
}

fn pairs_in(body: &[Statement<'_>], index: usize, scoping: &Scoping) -> Vec<Edit> {
    let stmt = &body[index];
    let mut finder = PairFinder {
        scoping,
        depth: 0,
        found: Vec::new(),
    };
    finder.visit_statement(stmt);
    finder
        .found
        .into_iter()
        .filter(|(attempt, polyfill, symbol, _)| {
            let found = references(stmt, scoping, *symbol);
            let local = found
                .iter()
                .filter(|r| {
                    attempt.contains_inclusive(r.span) || polyfill.contains_inclusive(r.span)
                })
                .count();
            local == found.len() && dead_after(body, index, local, scoping, *symbol)
        })
        .map(|(attempt, polyfill, _, declared)| Edit::Pair {
            attempt,
            polyfill,
            declared,
        })
        .collect()
}

enum Edit {
    Pair {
        attempt: Span,
        polyfill: Span,
        declared: bool,
    },
    Grab(Span),
}

#[derive(Default)]
struct FindGrab {
    depth: usize,
    found: Option<Span>,
}

impl<'a> VisitJs<'a> for FindGrab {
    fn visit_logical_expression(&mut self, it: &LogicalExpression<'a>) {
        if self.depth == 0 && self.found.is_none() && assigned_grab(it) {
            self.found = Some(it.span);
            return;
        }
        walk_js::walk_logical_expression(self, it);
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
}

fn holder_of(logical: &LogicalExpression<'_>, scoping: &Scoping) -> Option<SymbolId> {
    let member = logical.left.without_parentheses().as_member_expression()?;
    let Expression::AssignmentExpression(assigned) = member.object().without_parentheses() else {
        return None;
    };
    let AssignmentTarget::AssignmentTargetIdentifier(id) = &assigned.left else {
        return None;
    };
    scoping.get_reference(id.reference_id.get()?).symbol_id()
}

struct HolderOf<'s> {
    scoping: &'s Scoping,
    span: Span,
    symbol: Option<SymbolId>,
}

impl<'a> VisitJs<'a> for HolderOf<'_> {
    fn visit_logical_expression(&mut self, it: &LogicalExpression<'a>) {
        if it.span == self.span {
            self.symbol = holder_of(it, self.scoping);
            return;
        }
        walk_js::walk_logical_expression(self, it);
    }
}

fn dead_grab(list: &[Statement<'_>], index: usize, scoping: &Scoping) -> Option<Edit> {
    let stmt = list.get(index)?;
    let mut find = FindGrab::default();
    find.visit_statement(stmt);
    let span = find.found?;
    let mut holder = HolderOf {
        scoping,
        span,
        symbol: None,
    };
    holder.visit_statement(stmt);
    let symbol = holder.symbol?;
    let inside = references_in(stmt, scoping, symbol, span);
    let in_statement = references(stmt, scoping, symbol).len();
    let dead = if declares_around(stmt, symbol, span) || assigns_around(stmt, scoping, symbol, span)
    {
        in_statement == inside + usize::from(!declares_around(stmt, symbol, span))
    } else {
        in_statement == inside && dead_after(list, index, in_statement, scoping, symbol)
    };
    dead.then_some(Edit::Grab(span))
}

fn assigns_around(stmt: &Statement<'_>, scoping: &Scoping, symbol: SymbolId, span: Span) -> bool {
    let Statement::ExpressionStatement(node) = stmt else {
        return false;
    };
    let Expression::AssignmentExpression(assigned) = &node.expression else {
        return false;
    };
    let AssignmentTarget::AssignmentTargetIdentifier(id) = &assigned.left else {
        return false;
    };
    let target = id
        .reference_id
        .get()
        .and_then(|reference| scoping.get_reference(reference).symbol_id());
    assigned.operator == AssignmentOperator::Assign
        && target == Some(symbol)
        && assigned.right.span().contains_inclusive(span)
}

fn declares_around(stmt: &Statement<'_>, symbol: SymbolId, span: Span) -> bool {
    let Statement::VariableDeclaration(declaration) = stmt else {
        return false;
    };
    declaration.declarations.iter().any(|declarator| {
        matches!(&declarator.id, BindingPattern::BindingIdentifier(id) if id.symbol_id.get() == Some(symbol))
            && declarator
                .init
                .as_ref()
                .is_some_and(|init| init.span().contains_inclusive(span))
    })
}

fn references_in(stmt: &Statement<'_>, scoping: &Scoping, symbol: SymbolId, span: Span) -> usize {
    references(stmt, scoping, symbol)
        .iter()
        .filter(|reference| span.contains_inclusive(reference.span))
        .count()
}

struct DropGrab<'c, 'a, 'x> {
    cx: &'c Cx<'a, 'x>,
    span: Span,
    dropped: bool,
}

impl<'a> VisitMut<'a> for DropGrab<'_, 'a, '_> {
    fn visit_sequence_expression(&mut self, it: &mut SequenceExpression<'a>) {
        let before = it.expressions.len();
        if before > 1 {
            let span = self.span;
            it.expressions
                .retain(|expr| expr.without_parentheses().span() != span);
            if it.expressions.len() < before {
                self.dropped = true;
                return;
            }
        }
        walk_mut::walk_sequence_expression(self, it);
    }

    fn visit_expression(&mut self, it: &mut Expression<'a>) {
        if !self.dropped && it.without_parentheses().span() == self.span {
            let span = it.span();
            *it = Expression::new_unary_expression(
                span,
                UnaryOperator::Void,
                Expression::new_numeric_literal(span, 0.0, None, NumberBase::Decimal, self.cx),
                self.cx,
            );
            self.dropped = true;
            return;
        }
        walk_mut::walk_expression(self, it);
    }
}

fn bare_declaration<'a>(stmt: &mut Statement<'a>, cx: &Cx<'a, '_>) {
    let Statement::TryStatement(attempt) = stmt else {
        return;
    };
    let Some(mut inner) = attempt.block.body.take_in(cx).into_iter().next() else {
        return;
    };
    if let Statement::VariableDeclaration(declaration) = &mut inner {
        for declarator in declaration.declarations.iter_mut() {
            declarator.init = None;
        }
    }
    *stmt = inner;
}

struct Apply<'c, 'a, 'x> {
    cx: &'c Cx<'a, 'x>,
    edits: Vec<Edit>,
    applied: usize,
}

impl<'a> VisitMut<'a> for Apply<'_, 'a, '_> {
    fn visit_statements(&mut self, it: &mut AVec<'a, Statement<'a>>) {
        let mut drop_spans = Vec::new();
        for stmt in it.iter_mut() {
            let span = stmt.span();
            for edit in &self.edits {
                let Edit::Pair {
                    attempt,
                    polyfill,
                    declared,
                } = edit
                else {
                    continue;
                };
                if span == *attempt {
                    if *declared {
                        bare_declaration(stmt, self.cx);
                    } else {
                        drop_spans.push(span);
                    }
                    self.applied += 1;
                }
                if span == *polyfill {
                    drop_spans.push(span);
                }
            }
        }
        if !drop_spans.is_empty() {
            it.retain(|stmt| !drop_spans.contains(&stmt.span()));
        }
        walk_mut::walk_statements(self, it);
    }

    fn visit_expression(&mut self, it: &mut Expression<'a>) {
        let span = it.without_parentheses().span();
        if self
            .edits
            .iter()
            .any(|edit| matches!(edit, Edit::Grab(grab) if *grab == span))
        {
            let mut drop = DropGrab {
                cx: self.cx,
                span,
                dropped: false,
            };
            drop.visit_expression(it);
            self.applied += usize::from(drop.dropped);
            return;
        }
        walk_mut::walk_expression(self, it);
    }

    fn visit_sequence_expression(&mut self, it: &mut SequenceExpression<'a>) {
        let before = it.expressions.len();
        if before > 1 {
            let edits = &self.edits;
            it.expressions.retain(|expr| {
                let span = expr.without_parentheses().span();
                !edits
                    .iter()
                    .any(|edit| matches!(edit, Edit::Grab(grab) if *grab == span))
            });
            self.applied += before - it.expressions.len();
        }
        walk_mut::walk_sequence_expression(self, it);
    }
}

pub(super) fn strip_inline_prologues<'a>(body: &mut FunctionBody<'a>, cx: &Cx<'a, '_>) -> usize {
    let scoping = cx.scoping();
    let statements = &body.statements;
    let mut edits = Vec::new();
    for index in 0..statements.len() {
        if is_loop(&statements[index]) {
            continue;
        }
        edits.extend(pairs_in(statements, index, scoping));
        edits.extend(dead_grab(statements, index, scoping));
    }
    if edits.is_empty() {
        return 0;
    }
    let mut apply = Apply {
        cx,
        edits,
        applied: 0,
    };
    apply.visit_function_body(body);
    apply.applied
}
