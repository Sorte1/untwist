use oxc_ast::ast::*;
use oxc_ast_visit::{walk_js, VisitJs};
use oxc_semantic::Scoping;
use oxc_span::Span;
use oxc_syntax::operator::UnaryOperator;
use oxc_syntax::scope::{ScopeFlags, ScopeId};
use oxc_syntax::symbol::SymbolId;
use rustc_hash::{FxHashMap, FxHashSet};
use untwist_eval::fold::Const;

use super::eval::literal_argument;
use crate::bindings::resolved_symbol;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Role {
    Func,
    Data,
    Primitive,
}

pub(super) enum Arg {
    Literal(Const),
    Call(Span),
}

pub(super) struct Site {
    pub(super) span: Span,
    pub(super) position: u32,
    pub(super) symbol: SymbolId,
    pub(super) args: Option<Vec<Const>>,
    pub(super) nested: Option<Vec<Arg>>,
    pub(super) hoisted: Vec<ScopeId>,
}

#[derive(Default)]
pub(super) struct Uses {
    pub(super) sites: Vec<Site>,
    pub(super) escaped: FxHashSet<SymbolId>,
    pub(super) references: FxHashMap<SymbolId, Vec<Span>>,
    pub(super) declared_at: FxHashMap<SymbolId, u32>,
}

struct Walk<'s> {
    scoping: &'s Scoping,
    tracked: &'s FxHashMap<SymbolId, Role>,
    hoisted: Vec<ScopeId>,
    position: u32,
    found: Uses,
}

pub(super) fn uses(
    program: &Program<'_>,
    scoping: &Scoping,
    tracked: &FxHashMap<SymbolId, Role>,
) -> Uses {
    let mut walk = Walk {
        scoping,
        tracked,
        hoisted: Vec::new(),
        position: 0,
        found: Uses::default(),
    };
    walk.visit_program(program);
    walk.found
}

fn literal_arguments(arguments: &[Argument<'_>]) -> Option<Vec<Const>> {
    arguments
        .iter()
        .map(|argument| literal_argument(argument.as_expression()?))
        .collect()
}

fn nested_arguments(arguments: &[Argument<'_>]) -> Option<Vec<Arg>> {
    arguments
        .iter()
        .map(|argument| {
            let expr = argument.as_expression()?;
            match expr {
                Expression::CallExpression(call) => Some(Arg::Call(call.span)),
                other => literal_argument(other).map(Arg::Literal),
            }
        })
        .collect()
}

fn member_object<'k, 'a>(expr: &'k Expression<'a>) -> Option<&'k IdentifierReference<'a>> {
    let object = match expr.without_parentheses() {
        Expression::StaticMemberExpression(member) => &member.object,
        Expression::ComputedMemberExpression(member) => &member.object,
        _ => return None,
    };
    match object.without_parentheses() {
        Expression::Identifier(id) => Some(id),
        _ => None,
    }
}

fn member_key<'k, 'a>(expr: &'k Expression<'a>) -> Option<&'k Expression<'a>> {
    match expr.without_parentheses() {
        Expression::ComputedMemberExpression(member) => Some(&member.expression),
        _ => None,
    }
}

impl Walk<'_> {
    fn advance(&mut self) -> u32 {
        self.position += 1;
        self.position
    }

    fn role(&self, id: &IdentifierReference<'_>) -> Option<(SymbolId, Role)> {
        let symbol = resolved_symbol(id, self.scoping)?;
        Some((symbol, *self.tracked.get(&symbol)?))
    }

    fn record(&mut self, symbol: SymbolId, span: Span, escapes: bool) {
        self.found.references.entry(symbol).or_default().push(span);
        if escapes {
            self.found.escaped.insert(symbol);
        }
    }

    fn tracked_member(&self, expr: &Expression<'_>) -> Option<(SymbolId, Role, Span)> {
        let id = member_object(expr)?;
        let (symbol, role) = self.role(id)?;
        Some((symbol, role, id.span))
    }

    fn escape_member<'a>(&mut self, expr: &Expression<'a>) -> bool {
        let Some((symbol, role, span)) = self.tracked_member(expr) else {
            return false;
        };
        self.record(symbol, span, role != Role::Primitive);
        if let Some(key) = member_key(expr) {
            self.visit_expression(key);
        }
        true
    }

    fn direct_call(&mut self, it: &CallExpression<'_>) -> bool {
        if it.optional {
            return false;
        }
        let Expression::Identifier(callee) = &it.callee else {
            return false;
        };
        let Some((symbol, Role::Func)) = self.role(callee) else {
            return false;
        };
        self.record(symbol, callee.span, false);
        let position = self.advance();
        let args = literal_arguments(&it.arguments);
        let nested = args
            .is_none()
            .then(|| nested_arguments(&it.arguments))
            .flatten();
        self.found.sites.push(Site {
            span: it.span,
            position,
            symbol,
            args,
            nested,
            hoisted: self.hoisted.clone(),
        });
        true
    }
}

impl<'a> VisitJs<'a> for Walk<'_> {
    fn visit_function(&mut self, it: &Function<'a>, flags: ScopeFlags) {
        let hoisted = it.r#type == FunctionType::FunctionDeclaration;
        let scope = it.scope_id.get().filter(|_| hoisted);
        if let Some(scope) = scope {
            self.hoisted.push(scope);
        }
        walk_js::walk_function(self, it, flags);
        if scope.is_some() {
            self.hoisted.pop();
        }
    }

    fn visit_variable_declarator(&mut self, it: &VariableDeclarator<'a>) {
        walk_js::walk_variable_declarator(self, it);
        let BindingPattern::BindingIdentifier(id) = &it.id else {
            return;
        };
        let Some(symbol) = id
            .symbol_id
            .get()
            .filter(|symbol| self.tracked.contains_key(symbol))
        else {
            return;
        };
        let position = self.advance();
        self.found.declared_at.insert(symbol, position);
    }

    fn visit_call_expression(&mut self, it: &CallExpression<'a>) {
        if self.direct_call(it) || self.escape_member(&it.callee) {
            for argument in &it.arguments {
                self.visit_argument(argument);
            }
            return;
        }
        walk_js::walk_call_expression(self, it);
    }

    fn visit_tagged_template_expression(&mut self, it: &TaggedTemplateExpression<'a>) {
        if self.escape_member(&it.tag) {
            self.visit_template_literal(&it.quasi);
            return;
        }
        walk_js::walk_tagged_template_expression(self, it);
    }

    fn visit_unary_expression(&mut self, it: &UnaryExpression<'a>) {
        if it.operator == UnaryOperator::Delete && self.escape_member(&it.argument) {
            return;
        }
        walk_js::walk_unary_expression(self, it);
    }

    fn visit_expression(&mut self, it: &Expression<'a>) {
        let member = matches!(
            it,
            Expression::StaticMemberExpression(_) | Expression::ComputedMemberExpression(_)
        );
        if let Some((symbol, role, span)) = member.then(|| self.tracked_member(it)).flatten() {
            self.record(symbol, span, role == Role::Func);
            if let Some(key) = member_key(it) {
                self.visit_expression(key);
            }
            return;
        }
        walk_js::walk_expression(self, it);
    }

    fn visit_identifier_reference(&mut self, it: &IdentifierReference<'a>) {
        if let Some((symbol, role)) = self.role(it) {
            self.record(symbol, it.span, role != Role::Primitive);
        }
    }
}
