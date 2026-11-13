use oxc_allocator::{TakeIn, Vec as AVec};
use oxc_ast::ast::*;
use oxc_ast_visit::{walk_js, VisitJs};
use oxc_semantic::Scoping;
use oxc_syntax::scope::ScopeFlags;
use oxc_syntax::symbol::SymbolId;
use untwist_core::pass::Cx;

use super::identify::resolved;

pub const SHAPE: &str = "for-of/loop-closure/unsafe";

struct Scan {
    depth: u32,
    arrows: u32,
    arrow: bool,
    bad: bool,
}

impl<'a> VisitJs<'a> for Scan {
    fn visit_function(&mut self, it: &Function<'a>, flags: ScopeFlags) {
        if self.depth == 0 && it.is_declaration() {
            self.bad = true;
        }
        self.depth += 1;
        walk_js::walk_function(self, it, flags);
        self.depth -= 1;
    }

    fn visit_arrow_function_expression(&mut self, it: &ArrowFunctionExpression<'a>) {
        self.arrows += 1;
        walk_js::walk_arrow_function_expression(self, it);
        self.arrows -= 1;
    }

    fn visit_class(&mut self, it: &Class<'a>) {
        self.bad |= self.depth == 0 && it.is_declaration();
        walk_js::walk_class(self, it);
    }

    fn visit_return_statement(&mut self, it: &ReturnStatement<'a>) {
        self.bad |= self.depth == 0 && self.arrows == 0;
        walk_js::walk_return_statement(self, it);
    }

    fn visit_variable_declaration(&mut self, it: &VariableDeclaration<'a>) {
        self.bad |= self.depth == 0 && self.arrows == 0 && it.kind == VariableDeclarationKind::Var;
        walk_js::walk_variable_declaration(self, it);
    }

    fn visit_this_expression(&mut self, _it: &ThisExpression) {
        self.bad |= self.depth == 0 && !self.arrow;
    }

    fn visit_new_target(&mut self, _it: &NewTarget) {
        self.bad |= self.depth == 0 && !self.arrow;
    }

    fn visit_identifier_reference(&mut self, it: &IdentifierReference<'a>) {
        self.bad |= self.depth == 0 && !self.arrow && it.name.as_str() == "arguments";
    }
}

struct Refs<'s> {
    scoping: &'s Scoping,
    symbol: SymbolId,
    count: usize,
}

impl<'a> VisitJs<'a> for Refs<'_> {
    fn visit_identifier_reference(&mut self, it: &IdentifierReference<'a>) {
        let hit = it
            .reference_id
            .get()
            .and_then(|r| self.scoping.get_reference(r).symbol_id())
            == Some(self.symbol);
        self.count += usize::from(hit);
    }
}

fn later_only(
    statements: &[Statement<'_>],
    at: usize,
    symbol: SymbolId,
    scoping: &Scoping,
) -> bool {
    let mut refs = Refs {
        scoping,
        symbol,
        count: 0,
    };
    for statement in &statements[at + 1..] {
        refs.visit_statement(statement);
    }
    scoping.symbol_redeclarations(symbol).is_empty()
        && refs.count == scoping.get_resolved_reference_ids(symbol).len()
}

fn letable(statements: &[Statement<'_>], scoping: &Scoping) -> bool {
    statements.iter().enumerate().all(|(at, statement)| {
        let Statement::VariableDeclaration(declaration) = statement else {
            return true;
        };
        declaration.kind != VariableDeclarationKind::Var
            || declaration.declarations.iter().all(|declarator| {
                declarator.id.get_binding_identifiers().iter().all(|id| {
                    id.symbol_id
                        .get()
                        .is_some_and(|symbol| later_only(statements, at, symbol, scoping))
                })
            })
    })
}

fn body_of<'k, 'a>(
    expression: &'k Expression<'a>,
    scoping: &Scoping,
) -> Option<(&'k [Statement<'a>], bool)> {
    match expression.without_parentheses() {
        Expression::ArrowFunctionExpression(arrow) if !arrow.r#async => match &arrow.body {
            ArrowFunctionBody::FunctionBody(body) if body.directives.is_empty() => {
                arrow.params.items.is_empty().then_some(())?;
                arrow.params.rest.is_none().then_some(())?;
                Some((body.statements.as_slice(), true))
            }
            _ => None,
        },
        Expression::FunctionExpression(function) if !function.r#async && !function.generator => {
            let body = function.body.as_ref()?;
            let plain = body.directives.is_empty()
                && function.params.items.is_empty()
                && function.params.rest.is_none()
                && function.id.as_ref().is_none_or(|id| {
                    id.symbol_id
                        .get()
                        .is_some_and(|own| scoping.get_resolved_reference_ids(own).is_empty())
                });
            plain.then_some((body.statements.as_slice(), false))
        }
        _ => None,
    }
}

fn closure_of<'k, 'a>(
    declaration: &'k VariableDeclaration<'a>,
) -> Option<(SymbolId, &'k Expression<'a>)> {
    let [declarator] = declaration.declarations.as_slice() else {
        return None;
    };
    let BindingPattern::BindingIdentifier(id) = &declarator.id else {
        return None;
    };
    Some((id.symbol_id.get()?, declarator.init.as_ref()?))
}

fn calls(statement: &Statement<'_>, closure: SymbolId, scoping: &Scoping) -> bool {
    let Statement::BlockStatement(block) = statement else {
        return false;
    };
    let [Statement::ExpressionStatement(it)] = block.body.as_slice() else {
        return false;
    };
    matches!(&it.expression, Expression::CallExpression(call)
        if !call.optional && call.arguments.is_empty() && resolved(&call.callee, scoping) == Some(closure))
}

fn site(attempt: &TryStatement<'_>, scoping: &Scoping) -> bool {
    let [Statement::VariableDeclaration(declaration), Statement::ForStatement(looped)] =
        attempt.block.body.as_slice()
    else {
        return false;
    };
    let Some((closure, init)) = closure_of(declaration) else {
        return false;
    };
    let Some((statements, arrow)) = body_of(init, scoping) else {
        return false;
    };
    let mut scan = Scan {
        depth: 0,
        arrows: 0,
        arrow,
        bad: false,
    };
    for statement in statements {
        if let Statement::VariableDeclaration(top) = statement {
            if top.kind == VariableDeclarationKind::Var {
                for declarator in &top.declarations {
                    if let Some(init) = &declarator.init {
                        scan.visit_expression(init);
                    }
                }
                continue;
            }
        }
        scan.visit_statement(statement);
    }
    attempt.handler.is_some()
        && attempt.finalizer.is_some()
        && !scan.bad
        && scoping.get_resolved_reference_ids(closure).len() == 1
        && scoping.symbol_redeclarations(closure).is_empty()
        && calls(&looped.body, closure, scoping)
        && letable(statements, scoping)
}

fn take_body<'a>(init: &mut Expression<'a>, cx: &Cx<'a, '_>) -> Option<AVec<'a, Statement<'a>>> {
    match init {
        Expression::ArrowFunctionExpression(arrow) => match &mut arrow.body {
            ArrowFunctionBody::FunctionBody(body) => Some(body.statements.take_in(cx)),
            _ => None,
        },
        Expression::FunctionExpression(function) => {
            Some(function.body.as_mut()?.statements.take_in(cx))
        }
        _ => None,
    }
}

fn inline<'a>(attempt: &mut TryStatement<'a>, cx: &Cx<'a, '_>) -> Option<()> {
    let mut statements = attempt.block.body.take_in(cx);
    let Some(Statement::ForStatement(mut looped)) = statements.pop() else {
        return None;
    };
    let Some(Statement::VariableDeclaration(mut declaration)) = statements.pop() else {
        return None;
    };
    let mut moved = take_body(declaration.declarations.first_mut()?.init.as_mut()?, cx)?;
    for statement in moved.iter_mut() {
        if let Statement::VariableDeclaration(declaration) = statement {
            if declaration.kind == VariableDeclarationKind::Var {
                declaration.kind = VariableDeclarationKind::Let;
            }
        }
    }
    let Statement::BlockStatement(block) = &mut looped.body else {
        return None;
    };
    block.body = moved;
    attempt.block.body.push(Statement::ForStatement(looped));
    Some(())
}

pub fn rewrite_statements<'a>(
    statements: &mut AVec<'a, Statement<'a>>,
    cx: &mut Cx<'a, '_>,
) -> usize {
    let sites: Vec<usize> = statements
        .iter()
        .enumerate()
        .filter(|(_, statement)| {
            matches!(statement, Statement::TryStatement(attempt) if site(attempt, cx.scoping()))
        })
        .map(|(at, _)| at)
        .collect();
    let mut done = 0;
    for at in sites {
        if let Some(Statement::TryStatement(attempt)) = statements.get_mut(at) {
            if inline(attempt, cx).is_some() {
                done += 1;
            }
        }
    }
    if done > 0 {
        cx.binding_moved();
    }
    done
}

pub fn may_hold_site(statements: &[Statement<'_>]) -> bool {
    statements.iter().any(|statement| {
        matches!(statement, Statement::TryStatement(attempt)
        if attempt.block.body.len() == 2)
    })
}
