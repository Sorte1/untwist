use oxc_allocator::{Address, GetAddress, UnstableAddress};
use oxc_ast::ast::*;
use oxc_ast::AstKind;
use oxc_syntax::operator::AssignmentOperator;
use oxc_syntax::symbol::SymbolId;

use super::numeric::NumericWrite;
use super::{Walker, KIND_DEPTH};

impl<'s, 'a> Walker<'s, 'a> {
    pub(super) fn rewritten_before_here(
        &self,
        key: &Expression<'a>,
    ) -> Option<(SymbolId, Address)> {
        let Expression::Identifier(id) = key.without_parentheses() else {
            return None;
        };
        let symbol = self.symbol(id)?;
        let at = self.stack.iter().rposition(|kind| {
            matches!(
                kind,
                AstKind::Function(_)
                    | AstKind::ArrowFunctionExpression(_)
                    | AstKind::StaticBlock(_)
                    | AstKind::PropertyDefinition(_)
                    | AstKind::AccessorProperty(_)
            )
        })?;
        let AstKind::Function(function) = self.stack[at] else {
            return None;
        };
        let body = function.body.as_ref()?;
        let statement = match (self.stack.get(at + 1), self.stack.get(at + 2)) {
            (Some(AstKind::FunctionBody(_)), Some(statement)) => statement.address(),
            _ => return None,
        };
        let index = body
            .statements
            .iter()
            .position(|item| item.address() == statement)?;
        body.statements[..index]
            .iter()
            .any(|item| assigns(item, symbol, self))
            .then(|| (symbol, function.unstable_address()))
    }

    pub(super) fn numeric_rewrite(&self, symbol: SymbolId, function: Address) -> bool {
        let settled = !self.dynamic_scope
            && !self.opaque.contains(&symbol)
            && !self.shared_root(symbol)
            && !self.annex.may_write(symbol)
            && self.scoping.symbol_redeclarations(symbol).is_empty()
            && !self.arguments_used.contains(&function);
        settled && self.numeric_writes.contains_key(&symbol) && self.writes_numeric(symbol)
    }

    pub(super) fn writes_numeric(&self, symbol: SymbolId) -> bool {
        self.numeric_writes.get(&symbol).is_none_or(|writes| {
            writes.iter().all(|write| match write {
                NumericWrite::Numeric => true,
                NumericWrite::Value(value) => self.numeric_key(value, KIND_DEPTH),
                NumericWrite::Unknown => false,
            })
        })
    }
}

fn assigns(statement: &Statement<'_>, symbol: SymbolId, walker: &Walker<'_, '_>) -> bool {
    let Statement::ExpressionStatement(statement) = statement else {
        return false;
    };
    let Expression::AssignmentExpression(assign) = statement.expression.without_parentheses()
    else {
        return false;
    };
    let AssignmentTarget::AssignmentTargetIdentifier(target) = &assign.left else {
        return false;
    };
    let overwrites = match assign.operator {
        AssignmentOperator::Assign => true,
        operator => !operator.is_logical() && operator != AssignmentOperator::Addition,
    };
    if !overwrites {
        return false;
    }
    walker.symbol(target) == Some(symbol)
}
