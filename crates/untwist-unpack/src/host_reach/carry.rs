use oxc_ast::AstKind;
use oxc_syntax::operator::{AssignmentOperator, LogicalOperator, UnaryOperator};
use oxc_syntax::symbol::SymbolId;

use super::*;

const ENUMERABLE: &str = "enumerable";

impl<'a> Reach<'_, 'a> {
    pub(super) fn identifier_symbol(&self, expr: &Expression<'_>) -> Option<SymbolId> {
        match expr.without_parentheses() {
            Expression::Identifier(id) => self.symbol(id),
            _ => None,
        }
    }

    pub(super) fn read_source(&self, arrow: &ArrowFunctionExpression<'a>) -> Option<SymbolId> {
        match arrow.get_expression()?.without_parentheses() {
            Expression::ComputedMemberExpression(member) => self.identifier_symbol(&member.object),
            _ => None,
        }
    }

    pub(super) fn source_carries(&self, source: Option<SymbolId>) -> Option<Held<'a>> {
        source.and_then(|symbol| self.tainted.get(&symbol).copied())
    }

    pub(super) fn carry(
        &mut self,
        target: Option<&'a Expression<'a>>,
        carried: Option<Held<'a>>,
    ) -> bool {
        match carried {
            None => true,
            Some(held) => target.is_some_and(|target| self.carries(target, held)),
        }
    }

    pub(super) fn carries(&mut self, target: &Expression<'a>, held: Held<'a>) -> bool {
        match self.identifier_symbol(target) {
            Some(symbol) => {
                self.taint(symbol, held);
                true
            }
            None => false,
        }
    }

    pub(super) fn getter_carries(
        &mut self,
        call: &'a CallExpression<'a>,
        descriptor: &'a ObjectExpression<'a>,
        source: Option<SymbolId>,
        key: &'a Expression<'a>,
    ) -> bool {
        let kept = source.is_some_and(|source| {
            self.writes.stable(self.scoping, source)
                && self.keeps_enumerability(descriptor, source, key)
        });
        let carried = match kept {
            true => self.source_carries(source),
            false => Some(Held::Read(Some(key))),
        };
        let target = call.arguments.first().and_then(Argument::as_expression);
        self.carry(target, carried)
    }

    fn keeps_enumerability(
        &self,
        descriptor: &ObjectExpression<'a>,
        source: SymbolId,
        key: &Expression<'a>,
    ) -> bool {
        let mut enumerable = None;
        for property in &descriptor.properties {
            let ObjectPropertyKind::ObjectProperty(property) = property else {
                return false;
            };
            if property.computed {
                return false;
            }
            let named = property
                .key
                .static_name()
                .is_some_and(|name| name.as_str() == Some(ENUMERABLE));
            if named {
                enumerable = Some(&property.value);
            }
        }
        let Some(value) = enumerable else {
            return true;
        };
        let hidden =
            matches!(value.without_parentheses(), Expression::BooleanLiteral(flag) if !flag.value);
        hidden || (self.own_flag(value, source, key) && self.own_key(key, source))
    }

    fn own_flag(&self, value: &Expression<'a>, source: SymbolId, key: &Expression<'a>) -> bool {
        let Expression::LogicalExpression(logical) = value.without_parentheses() else {
            return false;
        };
        let Expression::UnaryExpression(not) = logical.left.without_parentheses() else {
            return false;
        };
        let Expression::AssignmentExpression(assign) = not.argument.without_parentheses() else {
            return false;
        };
        let AssignmentTarget::AssignmentTargetIdentifier(desc) = &assign.left else {
            return false;
        };
        let Expression::StaticMemberExpression(flag) = logical.right.without_parentheses() else {
            return false;
        };
        let desc = self.symbol(desc);
        logical.operator == LogicalOperator::Or
            && not.operator == UnaryOperator::LogicalNot
            && assign.operator == AssignmentOperator::Assign
            && desc.is_some()
            && flag.property.name == ENUMERABLE
            && self.identifier_symbol(&flag.object) == desc
            && self.reads_own(&assign.right, source, key)
    }

    fn reads_own(&self, read: &Expression<'a>, source: SymbolId, key: &Expression<'a>) -> bool {
        let Expression::CallExpression(read) = read.without_parentheses() else {
            return false;
        };
        let reader = self.builtin_member(&read.callee, &[OBJECT], &[DESCRIPTOR])
            || self.alias(&read.callee, &self.writes.readers);
        let arguments = match read.arguments.as_slice() {
            [object, named] => object
                .as_expression()
                .zip(named.as_expression())
                .is_some_and(|(object, named)| {
                    self.identifier_symbol(object) == Some(source)
                        && self.keys().same_key(key, named)
                }),
            _ => false,
        };
        reader && arguments
    }

    fn own_key(&self, key: &Expression<'a>, source: SymbolId) -> bool {
        let Some(symbol) = self.identifier_symbol(key) else {
            return false;
        };
        self.listed_by_loop(symbol, source) || self.listed_by_index(symbol, source)
    }

    fn listed_by_loop(&self, symbol: SymbolId, source: SymbolId) -> bool {
        let unwritten = self
            .scoping
            .get_resolved_reference_ids(symbol)
            .iter()
            .all(|reference| !self.scoping.get_reference(*reference).is_write());
        unwritten
            && self.stack.iter().any(|kind| {
                matches!(kind, AstKind::ForOfStatement(each)
                    if !each.r#await
                        && declares(&each.left, symbol)
                        && self.lists(&each.right, source))
            })
    }

    fn listed_by_index(&self, symbol: SymbolId, source: SymbolId) -> bool {
        self.writes
            .settled(self.scoping, symbol)
            .is_some_and(|writes| {
                !writes.is_empty()
                    && writes.iter().all(|write| match write {
                        Write::Value(value) => self.indexes_list(value, source),
                        _ => false,
                    })
            })
    }

    fn indexes_list(&self, value: &Expression<'_>, source: SymbolId) -> bool {
        let Expression::ComputedMemberExpression(member) = value.without_parentheses() else {
            return false;
        };
        self.identifier_symbol(&member.object)
            .and_then(|list| self.writes.settled(self.scoping, list))
            .is_some_and(
                |writes| matches!(writes, [Write::Value(init)] if self.lists(init, source)),
            )
    }

    fn lists(&self, expr: &Expression<'_>, source: SymbolId) -> bool {
        let Expression::CallExpression(call) = expr.without_parentheses() else {
            return false;
        };
        let listed = match call.arguments.as_slice() {
            [object] => object
                .as_expression()
                .is_some_and(|object| self.identifier_symbol(object) == Some(source)),
            _ => false,
        };
        listed && self.keys().lister(&call.callee, true)
    }

    pub(super) fn sink_carries(&mut self, call: &'a CallExpression<'a>, held: Held<'a>) -> bool {
        let descriptor = call.arguments.get(2).and_then(Argument::as_expression);
        let read = descriptor.and_then(|descriptor| match descriptor.without_parentheses() {
            Expression::CallExpression(read) => Some(read),
            _ => None,
        });
        let direct = read.filter(|read| {
            self.builtin_member(&read.callee, &[OBJECT], &[DESCRIPTOR])
                || self.alias(&read.callee, &self.writes.readers)
        });
        let carried = match direct {
            Some(read) => {
                let source = read
                    .arguments
                    .first()
                    .and_then(Argument::as_expression)
                    .and_then(|object| self.identifier_symbol(object));
                self.source_carries(source)
            }
            None => Some(Held::Read(held.key())),
        };
        let target = call.arguments.first().and_then(Argument::as_expression);
        self.carry(target, carried)
    }

    pub(super) fn copies_carry(
        &mut self,
        call: &'a CallExpression<'a>,
        descriptors: &'a CallExpression<'a>,
    ) -> bool {
        let source = descriptors
            .arguments
            .first()
            .and_then(Argument::as_expression)
            .and_then(|object| self.identifier_symbol(object));
        let carried = self.source_carries(source);
        let defines = self.builtin_member(&call.callee, &[OBJECT], &[COPIERS[0]]);
        match defines {
            true => {
                let target = call.arguments.first().and_then(Argument::as_expression);
                self.carry(target, carried)
            }
            false => carried.is_none(),
        }
    }
}

fn declares(left: &ForStatementLeft<'_>, symbol: SymbolId) -> bool {
    let ForStatementLeft::VariableDeclaration(declaration) = left else {
        return false;
    };
    matches!(declaration.declarations.as_slice(), [declarator]
        if matches!(&declarator.id, BindingPattern::BindingIdentifier(id)
            if id.symbol_id.get() == Some(symbol)))
}
