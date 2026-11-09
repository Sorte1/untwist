use std::rc::Rc;

use oxc_ast::ast::*;
use oxc_semantic::Scoping;
use oxc_span::Span;
use oxc_syntax::operator::{BinaryOperator, LogicalOperator, UnaryOperator, UpdateOperator};
use oxc_syntax::symbol::SymbolId;
use rustc_hash::FxHashMap;
use untwist_eval::coerce::{number_to_string, to_integer_or_infinity};
use untwist_eval::fold::{const_from_literal, fold_binary, fold_unary, Const};

use super::shape::{Body, STRING_METHODS};

const MAX_STEPS: u32 = 200_000;
const MAX_CALLS: u32 = 32;

#[derive(Clone)]
pub(super) enum V {
    Prim(Const),
    Arr(Rc<Vec<V>>),
}

impl V {
    fn prim(&self) -> Option<&Const> {
        match self {
            V::Prim(value) => Some(value),
            V::Arr(_) => None,
        }
    }

    pub(super) fn truthy(&self) -> Option<bool> {
        match self {
            V::Prim(value) => match fold_unary(UnaryOperator::LogicalNot, value)? {
                Const::Bool(negated) => Some(!negated),
                _ => None,
            },
            V::Arr(_) => Some(true),
        }
    }
}

enum Done {
    Normal,
    Return(V),
    Break,
    Continue,
}

type Env = FxHashMap<SymbolId, V>;

pub(super) struct Machine<'m, 'a> {
    pub(super) scoping: &'m Scoping,
    pub(super) callees: &'m FxHashMap<SymbolId, Body<'a>>,
    pub(super) resolved: &'m FxHashMap<Span, String>,
    pub(super) steps: u32,
    pub(super) calls: u32,
}

fn property_key(value: &Const) -> Option<String> {
    match value {
        Const::Str(text) => Some(text.clone()),
        Const::Num(number) => Some(number_to_string(*number)),
        Const::Bool(flag) => Some(flag.to_string()),
        Const::Null => Some("null".to_string()),
        Const::Undefined => Some("undefined".to_string()),
    }
}

fn array_index(key: &str) -> Option<usize> {
    let canonical = key == "0" || (!key.starts_with('0') && !key.is_empty());
    if !canonical || key.len() > 9 || !key.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    key.parse().ok()
}

fn unit_text(unit: u16) -> Option<String> {
    char::from_u32(u32::from(unit)).map(|c| c.to_string())
}

pub(super) fn literal(expr: &Expression<'_>) -> Option<V> {
    match expr.without_parentheses() {
        Expression::ArrayExpression(array) => {
            let items = array
                .elements
                .iter()
                .map(|element| literal(element.as_expression()?))
                .collect::<Option<Vec<_>>>()?;
            Some(V::Arr(Rc::new(items)))
        }
        Expression::UnaryExpression(unary) => {
            let inner = literal(&unary.argument)?;
            Some(V::Prim(fold_unary(unary.operator, inner.prim()?)?))
        }
        Expression::BinaryExpression(binary) if super::shape::is_arithmetic(binary.operator) => {
            let left = literal(&binary.left)?;
            let right = literal(&binary.right)?;
            let (Const::Num(_), Const::Num(_)) = (left.prim()?, right.prim()?) else {
                return None;
            };
            Some(V::Prim(fold_binary(
                binary.operator,
                left.prim()?,
                right.prim()?,
            )?))
        }
        other => const_from_literal(other).map(V::Prim),
    }
}

impl<'a> Machine<'_, 'a> {
    fn tick(&mut self) -> Option<()> {
        self.steps += 1;
        (self.steps <= MAX_STEPS).then_some(())
    }

    fn symbol(&self, id: &IdentifierReference<'_>) -> Option<SymbolId> {
        id.reference_id
            .get()
            .and_then(|reference| self.scoping.get_reference(reference).symbol_id())
    }

    pub(super) fn call(&mut self, func: Body<'a>, args: Vec<V>) -> Option<V> {
        self.calls += 1;
        if self.calls > MAX_CALLS {
            return None;
        }
        let mut env = Env::default();
        let mut args = args.into_iter();
        for param in &func.params.items {
            let BindingPattern::BindingIdentifier(id) = &param.pattern else {
                return None;
            };
            let value = args.next().unwrap_or(V::Prim(Const::Undefined));
            env.insert(id.symbol_id.get()?, value);
        }
        if let Some(value) = func.value {
            let result = self.expr(value, &mut env)?;
            self.calls -= 1;
            return Some(result);
        }
        let done = self.block(&func.block?.statements, &mut env)?;
        self.calls -= 1;
        match done {
            Done::Return(value) => Some(value),
            Done::Normal => Some(V::Prim(Const::Undefined)),
            Done::Break | Done::Continue => None,
        }
    }

    fn block(&mut self, statements: &'a [Statement<'a>], env: &mut Env) -> Option<Done> {
        for statement in statements {
            match self.statement(statement, env)? {
                Done::Normal => {}
                other => return Some(other),
            }
        }
        Some(Done::Normal)
    }

    fn statement(&mut self, statement: &'a Statement<'a>, env: &mut Env) -> Option<Done> {
        self.tick()?;
        match statement {
            Statement::BlockStatement(block) => self.block(&block.body, env),
            Statement::EmptyStatement(_) => Some(Done::Normal),
            Statement::VariableDeclaration(declaration) => {
                self.declare(declaration, env)?;
                Some(Done::Normal)
            }
            Statement::ExpressionStatement(it) => {
                self.expr(&it.expression, env)?;
                Some(Done::Normal)
            }
            Statement::IfStatement(it) => {
                if self.expr(&it.test, env)?.truthy()? {
                    self.statement(&it.consequent, env)
                } else if let Some(other) = &it.alternate {
                    self.statement(other, env)
                } else {
                    Some(Done::Normal)
                }
            }
            Statement::ForStatement(it) => self.for_loop(it, env),
            Statement::WhileStatement(it) => {
                while self.expr(&it.test, env)?.truthy()? {
                    match self.statement(&it.body, env)? {
                        Done::Break => break,
                        Done::Return(value) => return Some(Done::Return(value)),
                        Done::Normal | Done::Continue => {}
                    }
                }
                Some(Done::Normal)
            }
            Statement::ReturnStatement(it) => {
                let value = match &it.argument {
                    Some(argument) => self.expr(argument, env)?,
                    None => V::Prim(Const::Undefined),
                };
                Some(Done::Return(value))
            }
            Statement::BreakStatement(_) => Some(Done::Break),
            Statement::ContinueStatement(_) => Some(Done::Continue),
            _ => None,
        }
    }

    fn for_loop(&mut self, it: &'a ForStatement<'a>, env: &mut Env) -> Option<Done> {
        match &it.init {
            None => {}
            Some(ForStatementInit::VariableDeclaration(declaration)) => {
                self.declare(declaration, env)?;
            }
            Some(other) => {
                self.expr(other.as_expression()?, env)?;
            }
        }
        loop {
            if let Some(test) = &it.test {
                if !self.expr(test, env)?.truthy()? {
                    break;
                }
            }
            match self.statement(&it.body, env)? {
                Done::Break => break,
                Done::Return(value) => return Some(Done::Return(value)),
                Done::Normal | Done::Continue => {}
            }
            if let Some(update) = &it.update {
                self.expr(update, env)?;
            }
        }
        Some(Done::Normal)
    }

    fn declare(&mut self, declaration: &'a VariableDeclaration<'a>, env: &mut Env) -> Option<()> {
        for declarator in &declaration.declarations {
            let BindingPattern::BindingIdentifier(id) = &declarator.id else {
                return None;
            };
            let symbol = id.symbol_id.get()?;
            let value = match &declarator.init {
                None if declaration.kind.is_var() && env.contains_key(&symbol) => continue,
                None => V::Prim(Const::Undefined),
                Some(Expression::Identifier(outer))
                    if self.symbol(outer).is_some_and(|s| !env.contains_key(&s)) =>
                {
                    continue
                }
                Some(init) => self.expr(init, env)?,
            };
            env.insert(symbol, value);
        }
        Some(())
    }

    fn expr(&mut self, expr: &'a Expression<'a>, env: &mut Env) -> Option<V> {
        self.tick()?;
        match expr {
            Expression::Identifier(id) => match self.symbol(id) {
                Some(symbol) => env.get(&symbol).cloned(),
                None => literal_global(id.name.as_str()),
            },
            Expression::ParenthesizedExpression(inner) => self.expr(&inner.expression, env),
            Expression::ArrayExpression(array) => {
                let mut items = Vec::with_capacity(array.elements.len());
                for element in &array.elements {
                    items.push(self.expr(element.as_expression()?, env)?);
                }
                Some(V::Arr(Rc::new(items)))
            }
            Expression::UnaryExpression(unary) => self.unary(unary, env),
            Expression::BinaryExpression(binary) => {
                let left = self.expr(&binary.left, env)?;
                let right = self.expr(&binary.right, env)?;
                Some(V::Prim(fold_binary(
                    binary.operator,
                    left.prim()?,
                    right.prim()?,
                )?))
            }
            Expression::LogicalExpression(logical) => {
                let left = self.expr(&logical.left, env)?;
                let right_taken = match logical.operator {
                    LogicalOperator::And => left.truthy()?,
                    LogicalOperator::Or => !left.truthy()?,
                    LogicalOperator::Coalesce => {
                        matches!(left.prim(), Some(Const::Null | Const::Undefined))
                    }
                };
                if right_taken {
                    self.expr(&logical.right, env)
                } else {
                    Some(left)
                }
            }
            Expression::ConditionalExpression(it) => {
                if self.expr(&it.test, env)?.truthy()? {
                    self.expr(&it.consequent, env)
                } else {
                    self.expr(&it.alternate, env)
                }
            }
            Expression::SequenceExpression(sequence) => {
                let mut last = None;
                for part in &sequence.expressions {
                    last = Some(self.expr(part, env)?);
                }
                last
            }
            Expression::AssignmentExpression(it) => self.assign(it, env),
            Expression::UpdateExpression(it) => self.update(it, env),
            Expression::StaticMemberExpression(member) => {
                let object = self.expr(&member.object, env)?;
                get(&object, member.property.name.as_str())
            }
            Expression::ComputedMemberExpression(member) => {
                let object = self.expr(&member.object, env)?;
                let key = self.expr(&member.expression, env)?;
                get(&object, &property_key(key.prim()?)?)
            }
            Expression::CallExpression(call) => self.call_expr(call, env),
            other => const_from_literal(other).map(V::Prim),
        }
    }

    fn unary(&mut self, unary: &'a UnaryExpression<'a>, env: &mut Env) -> Option<V> {
        let argument = self.expr(&unary.argument, env)?;
        let value = match (unary.operator, &argument) {
            (UnaryOperator::Delete, _) => return None,
            (UnaryOperator::Void, _) => Const::Undefined,
            (UnaryOperator::LogicalNot, _) => Const::Bool(!argument.truthy()?),
            (UnaryOperator::Typeof, V::Arr(_)) => Const::Str("object".to_string()),
            (operator, _) => fold_unary(operator, argument.prim()?)?,
        };
        Some(V::Prim(value))
    }

    fn local(&self, target: &SimpleAssignmentTarget<'_>, env: &Env) -> Option<SymbolId> {
        let SimpleAssignmentTarget::AssignmentTargetIdentifier(id) = target else {
            return None;
        };
        self.symbol(id).filter(|symbol| env.contains_key(symbol))
    }

    fn assign(&mut self, it: &'a AssignmentExpression<'a>, env: &mut Env) -> Option<V> {
        let symbol = self.local(it.left.as_simple_assignment_target()?, env)?;
        let right = self.expr(&it.right, env)?;
        let value = if it.operator.is_assign() {
            right
        } else {
            let operator = it.operator.to_binary_operator()?;
            let old = env.get(&symbol)?.clone();
            V::Prim(fold_binary(operator, old.prim()?, right.prim()?)?)
        };
        env.insert(symbol, value.clone());
        Some(value)
    }

    fn update(&mut self, it: &'a UpdateExpression<'a>, env: &mut Env) -> Option<V> {
        let symbol = self.local(&it.argument, env)?;
        let old = env.get(&symbol)?.clone();
        let Const::Num(number) = fold_unary(UnaryOperator::UnaryPlus, old.prim()?)? else {
            return None;
        };
        let new = match it.operator {
            UpdateOperator::Increment => number + 1.0,
            UpdateOperator::Decrement => number - 1.0,
        };
        env.insert(symbol, V::Prim(Const::Num(new)));
        let shown = if it.prefix { new } else { number };
        Some(V::Prim(Const::Num(shown)))
    }

    fn call_expr(&mut self, call: &'a CallExpression<'a>, env: &mut Env) -> Option<V> {
        if let Some(text) = self.resolved.get(&call.span) {
            return Some(V::Prim(Const::Str(text.clone())));
        }
        let mut args = Vec::with_capacity(call.arguments.len());
        for argument in &call.arguments {
            args.push(self.expr(argument.as_expression()?, env)?);
        }
        match call.callee.without_parentheses() {
            Expression::Identifier(id) => {
                let func = *self.callees.get(&self.symbol(id)?)?;
                self.call(func, args)
            }
            Expression::StaticMemberExpression(member) => {
                let receiver = self.expr(&member.object, env)?;
                string_method(&receiver, member.property.name.as_str(), &args)
            }
            Expression::ComputedMemberExpression(member) => {
                let receiver = self.expr(&member.object, env)?;
                let key = self.expr(&member.expression, env)?;
                let Some(Const::Str(name)) = key.prim() else {
                    return None;
                };
                string_method(&receiver, name, &args)
            }
            _ => None,
        }
    }
}

fn literal_global(name: &str) -> Option<V> {
    let value = match name {
        "undefined" => Const::Undefined,
        "NaN" => Const::Num(f64::NAN),
        "Infinity" => Const::Num(f64::INFINITY),
        _ => return None,
    };
    Some(V::Prim(value))
}

fn get(object: &V, key: &str) -> Option<V> {
    match object {
        V::Arr(items) => {
            if key == "length" {
                return Some(V::Prim(Const::Num(items.len() as f64)));
            }
            let index = array_index(key)?;
            Some(
                items
                    .get(index)
                    .cloned()
                    .unwrap_or(V::Prim(Const::Undefined)),
            )
        }
        V::Prim(Const::Str(text)) => {
            let units: Vec<u16> = text.encode_utf16().collect();
            if key == "length" {
                return Some(V::Prim(Const::Num(units.len() as f64)));
            }
            let index = array_index(key)?;
            match units.get(index) {
                Some(unit) => Some(V::Prim(Const::Str(unit_text(*unit)?))),
                None => Some(V::Prim(Const::Undefined)),
            }
        }
        V::Prim(_) => None,
    }
}

fn string_method(receiver: &V, name: &str, args: &[V]) -> Option<V> {
    let V::Prim(Const::Str(text)) = receiver else {
        return None;
    };
    if !STRING_METHODS.contains(&name) {
        return None;
    }
    let position = match args.first() {
        None => 0.0,
        Some(value) => match fold_unary(UnaryOperator::UnaryPlus, value.prim()?)? {
            Const::Num(number) => to_integer_or_infinity(number),
            _ => return None,
        },
    };
    let units: Vec<u16> = text.encode_utf16().collect();
    let unit = (position >= 0.0 && position < units.len() as f64).then(|| units[position as usize]);
    let value = match (name, unit) {
        ("charCodeAt", Some(unit)) => Const::Num(f64::from(unit)),
        ("charCodeAt", None) => Const::Num(f64::NAN),
        (_, Some(unit)) => Const::Str(unit_text(unit)?),
        (_, None) => Const::Str(String::new()),
    };
    Some(V::Prim(value))
}

pub(super) fn is_binary_compare(operator: BinaryOperator) -> bool {
    operator.is_equality() || operator.is_compare()
}
