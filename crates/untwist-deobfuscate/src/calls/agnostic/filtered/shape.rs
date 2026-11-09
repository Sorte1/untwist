use oxc_ast::ast::*;
use oxc_semantic::Scoping;
use oxc_span::Span;
use oxc_syntax::operator::{BinaryOperator, UnaryOperator};
use oxc_syntax::scope::ScopeId;
use oxc_syntax::symbol::SymbolId;
use rustc_hash::{FxHashMap, FxHashSet};
use untwist_core::ast_util::js_text::js_text;

use super::super::bootstrap::Resolve;

pub(super) const STRING_METHODS: [&str; 2] = ["charCodeAt", "charAt"];

const MAX_NESTING: usize = 8;
const MAX_DEPTH: u32 = 64;

pub(super) type Sealed<'f, 'a> = &'f dyn Fn(SymbolId) -> Option<Body<'a>>;

#[derive(Clone, Copy)]
pub(in crate::calls::agnostic) struct Body<'a> {
    pub(super) params: &'a FormalParameters<'a>,
    pub(super) block: Option<&'a FunctionBody<'a>>,
    pub(super) value: Option<&'a Expression<'a>>,
    pub(super) scope: ScopeId,
}

impl<'a> Body<'a> {
    pub(in crate::calls::agnostic) fn of_function(func: &'a Function<'a>) -> Option<Self> {
        if func.generator || func.r#async {
            return None;
        }
        Some(Self {
            params: &func.params,
            block: Some(func.body.as_deref()?),
            value: None,
            scope: func.scope_id.get()?,
        })
    }

    pub(in crate::calls::agnostic) fn of_arrow(
        arrow: &'a ArrowFunctionExpression<'a>,
    ) -> Option<Self> {
        if arrow.r#async {
            return None;
        }
        let (block, value) = match &arrow.body {
            ArrowFunctionBody::FunctionBody(body) => (Some(&**body), None),
            other => (None, Some(other.as_expression()?)),
        };
        Some(Self {
            params: &arrow.params,
            block,
            value,
            scope: arrow.scope_id.get()?,
        })
    }

    pub(super) fn of_value(expr: &'a Expression<'a>) -> Option<Self> {
        match expr.without_parentheses() {
            Expression::FunctionExpression(func) => Self::of_function(func),
            Expression::ArrowFunctionExpression(arrow) => Self::of_arrow(arrow),
            _ => None,
        }
    }
}

pub(super) struct Shape<'f, 'a> {
    scoping: &'f Scoping,
    sealed: Sealed<'f, 'a>,
    resolve: Resolve<'f>,
    pub(super) callees: FxHashMap<SymbolId, Body<'a>>,
    pub(super) resolved: FxHashMap<Span, String>,
    checking: Vec<SymbolId>,
    aliases: FxHashSet<SymbolId>,
    scope: Option<ScopeId>,
}

impl<'f, 'a> Shape<'f, 'a> {
    pub(super) fn new(scoping: &'f Scoping, sealed: Sealed<'f, 'a>, resolve: Resolve<'f>) -> Self {
        Self {
            scoping,
            sealed,
            resolve,
            callees: FxHashMap::default(),
            resolved: FxHashMap::default(),
            checking: Vec::new(),
            aliases: FxHashSet::default(),
            scope: None,
        }
    }

    pub(super) fn callee(&mut self, symbol: SymbolId) -> bool {
        if self.callees.contains_key(&symbol) || self.checking.contains(&symbol) {
            return true;
        }
        if self.checking.len() >= MAX_NESTING
            || self.scoping.symbol_scope_id(symbol) == self.scoping.root_scope_id()
        {
            return false;
        }
        let Some(func) = (self.sealed)(symbol) else {
            return false;
        };
        self.checking.push(symbol);
        let outer = self.scope.take();
        let pure = self.function(func);
        self.scope = outer;
        self.checking.pop();
        if pure {
            self.callees.insert(symbol, func);
        }
        pure
    }

    fn function(&mut self, func: Body<'a>) -> bool {
        if func.params.rest.is_some() {
            return false;
        }
        let simple = func.params.items.iter().all(|param| {
            param.initializer.is_none()
                && matches!(param.pattern, BindingPattern::BindingIdentifier(_))
        });
        self.scope = Some(func.scope);
        simple
            && match (func.value, func.block) {
                (Some(value), _) => self.expr(value, 0),
                (None, Some(block)) => {
                    block.directives.is_empty() && self.statements(&block.statements)
                }
                (None, None) => false,
            }
    }

    fn statements(&mut self, statements: &'a [Statement<'a>]) -> bool {
        statements.iter().all(|statement| self.statement(statement))
    }

    fn statement(&mut self, statement: &'a Statement<'a>) -> bool {
        match statement {
            Statement::BlockStatement(block) => self.statements(&block.body),
            Statement::EmptyStatement(_) => true,
            Statement::VariableDeclaration(declaration) => self.declaration(declaration),
            Statement::ExpressionStatement(it) => self.expr(&it.expression, 0),
            Statement::IfStatement(it) => {
                if !self.expr(&it.test, 0) {
                    return false;
                }
                let alternate = |shape: &mut Self| {
                    it.alternate
                        .as_ref()
                        .is_none_or(|other| shape.statement(other))
                };
                match super::super::opaque::choice(&it.test, self.resolve) {
                    Some(true) => self.statement(&it.consequent),
                    Some(false) => alternate(self),
                    None => self.statement(&it.consequent) && alternate(self),
                }
            }
            Statement::ForStatement(it) => {
                let init = match &it.init {
                    None => true,
                    Some(ForStatementInit::VariableDeclaration(declaration)) => {
                        self.declaration(declaration)
                    }
                    Some(other) => other.as_expression().is_some_and(|e| self.expr(e, 0)),
                };
                init && it.test.as_ref().is_none_or(|test| self.expr(test, 0))
                    && it.update.as_ref().is_none_or(|update| self.expr(update, 0))
                    && self.statement(&it.body)
            }
            Statement::WhileStatement(it) => self.expr(&it.test, 0) && self.statement(&it.body),
            Statement::ReturnStatement(it) => it
                .argument
                .as_ref()
                .is_none_or(|argument| self.expr(argument, 0)),
            Statement::BreakStatement(it) => it.label.is_none(),
            Statement::ContinueStatement(it) => it.label.is_none(),
            _ => false,
        }
    }

    fn declaration(&mut self, declaration: &'a VariableDeclaration<'a>) -> bool {
        declaration.declarations.iter().all(|declarator| {
            let BindingPattern::BindingIdentifier(id) = &declarator.id else {
                return false;
            };
            let Some(init) = &declarator.init else {
                return true;
            };
            if let Expression::Identifier(outer) = init.without_parentheses() {
                if self.resolved_symbol(outer).is_some_and(|s| !self.local(s)) {
                    if let Some(symbol) = id.symbol_id.get() {
                        self.aliases.insert(symbol);
                        return true;
                    }
                }
            }
            self.expr(init, 0)
        })
    }

    fn resolved_symbol(&self, id: &IdentifierReference<'_>) -> Option<SymbolId> {
        id.reference_id
            .get()
            .and_then(|reference| self.scoping.get_reference(reference).symbol_id())
    }

    fn local(&self, symbol: SymbolId) -> bool {
        let Some(scope) = self.scope else {
            return false;
        };
        self.scoping
            .scope_ancestors(self.scoping.symbol_scope_id(symbol))
            .any(|ancestor| ancestor == scope)
    }

    fn value(&self, id: &IdentifierReference<'_>) -> bool {
        match self.resolved_symbol(id) {
            None => matches!(id.name.as_str(), "undefined" | "NaN" | "Infinity"),
            Some(symbol) => self.local(symbol) && !self.aliases.contains(&symbol),
        }
    }

    fn target(&self, target: &SimpleAssignmentTarget<'_>) -> bool {
        match target {
            SimpleAssignmentTarget::AssignmentTargetIdentifier(id) => self
                .resolved_symbol(id)
                .is_some_and(|symbol| self.local(symbol) && !self.aliases.contains(&symbol)),
            _ => false,
        }
    }

    fn expr(&mut self, expr: &'a Expression<'a>, depth: u32) -> bool {
        if depth > MAX_DEPTH {
            return false;
        }
        let next = depth + 1;
        match expr {
            Expression::NumericLiteral(_)
            | Expression::StringLiteral(_)
            | Expression::BooleanLiteral(_)
            | Expression::NullLiteral(_) => true,
            Expression::Identifier(id) => self.value(id),
            Expression::ParenthesizedExpression(inner) => self.expr(&inner.expression, next),
            Expression::ArrayExpression(array) => array.elements.iter().all(|element| {
                element
                    .as_expression()
                    .is_some_and(|inner| self.expr(inner, next))
            }),
            Expression::UnaryExpression(unary) => {
                unary.operator != UnaryOperator::Delete && self.expr(&unary.argument, next)
            }
            Expression::BinaryExpression(binary) => {
                !matches!(
                    binary.operator,
                    BinaryOperator::In | BinaryOperator::Instanceof
                ) && self.expr(&binary.left, next)
                    && self.expr(&binary.right, next)
            }
            Expression::LogicalExpression(logical) => {
                self.expr(&logical.left, next) && self.expr(&logical.right, next)
            }
            Expression::ConditionalExpression(it) => {
                self.expr(&it.test, next)
                    && match super::super::opaque::choice(&it.test, self.resolve) {
                        Some(true) => self.expr(&it.consequent, next),
                        Some(false) => self.expr(&it.alternate, next),
                        None => self.expr(&it.consequent, next) && self.expr(&it.alternate, next),
                    }
            }
            Expression::SequenceExpression(sequence) => sequence
                .expressions
                .iter()
                .all(|part| self.expr(part, next)),
            Expression::AssignmentExpression(it) => {
                let operator =
                    it.operator.is_assign() || it.operator.to_binary_operator().is_some();
                operator
                    && it
                        .left
                        .as_simple_assignment_target()
                        .is_some_and(|target| self.target(target))
                    && self.expr(&it.right, next)
            }
            Expression::UpdateExpression(it) => self.target(&it.argument),
            Expression::StaticMemberExpression(member) => {
                !member.optional && self.expr(&member.object, next)
            }
            Expression::ComputedMemberExpression(member) => {
                !member.optional
                    && self.expr(&member.object, next)
                    && self.expr(&member.expression, next)
            }
            Expression::CallExpression(call) => self.call(call, next),
            _ => false,
        }
    }

    fn call(&mut self, call: &'a CallExpression<'a>, depth: u32) -> bool {
        if let Some(text) = self.resolve.and_then(|map| map.get(&call.span)) {
            return match text.as_string() {
                Some(text) => {
                    self.resolved.insert(call.span, text);
                    true
                }
                None => false,
            };
        }
        if call.optional || call.arguments.iter().any(Argument::is_spread) {
            return false;
        }
        let arguments = call.arguments.iter().all(|argument| {
            argument
                .as_expression()
                .is_some_and(|expr| self.expr(expr, depth))
        });
        if !arguments {
            return false;
        }
        match call.callee.without_parentheses() {
            Expression::Identifier(id) => self
                .resolved_symbol(id)
                .is_some_and(|symbol| self.callee(symbol)),
            Expression::StaticMemberExpression(member) => {
                STRING_METHODS.contains(&member.property.name.as_str())
                    && self.expr(&member.object, depth)
            }
            Expression::ComputedMemberExpression(member) => {
                self.method_key(&member.expression).is_some() && self.expr(&member.object, depth)
            }
            _ => false,
        }
    }

    pub(super) fn method_key(&mut self, key: &'a Expression<'a>) -> Option<&'static str> {
        let text = self.key_text(key, 0)?;
        STRING_METHODS.iter().copied().find(|name| *name == text)
    }

    fn key_text(&mut self, key: &'a Expression<'a>, depth: u32) -> Option<String> {
        if depth > MAX_DEPTH {
            return None;
        }
        match key.without_parentheses() {
            Expression::StringLiteral(text) => Some(js_text(text.value).into_owned()),
            Expression::CallExpression(call) => {
                self.call(call, 0).then_some(())?;
                self.resolved.get(&call.span).cloned()
            }
            Expression::BinaryExpression(binary) if binary.operator == BinaryOperator::Addition => {
                let left = self.key_text(&binary.left, depth + 1)?;
                let right = self.key_text(&binary.right, depth + 1)?;
                Some(left + &right)
            }
            _ => None,
        }
    }
}

pub(super) fn is_arithmetic(operator: BinaryOperator) -> bool {
    matches!(
        operator,
        BinaryOperator::Addition
            | BinaryOperator::Subtraction
            | BinaryOperator::Multiplication
            | BinaryOperator::Division
            | BinaryOperator::Remainder
    )
}

fn numeric(expr: &Expression<'_>) -> bool {
    match expr.without_parentheses() {
        Expression::NumericLiteral(_) => true,
        Expression::UnaryExpression(unary) => {
            unary.operator == UnaryOperator::UnaryNegation && numeric(&unary.argument)
        }
        Expression::BinaryExpression(binary) => {
            is_arithmetic(binary.operator) && numeric(&binary.left) && numeric(&binary.right)
        }
        _ => false,
    }
}

pub(super) fn closed(expr: &Expression<'_>) -> bool {
    match expr.without_parentheses() {
        Expression::BinaryExpression(_) => numeric(expr),
        Expression::NumericLiteral(_)
        | Expression::StringLiteral(_)
        | Expression::BooleanLiteral(_)
        | Expression::NullLiteral(_) => true,
        Expression::UnaryExpression(unary) => {
            matches!(
                unary.operator,
                UnaryOperator::UnaryNegation | UnaryOperator::LogicalNot
            ) && closed(&unary.argument)
        }
        Expression::ArrayExpression(array) => array
            .elements
            .iter()
            .all(|element| element.as_expression().is_some_and(closed)),
        _ => false,
    }
}
