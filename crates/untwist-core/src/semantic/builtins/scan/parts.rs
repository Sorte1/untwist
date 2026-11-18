use oxc_ast::ast::*;
use oxc_ast_visit::{walk_js, VisitJs};

use super::reach::member_parts;
use super::Scan;

impl Scan<'_> {
    pub(super) fn visit_member_parts(&mut self, expression: &Expression<'_>) -> bool {
        let Some((object, key)) = member_parts(expression) else {
            return false;
        };
        self.visit_unless_root(object);
        if let Some(key) = key {
            self.visit_expression(key);
        }
        true
    }

    pub(super) fn visit_root_parts(&mut self, expression: &Expression<'_>) {
        match expression {
            Expression::BooleanLiteral(_)
            | Expression::NullLiteral(_)
            | Expression::NumericLiteral(_)
            | Expression::BigIntLiteral(_)
            | Expression::RegExpLiteral(_)
            | Expression::StringLiteral(_)
            | Expression::Identifier(_)
            | Expression::Super(_)
            | Expression::ThisExpression(_)
            | Expression::ImportMeta(_)
            | Expression::NewTarget(_) => {}
            Expression::ParenthesizedExpression(parenthesized) => {
                self.visit_root_parts(&parenthesized.expression);
            }
            Expression::StaticMemberExpression(_)
            | Expression::ComputedMemberExpression(_)
            | Expression::PrivateFieldExpression(_) => {
                self.visit_member_parts(expression);
            }
            Expression::ChainExpression(chain) => self.visit_root_chain(expression, chain),
            Expression::CallExpression(call) => self.visit_call_expression(call),
            Expression::TaggedTemplateExpression(tagged) => {
                self.visit_tagged_template_expression(tagged);
            }
            Expression::TemplateLiteral(_)
            | Expression::ArrayExpression(_)
            | Expression::ArrowFunctionExpression(_)
            | Expression::AssignmentExpression(_)
            | Expression::AwaitExpression(_)
            | Expression::BinaryExpression(_)
            | Expression::ClassExpression(_)
            | Expression::ConditionalExpression(_)
            | Expression::FunctionExpression(_)
            | Expression::ImportExpression(_)
            | Expression::LogicalExpression(_)
            | Expression::NewExpression(_)
            | Expression::ObjectExpression(_)
            | Expression::SequenceExpression(_)
            | Expression::UnaryExpression(_)
            | Expression::UpdateExpression(_)
            | Expression::YieldExpression(_)
            | Expression::PrivateInExpression(_)
            | Expression::JSXElement(_)
            | Expression::JSXFragment(_)
            | Expression::TSAsExpression(_)
            | Expression::TSSatisfiesExpression(_)
            | Expression::TSTypeAssertion(_)
            | Expression::TSNonNullExpression(_)
            | Expression::TSInstantiationExpression(_)
            | Expression::V8IntrinsicExpression(_) => walk_js::walk_expression(self, expression),
        }
    }

    fn visit_root_chain(&mut self, expression: &Expression<'_>, chain: &ChainExpression<'_>) {
        match &chain.expression {
            ChainElement::CallExpression(call) => self.visit_call_expression(call),
            ChainElement::StaticMemberExpression(_)
            | ChainElement::ComputedMemberExpression(_)
            | ChainElement::PrivateFieldExpression(_) => {
                self.visit_member_parts(expression);
            }
            ChainElement::TSNonNullExpression(_) => walk_js::walk_expression(self, expression),
        }
    }
}
