use oxc_allocator::Vec as AVec;
use oxc_ast::ast::*;
use oxc_ast::builder::AstBuilder;
use oxc_ast_visit::{walk_mut, VisitMut};
use oxc_span::{GetSpan, Span, SPAN};
use oxc_str::Ident;
use rustc_hash::{FxHashMap, FxHashSet};
use untwist_core::ast_util::respan::fresh_span;

use super::sites::Kind;

pub(super) struct Restore {
    pub(super) fns_span: Span,
    pub(super) names: Vec<String>,
    pub(super) exact: Vec<bool>,
}

pub(super) struct Rewrite<'b, 'a> {
    pub(super) builder: &'b AstBuilder<'a>,
    pub(super) standalone: FxHashMap<Span, (String, Kind)>,
    pub(super) pairs: FxHashMap<Span, Vec<(usize, String)>>,
    pub(super) machines: FxHashMap<Span, Restore>,
    pub(super) removals: FxHashSet<Span>,
    pub(super) applied: usize,
    pub(super) failed: bool,
}

impl<'a> Rewrite<'_, 'a> {
    fn span(&mut self) -> Span {
        match fresh_span() {
            Some(span) => span,
            None => {
                self.failed = true;
                SPAN
            }
        }
    }

    fn ident(&self, name: &str) -> Ident<'a> {
        Ident::from_str_in(name, self.builder)
    }

    fn reference(&mut self, name: &str) -> Expression<'a> {
        let span = self.span();
        Expression::new_identifier(span, self.ident(name), self.builder)
    }

    fn call(&mut self, name: &str, arguments: AVec<'a, Argument<'a>>) -> Expression<'a> {
        let callee = self.reference(name);
        let span = self.span();
        Expression::new_call_expression(span, callee, None, arguments, false, self.builder)
    }

    fn arguments(&mut self, store: Expression<'a>) -> AVec<'a, Argument<'a>> {
        let mut arguments = AVec::new_in(self.builder);
        let Expression::AssignmentExpression(assign) = store.without_parentheses_owned() else {
            self.failed = true;
            return arguments;
        };
        let Expression::ArrayExpression(array) = assign.unbox().right.without_parentheses_owned()
        else {
            self.failed = true;
            return arguments;
        };
        for element in array.unbox().elements {
            match element {
                ArrayExpressionElement::SpreadElement(spread) => {
                    arguments.push(Argument::SpreadElement(spread));
                }
                ArrayExpressionElement::Elision(_) => self.failed = true,
                other => arguments.push(Argument::from(other.into_expression())),
            }
        }
        arguments
    }

    fn sequence(&mut self, it: &mut Expression<'a>, pairs: Vec<(usize, String)>) {
        let Expression::SequenceExpression(sequence) = it else {
            self.failed = true;
            return;
        };
        let taken: Vec<Expression<'a>> = sequence.expressions.drain(..).collect();
        let mut rebuilt = AVec::new_in(self.builder);
        let mut skip = false;
        for (index, expression) in taken.into_iter().enumerate() {
            if skip {
                skip = false;
                continue;
            }
            match pairs.iter().find(|(at, _)| *at == index) {
                Some((_, name)) => {
                    let arguments = self.arguments(expression);
                    rebuilt.push(self.call(name, arguments));
                    self.applied += 1;
                    skip = true;
                }
                None => rebuilt.push(expression),
            }
        }
        if rebuilt.len() == 1 {
            *it = rebuilt
                .pop()
                .unwrap_or_else(|| Expression::new_null_literal(SPAN, self.builder));
            return;
        }
        sequence.expressions = rebuilt;
    }

    fn params(&mut self, function: &mut Function<'a>, exact: bool) {
        let Some(body) = function.body.as_mut() else {
            self.failed = true;
            return;
        };
        if body.statements.is_empty() {
            self.failed = true;
            return;
        }
        let Statement::VariableDeclaration(declaration) = &mut body.statements[0] else {
            self.failed = true;
            return;
        };
        if declaration.declarations.is_empty() {
            self.failed = true;
            return;
        }
        let declarator = declaration.declarations.remove(0);
        if declaration.declarations.is_empty() {
            body.statements.remove(0);
        }
        let Some(BindingPattern::ArrayPattern(pattern)) = Some(declarator.id) else {
            self.failed = true;
            return;
        };
        if exact {
            let span = pattern.span;
            let argument = BindingPattern::ArrayPattern(pattern);
            let rest = BindingRestElement::new(span, argument, self.builder);
            function.params.rest = Some(FormalParameterRest::boxed(
                span,
                AVec::new_in(self.builder),
                rest,
                None,
                self.builder,
            ));
            return;
        }
        let pattern = pattern.unbox();
        let mut items = AVec::new_in(self.builder);
        for element in pattern.elements {
            let Some(element) = element else {
                self.failed = true;
                return;
            };
            items.push(FormalParameter::new(
                element.span(),
                AVec::new_in(self.builder),
                element,
                None,
                None,
                false,
                None,
                false,
                false,
                self.builder,
            ));
        }
        function.params.items = items;
        function.params.rest = pattern.rest.map(|rest| {
            let rest = rest.unbox();
            FormalParameterRest::boxed(
                rest.span,
                AVec::new_in(self.builder),
                rest,
                None,
                self.builder,
            )
        });
    }

    fn declaration(
        &mut self,
        value: Expression<'a>,
        name: &str,
        (head, exact): (bool, bool),
    ) -> Option<Statement<'a>> {
        let Expression::FunctionExpression(mut function) = value else {
            return None;
        };
        if head {
            self.params(&mut function, exact);
        }
        function.r#type = FunctionType::FunctionDeclaration;
        let span = self.span();
        function.id = Some(BindingIdentifier::new(span, self.ident(name), self.builder));
        Some(Statement::FunctionDeclaration(function))
    }

    fn restore(&mut self, statement: Statement<'a>, restore: &Restore) -> Vec<Statement<'a>> {
        let mut out = Vec::new();
        let Statement::FunctionDeclaration(mut dispatcher) = statement else {
            self.failed = true;
            return out;
        };
        let Some(body) = dispatcher.body.as_mut() else {
            self.failed = true;
            return out;
        };
        let mut table = None;
        for statement in body.statements.iter_mut() {
            let Statement::VariableDeclaration(declaration) = statement else {
                continue;
            };
            for declarator in declaration.declarations.iter_mut() {
                if declarator.init.as_ref().map(GetSpan::span) == Some(restore.fns_span) {
                    table = declarator.init.take();
                }
            }
        }
        let Some(Expression::ObjectExpression(table)) = table else {
            self.failed = true;
            return out;
        };
        let properties = table.unbox().properties;
        if properties.len() != restore.names.len() {
            self.failed = true;
            return out;
        }
        for (at, (property, name)) in properties.into_iter().zip(&restore.names).enumerate() {
            let exact = restore.exact.get(at).copied().unwrap_or(true);
            let ObjectPropertyKind::ObjectProperty(property) = property else {
                self.failed = true;
                return out;
            };
            let property = property.unbox();
            let head = match &property.value {
                Expression::FunctionExpression(function) => function
                    .body
                    .as_ref()
                    .and_then(|b| b.statements.first())
                    .is_some_and(is_head),
                _ => false,
            };
            match self.declaration(property.value, name, (head, exact)) {
                Some(statement) => out.push(statement),
                None => {
                    self.failed = true;
                    return out;
                }
            }
        }
        self.applied += 1;
        out
    }
}

fn is_head(statement: &Statement<'_>) -> bool {
    let Statement::VariableDeclaration(declaration) = statement else {
        return false;
    };
    matches!(
        (declaration.kind, declaration.declarations.as_slice()),
        (VariableDeclarationKind::Var, [declarator, ..])
            if matches!(declarator.id, BindingPattern::ArrayPattern(_))
                && matches!(declarator.init, Some(Expression::Identifier(_)))
    )
}

trait Unparen<'a> {
    fn without_parentheses_owned(self) -> Expression<'a>;
}

impl<'a> Unparen<'a> for Expression<'a> {
    fn without_parentheses_owned(self) -> Expression<'a> {
        match self {
            Expression::ParenthesizedExpression(paren) => {
                paren.unbox().expression.without_parentheses_owned()
            }
            other => other,
        }
    }
}

impl<'a> VisitMut<'a> for Rewrite<'_, 'a> {
    fn visit_expression(&mut self, it: &mut Expression<'a>) {
        walk_mut::walk_expression(self, it);
        let span = it.span();
        if let Some((name, kind)) = self.standalone.remove(&span) {
            *it = match kind {
                Kind::NonCall => self.reference(&name),
                _ => self.call(&name, AVec::new_in(self.builder)),
            };
            self.applied += 1;
            return;
        }
        if let Some(pairs) = self.pairs.remove(&span) {
            self.sequence(it, pairs);
        }
    }

    fn visit_statements(&mut self, it: &mut AVec<'a, Statement<'a>>) {
        walk_mut::walk_statements(self, it);
        let hits = it.iter().any(|s| {
            let span = s.span();
            self.removals.contains(&span) || self.machines.contains_key(&span)
        });
        if !hits {
            return;
        }
        let taken: Vec<Statement<'a>> = it.drain(..).collect();
        for statement in taken {
            let span = statement.span();
            if self.removals.remove(&span) {
                self.applied += 1;
                continue;
            }
            match self.machines.remove(&span) {
                Some(restore) => {
                    let restored = self.restore(statement, &restore);
                    it.extend(restored);
                }
                None => it.push(statement),
            }
        }
    }
}
