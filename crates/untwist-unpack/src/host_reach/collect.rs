use oxc_ast::AstKind;
use oxc_syntax::operator::AssignmentOperator;
use oxc_syntax::symbol::SymbolId;

use super::*;

impl<'a> Reach<'_, 'a> {
    pub(super) fn collect(&mut self, kind: AstKind<'a>) {
        match kind {
            AstKind::VariableDeclarator(declarator) => self.collect_declarator(declarator),
            AstKind::AssignmentExpression(assign) => self.collect_assignment(assign),
            AstKind::CallExpression(call) => self.collect_call(call),
            AstKind::UpdateExpression(update) => {
                if let SimpleAssignmentTarget::AssignmentTargetIdentifier(id) = &update.argument {
                    if let Some(symbol) = self.symbol(id) {
                        self.writes.record(symbol, Write::Numeric, true);
                    }
                }
            }
            AstKind::StaticMemberExpression(member) => {
                let reshapes = member.property.name == PROTO;
                self.collect_reshape(&member.object, reshapes);
            }
            AstKind::ComputedMemberExpression(member) => {
                let reshapes = const_key(&member.expression).is_none_or(|key| key == PROTO);
                self.collect_reshape(&member.object, reshapes);
            }
            AstKind::IdentifierReference(id) => self.collect_escape(id),
            AstKind::Function(function) => self.collect_function(function),
            _ => {}
        }
    }

    pub(super) fn collect_declarator(&mut self, declarator: &'a VariableDeclarator<'a>) {
        let BindingPattern::BindingIdentifier(id) = &declarator.id else {
            return;
        };
        let Some(symbol) = id.symbol_id.get() else {
            return;
        };
        let at = self.stack.len().saturating_sub(1);
        let each = at >= 2
            && matches!(
                self.stack[at - 2],
                AstKind::ForInStatement(_) | AstKind::ForOfStatement(_)
            );
        if each {
            self.writes.loose.insert(symbol);
            return;
        }
        self.writes.declared.insert(symbol);
        let Some(init) = &declarator.init else {
            return;
        };
        self.writes.record(symbol, Write::Value(init), false);
        match init.without_parentheses() {
            Expression::ArrowFunctionExpression(arrow) => {
                self.collect_params(symbol, &arrow.params, false);
            }
            Expression::FunctionExpression(function) => {
                self.collect_params(symbol, &function.params, reads_arguments(function));
            }
            other => match other.as_member_expression().and_then(member_key).as_deref() {
                Some(DESCRIPTOR) => {
                    self.writes.readers.insert(symbol);
                }
                Some(DEFINE) => {
                    self.writes.definers.insert(symbol);
                }
                Some(name) if NAME_LISTERS.contains(&name) => {
                    self.writes.listers.insert(symbol);
                }
                _ => {}
            },
        }
    }

    pub(super) fn collect_reshape(&mut self, object: &Expression<'_>, reshapes: bool) {
        let at = self.stack.len() - 1;
        if !reshapes || !self.written(at) {
            return;
        }
        let Expression::Identifier(id) = object.without_parentheses() else {
            return;
        };
        let Some(symbol) = self.symbol(id) else {
            return;
        };
        match self.stored_value(at) {
            Some(None) => {}
            Some(Some(value)) => self.writes.stored.entry(symbol).or_default().push(value),
            None => {
                self.writes.reshaped.insert(symbol);
            }
        }
    }

    fn stored_value(&self, at: usize) -> Option<Option<&'a Expression<'a>>> {
        let (up, _) = self.parent(at)?;
        match self.stack[up] {
            AstKind::UpdateExpression(_) => Some(None),
            AstKind::AssignmentExpression(assign) => match assign.operator {
                AssignmentOperator::Assign => Some(Some(&assign.right)),
                operator if operator.is_logical() => None,
                _ => Some(None),
            },
            _ => None,
        }
    }

    pub(super) fn collect_escape(&mut self, id: &IdentifierReference<'_>) {
        let Some(symbol) = self.symbol(id) else {
            return;
        };
        if !self.is_read(id) {
            return;
        }
        let at = self.stack.len() - 1;
        let held = self
            .parent(at)
            .is_some_and(|(up, child)| match self.stack[up] {
                AstKind::StaticMemberExpression(member) => same(&member.object, child),
                AstKind::ComputedMemberExpression(member) => same(&member.object, child),
                AstKind::ForInStatement(each) => same(&each.right, child),
                AstKind::LogicalExpression(logical) => {
                    logical.operator == LogicalOperator::And && same(&logical.left, child)
                }
                AstKind::ConditionalExpression(choice) => same(&choice.test, child),
                AstKind::CallExpression(call) => {
                    !same(&call.callee, child) && self.lists_names(&call.callee)
                }
                AstKind::UnaryExpression(_)
                | AstKind::BinaryExpression(_)
                | AstKind::IfStatement(_)
                | AstKind::WhileStatement(_) => true,
                _ => false,
            });
        if !held {
            self.writes.escaped.insert(symbol);
        }
    }

    pub(super) fn lists_names(&self, callee: &Expression<'_>) -> bool {
        match callee.without_parentheses() {
            Expression::Identifier(id) => self
                .symbol(id)
                .is_some_and(|symbol| self.writes.listers.contains(&symbol)),
            other => other
                .as_member_expression()
                .and_then(member_key)
                .is_some_and(|name| NAME_LISTERS.contains(&name.as_str())),
        }
    }

    pub(super) fn collect_function(&mut self, function: &'a Function<'a>) {
        let Some(id) = function.id.as_ref().filter(|_| function.is_declaration()) else {
            return;
        };
        if let Some(symbol) = id.symbol_id.get() {
            self.collect_params(symbol, &function.params, reads_arguments(function));
        }
    }

    pub(super) fn collect_params(
        &mut self,
        symbol: SymbolId,
        params: &FormalParameters<'a>,
        loose: bool,
    ) {
        let symbols: Vec<Option<SymbolId>> = params
            .items
            .iter()
            .map(|param| match &param.pattern {
                BindingPattern::BindingIdentifier(id) if !loose => id.symbol_id.get(),
                _ => None,
            })
            .collect();
        for (index, param) in symbols.iter().enumerate() {
            if let Some(param) = param {
                self.writes.params.insert(*param, (symbol, index));
            }
        }
        self.writes.fn_params.insert(symbol, symbols);
    }

    pub(super) fn collect_call(&mut self, call: &'a CallExpression<'a>) {
        let params = match call.callee.without_parentheses() {
            Expression::ArrowFunctionExpression(arrow) => Some(&arrow.params),
            Expression::FunctionExpression(function) if !reads_arguments(function) => {
                Some(&function.params)
            }
            _ => None,
        };
        if let Some(params) = params {
            self.collect_iife(params, &call.arguments);
            return;
        }
        let Expression::Identifier(id) = call.callee.without_parentheses() else {
            return;
        };
        if let Some(symbol) = self.symbol(id) {
            self.writes
                .calls
                .entry(symbol)
                .or_default()
                .push(&call.arguments);
        }
    }

    pub(super) fn collect_iife(
        &mut self,
        params: &FormalParameters<'a>,
        arguments: &'a [Argument<'a>],
    ) {
        if arguments.iter().any(Argument::is_spread) {
            return;
        }
        for (index, param) in params.items.iter().enumerate() {
            let BindingPattern::BindingIdentifier(param) = &param.pattern else {
                continue;
            };
            let passed = arguments
                .get(index)
                .and_then(Argument::as_expression)
                .is_some_and(literal_data);
            if let (true, Some(symbol)) = (passed, param.symbol_id.get()) {
                self.writes.passed.insert(symbol);
            }
        }
    }

    pub(super) fn collect_assignment(&mut self, assign: &'a AssignmentExpression<'a>) {
        let AssignmentTarget::AssignmentTargetIdentifier(id) = &assign.left else {
            return;
        };
        let Some(symbol) = self.symbol(id) else {
            return;
        };
        let write = match assign.operator {
            AssignmentOperator::Assign => Write::Value(&assign.right),
            operator if operator.is_logical() => Write::Value(&assign.right),
            AssignmentOperator::Addition => Write::Plus(&assign.right),
            _ => Write::Numeric,
        };
        self.writes.record(symbol, write, true);
    }
}
