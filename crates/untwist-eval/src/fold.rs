use oxc_ast::ast::{
    ArrayExpression, ArrayExpressionElement, BinaryExpression, CallExpression,
    ConditionalExpression, Expression, IdentifierReference, LogicalExpression, MemberExpression,
    ObjectExpression, ObjectPropertyKind, SequenceExpression, Str, TemplateLiteral,
    UnaryExpression,
};
use oxc_ast::builder::GetAstBuilder;
use oxc_span::{Span, SPAN};
use oxc_syntax::number::NumberBase;
use oxc_syntax::operator::{BinaryOperator, LogicalOperator, UnaryOperator};
use std::cell::RefCell;
use std::rc::Rc;

use rustc_hash::{FxHashMap, FxHashSet};

use crate::bail::{Bail, ErrorKind, EvalResult};
use crate::budget::Budget;
use crate::builtins::args::charge_for_length;
use crate::builtins::object::static_property_key;
use crate::builtins::{Globals, Realm};
use crate::heap::{reserve, Charge, HeapLedger};
use crate::host::Host;
use crate::object::{lookup_property, set_property, string_property};
use crate::ops::{binary_op, string_key, to_boolean, to_js_string, to_property_key, unary_op};
use crate::stack;
use crate::unmodelled::{
    refuse_boolean_member, refuse_inherited_miss, refuse_own_undefined, refuse_primitive_miss,
};
use crate::value::{
    js_string_from_js_str, js_string_from_literal, Abrupt, Eval, JsString, ObjKind, ObjRef,
    PropKey, Value,
};

mod budget_host;
mod layer;

pub(crate) use budget_host::BudgetHost;
pub use layer::{fold_layer_text, LayerFold, LayerText};

pub const FOLD_STEP_BUDGET: u64 = 100_000;

#[derive(Clone, Debug, PartialEq)]
pub enum Const {
    Num(f64),
    Str(String),
    Bool(bool),
    Null,
    Undefined,
}

pub(crate) struct PureHost {
    budget: Budget,
    realm: Realm<'static>,
    globals: Globals<'static>,
    allow_local_mutation: bool,
    heap: Option<Rc<HeapLedger>>,
    roots: FxHashSet<usize>,
}

pub(crate) type ConstHost = BudgetHost;

impl PureHost {
    pub(crate) fn new() -> Self {
        let (realm, globals) = Realm::with_globals();
        realm.compact(&globals);
        let roots = globals
            .objects()
            .iter()
            .map(|(obj, _)| Rc::as_ptr(obj) as usize)
            .collect();
        Self {
            budget: Budget::new(FOLD_STEP_BUDGET),
            realm,
            globals,
            allow_local_mutation: false,
            heap: None,
            roots,
        }
    }

    pub(crate) fn realm(&self) -> &Realm<'static> {
        &self.realm
    }

    pub(crate) fn global(&self, name: &str) -> Option<Value<'static>> {
        self.globals.get(name)
    }

    fn reset(&mut self) {
        self.budget = Budget::new(FOLD_STEP_BUDGET);
        self.allow_local_mutation = false;
    }
}

thread_local! {
    static SHARED_PURE_HOST: RefCell<PureHost> = RefCell::new(PureHost::new());
}

fn with_pure_host<R>(body: impl FnOnce(&mut PureHost) -> R) -> R {
    SHARED_PURE_HOST.with(|cell| match cell.try_borrow_mut() {
        Ok(mut host) => {
            host.reset();
            body(&mut host)
        }
        Err(_) => body(&mut PureHost::new()),
    })
}

impl Default for PureHost {
    fn default() -> Self {
        Self::new()
    }
}

impl Host<'static> for PureHost {
    fn spend(&mut self, steps: u64) -> EvalResult<()> {
        self.budget.spend(steps)
    }

    fn reserve_live(&mut self, bytes: u64) -> EvalResult<Option<Charge>> {
        match &self.heap {
            Some(ledger) => reserve(ledger, bytes).map(Some),
            None => Ok(None),
        }
    }

    fn charges_heap(&self) -> bool {
        self.heap.is_some()
    }

    fn check_heap(&self) -> EvalResult<()> {
        match &self.heap {
            Some(ledger) => ledger.check(),
            None => Ok(()),
        }
    }

    fn throw(&self, kind: ErrorKind, message: &str) -> Abrupt<'static> {
        Abrupt::throw(self.realm.error_value(kind, message))
    }

    fn note(&mut self, _message: &str) {}

    fn new_object(
        &mut self,
        kind: ObjKind<'static>,
        proto: Option<ObjRef<'static>>,
    ) -> Value<'static> {
        self.realm.new_object(kind, proto)
    }

    fn object_prototype(&self) -> Option<ObjRef<'static>> {
        self.realm.object_prototype()
    }

    fn refuse_namespace(&self, target: &Value<'static>) -> EvalResult<()> {
        match target.as_object() {
            Some(obj) => self.realm.refuse_namespace(obj),
            None => Ok(()),
        }
    }

    fn new_array(&mut self, items: Vec<Value<'static>>) -> Value<'static> {
        self.realm.new_array(items, None)
    }

    fn call_value(
        &mut self,
        callee: &Value<'static>,
        this: Value<'static>,
        args: &[Value<'static>],
    ) -> Eval<'static, Value<'static>> {
        self.spend(1)?;
        let Some(obj) = callee.as_object() else {
            return Err(self.throw(ErrorKind::TypeError, "not a function"));
        };
        let id = match obj.borrow().kind {
            ObjKind::Native(id, _) => id,
            _ => return Err(Bail::Unsupported(&"user function call during folding").into()),
        };
        if !self.realm.is_pure_native(id)
            && !(self.allow_local_mutation && self.realm.is_locally_pure_native(id))
        {
            return Err(Bail::Unsupported(&"impure native during folding").into());
        }
        let table = self.realm.natives();
        let Some((function, shape)) = table.get(id) else {
            return Err(Bail::Unsupported(&"unknown native during folding").into());
        };
        if !shape.admits(&this, args) {
            return Err(Bail::Unsupported(&crate::builtins::OUTSIDE_SHAPE).into());
        }
        function(self, this, args)
    }

    fn construct_value(
        &mut self,
        _callee: &Value<'static>,
        _args: &[Value<'static>],
    ) -> Eval<'static, Value<'static>> {
        Err(Bail::Unsupported(&"new during folding").into())
    }

    fn get_property(
        &mut self,
        target: &Value<'static>,
        key: &PropKey,
    ) -> Eval<'static, Value<'static>> {
        self.spend(1)?;
        match target {
            Value::Undefined | Value::Null => Err(self.throw(
                ErrorKind::TypeError,
                "cannot read a property of null or undefined",
            )),
            Value::Str(text) => match string_property(text, key) {
                Some(value) => Ok(value),
                None => {
                    refuse_primitive_miss(target, key, self.realm.string_prototype_member(key)?)
                }
            },
            Value::Number(_) => {
                refuse_primitive_miss(target, key, self.realm.number_prototype_member(key)?)
            }
            Value::Bool(_) => refuse_boolean_member(),
            Value::Object(obj) => {
                if let Some(name) = self.realm.hostile_name(obj) {
                    return Err(Bail::hostile_global(name).into());
                }
                let own = lookup_property(obj, key)?;
                if matches!(own, Some(Value::Undefined)) {
                    refuse_own_undefined(obj, key)?;
                }
                if let Some(value) = own {
                    return Ok(value);
                }
                let inherited = self.realm.intrinsic_member(target, key)?;
                let root = self.roots.contains(&(Rc::as_ptr(obj) as usize));
                refuse_inherited_miss(&self.realm, (target, obj, root), key, inherited.as_ref())?;
                Ok(inherited.unwrap_or(Value::Undefined))
            }
        }
    }

    fn note_protocol_member(&mut self, target: &Value<'static>, key: &PropKey) -> EvalResult<()> {
        if target.as_object().is_some() {
            self.realm.intrinsic_member(target, key)?;
        }
        Ok(())
    }

    fn function_source(&mut self, _target: &Value<'static>) -> Eval<'static, JsString> {
        Err(Bail::Unsupported(&"toString during folding").into())
    }

    fn enter_nesting(&mut self) -> EvalResult<()> {
        self.budget.enter_nesting()
    }

    fn leave_nesting(&mut self) {
        self.budget.leave_nesting();
    }

    fn get_prototype_of(&mut self, _value: &Value<'static>) -> Eval<'static, Value<'static>> {
        Err(Bail::Unsupported(&"getPrototypeOf during folding").into())
    }
}

pub(crate) fn const_to_value(value: &Const) -> Value<'static> {
    match value {
        Const::Num(n) => Value::Number(*n),
        Const::Str(text) => Value::Str(JsString::from_str(text)),
        Const::Bool(b) => Value::Bool(*b),
        Const::Null => Value::Null,
        Const::Undefined => Value::Undefined,
    }
}

pub fn value_to_const(value: &Value<'_>) -> Option<Const> {
    match value {
        Value::Number(n) => Some(Const::Num(*n)),
        Value::Str(text) => Some(Const::Str(text.as_string()?)),
        Value::Bool(b) => Some(Const::Bool(*b)),
        Value::Null => Some(Const::Null),
        Value::Undefined => Some(Const::Undefined),
        Value::Object(_) => None,
    }
}

pub fn fold_binary(op: BinaryOperator, left: &Const, right: &Const) -> Option<Const> {
    let mut host = ConstHost::default();
    let result = binary_op(op, &const_to_value(left), &const_to_value(right), &mut host).ok()?;
    value_to_const(&result)
}

pub fn fold_unary(op: UnaryOperator, value: &Const) -> Option<Const> {
    let mut host = ConstHost::default();
    let result = unary_op(op, &const_to_value(value), &mut host).ok()?;
    value_to_const(&result)
}

pub fn const_from_literal(expr: &Expression<'_>) -> Option<Const> {
    match expr.without_parentheses() {
        Expression::NumericLiteral(n) => Some(Const::Num(n.value)),
        Expression::StringLiteral(s) => Some(Const::Str(s.value.as_str()?.to_string())),
        Expression::BooleanLiteral(b) => Some(Const::Bool(b.value)),
        Expression::NullLiteral(_) => Some(Const::Null),
        Expression::UnaryExpression(u) if u.operator == UnaryOperator::UnaryNegation => {
            match &u.argument {
                Expression::NumericLiteral(n) => Some(Const::Num(-n.value)),
                _ => None,
            }
        }
        _ => None,
    }
}

pub fn const_to_expression<'a>(
    value: &Const,
    span: Span,
    b: &impl GetAstBuilder<'a>,
) -> Option<Expression<'a>> {
    match value {
        Const::Num(v) => {
            if !v.is_finite() {
                return None;
            }
            if v.is_sign_negative() {
                let literal =
                    Expression::new_numeric_literal(SPAN, v.abs(), None, NumberBase::Decimal, b);
                Some(Expression::new_unary_expression(
                    span,
                    UnaryOperator::UnaryNegation,
                    literal,
                    b,
                ))
            } else {
                Some(Expression::new_numeric_literal(
                    span,
                    *v,
                    None,
                    NumberBase::Decimal,
                    b,
                ))
            }
        }
        Const::Str(text) => {
            let value = Str::from_str_in(text, b.builder());
            Some(Expression::new_string_literal(span, value, None, b))
        }
        Const::Bool(v) => Some(Expression::new_boolean_literal(span, *v, b)),
        Const::Null => Some(Expression::new_null_literal(span, b)),
        Const::Undefined => None,
    }
}

#[derive(Clone, Debug)]
pub struct FoldOptions {
    pub allow_pure_calls: bool,
    pub allow_local_mutation: bool,
    pub bindings: FxHashMap<String, Const>,
}

impl Default for FoldOptions {
    fn default() -> Self {
        Self {
            allow_pure_calls: true,
            allow_local_mutation: false,
            bindings: FxHashMap::default(),
        }
    }
}

pub fn fold_to_const(expr: &Expression<'_>, options: &FoldOptions) -> Option<Const> {
    if options.allow_local_mutation && !options.bindings.is_empty() {
        return None;
    }
    with_pure_host(|host| {
        host.allow_local_mutation = options.allow_local_mutation;
        fold_with(expr, options, host)
            .as_ref()
            .and_then(value_to_const)
    })
}

pub fn fold_literal_chain(expr: &Expression<'_>) -> Option<Const> {
    let options = FoldOptions {
        allow_pure_calls: true,
        allow_local_mutation: true,
        bindings: FxHashMap::default(),
    };
    fold_to_const(expr, &options)
}

pub fn fold_literal_chain_expression<'a>(
    expr: &Expression<'_>,
    span: Span,
    b: &impl GetAstBuilder<'a>,
) -> Option<Expression<'a>> {
    let options = FoldOptions {
        allow_pure_calls: true,
        allow_local_mutation: true,
        bindings: FxHashMap::default(),
    };
    with_pure_host(|host| {
        host.allow_local_mutation = true;
        match fold_with(expr, &options, host)? {
            Value::Str(text) => Some(crate::decode::string_literal(&text, span, b)),
            value => const_to_expression(&value_to_const(&value)?, span, b),
        }
    })
}

fn fold_with(
    expr: &Expression<'_>,
    options: &FoldOptions,
    host: &mut PureHost,
) -> Option<Value<'static>> {
    host.enter_nesting().ok()?;
    let result = stack::maybe_grow(|| fold_with_inner(expr, options, host));
    host.leave_nesting();
    result
}

fn fold_with_inner(
    expr: &Expression<'_>,
    options: &FoldOptions,
    host: &mut PureHost,
) -> Option<Value<'static>> {
    host.spend(1).ok()?;
    match expr {
        Expression::NumericLiteral(node) => Some(Value::Number(node.value)),
        Expression::StringLiteral(node) => Some(Value::Str(js_string_from_literal(node.as_ref()))),
        Expression::BooleanLiteral(node) => Some(Value::Bool(node.value)),
        Expression::NullLiteral(_) => Some(Value::Null),
        Expression::ParenthesizedExpression(node) => fold_with(&node.expression, options, host),
        Expression::Identifier(node) => fold_identifier(node.as_ref(), options),
        Expression::UnaryExpression(node) => fold_unary_expr(node.as_ref(), options, host),
        Expression::BinaryExpression(node) => fold_binary_expr(node.as_ref(), options, host),
        Expression::LogicalExpression(node) => fold_logical_expr(node.as_ref(), options, host),
        Expression::ConditionalExpression(node) => {
            fold_conditional_expr(node.as_ref(), options, host)
        }
        Expression::SequenceExpression(node) => fold_sequence_expr(node.as_ref(), options, host),
        Expression::TemplateLiteral(node) => fold_template(node.as_ref(), options, host),
        Expression::ArrayExpression(node) => fold_array_expr(node.as_ref(), options, host),
        Expression::ObjectExpression(node) => fold_object_expr(node.as_ref(), options, host),
        Expression::StaticMemberExpression(_) | Expression::ComputedMemberExpression(_) => {
            fold_member_read(expr, options, host)
        }
        Expression::CallExpression(node) => {
            if !options.allow_pure_calls || node.optional {
                return None;
            }
            fold_call(node.as_ref(), options, host)
        }
        _ => None,
    }
}

fn fold_identifier(
    node: &IdentifierReference<'_>,
    options: &FoldOptions,
) -> Option<Value<'static>> {
    match node.name.as_str() {
        "undefined" => Some(Value::Undefined),
        "NaN" => Some(Value::Number(f64::NAN)),
        "Infinity" => Some(Value::Number(f64::INFINITY)),
        name => options.bindings.get(name).map(const_to_value),
    }
}

fn fold_unary_expr(
    node: &UnaryExpression<'_>,
    options: &FoldOptions,
    host: &mut PureHost,
) -> Option<Value<'static>> {
    if node.operator == UnaryOperator::Delete {
        return None;
    }
    let argument = fold_with(&node.argument, options, host)?;
    unary_op(node.operator, &argument, host).ok()
}

fn fold_binary_expr(
    node: &BinaryExpression<'_>,
    options: &FoldOptions,
    host: &mut PureHost,
) -> Option<Value<'static>> {
    if matches!(
        node.operator,
        BinaryOperator::In | BinaryOperator::Instanceof
    ) {
        return None;
    }
    let left = fold_with(&node.left, options, host)?;
    let right = fold_with(&node.right, options, host)?;
    binary_op(node.operator, &left, &right, host).ok()
}

fn fold_logical_expr(
    node: &LogicalExpression<'_>,
    options: &FoldOptions,
    host: &mut PureHost,
) -> Option<Value<'static>> {
    let left = fold_with(&node.left, options, host)?;
    let take_right = match node.operator {
        LogicalOperator::And => to_boolean(&left),
        LogicalOperator::Or => !to_boolean(&left),
        LogicalOperator::Coalesce => left.is_nullish(),
    };
    if take_right {
        return fold_with(&node.right, options, host);
    }
    Some(left)
}

fn fold_conditional_expr(
    node: &ConditionalExpression<'_>,
    options: &FoldOptions,
    host: &mut PureHost,
) -> Option<Value<'static>> {
    let test = fold_with(&node.test, options, host)?;
    if to_boolean(&test) {
        fold_with(&node.consequent, options, host)
    } else {
        fold_with(&node.alternate, options, host)
    }
}

fn fold_sequence_expr(
    node: &SequenceExpression<'_>,
    options: &FoldOptions,
    host: &mut PureHost,
) -> Option<Value<'static>> {
    let mut last = None;
    for part in &node.expressions {
        last = Some(fold_with(part, options, host)?);
    }
    last
}

fn fold_array_expr(
    node: &ArrayExpression<'_>,
    options: &FoldOptions,
    host: &mut PureHost,
) -> Option<Value<'static>> {
    let mut items = Vec::with_capacity(node.elements.len());
    for element in &node.elements {
        match element {
            ArrayExpressionElement::SpreadElement(_) => return None,
            ArrayExpressionElement::Elision(_) => return None,
            other => items.push(fold_with(other.as_expression()?, options, host)?),
        }
    }
    Some(
        host.realm()
            .new_array(items, host.realm().array_prototype()),
    )
}

fn fold_object_expr(
    node: &ObjectExpression<'_>,
    options: &FoldOptions,
    host: &mut PureHost,
) -> Option<Value<'static>> {
    let built = host
        .realm()
        .new_object(ObjKind::Plain(0), host.realm().object_prototype());
    let obj = built.as_object()?.clone();
    for property in &node.properties {
        let ObjectPropertyKind::ObjectProperty(entry) = property else {
            return None;
        };
        if crate::builtins::object::unmodelled_literal_entry(entry).is_some() {
            return None;
        }
        let key = static_property_key(&entry.key)?;
        let value = fold_with(&entry.value, options, host)?;
        set_property(&obj, key, value).ok()?;
    }
    Some(built)
}

fn fold_member_read(
    expr: &Expression<'_>,
    options: &FoldOptions,
    host: &mut PureHost,
) -> Option<Value<'static>> {
    let member = expr.as_member_expression()?;
    let (object, key) = fold_member(member, options, host)?;
    host.get_property(&object, &key).ok()
}

fn fold_template(
    node: &TemplateLiteral<'_>,
    options: &FoldOptions,
    host: &mut PureHost,
) -> Option<Value<'static>> {
    let mut out = JsString::empty();
    for (index, quasi) in node.quasis.iter().enumerate() {
        let cooked = quasi.value.cooked?;
        let chunk = js_string_from_js_str(cooked);
        charge_for_length(host, out.len() + chunk.len()).ok()?;
        out = out.concat(&chunk).ok()?;
        if let Some(part) = node.expressions.get(index) {
            let value = fold_with(part, options, host)?;
            let rendered = to_js_string(&value, host).ok()?;
            charge_for_length(host, out.len() + rendered.len()).ok()?;
            out = out.concat(&rendered).ok()?;
        }
    }
    Some(Value::Str(out))
}

fn fold_member(
    member: &MemberExpression<'_>,
    options: &FoldOptions,
    host: &mut PureHost,
) -> Option<(Value<'static>, PropKey)> {
    match member {
        MemberExpression::StaticMemberExpression(node) => {
            if node.optional {
                return None;
            }
            let object = fold_object_or_intrinsic(&node.object, options, host)?;
            Some((
                object,
                string_key(JsString::from_str(node.property.name.as_str())),
            ))
        }
        MemberExpression::ComputedMemberExpression(node) => {
            if node.optional {
                return None;
            }
            let object = fold_object_or_intrinsic(&node.object, options, host)?;
            let key = fold_with(&node.expression, options, host)?;
            Some((object, to_property_key(&key, host).ok()?))
        }
        MemberExpression::PrivateFieldExpression(_) => None,
    }
}

fn fold_object_or_intrinsic(
    expr: &Expression<'_>,
    options: &FoldOptions,
    host: &mut PureHost,
) -> Option<Value<'static>> {
    if let Expression::Identifier(id) = expr {
        if !options.bindings.contains_key(id.name.as_str()) {
            if let Some(value) = host.global(id.name.as_str()) {
                return Some(value);
            }
        }
    }
    fold_with(expr, options, host)
}

fn resolve_callee(
    node: &CallExpression<'_>,
    options: &FoldOptions,
    host: &mut PureHost,
) -> Option<(Value<'static>, Value<'static>)> {
    match node.callee.as_member_expression() {
        Some(member) => {
            let (receiver, key) = fold_member(member, options, host)?;
            let callee = host.get_property(&receiver, &key).ok()?;
            Some((callee, receiver))
        }
        None => {
            let callee = fold_object_or_intrinsic(&node.callee, options, host)?;
            Some((callee, Value::Undefined))
        }
    }
}

fn fold_call(
    node: &CallExpression<'_>,
    options: &FoldOptions,
    host: &mut PureHost,
) -> Option<Value<'static>> {
    let mut args = Vec::with_capacity(node.arguments.len());
    for argument in &node.arguments {
        args.push(fold_with(argument.as_expression()?, options, host)?);
    }
    let (callee, this) = resolve_callee(node, options, host)?;
    host.call_value(&callee, this, &args).ok()
}
