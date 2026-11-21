use oxc_ast::AstKind;
use oxc_syntax::operator::{BinaryOperator, UnaryOperator};

use super::holder_uses::harmless_binary;
use super::{same, Walker};

impl<'s, 'a> Walker<'s, 'a> {
    pub(super) fn discarded_read(&self) -> bool {
        let at = self.stack.len() - 1;
        match self.parent(at) {
            Some((up, child)) if self.called_by(up, child) => self.discarded(up),
            _ => self.discarded(at),
        }
    }

    fn called_by(&self, up: usize, child: oxc_allocator::Address) -> bool {
        match self.stack[up] {
            AstKind::CallExpression(call) => same(&call.callee, child),
            AstKind::NewExpression(new) => same(&new.callee, child),
            _ => false,
        }
    }

    fn discarded(&self, at: usize) -> bool {
        let Some((up, child)) = self.parent(at) else {
            return false;
        };
        match self.stack[up] {
            AstKind::ExpressionStatement(_) => true,
            AstKind::UnaryExpression(unary) => match unary.operator {
                UnaryOperator::Typeof | UnaryOperator::Void | UnaryOperator::LogicalNot => true,
                UnaryOperator::Delete => false,
                _ => self.builtins_intact(),
            },
            AstKind::BinaryExpression(binary) => {
                harmless_binary(binary.operator, same(&binary.right, child))
                    || (binary.operator != BinaryOperator::Instanceof && self.builtins_intact())
            }
            AstKind::TemplateLiteral(_) => {
                let tagged = up.checked_sub(1).is_some_and(|tag| {
                    matches!(self.stack[tag], AstKind::TaggedTemplateExpression(_))
                });
                !tagged && self.builtins_intact()
            }
            AstKind::IfStatement(statement) => same(&statement.test, child),
            AstKind::WhileStatement(statement) => same(&statement.test, child),
            AstKind::DoWhileStatement(statement) => same(&statement.test, child),
            AstKind::ConditionalExpression(conditional) => same(&conditional.test, child),
            _ => false,
        }
    }
}
