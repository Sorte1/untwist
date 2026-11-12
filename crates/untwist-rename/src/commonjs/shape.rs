use oxc_ast::ast::*;
use oxc_ast_visit::{walk_js, VisitJs};
use oxc_semantic::Scoping;
use oxc_syntax::operator::{AssignmentOperator, LogicalOperator, UnaryOperator};
use oxc_syntax::scope::{ScopeFlags, ScopeId};
use oxc_syntax::symbol::SymbolId;
use rustc_hash::FxHashSet;

use crate::collect::reference_symbol;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Holder {
    Value(SymbolId),
    Module(SymbolId),
}

impl Holder {
    pub fn symbol(self) -> SymbolId {
        match self {
            Holder::Value(symbol) | Holder::Module(symbol) => symbol,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Wrapper {
    pub function: Option<SymbolId>,
    pub scope: Option<ScopeId>,
    pub flag: SymbolId,
    pub holder: Option<Holder>,
    pub params: Option<(SymbolId, Option<SymbolId>)>,
    pub factory: Option<(Option<SymbolId>, Option<SymbolId>)>,
    pub start: u32,
}

struct Guarded<'k, 'a> {
    flag: SymbolId,
    holder: Holder,
    rest: &'k [Statement<'a>],
}

fn is_one(expression: &Expression<'_>) -> bool {
    match expression.without_parentheses() {
        Expression::NumericLiteral(literal) => literal.value == 1.0,
        _ => false,
    }
}

fn hidden_exports_key(key: &Expression<'_>) -> bool {
    match key.without_parentheses() {
        Expression::StringLiteral(literal) => {
            untwist_core::ast_util::js_text::js_text(literal.value) == "exports"
        }
        Expression::CallExpression(call) => call.arguments.iter().all(|argument| {
            matches!(
                argument,
                Argument::NumericLiteral(_) | Argument::StringLiteral(_)
            )
        }),
        _ => false,
    }
}

struct Matcher<'s> {
    scoping: Option<&'s Scoping>,
    names: std::cell::RefCell<rustc_hash::FxHashMap<String, usize>>,
}

impl<'s> Matcher<'s> {
    fn new(scoping: Option<&'s Scoping>) -> Self {
        Self {
            scoping,
            names: std::cell::RefCell::default(),
        }
    }

    fn key(&self, id: &IdentifierReference<'_>) -> Option<SymbolId> {
        if let Some(scoping) = self.scoping {
            return reference_symbol(id, scoping);
        }
        let mut names = self.names.borrow_mut();
        let next = names.len();
        let index = *names.entry(id.name.to_string()).or_insert(next);
        Some(SymbolId::from_usize(index))
    }

    fn identifier(&self, expression: &Expression<'_>) -> Option<SymbolId> {
        match expression.without_parentheses() {
            Expression::Identifier(id) => self.key(id),
            _ => None,
        }
    }

    fn holder(&self, expression: &Expression<'_>) -> Option<Holder> {
        match expression.without_parentheses() {
            Expression::Identifier(id) => self.key(id).map(Holder::Value),
            Expression::StaticMemberExpression(member)
                if member.property.name == "exports" && !member.optional =>
            {
                self.identifier(&member.object).map(Holder::Module)
            }
            Expression::ComputedMemberExpression(member)
                if !member.optional && hidden_exports_key(&member.expression) =>
            {
                self.identifier(&member.object).map(Holder::Module)
            }
            _ => None,
        }
    }

    fn holder_target(&self, target: &AssignmentTarget<'_>) -> Option<Holder> {
        match target {
            AssignmentTarget::AssignmentTargetIdentifier(id) => self.key(id).map(Holder::Value),
            AssignmentTarget::StaticMemberExpression(member)
                if member.property.name == "exports" =>
            {
                self.identifier(&member.object).map(Holder::Module)
            }
            AssignmentTarget::ComputedMemberExpression(member)
                if hidden_exports_key(&member.expression) =>
            {
                self.identifier(&member.object).map(Holder::Module)
            }
            _ => None,
        }
    }

    fn sets_flag(&self, expression: &Expression<'_>) -> Option<SymbolId> {
        let Expression::AssignmentExpression(assign) = expression.without_parentheses() else {
            return None;
        };
        if assign.operator != AssignmentOperator::Assign || !is_one(&assign.right) {
            return None;
        }
        let AssignmentTarget::AssignmentTargetIdentifier(id) = &assign.left else {
            return None;
        };
        self.key(id)
    }

    fn sets_flag_statement(&self, statement: &Statement<'_>) -> Option<SymbolId> {
        match statement {
            Statement::ExpressionStatement(statement) => self.sets_flag(&statement.expression),
            _ => None,
        }
    }

    fn returned(&self, expression: &Expression<'_>) -> Option<Holder> {
        match expression.without_parentheses() {
            Expression::AssignmentExpression(assign)
                if assign.operator == AssignmentOperator::Assign =>
            {
                self.holder_target(&assign.left)
            }
            Expression::SequenceExpression(sequence) => self.returned(sequence.expressions.last()?),
            other => self.holder(other),
        }
    }

    fn returns(&self, statement: &Statement<'_>) -> Option<Holder> {
        match statement {
            Statement::ReturnStatement(statement) => self.returned(statement.argument.as_ref()?),
            Statement::BlockStatement(block) => match &block.body[..] {
                [only] => self.returns(only),
                _ => None,
            },
            _ => None,
        }
    }

    fn returns_plain(&self, statement: &Statement<'_>) -> Option<Holder> {
        let returned = match statement {
            Statement::ReturnStatement(statement) => statement.argument.as_ref()?,
            Statement::BlockStatement(block) => match &block.body[..] {
                [only] => return self.returns_plain(only),
                _ => return None,
            },
            _ => return None,
        };
        self.holder(returned)
    }

    fn early_return<'k, 'a>(&self, body: &'k [Statement<'a>]) -> Option<Guarded<'k, 'a>> {
        let (Statement::IfStatement(guard), rest) = body.split_first()? else {
            return None;
        };
        let flag = self.identifier(&guard.test)?;
        let holder = self.returns_plain(&guard.consequent)?;
        let rest: &'k [Statement<'a>] = match (&guard.alternate, rest) {
            (None, rest) => rest,
            (Some(Statement::BlockStatement(block)), []) => &block.body,
            _ => return None,
        };
        let (set, rest) = rest.split_first()?;
        (self.sets_flag_statement(set)? == flag).then_some(())?;
        let last = rest.last()?;
        (self.returns(last)? == holder).then_some(Guarded { flag, holder, rest })
    }

    fn negated_flag(&self, expression: &Expression<'_>) -> Option<SymbolId> {
        let Expression::UnaryExpression(unary) = expression.without_parentheses() else {
            return None;
        };
        (unary.operator == UnaryOperator::LogicalNot)
            .then(|| self.identifier(&unary.argument))
            .flatten()
    }

    fn guarded_block<'k, 'a>(&self, body: &'k [Statement<'a>]) -> Option<Guarded<'k, 'a>> {
        let [Statement::IfStatement(guard), last] = body else {
            return None;
        };
        if guard.alternate.is_some() {
            return None;
        }
        let flag = self.negated_flag(&guard.test)?;
        let Statement::BlockStatement(block) = &guard.consequent else {
            return None;
        };
        let (set, rest) = block.body.split_first()?;
        (self.sets_flag_statement(set)? == flag).then_some(())?;
        let holder = self.returns_plain(last)?;
        Some(Guarded { flag, holder, rest })
    }

    fn short_circuit(&self, sequence: &SequenceExpression<'_>) -> Option<(SymbolId, Holder)> {
        let [guard, holder] = &sequence.expressions[..] else {
            return None;
        };
        let Expression::LogicalExpression(logical) = guard.without_parentheses() else {
            return None;
        };
        if logical.operator != LogicalOperator::Or {
            return None;
        }
        let flag = self.identifier(&logical.left)?;
        let set = match logical.right.without_parentheses() {
            Expression::SequenceExpression(inner) => inner.expressions.first()?,
            other => other,
        };
        (self.sets_flag(set)? == flag).then_some(())?;
        Some((flag, self.holder(holder)?))
    }

    fn ternary(&self, conditional: &ConditionalExpression<'_>) -> Option<(SymbolId, Holder)> {
        let flag = self.identifier(&conditional.test)?;
        let holder = self.holder(&conditional.consequent)?;
        let Expression::SequenceExpression(sequence) = conditional.alternate.without_parentheses()
        else {
            return None;
        };
        (self.sets_flag(sequence.expressions.first()?)? == flag).then_some(())?;
        (self.returned(sequence.expressions.last()?)? == holder).then_some((flag, holder))
    }

    fn inline_guard(&self, guard: &IfStatement<'_>) -> Option<SymbolId> {
        if guard.alternate.is_some() {
            return None;
        }
        let flag = self.negated_flag(&guard.test)?;
        let Statement::BlockStatement(block) = &guard.consequent else {
            return None;
        };
        let set = block.body.first()?;
        (self.sets_flag_statement(set)? == flag).then_some(flag)
    }

    fn function_body(&self, body: &FunctionBody<'_>) -> Option<(SymbolId, Holder)> {
        let statements = &body.statements[..];
        match statements {
            [Statement::ReturnStatement(statement)] => {
                match statement.argument.as_ref()?.without_parentheses() {
                    Expression::SequenceExpression(sequence) => self.short_circuit(sequence),
                    Expression::ConditionalExpression(conditional) => self.ternary(conditional),
                    _ => None,
                }
            }
            _ => None,
        }
    }
}

fn plain_params(params: &FormalParameters<'_>) -> bool {
    params.items.is_empty() && params.rest.is_none()
}

fn bound(pattern: &BindingPattern<'_>) -> Option<SymbolId> {
    match pattern {
        BindingPattern::BindingIdentifier(id) => id.symbol_id.get(),
        _ => None,
    }
}

struct Finder<'s> {
    matcher: Matcher<'s>,
    found: Vec<Wrapper>,
    factories: FxHashSet<SymbolId>,
}

fn factory_params(
    argument: &Argument<'_>,
) -> Option<(Option<SymbolId>, Option<SymbolId>, ScopeId)> {
    let (params, scope) = match argument.as_expression()?.without_parentheses() {
        Expression::ArrowFunctionExpression(arrow) => (&arrow.params, arrow.scope_id.get()?),
        Expression::FunctionExpression(function) => (&function.params, function.scope_id.get()?),
        _ => return None,
    };
    let at = |index: usize| {
        params
            .items
            .get(index)
            .and_then(|param| bound(&param.pattern))
    };
    Some((at(0), at(1), scope))
}

impl Finder<'_> {
    fn module_params(
        &self,
        rest: &[Statement<'_>],
        holder: Holder,
    ) -> Option<(SymbolId, Option<SymbolId>)> {
        let Holder::Module(module) = holder else {
            return None;
        };
        rest.iter().find_map(|statement| {
            let Statement::ExpressionStatement(statement) = statement else {
                return None;
            };
            let Expression::CallExpression(call) = statement.expression.without_parentheses()
            else {
                return None;
            };
            let Expression::FunctionExpression(function) = call.callee.without_parentheses() else {
                return None;
            };
            let first = call.arguments.first()?.as_expression()?;
            (self.matcher.identifier(first)? == module).then_some(())?;
            if let Some(second) = call.arguments.get(1) {
                (self.matcher.holder(second.as_expression()?)? == holder).then_some(())?;
            }
            if call.arguments.len() > 2 || function.params.rest.is_some() {
                return None;
            }
            let params = &function.params.items;
            let first = bound(&params.first()?.pattern)?;
            let second = match (params.get(1), call.arguments.len()) {
                (Some(param), 2) => Some(bound(&param.pattern)?),
                (None, _) => None,
                _ => return None,
            };
            (params.len() <= 2).then_some((first, second))
        })
    }

    fn function(&mut self, function: &Function<'_>, symbol: Option<SymbolId>) {
        if function.r#async || function.generator || !plain_params(&function.params) {
            return;
        }
        let Some(body) = function.body.as_deref() else {
            return;
        };
        let statements = &body.statements[..];
        let guarded = self
            .matcher
            .early_return(statements)
            .or_else(|| self.matcher.guarded_block(statements));
        let (flag, holder, params) = match guarded {
            Some(guarded) => (
                guarded.flag,
                guarded.holder,
                self.module_params(guarded.rest, guarded.holder),
            ),
            None => match self.matcher.function_body(body) {
                Some((flag, holder)) => (flag, holder, None),
                None => return,
            },
        };
        if flag == holder.symbol()
            || symbol.is_some_and(|own| own == flag || own == holder.symbol())
        {
            return;
        }
        self.found.push(Wrapper {
            function: symbol,
            scope: function.scope_id.get(),
            flag,
            holder: Some(holder),
            params,
            factory: None,
            start: function.span.start,
        });
    }
}

impl<'a> VisitJs<'a> for Finder<'_> {
    fn visit_function(&mut self, it: &Function<'a>, flags: ScopeFlags) {
        let symbol = if it.is_declaration() {
            it.id.as_ref().and_then(|id| id.symbol_id.get())
        } else {
            None
        };
        if it.is_declaration() {
            self.function(it, symbol);
        }
        walk_js::walk_function(self, it, flags);
    }

    fn visit_variable_declarator(&mut self, it: &VariableDeclarator<'a>) {
        if let Some(Expression::CallExpression(call)) =
            it.init.as_ref().map(Expression::without_parentheses)
        {
            let helper = match &call.callee {
                Expression::Identifier(callee) => self.matcher.key(callee),
                _ => None,
            };
            let made = helper.is_some_and(|helper| self.factories.contains(&helper));
            if let (true, [argument], Some(binding)) = (made, &call.arguments[..], bound(&it.id)) {
                if let Some((exports, module, scope)) = factory_params(argument) {
                    self.found.push(Wrapper {
                        function: None,
                        scope: Some(scope),
                        flag: binding,
                        holder: None,
                        params: None,
                        factory: Some((exports, module)),
                        start: it.span.start,
                    });
                }
            }
        }
        if let Some(Expression::FunctionExpression(function)) =
            it.init.as_ref().map(Expression::without_parentheses)
        {
            if function.id.is_none() {
                self.function(function, bound(&it.id));
            }
        }
        walk_js::walk_variable_declarator(self, it);
    }

    fn visit_call_expression(&mut self, it: &CallExpression<'a>) {
        if let Expression::FunctionExpression(function) = it.callee.without_parentheses() {
            if it.arguments.is_empty() {
                self.function(function, None);
            }
        }
        walk_js::walk_call_expression(self, it);
    }

    fn visit_sequence_expression(&mut self, it: &SequenceExpression<'a>) {
        if let Some((flag, holder)) = self.matcher.short_circuit(it) {
            self.found.push(Wrapper {
                function: None,
                scope: None,
                flag,
                holder: Some(holder),
                params: None,
                factory: None,
                start: it.span.start,
            });
        }
        walk_js::walk_sequence_expression(self, it);
    }

    fn visit_conditional_expression(&mut self, it: &ConditionalExpression<'a>) {
        if let Some((flag, holder)) = self.matcher.ternary(it) {
            self.found.push(Wrapper {
                function: None,
                scope: None,
                flag,
                holder: Some(holder),
                params: None,
                factory: None,
                start: it.span.start,
            });
        }
        walk_js::walk_conditional_expression(self, it);
    }

    fn visit_if_statement(&mut self, it: &IfStatement<'a>) {
        if let Some(flag) = self.matcher.inline_guard(it) {
            self.found.push(Wrapper {
                function: None,
                scope: None,
                flag,
                holder: None,
                params: None,
                factory: None,
                start: it.span.start,
            });
        }
        walk_js::walk_if_statement(self, it);
    }
}

pub fn wrappers(
    program: &Program<'_>,
    scoping: &Scoping,
    factories: FxHashSet<SymbolId>,
) -> Vec<Wrapper> {
    let mut finder = Finder {
        matcher: Matcher::new(Some(scoping)),
        found: Vec::new(),
        factories,
    };
    finder.visit_program(program);
    let mut seen = FxHashSet::default();
    let mut wrappers: Vec<Wrapper> = finder
        .found
        .into_iter()
        .filter(|wrapper| seen.insert(wrapper.flag))
        .filter(|wrapper| {
            wrapper.factory.is_some() || scoping.get_resolved_reference_ids(wrapper.flag).len() == 2
        })
        .collect();
    wrappers.sort_by_key(|wrapper| wrapper.start);
    wrappers
}

pub fn may_hold_wrappers(program: &Program<'_>) -> bool {
    let mut finder = Finder {
        matcher: Matcher::new(None),
        found: Vec::new(),
        factories: FxHashSet::default(),
    };
    finder.visit_program(program);
    !finder.found.is_empty()
}
