use oxc_allocator::{Allocator, TakeIn, Vec as AVec};
use oxc_ast::ast::*;
use oxc_ast::builder::GetAstBuilder;
use oxc_ast_visit::{walk_mut, VisitMut};
use oxc_span::Span;
use oxc_str::Ident;
use oxc_syntax::operator::{BinaryOperator, UnaryOperator};
use rustc_hash::FxHashMap;

use super::rewrite::Site;

pub fn apply<'a, B: GetAstBuilder<'a>>(
    program: &mut Program<'a>,
    sites: &FxHashMap<Span, Site>,
    alloc: &'a Allocator,
    builder: &B,
) -> Vec<&'static str> {
    let mut rewriter = Rewriter {
        sites,
        alloc,
        builder,
        done: Vec::new(),
    };
    rewriter.visit_program(program);
    rewriter.done
}

struct Rewriter<'r, 'a, B> {
    sites: &'r FxHashMap<Span, Site>,
    alloc: &'a Allocator,
    builder: &'r B,
    done: Vec<&'static str>,
}

impl<'a, B: GetAstBuilder<'a>> Rewriter<'_, 'a, B> {
    fn name(&self, text: &str) -> Ident<'a> {
        Ident::from(self.alloc.alloc_str(text))
    }

    fn global(&self, span: Span, name: &str) -> Expression<'a> {
        Expression::new_identifier(span, self.name(name), self.builder)
    }

    fn member(&self, span: Span, object: Expression<'a>, property: &str) -> Expression<'a> {
        let property = IdentifierName::new(span, self.name(property), self.builder);
        Expression::new_static_member_expression(span, object, property, false, self.builder)
    }

    fn call(
        &self,
        span: Span,
        callee: Expression<'a>,
        arguments: AVec<'a, Argument<'a>>,
    ) -> Expression<'a> {
        Expression::new_call_expression(span, callee, None, arguments, false, self.builder)
    }

    fn unbound(&self, span: Span, callee: Expression<'a>) -> Expression<'a> {
        let zero =
            Expression::new_numeric_literal(span, 0.0, None, NumberBase::Decimal, self.builder);
        let mut parts = AVec::with_capacity_in(2, &self.alloc);
        parts.push(zero);
        parts.push(callee);
        Expression::new_sequence_expression(span, parts, self.builder)
    }

    fn replacement(&self, call: &mut CallExpression<'a>, site: &Site) -> Option<Expression<'a>> {
        let span = call.span;
        Some(match site {
            Site::Symbol(name) => self.member(span, self.global(span, "Symbol"), name),
            Site::Global(name) => self.global(span, name),
            Site::Member {
                object,
                property,
                unbound,
            } => {
                let member = self.member(span, self.global(span, object), property);
                match unbound {
                    true => self.unbound(span, member),
                    false => member,
                }
            }
            Site::HasOwn => {
                let callee = self.member(span, self.global(span, "Object"), "hasOwn");
                let arguments = call.arguments.take_in(&self.alloc);
                self.call(span, callee, arguments)
            }
            Site::Method { object, method } => {
                let prototype = self.member(span, self.global(span, object), "prototype");
                let callee = self.member(span, self.member(span, prototype, method), "call");
                let arguments = call.arguments.take_in(&self.alloc);
                self.call(span, callee, arguments)
            }
            Site::Typeof => {
                let argument = call.arguments.pop()?.into_expression();
                let typeof_ = Expression::new_unary_expression(
                    span,
                    UnaryOperator::Typeof,
                    argument,
                    self.builder,
                );
                let function = Expression::new_string_literal(span, "function", None, self.builder);
                Expression::new_binary_expression(
                    span,
                    typeof_,
                    BinaryOperator::Equality,
                    function,
                    self.builder,
                )
            }
        })
    }
}

impl<'a, B: GetAstBuilder<'a>> VisitMut<'a> for Rewriter<'_, 'a, B> {
    fn visit_expression(&mut self, it: &mut Expression<'a>) {
        walk_mut::walk_expression(self, it);
        let Expression::CallExpression(call) = it else {
            return;
        };
        let Some(site) = self.sites.get(&call.span) else {
            return;
        };
        if let Some(replacement) = self.replacement(call, site) {
            self.done.push(site.kind());
            *it = replacement;
        }
    }
}
