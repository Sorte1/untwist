use oxc_ast::ast::*;
use oxc_semantic::Scoping;
use oxc_span::{GetSpan, Span};
use oxc_syntax::operator::{AssignmentOperator, BinaryOperator, UnaryOperator, UpdateOperator};
use oxc_syntax::symbol::SymbolId;
use rustc_hash::FxHashMap;
use untwist_eval::fold::{fold_binary, fold_unary, Const};

use super::super::eval::{Machine, Shape, MAX_STRING};
use super::super::scan::simple_params;
use crate::bindings::{is_block_function, resolved_symbol};

pub(super) struct State {
    pub(super) statement: Span,
    pub(super) init: Const,
    pub(super) writable: bool,
}

enum Step<'p> {
    Assign(SymbolId, AssignmentOperator, &'p Expression<'p>),
    Update(SymbolId, UpdateOperator),
    Discard(&'p Expression<'p>),
}

pub(super) struct Func<'p> {
    pub(super) symbol: SymbolId,
    pub(super) statement: Span,
    pub(super) hoisted: bool,
    params: Vec<SymbolId>,
    steps: Vec<Step<'p>>,
    returned: Option<&'p Expression<'p>>,
    pub(super) touched: Vec<SymbolId>,
    pub(super) writes: bool,
}

struct Parse<'s, 'p> {
    scoping: &'s Scoping,
    states: &'s FxHashMap<SymbolId, State>,
    steps: Vec<Step<'p>>,
    touched: Vec<SymbolId>,
    writes: bool,
}

impl<'p> Parse<'_, 'p> {
    fn touch(&mut self, symbol: SymbolId) {
        if !self.touched.contains(&symbol) {
            self.touched.push(symbol);
        }
    }

    fn pure(&mut self, params: &[SymbolId], expr: &Expression<'_>) -> bool {
        let states = self.states;
        let is_state = |symbol: SymbolId| states.contains_key(&symbol);
        let mut shape = Shape {
            scoping: self.scoping,
            params,
            constants: &is_state,
            consulted: Vec::new(),
        };
        let pure = shape.pure(expr);
        for symbol in shape.consulted {
            self.touch(symbol);
        }
        pure
    }

    fn target(&mut self, target: &SimpleAssignmentTarget<'_>) -> Option<SymbolId> {
        let SimpleAssignmentTarget::AssignmentTargetIdentifier(id) = target else {
            return None;
        };
        let symbol = resolved_symbol(id, self.scoping)?;
        if !self.states.get(&symbol)?.writable {
            return None;
        }
        self.touch(symbol);
        self.writes = true;
        Some(symbol)
    }

    fn step(&mut self, params: &[SymbolId], expr: &'p Expression<'p>) -> bool {
        match expr {
            Expression::ParenthesizedExpression(inner) => self.step(params, &inner.expression),
            Expression::SequenceExpression(sequence) => sequence
                .expressions
                .iter()
                .all(|part| self.step(params, part)),
            Expression::AssignmentExpression(assign) => {
                if assign.operator.is_logical() || !self.pure(params, &assign.right) {
                    return false;
                }
                let Some(simple) = assign.left.as_simple_assignment_target() else {
                    return false;
                };
                let Some(symbol) = self.target(simple) else {
                    return false;
                };
                self.steps
                    .push(Step::Assign(symbol, assign.operator, &assign.right));
                true
            }
            Expression::UpdateExpression(update) => {
                let Some(symbol) = self.target(&update.argument) else {
                    return false;
                };
                self.steps.push(Step::Update(symbol, update.operator));
                true
            }
            other => {
                let pure = self.pure(params, other);
                self.steps.push(Step::Discard(other));
                pure
            }
        }
    }

    fn statements(
        &mut self,
        params: &[SymbolId],
        statements: &'p [Statement<'p>],
    ) -> Option<Option<&'p Expression<'p>>> {
        let (last, leading) = match statements.split_last() {
            Some((Statement::ReturnStatement(ret), leading)) => (ret.argument.as_ref(), leading),
            _ => (None, statements),
        };
        for statement in leading {
            match statement {
                Statement::ExpressionStatement(statement) => {
                    if !self.step(params, &statement.expression) {
                        return None;
                    }
                }
                Statement::EmptyStatement(_) => {}
                _ => return None,
            }
        }
        if let Some(returned) = last {
            if !self.pure(params, returned) {
                return None;
            }
        }
        Some(last)
    }

    fn expression_body(
        &mut self,
        params: &[SymbolId],
        expr: &'p Expression<'p>,
    ) -> Option<&'p Expression<'p>> {
        let parts: Vec<&'p Expression<'p>> = match expr.without_parentheses() {
            Expression::SequenceExpression(sequence) => sequence.expressions.iter().collect(),
            other => vec![other],
        };
        let (last, leading) = parts.split_last()?;
        if !leading.iter().all(|part| self.step(params, part)) {
            return None;
        }
        self.pure(params, last).then_some(*last)
    }
}

enum Code<'p> {
    Block(&'p FunctionBody<'p>),
    Expression(&'p Expression<'p>),
}

fn function_code<'p>(function: &'p Function<'p>) -> Option<(Code<'p>, Vec<SymbolId>)> {
    if function.r#async || function.generator {
        return None;
    }
    let body = function.body.as_deref()?;
    Some((Code::Block(body), simple_params(&function.params)?))
}

fn initialised<'p>(init: &'p Expression<'p>) -> Option<(Code<'p>, Vec<SymbolId>)> {
    match init {
        Expression::ArrowFunctionExpression(arrow) if !arrow.r#async => {
            let code = match &arrow.body {
                ArrowFunctionBody::FunctionBody(body) => Code::Block(body),
                other => Code::Expression(other.as_expression()?),
            };
            Some((code, simple_params(&arrow.params)?))
        }
        Expression::FunctionExpression(function) if function.id.is_none() => {
            function_code(function)
        }
        _ => None,
    }
}

pub(super) fn parse<'p>(
    statement: &'p Statement<'p>,
    scoping: &Scoping,
    states: &FxHashMap<SymbolId, State>,
) -> Option<Func<'p>> {
    let (symbol, hoisted, (code, params)) = match statement {
        Statement::FunctionDeclaration(function) => {
            let symbol = function.id.as_ref()?.symbol_id.get()?;
            if is_block_function(scoping, function) {
                return None;
            }
            (symbol, true, function_code(function)?)
        }
        Statement::VariableDeclaration(declaration) => {
            let [declarator] = declaration.declarations.as_slice() else {
                return None;
            };
            let BindingPattern::BindingIdentifier(id) = &declarator.id else {
                return None;
            };
            (
                id.symbol_id.get()?,
                false,
                initialised(declarator.init.as_ref()?)?,
            )
        }
        _ => return None,
    };
    let mut parse = Parse {
        scoping,
        states,
        steps: Vec::new(),
        touched: Vec::new(),
        writes: false,
    };
    let returned = match code {
        Code::Block(body) => {
            if !body.directives.is_empty() {
                return None;
            }
            parse.statements(&params, &body.statements)?
        }
        Code::Expression(expr) => Some(parse.expression_body(&params, expr)?),
    };
    Some(Func {
        symbol,
        statement: statement.span(),
        hoisted,
        params,
        steps: parse.steps,
        returned,
        touched: parse.touched,
        writes: parse.writes,
    })
}

fn fits(value: &Const) -> bool {
    !matches!(value, Const::Str(text) if text.len() > MAX_STRING)
}

fn updated(old: &Const, operator: UpdateOperator) -> Option<Const> {
    let number = fold_unary(UnaryOperator::UnaryPlus, old)?;
    let binary = match operator {
        UpdateOperator::Increment => BinaryOperator::Addition,
        UpdateOperator::Decrement => BinaryOperator::Subtraction,
    };
    fold_binary(binary, &number, &Const::Num(1.0))
}

impl<'p> Func<'p> {
    pub(super) fn run(
        &self,
        scoping: &Scoping,
        args: &[Const],
        env: &mut FxHashMap<SymbolId, Const>,
    ) -> Option<Const> {
        let mut known: Vec<SymbolId> = self.params.clone();
        let mut bound: FxHashMap<SymbolId, Const> = self
            .params
            .iter()
            .copied()
            .zip(args.iter().cloned())
            .collect();
        for symbol in &self.touched {
            bound.insert(*symbol, env.get(symbol)?.clone());
            known.push(*symbol);
        }
        let none = |_: SymbolId| None;
        let mut machine = Machine::new(scoping, &known, bound, &none);
        for step in &self.steps {
            match step {
                Step::Assign(symbol, operator, value) => {
                    let right = machine.value(value)?;
                    let next = match operator.to_binary_operator() {
                        None => right,
                        Some(binary) => fold_binary(binary, machine.current(*symbol)?, &right)?,
                    };
                    if !fits(&next) {
                        return None;
                    }
                    machine.assign(*symbol, next);
                }
                Step::Update(symbol, operator) => {
                    let next = updated(machine.current(*symbol)?, *operator)?;
                    machine.assign(*symbol, next);
                }
                Step::Discard(expr) => {
                    machine.value(expr)?;
                }
            }
        }
        let value = match self.returned {
            Some(expr) => machine.value(expr)?,
            None => Const::Undefined,
        };
        for symbol in &self.touched {
            env.insert(*symbol, machine.current(*symbol)?.clone());
        }
        Some(value)
    }
}
