use oxc_allocator::{Box as ABox, Vec as AVec};
use oxc_ast::ast::*;
use oxc_semantic::Scoping;
use oxc_span::{Span, SPAN};
use oxc_syntax::scope::ScopeFlags;
use oxc_syntax::symbol::SymbolId;
use oxc_traverse::Ancestor;
use untwist_core::pass::{Cx, Safety};

use super::class_ctor::{self, CtorPlan};
use super::class_members;
use super::class_proto::Prototypes;
use super::identify::{resolved, Table};
use super::shapes::Kind;
use super::spread::Gate;
use super::values::into_inner;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Form {
    Iife,
    Direct,
}

#[derive(Clone, Copy)]
struct Tail {
    define: usize,
    returns_class: bool,
}

struct Parts<'k, 'a> {
    form: Form,
    strict: bool,
    define_index: usize,
    ctor: &'k Function<'a>,
    symbol: SymbolId,
    define: Option<&'k CallExpression<'a>>,
    parent: Option<SymbolId>,
}

fn kind_of(callee: &Expression<'_>, table: &Table, scoping: &Scoping) -> Option<Kind> {
    table.kind_of_callee(callee, scoping)
}

fn is_class_trigger(call: &CallExpression<'_>, table: &Table, scoping: &Scoping) -> bool {
    matches!(
        kind_of(&call.callee, table, scoping),
        Some(Kind::CreateClass | Kind::CreateClassOpen)
    )
}

fn argument<'k, 'a>(call: &'k CallExpression<'a>, index: usize) -> Option<&'k Expression<'a>> {
    call.arguments
        .get(index)
        .and_then(Argument::as_expression)
        .map(Expression::without_parentheses)
}

fn plain_arguments(call: &CallExpression<'_>) -> bool {
    !call.optional
        && call
            .arguments
            .iter()
            .all(|argument| !matches!(argument, Argument::SpreadElement(_)))
}

fn member_slot(call: &CallExpression<'_>, index: usize, scoping: &Scoping) -> bool {
    match argument(call, index) {
        None | Some(Expression::ArrayExpression(_) | Expression::NullLiteral(_)) => true,
        Some(other) => super::values::is_skipped_source(other, scoping),
    }
}

fn define_call_of<'k, 'a>(
    call: &'k CallExpression<'a>,
    symbol: Option<SymbolId>,
    table: &Table,
    scoping: &Scoping,
) -> Option<&'k CallExpression<'a>> {
    let shaped = plain_arguments(call)
        && (1..=3).contains(&call.arguments.len())
        && is_class_trigger(call, table, scoping)
        && member_slot(call, 1, scoping)
        && member_slot(call, 2, scoping);
    if !shaped {
        return None;
    }
    match symbol {
        Some(symbol) => (resolved(argument(call, 0)?, scoping) == Some(symbol)).then_some(call),
        None => Some(call),
    }
}

fn function_symbol(function: &Function<'_>) -> Option<SymbolId> {
    function.id.as_ref()?.symbol_id.get()
}

fn constant_symbol(symbol: SymbolId, scoping: &Scoping) -> bool {
    scoping.symbol_redeclarations(symbol).is_empty()
        && !scoping
            .get_resolved_references(symbol)
            .any(|reference| reference.is_write())
}

#[inline]
fn leads_with_function(call: &CallExpression<'_>) -> bool {
    matches!(
        call.arguments.first(),
        Some(Argument::FunctionExpression(_))
    )
}

fn direct<'k, 'a>(
    call: &'k CallExpression<'a>,
    table: &Table,
    scoping: &Scoping,
) -> Option<Parts<'k, 'a>> {
    let define = define_call_of(call, None, table, scoping)?;
    let Expression::FunctionExpression(ctor) = argument(call, 0)? else {
        return None;
    };
    Some(Parts {
        form: Form::Direct,
        strict: false,
        define_index: 0,
        symbol: function_symbol(ctor)?,
        ctor,
        define: Some(define),
        parent: None,
    })
}

fn call_of<'k, 'a>(statement: &'k Statement<'a>) -> Option<&'k CallExpression<'a>> {
    let expression = match statement {
        Statement::ExpressionStatement(statement) => &statement.expression,
        Statement::ReturnStatement(statement) => statement.argument.as_ref()?,
        _ => return None,
    };
    match expression.without_parentheses() {
        Expression::CallExpression(call) => Some(call),
        _ => None,
    }
}

fn returns_symbol(statement: &Statement<'_>, symbol: SymbolId, scoping: &Scoping) -> bool {
    let Statement::ReturnStatement(statement) = statement else {
        return false;
    };
    statement
        .argument
        .as_ref()
        .is_some_and(|argument| resolved(argument, scoping) == Some(symbol))
}

fn is_inherits(
    statement: &Statement<'_>,
    symbol: SymbolId,
    parent: SymbolId,
    table: &Table,
    scoping: &Scoping,
) -> bool {
    let (Statement::ExpressionStatement(_), Some(call)) = (statement, call_of(statement)) else {
        return false;
    };
    plain_arguments(call)
        && call.arguments.len() == 2
        && matches!(
            kind_of(&call.callee, table, scoping),
            Some(Kind::Inherits | Kind::InheritsOpen)
        )
        && argument(call, 0).and_then(|it| resolved(it, scoping)) == Some(symbol)
        && argument(call, 1).and_then(|it| resolved(it, scoping)) == Some(parent)
}

fn tail_of(
    statements: &[Statement<'_>],
    symbol: SymbolId,
    table: &Table,
    scoping: &Scoping,
) -> Option<Tail> {
    let last = statements.len().checked_sub(1)?;
    let ret = statements.get(last)?;
    if matches!(ret, Statement::ReturnStatement(_)) {
        if let Some(call) = call_of(ret) {
            define_call_of(call, Some(symbol), table, scoping)?;
            return Some(Tail {
                define: last,
                returns_class: false,
            });
        }
    }
    if !returns_symbol(ret, symbol, scoping) {
        return None;
    }
    let define = last.checked_sub(1)?;
    let call = call_of(statements.get(define)?)?;
    define_call_of(call, Some(symbol), table, scoping)?;
    matches!(statements.get(define)?, Statement::ExpressionStatement(_)).then_some(Tail {
        define,
        returns_class: true,
    })
}

fn only_use_strict(body: &FunctionBody<'_>) -> bool {
    body.directives
        .iter()
        .all(|directive| directive.directive.as_str() == "use strict")
}

fn iife_callee<'k, 'a>(call: &'k CallExpression<'a>) -> Option<&'k Function<'a>> {
    let Expression::FunctionExpression(function) = call.callee.without_parentheses() else {
        return None;
    };
    let simple = function.id.is_none()
        && !function.r#async
        && !function.generator
        && function.params.rest.is_none()
        && function.params.items.len() == call.arguments.len()
        && call.arguments.len() <= 1
        && plain_arguments(call);
    simple.then_some(function)
}

fn parent_symbol(function: &Function<'_>) -> Option<Option<SymbolId>> {
    match function.params.items.first() {
        None => Some(None),
        Some(param) => match &param.pattern {
            BindingPattern::BindingIdentifier(id) if param.initializer.is_none() => {
                Some(Some(id.symbol_id.get()?))
            }
            _ => None,
        },
    }
}

fn iife<'k, 'a>(
    call: &'k CallExpression<'a>,
    table: &Table,
    scoping: &Scoping,
) -> Option<Parts<'k, 'a>> {
    let function = iife_callee(call)?;
    let body = function.body.as_ref()?;
    if !only_use_strict(body) {
        return None;
    }
    let parent = parent_symbol(function)?;
    let statements = body.statements.as_slice();
    let (index, ctor) = statements
        .iter()
        .enumerate()
        .find_map(|(index, it)| match it {
            Statement::FunctionDeclaration(function) => Some((index, function.as_ref())),
            _ => None,
        })?;
    let symbol = function_symbol(ctor)?;
    let tail = tail_of(statements, symbol, table, scoping)?;
    let expected = 2 + usize::from(parent.is_some()) + usize::from(tail.returns_class);
    if statements.len() != expected {
        return None;
    }
    let inherits_ok = statements.iter().enumerate().all(|(at, statement)| {
        at == index
            || at >= tail.define
            || parent.is_some_and(|parent| is_inherits(statement, symbol, parent, table, scoping))
    });
    if !inherits_ok || index > tail.define {
        return None;
    }
    if let Some(parent) = parent {
        if scoping.get_resolved_reference_ids(parent).len() != 1 {
            return None;
        }
    }
    Some(Parts {
        form: Form::Iife,
        strict: function.has_use_strict_directive(),
        define_index: tail.define,
        ctor,
        symbol,
        define: call_of(statements.get(tail.define)?),
        parent,
    })
}

fn open(parts: &Parts<'_, '_>, table: &Table, scoping: &Scoping) -> bool {
    parts.define.is_some_and(|define| {
        kind_of(&define.callee, table, scoping) == Some(Kind::CreateClassOpen)
    })
}

struct Checked {
    form: Form,
    plan: CtorPlan,
}

pub enum Outcome {
    Fired(&'static str, usize),
    RefusedOpen,
    Refused,
}

pub fn context_strict(cx: &Cx<'_, '_>) -> bool {
    cx.tcx.ancestors().any(|ancestor| match ancestor {
        Ancestor::FunctionBodyStatements(body) => body
            .directives()
            .iter()
            .any(|directive| directive.directive.as_str() == "use strict"),
        Ancestor::ClassBodyBody(_) => true,
        _ => false,
    })
}

fn check(parts: &Parts<'_, '_>, strict: bool, table: &Table, cx: &Cx<'_, '_>) -> Option<Checked> {
    let scoping = cx.scoping();
    let ctor = parts.ctor;
    let strict = strict || parts.strict || context_strict(cx);
    let ctor_strict = strict || ctor.has_use_strict_directive();
    if !ctor_strict || ctor.r#async || ctor.generator || !constant_symbol(parts.symbol, scoping) {
        return None;
    }
    let guard = (parts.form == Form::Direct).then(|| class_members::NameGuard {
        name: ctor.id.as_ref().map_or("", |id| id.name.as_str()),
        symbol: parts.symbol,
    });
    let context = class_members::Context { guard, strict };
    let define = parts.define?;
    for (index, is_static) in [(1, false), (2, true)] {
        if let Some(Expression::ArrayExpression(array)) = argument(define, index) {
            if !class_members::admissible(array, is_static, &context, scoping) {
                return None;
            }
        }
    }
    let plan = class_ctor::plan(ctor, parts.symbol, parts.parent.is_some(), table, scoping)?;
    Some(Checked {
        form: parts.form,
        plan,
    })
}

fn shape(form: Form, derived: bool) -> &'static str {
    match (form, derived) {
        (Form::Iife, false) => "class/iife/base/unsafe",
        (Form::Iife, true) => "class/iife/derived/unsafe",
        (Form::Direct, _) => "class/direct/unsafe",
    }
}

#[inline]
pub fn may_be_class(node: &Expression<'_>) -> bool {
    let Expression::CallExpression(call) = node else {
        return false;
    };
    matches!(call.callee, Expression::FunctionExpression(_))
        || matches!(&call.callee, Expression::ParenthesizedExpression(inner)
            if matches!(inner.expression, Expression::FunctionExpression(_)))
        || leads_with_function(call)
}

pub fn rewrite<'a>(
    node: &mut Expression<'a>,
    strict: bool,
    table: &Table,
    gate: &Gate<'_>,
    prototypes: &Prototypes,
    cx: &mut Cx<'a, '_>,
) -> Option<Outcome> {
    let Expression::CallExpression(call) = &*node else {
        return None;
    };
    let parts = match call.callee.without_parentheses() {
        Expression::FunctionExpression(_) => iife(call, table, cx.scoping())?,
        Expression::Identifier(_) if leads_with_function(call) => {
            direct(call, table, cx.scoping())?
        }
        _ => return None,
    };
    if open(&parts, table, cx.scoping()) && !prototypes.closed(parts.symbol, table) {
        return Some(Outcome::RefusedOpen);
    }
    let Some(checked) = check(&parts, strict, table, cx) else {
        return Some(Outcome::Refused);
    };
    if !gate.allows(Safety::Unsafe) {
        return Some(Outcome::Refused);
    }
    let derived = parts.parent.is_some();
    let fields = checked.plan.field_count();
    let built = match checked.form {
        Form::Iife => take_iife(node, parts.define_index, cx),
        Form::Direct => take_direct(node, cx),
    };
    let Some(built) = built else {
        return Some(Outcome::Refused);
    };
    *node = assemble(built, checked.plan, table, cx);
    Some(Outcome::Fired(shape(checked.form, derived), fields))
}

struct Taken<'a> {
    span: Span,
    ctor: ABox<'a, Function<'a>>,
    protos: Option<ArrayExpression<'a>>,
    statics: Option<ArrayExpression<'a>>,
    parent: Option<Expression<'a>>,
}

fn take_array<'a>(argument: Option<Argument<'a>>) -> Option<ArrayExpression<'a>> {
    match argument.and_then(|argument| Expression::try_from(argument).ok()) {
        Some(expression) => match into_inner(expression) {
            Expression::ArrayExpression(array) => Some(array.unbox()),
            _ => None,
        },
        None => None,
    }
}

fn split_define<'a>(
    arguments: AVec<'a, Argument<'a>>,
) -> (
    Option<Argument<'a>>,
    Option<ArrayExpression<'a>>,
    Option<ArrayExpression<'a>>,
) {
    let mut arguments = arguments.into_iter();
    let first = arguments.next();
    let protos = take_array(arguments.next());
    let statics = take_array(arguments.next());
    (first, protos, statics)
}

fn take_call<'a>(
    node: &mut Expression<'a>,
    cx: &Cx<'a, '_>,
) -> Option<(Span, Expression<'a>, AVec<'a, Argument<'a>>)> {
    let taken = std::mem::replace(node, Expression::new_null_literal(SPAN, cx));
    match into_inner(taken) {
        Expression::CallExpression(call) => {
            let call = call.unbox();
            Some((call.span, call.callee, call.arguments))
        }
        _ => None,
    }
}

fn take_direct<'a>(node: &mut Expression<'a>, cx: &Cx<'a, '_>) -> Option<Taken<'a>> {
    let (span, _, arguments) = take_call(node, cx)?;
    let (first, protos, statics) = split_define(arguments);
    let expression = first.and_then(|argument| Expression::try_from(argument).ok())?;
    let Expression::FunctionExpression(ctor) = into_inner(expression) else {
        return None;
    };
    Some(Taken {
        span,
        ctor,
        protos,
        statics,
        parent: None,
    })
}

fn define_arguments(statement: Statement<'_>) -> Option<AVec<'_, Argument<'_>>> {
    let expression = match statement {
        Statement::ReturnStatement(statement) => statement.unbox().argument?,
        Statement::ExpressionStatement(statement) => statement.unbox().expression,
        _ => return None,
    };
    match into_inner(expression) {
        Expression::CallExpression(call) => Some(call.unbox().arguments),
        _ => None,
    }
}

fn take_iife<'a>(node: &mut Expression<'a>, define: usize, cx: &Cx<'a, '_>) -> Option<Taken<'a>> {
    let (span, callee, arguments) = take_call(node, cx)?;
    let parent = arguments
        .into_iter()
        .next()
        .and_then(|argument| Expression::try_from(argument).ok());
    let function = match into_inner(callee) {
        Expression::FunctionExpression(function) => function.unbox(),
        _ => return None,
    };
    let body = function.body?.unbox();
    let mut ctor = None;
    let mut members = None;
    for (index, statement) in body.statements.into_iter().enumerate() {
        match statement {
            Statement::FunctionDeclaration(function) => ctor = Some(function),
            statement if index == define => members = define_arguments(statement),
            _ => {}
        }
    }
    let (_, protos, statics) = split_define(members?);
    Some(Taken {
        span,
        ctor: ctor?,
        protos,
        statics,
        parent,
    })
}

fn assemble<'a>(
    mut taken: Taken<'a>,
    plan: CtorPlan,
    table: &Table,
    cx: &mut Cx<'a, '_>,
) -> Expression<'a> {
    let id = taken.ctor.id.take();
    let (constructor, fields) = class_ctor::rewrite(taken.ctor, plan, table, cx);
    let mut body = AVec::with_capacity_in(
        fields.len()
            + usize::from(constructor.is_some())
            + taken.protos.as_ref().map_or(0, |it| it.elements.len())
            + taken.statics.as_ref().map_or(0, |it| it.elements.len()),
        cx,
    );
    body.extend(fields);
    body.extend(constructor);
    if let Some(protos) = taken.protos.take() {
        class_members::build(protos, false, &mut body, cx);
    }
    if let Some(statics) = taken.statics.take() {
        class_members::build(statics, true, &mut body, cx);
    }
    let heritage = taken
        .parent
        .take()
        .map(|parent| ClassHeritage::new(parent, None, cx));
    let scope = cx
        .tcx
        .create_child_scope(cx.current_scope_id(), ScopeFlags::StrictMode);
    let body = ClassBody::boxed(SPAN, body, cx);
    Expression::new_class_expression_with_scope_id(
        taken.span,
        ClassType::ClassExpression,
        AVec::new_in(cx),
        id,
        None,
        heritage,
        AVec::new_in(cx),
        body,
        false,
        false,
        scope,
        cx,
    )
}
