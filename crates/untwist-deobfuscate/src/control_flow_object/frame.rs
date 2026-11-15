use oxc_allocator::Vec as AVec;
use oxc_ast::ast::*;
use oxc_ast_visit::{walk_js, VisitJs};
use oxc_semantic::Scoping;
use oxc_syntax::operator::{BinaryOperator, UnaryOperator};
use oxc_syntax::scope::ScopeId;
use oxc_syntax::symbol::SymbolFlags;
use untwist_core::ast_util::frame_reads::reads_function_frame_through_bindings;
use untwist_core::ast_util::inline::returned_expression;

use super::is_literal_only;

const CALLEE: &str = "callee";

#[derive(Default)]
struct CalleeReads {
    found: bool,
}

impl<'a> VisitJs<'a> for CalleeReads {
    fn visit_static_member_expression(&mut self, it: &StaticMemberExpression<'a>) {
        self.found |= it.property.name == CALLEE;
        walk_js::walk_static_member_expression(self, it);
    }

    fn visit_string_literal(&mut self, it: &StringLiteral<'a>) {
        self.found |= it.value.as_str().is_some_and(|text| text.contains(CALLEE));
    }

    fn visit_template_literal(&mut self, it: &TemplateLiteral<'a>) {
        self.found |= it
            .quasis
            .iter()
            .any(|quasi| quasi.value.raw.as_str().contains(CALLEE));
        walk_js::walk_template_literal(self, it);
    }
}

pub(super) fn frame_read_reachable(program: &Program<'_>, scoping: &Scoping) -> bool {
    if reads_function_frame_through_bindings(program, scoping) {
        return true;
    }
    let mut callee = CalleeReads::default();
    callee.visit_program(program);
    callee.found
}

fn is_strict(scoping: &Scoping, scope: Option<ScopeId>) -> bool {
    scope.is_some_and(|scope| scoping.scope_flags(scope).is_strict_mode())
}

pub(super) fn proxy_parent_is_strict(function: &Function<'_>, scoping: &Scoping) -> bool {
    let parent = function
        .scope_id
        .get()
        .and_then(|scope| scoping.scope_parent_id(scope));
    is_strict(scoping, parent)
}

fn param_index(function: &Function<'_>, name: &str) -> Option<usize> {
    let items = function.params.items.iter().map(|item| &item.pattern);
    let rest = function.params.rest.iter().map(|rest| &rest.rest.argument);
    items.chain(rest).position(|pattern| match pattern {
        BindingPattern::BindingIdentifier(id) => id.name == name,
        _ => false,
    })
}

struct Exposure<'k, 'f, 'g> {
    function: &'k Function<'f>,
    arguments: &'k [Argument<'g>],
    scoping: &'k Scoping,
    exposed: bool,
}

impl<'g> Exposure<'_, '_, 'g> {
    fn bound_argument(&self, id: &IdentifierReference<'_>) -> Option<Option<&Argument<'g>>> {
        let index = param_index(self.function, id.name.as_str())?;
        Some(self.arguments.get(index))
    }

    fn is_known_strict_function(&self, argument: &Argument<'_>) -> bool {
        let Some(expr) = argument.as_expression() else {
            return false;
        };
        match expr.without_parentheses() {
            Expression::FunctionExpression(function) => {
                is_strict(self.scoping, function.scope_id.get())
            }
            Expression::ArrowFunctionExpression(arrow) => {
                is_strict(self.scoping, arrow.scope_id.get())
            }
            Expression::Identifier(id) => self.is_strict_declaration(id),
            _ => false,
        }
    }

    fn is_strict_declaration(&self, id: &IdentifierReference<'_>) -> bool {
        let symbol = id
            .reference_id
            .get()
            .and_then(|reference| self.scoping.get_reference(reference).symbol_id());
        let Some(symbol) = symbol else {
            return false;
        };
        let declared = self.scoping.symbol_flags(symbol) == SymbolFlags::Function;
        let never_written = self
            .scoping
            .get_resolved_references(symbol)
            .all(|reference| !reference.is_write());
        declared
            && never_written
            && is_strict(self.scoping, Some(self.scoping.symbol_scope_id(symbol)))
    }

    fn is_primitive(&self, expr: &Expression<'_>) -> bool {
        match expr.without_parentheses() {
            Expression::BinaryExpression(_)
            | Expression::UnaryExpression(_)
            | Expression::TemplateLiteral(_) => true,
            Expression::Identifier(id) => match self.bound_argument(id) {
                Some(Some(argument)) => argument.as_expression().is_some_and(is_literal_only),
                Some(None) => true,
                None => false,
            },
            other => is_literal_only(other),
        }
    }

    fn coerces(&mut self, operands: &[&Expression<'_>]) {
        self.exposed |= operands.iter().any(|operand| !self.is_primitive(operand));
    }

    fn called(&mut self, callee: &Expression<'_>) {
        let strict = match callee.without_parentheses() {
            Expression::Identifier(id) => match self.bound_argument(id) {
                Some(Some(argument)) => self.is_known_strict_function(argument),
                Some(None) => true,
                None => false,
            },
            _ => false,
        };
        self.exposed |= !strict;
    }
}

impl<'a> VisitJs<'a> for Exposure<'_, '_, '_> {
    fn visit_expression(&mut self, it: &Expression<'a>) {
        if self.exposed {
            return;
        }
        match it {
            Expression::BooleanLiteral(_)
            | Expression::NullLiteral(_)
            | Expression::NumericLiteral(_)
            | Expression::BigIntLiteral(_)
            | Expression::StringLiteral(_)
            | Expression::RegExpLiteral(_)
            | Expression::ThisExpression(_)
            | Expression::FunctionExpression(_)
            | Expression::ArrowFunctionExpression(_)
            | Expression::Identifier(_)
            | Expression::ParenthesizedExpression(_)
            | Expression::SequenceExpression(_)
            | Expression::ConditionalExpression(_)
            | Expression::LogicalExpression(_)
            | Expression::ArrayExpression(_)
            | Expression::CallExpression(_)
            | Expression::NewExpression(_)
            | Expression::BinaryExpression(_)
            | Expression::UnaryExpression(_)
            | Expression::TemplateLiteral(_) => walk_js::walk_expression(self, it),
            _ => self.exposed = true,
        }
    }

    fn visit_identifier_reference(&mut self, it: &IdentifierReference<'a>) {
        if self.bound_argument(it).is_some() {
            return;
        }
        let resolved = it
            .reference_id
            .get()
            .and_then(|reference| self.scoping.get_reference(reference).symbol_id());
        self.exposed |= resolved.is_none();
    }

    fn visit_function(&mut self, _it: &Function<'a>, _flags: oxc_syntax::scope::ScopeFlags) {}

    fn visit_arrow_function_expression(&mut self, _it: &ArrowFunctionExpression<'a>) {}

    fn visit_call_expression(&mut self, it: &CallExpression<'a>) {
        self.called(&it.callee);
        walk_js::walk_call_expression(self, it);
    }

    fn visit_new_expression(&mut self, it: &NewExpression<'a>) {
        self.called(&it.callee);
        walk_js::walk_new_expression(self, it);
    }

    fn visit_array_expression(&mut self, it: &ArrayExpression<'a>) {
        self.exposed |= it
            .elements
            .iter()
            .any(|element| matches!(element, ArrayExpressionElement::SpreadElement(_)));
        walk_js::walk_array_expression(self, it);
    }

    fn visit_argument(&mut self, it: &Argument<'a>) {
        self.exposed |= matches!(it, Argument::SpreadElement(_));
        walk_js::walk_argument(self, it);
    }

    fn visit_binary_expression(&mut self, it: &BinaryExpression<'a>) {
        if !matches!(
            it.operator,
            BinaryOperator::StrictEquality | BinaryOperator::StrictInequality
        ) {
            self.coerces(&[&it.left, &it.right]);
        }
        walk_js::walk_binary_expression(self, it);
    }

    fn visit_unary_expression(&mut self, it: &UnaryExpression<'a>) {
        match it.operator {
            UnaryOperator::LogicalNot | UnaryOperator::Typeof | UnaryOperator::Void => {}
            UnaryOperator::Delete => self.exposed = true,
            UnaryOperator::UnaryNegation | UnaryOperator::UnaryPlus | UnaryOperator::BitwiseNot => {
                self.coerces(&[&it.argument]);
            }
        }
        walk_js::walk_unary_expression(self, it);
    }

    fn visit_template_literal(&mut self, it: &TemplateLiteral<'a>) {
        let operands: Vec<&Expression<'a>> = it.expressions.iter().collect();
        self.coerces(&operands);
        walk_js::walk_template_literal(self, it);
    }
}

pub(super) fn exposes_a_frame(
    function: &Function<'_>,
    arguments: &AVec<'_, Argument<'_>>,
    site_is_strict: bool,
    scoping: &Scoping,
) -> bool {
    if site_is_strict && is_strict(scoping, function.scope_id.get()) {
        return false;
    }
    let Some(returned) = returned_expression(function) else {
        return true;
    };
    let mut exposure = Exposure {
        function,
        arguments: arguments.as_slice(),
        scoping,
        exposed: false,
    };
    exposure.visit_expression(returned);
    exposure.exposed
}
