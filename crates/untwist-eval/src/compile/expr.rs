use std::rc::Rc;

use oxc_ast::ast::{
    Argument, BinaryExpression, CallExpression, ChainElement, ChainExpression,
    ComputedMemberExpression, Expression, IdentifierReference, LogicalExpression, MemberExpression,
    StringLiteral, UnaryExpression,
};
use oxc_span::GetSpan;
use oxc_syntax::operator::{BinaryOperator, LogicalOperator, UnaryOperator};

use crate::bail::Bail;
use crate::builtins::args::charge_for_length;
use crate::compile::assign::{compile_assignment, compile_delete, compile_update, PRIVATE_FIELD};
use crate::compile::depth::{self, Reach};
use crate::compile::literal::{compile_array, compile_object, compile_template};
use crate::compile::scope::{unlaid, Resolved, ScopeChain};
use crate::compile::{guard, konst, leveled, refuse, Code, Compiler, Guard, Placeholder};
use crate::frame::{Frame, FrameRef};
use crate::host::Host;
use crate::interp::Interp;
use crate::names::Name;
use crate::ops::{binary_fast, binary_op, string_key, to_boolean, unary_op};
use crate::stack;
use crate::value::{js_string_from_literal, Eval, FnCode, JsString, PropKey, Value};

trait Linked<'a, T> {
    fn eval(
        &self,
        interp: &mut Interp<'a>,
        frame: &FrameRef<'a>,
        reach: Reach,
    ) -> Eval<'a, Option<T>>;
}

struct LinkFn<F>(F);

impl<'a, T, F> Linked<'a, T> for LinkFn<F>
where
    F: Fn(&mut Interp<'a>, &FrameRef<'a>, Reach) -> Eval<'a, Option<T>>,
{
    fn eval(
        &self,
        interp: &mut Interp<'a>,
        frame: &FrameRef<'a>,
        reach: Reach,
    ) -> Eval<'a, Option<T>> {
        (self.0)(interp, frame, reach)
    }
}

fn unlinked<'a, T>(_: &mut Interp<'a>, _: &FrameRef<'a>, _: Reach) -> Eval<'a, Option<T>> {
    Ok(None)
}

type RawLink<'a> = Box<dyn Linked<'a, Value<'a>> + 'a>;
type RawMemberLink<'a> = Box<dyn Linked<'a, (Value<'a>, PropKey)> + 'a>;
type Link<'a> = Guard<RawLink<'a>>;
type MemberLink<'a> = Guard<RawMemberLink<'a>>;

impl Placeholder for RawLink<'_> {
    fn placeholder() -> Self {
        Box::new(LinkFn(unlinked))
    }
}

impl Placeholder for RawMemberLink<'_> {
    fn placeholder() -> Self {
        Box::new(LinkFn(unlinked))
    }
}

fn link<'a>(
    body: impl Fn(&mut Interp<'a>, &FrameRef<'a>, Reach) -> Eval<'a, Option<Value<'a>>> + 'a,
) -> Link<'a> {
    guard(Box::new(LinkFn(body)))
}

fn member_link<'a>(
    body: impl Fn(&mut Interp<'a>, &FrameRef<'a>, Reach) -> Eval<'a, Option<(Value<'a>, PropKey)>> + 'a,
) -> MemberLink<'a> {
    guard(Box::new(LinkFn(body)))
}

pub(super) fn node<'a>(
    body: impl Fn(&mut Interp<'a>, &FrameRef<'a>, Reach) -> Eval<'a, Value<'a>> + 'a,
) -> Code<'a> {
    leveled(body)
}

pub(crate) fn compile_expression<'a>(
    expr: &'a Expression<'a>,
    scope: &Rc<ScopeChain>,
    cx: &mut Compiler<'_, 'a>,
) -> Code<'a> {
    #[cfg(feature = "test-hooks")]
    crate::test_hooks::note_compiled_node(expr.span());
    cx.deeper(|cx| {
        if let Some(folded) = folded(expr, cx) {
            return folded;
        }
        stack::maybe_grow(|| compile_inner(expr, scope, cx))
    })
}

fn folded<'a>(expr: &'a Expression<'a>, cx: &mut Compiler<'_, 'a>) -> Option<Code<'a>> {
    if !konst::foldable_root(expr) || cx.unfoldable.is_some_and(|at| encloses(expr.span(), at)) {
        return None;
    }
    match konst::fold(expr) {
        Ok(folded) => {
            #[cfg(feature = "test-hooks")]
            crate::test_hooks::note_folded_constant();
            Some(node(move |interp, _, _| folded.run(interp, expr)))
        }
        Err(at) => {
            cx.unfoldable = Some(at);
            None
        }
    }
}

fn encloses(outer: oxc_span::Span, inner: oxc_span::Span) -> bool {
    outer.start <= inner.start && inner.end <= outer.end
}

fn compile_inner<'a>(
    expr: &'a Expression<'a>,
    scope: &Rc<ScopeChain>,
    cx: &mut Compiler<'_, 'a>,
) -> Code<'a> {
    match expr {
        Expression::NumericLiteral(literal) => {
            let value = literal.value;
            node(move |_, _, _| Ok(Value::Number(value)))
        }
        Expression::StringLiteral(literal) => {
            let text = js_string_from_literal(literal);
            node(move |_, _, _| Ok(Value::Str(text.clone())))
        }
        Expression::BooleanLiteral(literal) => {
            let value = literal.value;
            node(move |_, _, _| Ok(Value::Bool(value)))
        }
        Expression::NullLiteral(_) => node(|_, _, _| Ok(Value::Null)),
        Expression::BigIntLiteral(_) => refuse::expression(&"BigInt literal"),
        Expression::Identifier(id) => identifier(id, scope, cx),
        Expression::ParenthesizedExpression(paren) => {
            let inner = compile_expression(&paren.expression, scope, cx);
            node(move |interp, frame, reach| inner.run(interp, frame, reach))
        }
        Expression::UnaryExpression(unary) => compile_unary(unary, scope, cx),
        _ => compile_compound(expr, scope, cx),
    }
}

fn compile_compound<'a>(
    expr: &'a Expression<'a>,
    scope: &Rc<ScopeChain>,
    cx: &mut Compiler<'_, 'a>,
) -> Code<'a> {
    match expr {
        Expression::BinaryExpression(binary) => compile_binary(binary, scope, cx),
        Expression::LogicalExpression(logical) => compile_logical(logical, scope, cx),
        Expression::ConditionalExpression(conditional) => {
            let test = compile_expression(&conditional.test, scope, cx);
            let consequent = compile_expression(&conditional.consequent, scope, cx);
            let alternate = compile_expression(&conditional.alternate, scope, cx);
            node(
                move |interp, frame, reach| match to_boolean(&test.run(interp, frame, reach)?) {
                    true => consequent.run(interp, frame, reach),
                    false => alternate.run(interp, frame, reach),
                },
            )
        }
        Expression::SequenceExpression(sequence) => {
            let parts: Vec<Code<'a>> = sequence
                .expressions
                .iter()
                .map(|part| compile_expression(part, scope, cx))
                .collect();
            node(move |interp, frame, reach| {
                let mut last = Value::Undefined;
                for part in &parts {
                    last = part.run(interp, frame, reach)?;
                }
                Ok(last)
            })
        }
        Expression::StaticMemberExpression(_) | Expression::ComputedMemberExpression(_) => {
            match expr.as_member_expression() {
                Some(member) => compile_member_value(member, scope, cx),
                None => refuse::expression(&"member expression"),
            }
        }
        Expression::PrivateFieldExpression(_) => refused(&PRIVATE_FIELD),
        Expression::ChainExpression(chain) => compile_chain(chain, scope, cx),
        _ => compile_effect(expr, scope, cx),
    }
}

fn compile_effect<'a>(
    expr: &'a Expression<'a>,
    scope: &Rc<ScopeChain>,
    cx: &mut Compiler<'_, 'a>,
) -> Code<'a> {
    match expr {
        Expression::CallExpression(call) => {
            let link = compile_call_link(call, scope, cx);
            node(move |interp, frame, reach| {
                Ok(link.eval(interp, frame, reach)?.unwrap_or(Value::Undefined))
            })
        }
        Expression::AssignmentExpression(assignment) => compile_assignment(assignment, scope, cx),
        Expression::UpdateExpression(update) => compile_update(update, scope, cx),
        Expression::ThisExpression(_) => node(|interp, _, _| interp.read_this()),
        Expression::NewExpression(new) => {
            let callee = compile_expression(&new.callee, scope, cx);
            let args = compile_arguments(&new.arguments, scope, cx);
            node(move |interp, frame, reach| {
                let callee = callee.run(interp, frame, reach)?;
                let values = eval_arguments(&args, interp, (frame, reach))?;
                interp.construct(&callee, &values)
            })
        }
        Expression::FunctionExpression(function) => {
            let code = FnCode::Function(function);
            cx.functions.register(code, scope);
            node(move |interp, frame, _| Ok(interp.make_closure(code, frame)))
        }
        Expression::ArrowFunctionExpression(arrow) => {
            let code = FnCode::Arrow(arrow);
            cx.functions.register(code, scope);
            node(move |interp, frame, _| {
                let mut closure = interp.make_closure(code, frame);
                interp.bind_lexical_this(&mut closure);
                Ok(closure)
            })
        }
        _ => compile_literal(expr, scope, cx),
    }
}

fn compile_literal<'a>(
    expr: &'a Expression<'a>,
    scope: &Rc<ScopeChain>,
    cx: &mut Compiler<'_, 'a>,
) -> Code<'a> {
    match expr {
        Expression::ArrayExpression(array) => compile_array(array, scope, cx),
        Expression::ObjectExpression(object) => compile_object(object, scope, cx),
        Expression::TemplateLiteral(template) => compile_template(template, scope, cx),
        Expression::RegExpLiteral(regexp) => {
            node(move |interp, _, _| interp.eval_regexp_literal(regexp))
        }
        _ => refuse::unported(),
    }
}

pub(super) fn refused<'a>(why: &'static &'static str) -> Code<'a> {
    node(move |_, _, _| Err(Bail::Unsupported(why).into()))
}

fn compile_chain<'a>(
    chain: &'a ChainExpression<'a>,
    scope: &Rc<ScopeChain>,
    cx: &mut Compiler<'_, 'a>,
) -> Code<'a> {
    let link = match &chain.expression {
        ChainElement::CallExpression(call) => compile_call_link(call, scope, cx),
        ChainElement::PrivateFieldExpression(_) => return refused(&PRIVATE_FIELD),
        element => match element.as_member_expression() {
            Some(member) => compile_member_read_link(member, scope, cx),
            None => return refused(&"chain element"),
        },
    };
    node(move |interp, frame, reach| {
        Ok(link.eval(interp, frame, reach)?.unwrap_or(Value::Undefined))
    })
}

pub(super) struct Binding<'a> {
    pub(super) resolved: Resolved,
    pub(super) name: Name,
    pub(super) text: &'a str,
}

pub(super) fn binding<'a>(
    id: &'a IdentifierReference<'a>,
    scope: &Rc<ScopeChain>,
    cx: &mut Compiler<'_, 'a>,
) -> Binding<'a> {
    let name = cx.names.intern(id.name.as_str());
    let resolved = scope.resolve(name);
    #[cfg(feature = "test-hooks")]
    if resolved.is_maybe() {
        crate::test_hooks::note_maybe_node();
    }
    Binding {
        resolved,
        name,
        text: id.name.as_str(),
    }
}

fn identifier<'a>(
    id: &'a IdentifierReference<'a>,
    scope: &Rc<ScopeChain>,
    cx: &mut Compiler<'_, 'a>,
) -> Code<'a> {
    let read = binding(id, scope, cx);
    match read.resolved {
        Resolved::Local(slot) => {
            node(move |_, frame, _| frame.borrow().body.read_slot(slot).ok_or_else(unlaid))
        }
        Resolved::Outer { hops: 1, slot } => {
            node(move |_, frame, _| Frame::parent_slot(frame, slot).ok_or_else(unlaid))
        }
        _ => node(move |interp, frame, _| read.resolved.read(interp, frame, read.text)),
    }
}

fn compile_unary<'a>(
    unary: &'a UnaryExpression<'a>,
    scope: &Rc<ScopeChain>,
    cx: &mut Compiler<'_, 'a>,
) -> Code<'a> {
    let operator = unary.operator;
    if operator == UnaryOperator::Delete {
        return compile_delete(&unary.argument, scope, cx);
    }
    let probe = match (&unary.argument, operator) {
        (Expression::Identifier(id), UnaryOperator::Typeof) => Some(binding(id, scope, cx)),
        _ => None,
    };
    let argument = compile_expression(&unary.argument, scope, cx);
    node(move |interp, frame, reach| {
        if let Some(probe) = &probe {
            if let Some(early) = typeof_probe(interp, frame, probe)? {
                return Ok(early);
            }
        }
        let value = argument.run(interp, frame, reach)?;
        unary_op(operator, &value, interp)
    })
}

fn typeof_probe<'a>(
    interp: &mut Interp<'a>,
    frame: &FrameRef<'a>,
    probe: &Binding<'a>,
) -> Eval<'a, Option<Value<'a>>> {
    let decoding = crate::object::decoding();
    match probe.resolved.lookup(interp, frame)? {
        None if decoding || crate::unmodelled::is_node_global(probe.text) => {
            Err(Bail::unresolved_global(probe.text).into())
        }
        None => Ok(Some(Value::Str(JsString::from_str("undefined")))),
        Some(Value::Object(obj)) => match interp.realm.hostile_name(&obj) {
            Some(hostile) if decoding || !crate::unmodelled::is_node_global(&hostile) => {
                Err(Bail::hostile_global(hostile).into())
            }
            _ => Ok(None),
        },
        Some(_) => Ok(None),
    }
}

fn compile_binary<'a>(
    binary: &'a BinaryExpression<'a>,
    scope: &Rc<ScopeChain>,
    cx: &mut Compiler<'_, 'a>,
) -> Code<'a> {
    let left = compile_expression(&binary.left, scope, cx);
    let right = compile_expression(&binary.right, scope, cx);
    let operator = binary.operator;
    node(move |interp, frame, reach| {
        let left = left.run(interp, frame, reach)?;
        let right = right.run(interp, frame, reach)?;
        match operator {
            BinaryOperator::In | BinaryOperator::Instanceof => {
                binary_value(interp, operator, &left, &right)
            }
            op => binary_fast(op, &left, &right, interp),
        }
    })
}

pub(crate) fn binary_value<'a>(
    interp: &mut Interp<'a>,
    operator: BinaryOperator,
    left: &Value<'a>,
    right: &Value<'a>,
) -> Eval<'a, Value<'a>> {
    match operator {
        BinaryOperator::In => {
            if crate::object::decoding() && right.as_object().is_none() {
                return Err(Bail::Unsupported(&"`in` with a primitive on the right").into());
            }
            let key = interp.operator_key(left)?;
            Ok(Value::Bool(interp.has_property(&key, right)?))
        }
        BinaryOperator::Instanceof => Ok(Value::Bool(interp.is_instance_of(left, right)?)),
        op => binary_op(op, left, right, interp),
    }
}

fn compile_logical<'a>(
    logical: &'a LogicalExpression<'a>,
    scope: &Rc<ScopeChain>,
    cx: &mut Compiler<'_, 'a>,
) -> Code<'a> {
    let left = compile_expression(&logical.left, scope, cx);
    let right = compile_expression(&logical.right, scope, cx);
    let (operator, span) = (logical.operator, logical.right.span());
    node(move |interp, frame, reach| {
        let value = left.run(interp, frame, reach)?;
        let take_right = match operator {
            LogicalOperator::And => to_boolean(&value),
            LogicalOperator::Or => !to_boolean(&value),
            LogicalOperator::Coalesce => value.is_nullish(),
        };
        if !take_right {
            return Ok(value);
        }
        let entered = crate::state::enter_gate(span);
        let result = right.run(interp, frame, reach);
        crate::state::leave_gate(entered, result.is_ok());
        result
    })
}

pub(super) enum Key<'a> {
    Static(PropKey),
    Literal(PropKey),
    Computed(Code<'a>),
    Private,
}

impl<'a> Key<'a> {
    pub(super) fn of(
        member: &'a MemberExpression<'a>,
        scope: &Rc<ScopeChain>,
        cx: &mut Compiler<'_, 'a>,
    ) -> Self {
        match member {
            MemberExpression::ComputedMemberExpression(node) => match &node.expression {
                Expression::StringLiteral(literal) => literal_key(literal, cx),
                expression => Key::Computed(compile_expression(expression, scope, cx)),
            },
            MemberExpression::StaticMemberExpression(node) => {
                Key::Static(string_key(JsString::from_str(node.property.name.as_str())))
            }
            MemberExpression::PrivateFieldExpression(_) => Key::Private,
        }
    }

    pub(super) fn eval(
        &self,
        interp: &mut Interp<'a>,
        frame: &FrameRef<'a>,
        reach: Reach,
    ) -> Eval<'a, PropKey> {
        match self {
            Key::Static(key) => Ok(key.clone()),
            Key::Literal(key) => {
                depth::level(interp, reach, |interp| Ok(interp.budget.spend(1)?))?;
                charge_for_length(interp, key_length(key))?;
                Ok(key.clone())
            }
            Key::Computed(code) => {
                let value = code.run(interp, frame, reach)?;
                interp.operator_key(&value)
            }
            Key::Private => Err(Bail::Unsupported(&PRIVATE_FIELD).into()),
        }
    }
}

fn literal_key<'a>(literal: &'a StringLiteral<'a>, cx: &mut Compiler<'_, 'a>) -> Key<'a> {
    #[cfg(feature = "test-hooks")]
    crate::test_hooks::note_compiled_node(literal.span());
    cx.deeper(|_| ());
    Key::Literal(string_key(js_string_from_literal(literal)))
}

fn key_length(key: &PropKey) -> usize {
    match key {
        PropKey::Name(text) => text.len(),
        PropKey::Index(index) => index
            .checked_ilog10()
            .map_or(1, |digits| digits as usize + 1),
    }
}

fn private(member: &MemberExpression<'_>) -> bool {
    matches!(member, MemberExpression::PrivateFieldExpression(_))
}

fn compile_member_value<'a>(
    member: &'a MemberExpression<'a>,
    scope: &Rc<ScopeChain>,
    cx: &mut Compiler<'_, 'a>,
) -> Code<'a> {
    if private(member) {
        return refused(&PRIVATE_FIELD);
    }
    let optional = member.optional();
    let object = compile_expression(member.object(), scope, cx);
    let key = Key::of(member, scope, cx);
    if let Key::Computed(index) = key {
        return node(move |interp, frame, reach| {
            let target = object.run(interp, frame, reach)?;
            let held = index.run(interp, frame, reach)?;
            let Value::Number(n) = held else {
                let key = interp.operator_key(&held)?;
                drop(held);
                if optional && target.is_nullish() {
                    return Ok(Value::Undefined);
                }
                return interp.get_property(&target, &key);
            };
            if optional && target.is_nullish() {
                return Ok(Value::Undefined);
            }
            if let Some(value) = interp.element(&target, &held)? {
                return Ok(value);
            }
            interp.get_property(&target, &crate::ops::number_key(n))
        });
    }
    node(move |interp, frame, reach| {
        let target = object.run(interp, frame, reach)?;
        let key = key.eval(interp, frame, reach)?;
        if optional && target.is_nullish() {
            return Ok(Value::Undefined);
        }
        interp.get_property(&target, &key)
    })
}

fn compile_chain_base<'a>(
    expr: &'a Expression<'a>,
    scope: &Rc<ScopeChain>,
    cx: &mut Compiler<'_, 'a>,
) -> Link<'a> {
    cx.deeper(|cx| stack::maybe_grow(|| compile_chain_base_inner(expr, scope, cx)))
}

fn compile_chain_base_inner<'a>(
    expr: &'a Expression<'a>,
    scope: &Rc<ScopeChain>,
    cx: &mut Compiler<'_, 'a>,
) -> Link<'a> {
    match expr {
        Expression::StaticMemberExpression(_) | Expression::ComputedMemberExpression(_) => {
            match expr.as_member_expression() {
                Some(member) => compile_member_read_link(member, scope, cx),
                None => link(|_, _, _| Err(Bail::Unsupported(&"chain element").into())),
            }
        }
        Expression::PrivateFieldExpression(_) => {
            link(|_, _, _| Err(Bail::Unsupported(&PRIVATE_FIELD).into()))
        }
        Expression::CallExpression(call) => {
            #[cfg(feature = "test-hooks")]
            crate::test_hooks::note_compiled_node(call.span);
            compile_call_link(call, scope, cx)
        }
        other => {
            let code = compile_expression(other, scope, cx);
            link(move |interp, frame, reach| code.run(interp, frame, reach).map(Some))
        }
    }
}

fn compile_member_read_link<'a>(
    member: &'a MemberExpression<'a>,
    scope: &Rc<ScopeChain>,
    cx: &mut Compiler<'_, 'a>,
) -> Link<'a> {
    if let MemberExpression::ComputedMemberExpression(node) = member {
        return compile_computed_read_link(node, scope, cx);
    }
    let member = compile_member_link(member, scope, cx);
    link(
        move |interp, frame, reach| match member.eval(interp, frame, reach)? {
            Some((target, key)) => Ok(Some(interp.get_property(&target, &key)?)),
            None => Ok(None),
        },
    )
}

fn compile_computed_read_link<'a>(
    node: &'a ComputedMemberExpression<'a>,
    scope: &Rc<ScopeChain>,
    cx: &mut Compiler<'_, 'a>,
) -> Link<'a> {
    #[cfg(feature = "test-hooks")]
    crate::test_hooks::note_compiled_node(node.span);
    let optional = node.optional;
    let base = compile_chain_base(&node.object, scope, cx);
    let key = compile_expression(&node.expression, scope, cx);
    link(move |interp, frame, reach| {
        interp.budget.spend(1)?;
        let Some(target) = stack::maybe_grow(|| base.eval(interp, frame, reach))? else {
            return Ok(None);
        };
        if optional && target.is_nullish() {
            return Ok(None);
        }
        let held = key.run(interp, frame, reach)?;
        if let Some(value) = interp.element(&target, &held)? {
            return Ok(Some(value));
        }
        let key = interp.operator_key(&held)?;
        let value = interp.get_property(&target, &key)?;
        drop(held);
        Ok(Some(value))
    })
}

fn compile_member_link<'a>(
    member: &'a MemberExpression<'a>,
    scope: &Rc<ScopeChain>,
    cx: &mut Compiler<'_, 'a>,
) -> MemberLink<'a> {
    if private(member) {
        return member_link(|interp, _, _| {
            interp.budget.spend(1)?;
            Err(Bail::Unsupported(&PRIVATE_FIELD).into())
        });
    }
    #[cfg(feature = "test-hooks")]
    crate::test_hooks::note_compiled_node(member.span());
    let optional = member.optional();
    let base = compile_chain_base(member.object(), scope, cx);
    let key = Key::of(member, scope, cx);
    member_link(move |interp, frame, reach| {
        interp.budget.spend(1)?;
        let Some(target) = stack::maybe_grow(|| base.eval(interp, frame, reach))? else {
            return Ok(None);
        };
        if optional && target.is_nullish() {
            return Ok(None);
        }
        let key = key.eval(interp, frame, reach)?;
        Ok(Some((target, key)))
    })
}

enum Callee<'a> {
    Member(MemberLink<'a>),
    Value(Code<'a>),
}

enum Arg<'a> {
    Value(Code<'a>),
    Spread,
}

fn compile_call_link<'a>(
    call: &'a CallExpression<'a>,
    scope: &Rc<ScopeChain>,
    cx: &mut Compiler<'_, 'a>,
) -> Link<'a> {
    let callee = match call.callee.without_parentheses().as_member_expression() {
        Some(member) => Callee::Member(compile_member_link(member, scope, cx)),
        None => Callee::Value(compile_expression(&call.callee, scope, cx)),
    };
    let args = compile_arguments(&call.arguments, scope, cx);
    let optional = call.optional;
    link(move |interp, frame, reach| {
        let (function, this) = match &callee {
            Callee::Member(link) => match link.eval(interp, frame, reach)? {
                None => return Ok(None),
                Some((receiver, key)) => (interp.get_property(&receiver, &key)?, receiver),
            },
            Callee::Value(code) => (code.run(interp, frame, reach)?, Value::Undefined),
        };
        if optional && function.is_nullish() {
            return Ok(None);
        }
        let values = eval_arguments(&args, interp, (frame, reach))?;
        Ok(Some(interp.call(&function, this, &values)?))
    })
}

fn compile_arguments<'a>(
    arguments: &'a oxc_allocator::Vec<'a, Argument<'a>>,
    scope: &Rc<ScopeChain>,
    cx: &mut Compiler<'_, 'a>,
) -> Box<[Arg<'a>]> {
    arguments
        .iter()
        .map(|argument| match argument.as_expression() {
            Some(expr) => Arg::Value(compile_expression(expr, scope, cx)),
            None => Arg::Spread,
        })
        .collect()
}

fn eval_arguments<'a>(
    args: &[Arg<'a>],
    interp: &mut Interp<'a>,
    (frame, reach): (&FrameRef<'a>, Reach),
) -> Eval<'a, Vec<Value<'a>>> {
    let mut values = Vec::with_capacity(args.len());
    for arg in args {
        match arg {
            Arg::Value(code) => values.push(code.run(interp, frame, reach)?),
            Arg::Spread => return Err(Bail::Unsupported(&"spread argument").into()),
        }
    }
    Ok(values)
}

#[cfg(test)]
mod tests {
    use std::rc::Rc;

    use oxc_allocator::Allocator;
    use oxc_ast::ast::{ClassElement, Expression, Statement};

    use super::Key;
    use crate::budget::Budget;
    use crate::compile::scope::{Provable, ScopeChain};
    use crate::compile::Reach;
    use crate::frame::{FrameLayout, FrameRef};
    use crate::interp::Interp;

    fn private_member<'a>(program: &'a oxc_ast::ast::Program<'a>) -> &'a Expression<'a> {
        let Some(Statement::ClassDeclaration(class)) = program.body.first() else {
            panic!("a class");
        };
        let Some(ClassElement::MethodDefinition(method)) = class.body.body.get(1) else {
            panic!("a method");
        };
        let body = method.value.body.as_ref().expect("a body");
        let Some(Statement::ExpressionStatement(statement)) = body.statements.first() else {
            panic!("an expression statement");
        };
        &statement.expression
    }

    #[test]
    fn a_private_field_key_refuses_instead_of_reading_an_index() {
        let alloc = Allocator::default();
        let source = "class A { #x; m() { this.#x; } }";
        let parsed = untwist_core::parse::parse_program(&alloc, source);
        let member = private_member(&parsed.program)
            .as_member_expression()
            .expect("a member expression");
        let mut interp = Interp::new(source, Budget::default());
        let layout = Rc::new(FrameLayout::new(Vec::new()).expect("a layout"));
        let scope = ScopeChain::new(layout.clone(), Provable::All, None);
        let key = Key::of(member, &scope, &mut interp.compiler());
        let frame: FrameRef<'_> = interp.realm.new_fixed_env(&layout, &interp.global_env());
        let refused = match key.eval(&mut interp, &frame, Reach::CHECKED) {
            Ok(_) => String::new(),
            Err(abrupt) => match abrupt.kind() {
                crate::bail::AbruptKind::Bail(bail) => bail.to_string(),
                crate::bail::AbruptKind::Throw(_) => "a throw".to_string(),
            },
        };
        assert!(refused.contains("private field access"), "{refused}");
    }
}
