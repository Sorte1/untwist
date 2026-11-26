use oxc_allocator::{Box as ABox, Vec as AVec};
use oxc_ast::ast::*;
use oxc_ast::builder::AstBuilder;
use oxc_ast_visit::{walk_mut, VisitMut};
use oxc_span::{GetSpan, Span, SPAN};
use oxc_str::Ident;
use oxc_syntax::operator::UnaryOperator;
use rustc_hash::FxHashMap;
use untwist_core::ast_util::respan::fresh_span;

use super::sites::Site;

pub(super) struct Sites<'b, 'a> {
    pub(super) builder: &'b AstBuilder<'a>,
    pub(super) expressions: FxHashMap<Span, Site>,
    pub(super) targets: FxHashMap<Span, Site>,
    pub(super) created: Vec<(Span, oxc_syntax::symbol::SymbolId)>,
    pub(super) failed: bool,
}

impl<'a> Sites<'_, 'a> {
    fn span(&mut self) -> Span {
        fresh_span().unwrap_or_else(|| {
            self.failed = true;
            SPAN
        })
    }

    fn reference(&mut self, site: &Site) -> IdentifierReference<'a> {
        let span = self.span();
        self.created.push((span, site.target));
        IdentifierReference::new(
            span,
            Ident::from_str_in(&site.name, self.builder),
            self.builder,
        )
    }
}

impl<'a> VisitMut<'a> for Sites<'_, 'a> {
    fn visit_expression(&mut self, it: &mut Expression<'a>) {
        if let Some(site) = self.expressions.remove(&it.span()) {
            let reference = self.reference(&site);
            let identifier = Expression::Identifier(ABox::new_in(reference, self.builder));
            *it = match site.typeof_read {
                true => {
                    let span = self.span();
                    Expression::new_unary_expression(
                        span,
                        UnaryOperator::Typeof,
                        identifier,
                        self.builder,
                    )
                }
                false => identifier,
            };
            return;
        }
        walk_mut::walk_expression(self, it);
    }

    fn visit_simple_assignment_target(&mut self, it: &mut SimpleAssignmentTarget<'a>) {
        if let Some(site) = self.targets.remove(&it.span()) {
            let reference = self.reference(&site);
            *it = SimpleAssignmentTarget::AssignmentTargetIdentifier(ABox::new_in(
                reference,
                self.builder,
            ));
            return;
        }
        walk_mut::walk_simple_assignment_target(self, it);
    }
}

pub(super) struct Taken<'a> {
    pub(super) params: Option<BindingPattern<'a>>,
    pub(super) body: AVec<'a, Statement<'a>>,
}

pub(super) fn take_flat<'a>(
    function: &mut Function<'a>,
    strict: bool,
    builder: &AstBuilder<'a>,
) -> Option<Taken<'a>> {
    let body = function.body.as_mut()?;
    if strict {
        if body.statements.is_empty() {
            return None;
        }
        let Statement::VariableDeclaration(declaration) = body.statements.remove(0) else {
            return None;
        };
        let declarator = declaration.unbox().declarations.pop()?;
        let BindingPattern::ArrayPattern(pattern) = declarator.id else {
            return None;
        };
        let params = pattern.unbox().elements.into_iter().nth(1).flatten();
        let statements = std::mem::replace(&mut body.statements, AVec::new_in(builder));
        return Some(Taken {
            params,
            body: statements,
        });
    }
    let params = function.params.items.pop().map(|p| p.pattern);
    let empty = AVec::new_in(builder);
    let statements = std::mem::replace(&mut body.statements, empty);
    Some(Taken {
        params,
        body: statements,
    })
}

pub(super) struct Install<'b, 'a> {
    pub(super) builder: &'b AstBuilder<'a>,
    pub(super) wrappers: FxHashMap<Span, Taken<'a>>,
    pub(super) failed: bool,
}

impl<'a> VisitMut<'a> for Install<'_, 'a> {
    fn visit_function(&mut self, it: &mut Function<'a>, flags: oxc_syntax::scope::ScopeFlags) {
        if let Some(taken) = self.wrappers.remove(&it.span) {
            let Some(body) = it.body.as_mut() else {
                self.failed = true;
                return;
            };
            body.statements = taken.body;
            it.params.rest = taken.params.map(|pattern| {
                let span = pattern.span();
                let rest = BindingRestElement::new(span, pattern, self.builder);
                FormalParameterRest::boxed(
                    span,
                    AVec::new_in(self.builder),
                    rest,
                    None,
                    self.builder,
                )
            });
        }
        walk_mut::walk_function(self, it, flags);
    }
}
