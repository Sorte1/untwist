use oxc_ast::ast::*;
use oxc_syntax::operator::{BinaryOperator, UnaryOperator, UpdateOperator};
use oxc_syntax::symbol::SymbolId;
use rustc_hash::FxHashMap;

use crate::builtins::{self, Owner, Param};
use crate::facts::{Ctx, Facts, FnRef, Source};
use crate::ty::{Kind, Ty, Val};

const ROUNDS: usize = 64;
const EVAL_DEPTH: usize = 192;

pub struct Env<'f, 'a> {
    pub cx: &'f Ctx<'f, 'a>,
    pub facts: &'f Facts<'a>,
    pub vals: FxHashMap<SymbolId, Val>,
    pub returns: Vec<Val>,
    pub builtins: bool,
    pub promises: bool,
    depth: std::cell::Cell<usize>,
}

pub fn global_name<'a>(cx: &Ctx<'_, 'a>, ident: &IdentifierReference<'a>) -> Option<&'a str> {
    let reference = ident.reference_id.get()?;
    match cx.scoping.get_reference(reference).symbol_id() {
        Some(_) => None,
        None => Some(ident.name.as_str()),
    }
}

pub fn resolved(cx: &Ctx<'_, '_>, ident: &IdentifierReference<'_>) -> Option<SymbolId> {
    let reference = ident.reference_id.get()?;
    cx.scoping.get_reference(reference).symbol_id()
}

fn numeric(a: &Ty, b: &Ty) -> Ty {
    let a_number = !a.any_of(Kind::BigInt);
    let b_number = !b.any_of(Kind::BigInt);
    if a_number || b_number {
        return Ty::number();
    }
    match (a.only(Kind::BigInt), b.only(Kind::BigInt)) {
        (true, true) => Ty::bigint(),
        _ => Ty::number().join(&Ty::bigint()),
    }
}

fn numeric_one(a: &Ty) -> Ty {
    match (a.any_of(Kind::BigInt), a.only(Kind::BigInt)) {
        (false, _) => Ty::number(),
        (true, true) => Ty::bigint(),
        (true, false) => Ty::number().join(&Ty::bigint()),
    }
}

fn plus(a: &Ty, b: &Ty) -> Val {
    if a.only(Kind::String) || b.only(Kind::String) {
        return Val::Known(Ty::string());
    }
    let stringish = |ty: &Ty| ty.any_of(Kind::String) || ty.any_of(Kind::Promise);
    if stringish(a) || stringish(b) {
        return Val::Top;
    }
    Val::Known(numeric(a, b))
}

pub fn binary_result(operator: BinaryOperator, left: &Val, right: &Val) -> Val {
    if operator.is_equality()
        || operator.is_compare()
        || matches!(operator, BinaryOperator::In | BinaryOperator::Instanceof)
    {
        return Val::Known(Ty::boolean());
    }
    if operator == BinaryOperator::ShiftRightZeroFill {
        return strict2(left, right, |_, _| Val::Known(Ty::number()));
    }
    if operator == BinaryOperator::Addition {
        return strict2(left, right, plus);
    }
    match (left, right) {
        (Val::Bottom, _) | (_, Val::Bottom) => Val::Bottom,
        (Val::Known(a), _) if !a.any_of(Kind::BigInt) => Val::Known(Ty::number()),
        (_, Val::Known(b)) if !b.any_of(Kind::BigInt) => Val::Known(Ty::number()),
        (Val::Known(a), Val::Known(b)) => Val::Known(numeric(a, b)),
        _ => Val::Known(Ty::number().join(&Ty::bigint())),
    }
}

fn strict2(left: &Val, right: &Val, f: impl FnOnce(&Ty, &Ty) -> Val) -> Val {
    match (left, right) {
        (Val::Bottom, _) | (_, Val::Bottom) => Val::Bottom,
        (Val::Known(a), Val::Known(b)) => f(a, b),
        _ => Val::Top,
    }
}

impl<'f, 'a> Env<'f, 'a> {
    pub fn new(cx: &'f Ctx<'f, 'a>, facts: &'f Facts<'a>, builtins: bool, promises: bool) -> Self {
        Self {
            cx,
            facts,
            vals: facts
                .slots
                .keys()
                .map(|symbol| (*symbol, Val::Bottom))
                .collect(),
            returns: vec![Val::Bottom; facts.functions.len()],
            builtins,
            promises,
            depth: std::cell::Cell::new(0),
        }
    }

    pub fn solve(&mut self) {
        let mut symbols: Vec<SymbolId> = self.facts.slots.keys().copied().collect();
        symbols.sort_unstable();
        for _ in 0..ROUNDS {
            let mut changed = false;
            for symbol in &symbols {
                let next = self.slot_val(*symbol);
                if self.vals.get(symbol) != Some(&next) {
                    self.vals.insert(*symbol, next);
                    changed = true;
                }
            }
            for index in 0..self.facts.functions.len() {
                let next = self.return_val(index);
                if self.returns[index] != next {
                    self.returns[index] = next;
                    changed = true;
                }
            }
            if !changed {
                return;
            }
        }
        self.vals.values_mut().for_each(|val| *val = Val::Top);
        self.returns.iter_mut().for_each(|val| *val = Val::Top);
    }

    fn slot_val(&self, symbol: SymbolId) -> Val {
        let Some(slot) = self.facts.slots.get(&symbol) else {
            return Val::Top;
        };
        slot.sources.iter().fold(Val::Bottom, |held, source| {
            held.join(&self.source_val(source))
        })
    }

    fn source_val(&self, source: &Source<'a>) -> Val {
        match source {
            Source::Expr(expression) => self.eval(expression),
            Source::Undefined => Val::Known(Ty::undefined()),
            Source::String => Val::Known(Ty::string()),
            Source::Top => Val::Top,
            Source::Compound(operator, symbol, right) => {
                let left = self.vals.get(symbol).cloned().unwrap_or(Val::Top);
                binary_result(*operator, &left, &self.eval(right))
            }
            Source::Update(symbol) => {
                let held = self.vals.get(symbol).cloned().unwrap_or(Val::Top);
                held.map(|ty| Val::Known(numeric_one(ty)))
            }
            Source::Defaulted(argument, default) => {
                let given = argument.map_or(Val::Known(Ty::undefined()), |arg| self.eval(arg));
                match given {
                    Val::Known(ty) if !ty.any_of(Kind::Undefined) => Val::Known(ty),
                    Val::Known(ty) => {
                        let rest = ty.without_undefined().map_or(Val::Bottom, Val::Known);
                        rest.join(&self.eval(default))
                    }
                    other => other,
                }
            }
        }
    }

    fn return_val(&self, index: usize) -> Val {
        let fact = &self.facts.functions[index];
        if !fact.returns_typed && !fact.closed {
            return Val::Top;
        }
        if let FnRef::Function(function) = fact.func {
            if function.generator {
                return Val::Top;
            }
        }
        let mut val = match fact.func.expression_body() {
            Some(body) => self.eval(body),
            None => Val::Bottom,
        };
        for argument in &fact.returns {
            let next = argument.map_or(Val::Known(Ty::undefined()), |arg| self.eval(arg));
            val = val.join(&next);
        }
        if fact.falls_through {
            val = val.join(&Val::Known(Ty::undefined()));
        }
        if !fact.func.is_async() {
            return val;
        }
        match val {
            Val::Known(ty) if ty.promise_free() && self.promises => Val::Known(Ty::promise(ty)),
            _ => Val::Top,
        }
    }

    pub fn call_result(&self, index: usize) -> Val {
        let fact = &self.facts.functions[index];
        if !fact.closed {
            return Val::Top;
        }
        if let FnRef::Function(function) = fact.func {
            if function.generator {
                return Val::Top;
            }
        }
        if fact.func.is_async() && self.returns[index] == Val::Bottom {
            return Val::Top;
        }
        self.returns[index].clone()
    }

    pub fn eval(&self, expression: &Expression<'a>) -> Val {
        let depth = self.depth.get();
        if depth > EVAL_DEPTH {
            return Val::Top;
        }
        self.depth.set(depth + 1);
        let val = self.eval_at(expression);
        self.depth.set(depth);
        val
    }

    fn eval_at(&self, expression: &Expression<'a>) -> Val {
        match expression {
            Expression::StringLiteral(literal) => match literal.value.as_str() {
                Some(text) => Val::Known(Ty::string_literal(text)),
                None => Val::Known(Ty::string()),
            },
            Expression::NumericLiteral(literal) => Val::Known(Ty::number_literal(literal.value)),
            Expression::BigIntLiteral(_) => Val::Known(Ty::bigint()),
            Expression::BooleanLiteral(literal) => Val::Known(Ty::boolean_literal(literal.value)),
            Expression::NullLiteral(_) => Val::Known(Ty::null()),
            Expression::TemplateLiteral(_) => Val::Known(Ty::string()),
            Expression::Identifier(ident) => self.ident(ident),
            Expression::ParenthesizedExpression(inner) => self.eval(&inner.expression),
            Expression::UnaryExpression(unary) => self.unary(unary),
            Expression::BinaryExpression(binary) => binary_result(
                binary.operator,
                &self.eval(&binary.left),
                &self.eval(&binary.right),
            ),
            Expression::LogicalExpression(logical) => {
                self.eval(&logical.left).join(&self.eval(&logical.right))
            }
            Expression::ConditionalExpression(conditional) => self
                .eval(&conditional.consequent)
                .join(&self.eval(&conditional.alternate)),
            Expression::SequenceExpression(sequence) => sequence
                .expressions
                .last()
                .map_or(Val::Top, |last| self.eval(last)),
            Expression::AssignmentExpression(assign) => self.assignment(assign),
            Expression::UpdateExpression(update) => self.update(update),
            Expression::AwaitExpression(awaited) => match self.eval(&awaited.argument) {
                Val::Known(ty) if ty.promise_free() => Val::Known(ty),
                Val::Bottom => Val::Bottom,
                _ => Val::Top,
            },
            Expression::CallExpression(call) => self.call(call),
            Expression::StaticMemberExpression(member) => self.read(member),
            _ => Val::Top,
        }
    }

    fn ident(&self, ident: &IdentifierReference<'a>) -> Val {
        if let Some(symbol) = resolved(self.cx, ident) {
            return self.vals.get(&symbol).cloned().unwrap_or(Val::Top);
        }
        match global_name(self.cx, ident) {
            Some("undefined") => Val::Known(Ty::undefined()),
            Some("NaN" | "Infinity") => Val::Known(Ty::number()),
            _ => Val::Top,
        }
    }

    fn unary(&self, unary: &UnaryExpression<'a>) -> Val {
        match unary.operator {
            UnaryOperator::LogicalNot | UnaryOperator::Delete => Val::Known(Ty::boolean()),
            UnaryOperator::Typeof => Val::Known(Ty::string()),
            UnaryOperator::Void => Val::Known(Ty::undefined()),
            UnaryOperator::UnaryPlus => match self.eval(&unary.argument) {
                Val::Bottom => Val::Bottom,
                _ => Val::Known(Ty::number()),
            },
            UnaryOperator::UnaryNegation | UnaryOperator::BitwiseNot => {
                match self.eval(&unary.argument) {
                    Val::Bottom => Val::Bottom,
                    Val::Known(ty) => Val::Known(numeric_one(&ty)),
                    Val::Top => Val::Known(Ty::number().join(&Ty::bigint())),
                }
            }
        }
    }

    fn assignment(&self, assign: &AssignmentExpression<'a>) -> Val {
        let right = self.eval(&assign.right);
        match assign.operator.to_binary_operator() {
            None if assign.operator.is_assign() => right,
            None => match assign.left.get_identifier_name() {
                Some(_) => self.target_val(&assign.left).join(&right),
                None => Val::Top,
            },
            Some(operator) => binary_result(operator, &self.target_val(&assign.left), &right),
        }
    }

    fn target_val(&self, target: &AssignmentTarget<'a>) -> Val {
        match target {
            AssignmentTarget::AssignmentTargetIdentifier(ident) => self.ident(ident),
            _ => Val::Top,
        }
    }

    fn update(&self, update: &UpdateExpression<'a>) -> Val {
        let _ = UpdateOperator::Increment;
        let held = match &update.argument {
            SimpleAssignmentTarget::AssignmentTargetIdentifier(ident) => self.ident(ident),
            _ => Val::Top,
        };
        match held {
            Val::Known(ty) => Val::Known(numeric_one(&ty)),
            Val::Bottom => Val::Bottom,
            Val::Top => Val::Known(Ty::number().join(&Ty::bigint())),
        }
    }

    fn call(&self, call: &CallExpression<'a>) -> Val {
        if call.optional {
            return Val::Top;
        }
        match call.callee.without_parentheses() {
            Expression::Identifier(ident) => {
                if let Some(symbol) = resolved(self.cx, ident) {
                    return match self.facts.by_symbol.get(&symbol) {
                        Some(&index) => self.call_result(index),
                        None => Val::Top,
                    };
                }
                let Some(name) = global_name(self.cx, ident) else {
                    return Val::Top;
                };
                self.row_val(builtins::find(Owner::Global, name, true), call)
            }
            Expression::FunctionExpression(function) => self.iife(function.node_id.get()),
            Expression::ArrowFunctionExpression(arrow) => self.iife(arrow.node_id.get()),
            Expression::StaticMemberExpression(member) => self.method(member, call),
            _ => Val::Top,
        }
    }

    fn iife(&self, node: oxc_syntax::node::NodeId) -> Val {
        match self.facts.by_node.get(&node) {
            Some(&index) => self.call_result(index),
            None => Val::Top,
        }
    }

    fn row_val(&self, row: Option<&builtins::Row>, call: &CallExpression<'a>) -> Val {
        let Some(row) = row else {
            return Val::Top;
        };
        if !self.builtins {
            return Val::Top;
        }
        if row.params.contains(&Param::Pattern) && !self.plain_pattern(call) {
            return Val::Top;
        }
        Val::Known(row.proof.ty())
    }

    fn plain_pattern(&self, call: &CallExpression<'a>) -> bool {
        match call.arguments.first().and_then(Argument::as_expression) {
            Some(Expression::RegExpLiteral(_)) => true,
            Some(expression) => {
                matches!(self.eval(expression), Val::Known(ty) if ty.only(Kind::String))
            }
            None => false,
        }
    }

    pub fn static_owner(&self, object: &Expression<'a>) -> Option<&'a str> {
        match object.without_parentheses() {
            Expression::Identifier(ident) => global_name(self.cx, ident),
            _ => None,
        }
    }

    pub fn receiver_kind(&self, object: &Expression<'a>) -> Option<Kind> {
        let Val::Known(ty) = self.eval(object) else {
            return None;
        };
        let mut kinds = ty
            .kinds()
            .filter(|kind| !matches!(kind, Kind::Null | Kind::Undefined));
        let first = kinds.next()?;
        kinds.all(|kind| kind == first).then_some(first)
    }

    fn method(&self, member: &StaticMemberExpression<'a>, call: &CallExpression<'a>) -> Val {
        let key = member.property.name.as_str();
        if member.optional {
            return Val::Top;
        }
        if let Some(root) = self.static_owner(&member.object) {
            return self.row_val(builtins::find(Owner::Static(leak(root)), key, true), call);
        }
        match self.receiver_kind(&member.object) {
            Some(kind) => self.row_val(builtins::find(Owner::Proto(kind), key, true), call),
            None => Val::Top,
        }
    }

    fn read(&self, member: &StaticMemberExpression<'a>) -> Val {
        let key = member.property.name.as_str();
        if member.optional {
            return Val::Top;
        }
        if let Some(root) = self.static_owner(&member.object) {
            return match builtins::find(Owner::Static(leak(root)), key, false) {
                Some(row) if self.builtins => Val::Known(row.proof.ty()),
                _ => Val::Top,
            };
        }
        if key == "length" && self.receiver_kind(&member.object) == Some(Kind::String) {
            return Val::Known(Ty::number());
        }
        Val::Top
    }
}

fn leak(root: &str) -> &'static str {
    builtins::ROWS
        .iter()
        .find_map(|row| match row.owner {
            Owner::Static(name) if name == root => Some(name),
            _ => None,
        })
        .unwrap_or("")
}
