use oxc_allocator::{TakeIn, Vec as AVec};
use oxc_ast::ast::*;
use oxc_ast_visit::{walk_js, walk_mut, VisitJs, VisitMut};
use oxc_semantic::Scoping;
use oxc_span::SPAN;
use oxc_str::Ident;
use oxc_syntax::operator::UnaryOperator;
use oxc_syntax::scope::{ScopeFlags, ScopeId};
use oxc_syntax::symbol::SymbolFlags;

use untwist_core::ast_util::frame::{self, Frame, FrameScan};
use untwist_core::pass::Cx;

use super::generator::{plain_kind, returned_call};
use super::identify::Table;
use super::shapes::Kind;

pub const AWAITER: &str = "async/awaiter/unsafe";
pub const ESBUILD: &str = "async/esbuild/unsafe";
pub const BABEL: &str = "async/babel/unsafe";

#[derive(Default)]
pub(super) struct Scan {
    frame: Frame,
    pub(super) bad: bool,
    this: bool,
    arguments: bool,
}

impl Scan {
    fn top(&self) -> bool {
        self.frame.is_own()
    }
}

impl<'a> FrameScan<'a> for Scan {
    fn frame(&mut self) -> &mut Frame {
        &mut self.frame
    }
}

impl<'a> VisitJs<'a> for Scan {
    fn visit_function(&mut self, it: &Function<'a>, flags: ScopeFlags) {
        frame::walk_function(self, it, flags);
    }

    fn visit_arrow_function_expression(&mut self, it: &ArrowFunctionExpression<'a>) {
        frame::walk_arrow(self, it);
    }

    fn visit_identifier_reference(&mut self, it: &IdentifierReference<'a>) {
        let name = it.name.as_str();
        self.bad |= name == "await";
        self.arguments |= name == "arguments" && self.top();
    }

    fn visit_this_expression(&mut self, _it: &ThisExpression) {
        self.this |= self.top();
    }

    fn visit_binding_identifier(&mut self, it: &BindingIdentifier<'a>) {
        self.bad |= it.name.as_str() == "await";
    }

    fn visit_label_identifier(&mut self, it: &LabelIdentifier<'a>) {
        self.bad |= it.name.as_str() == "await";
    }

    fn visit_new_target(&mut self, _it: &NewTarget) {
        self.bad |= self.top();
    }

    fn visit_yield_expression(&mut self, it: &YieldExpression<'a>) {
        self.bad |= it.delegate && self.frame.is_top();
        walk_js::walk_yield_expression(self, it);
    }

    fn visit_for_of_statement(&mut self, it: &ForOfStatement<'a>) {
        self.bad |= it.r#await && self.top();
        walk_js::walk_for_of_statement(self, it);
    }
}

pub(super) struct Awaits<'c, 'a, 'x> {
    pub(super) functions: u32,
    pub(super) cx: &'x Cx<'a, 'c>,
}

impl<'a> VisitMut<'a> for Awaits<'_, 'a, '_> {
    fn visit_function(&mut self, it: &mut Function<'a>, flags: ScopeFlags) {
        self.functions += 1;
        walk_mut::walk_function(self, it, flags);
        self.functions -= 1;
    }

    fn visit_expression(&mut self, it: &mut Expression<'a>) {
        walk_mut::walk_expression(self, it);
        if self.functions > 0 {
            return;
        }
        let Expression::YieldExpression(yielded) = it else {
            return;
        };
        let argument = yielded.argument.take().unwrap_or_else(|| {
            let zero =
                Expression::new_numeric_literal(SPAN, 0.0, None, NumberBase::Decimal, self.cx);
            Expression::new_unary_expression(SPAN, UnaryOperator::Void, zero, self.cx)
        });
        *it = Expression::new_await_expression(yielded.span, argument, self.cx);
    }
}

fn is_undefined(expression: &Expression<'_>, scoping: &Scoping) -> bool {
    match expression.without_parentheses() {
        Expression::UnaryExpression(it) => {
            it.operator == UnaryOperator::Void
                && matches!(
                    it.argument.without_parentheses(),
                    Expression::NumericLiteral(_)
                )
        }
        Expression::Identifier(id) => {
            id.name.as_str() == "undefined"
                && id
                    .reference_id
                    .get()
                    .is_some_and(|reference| scoping.get_reference(reference).symbol_id().is_none())
        }
        _ => false,
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Receiver {
    This,
    Inert,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Args {
    Empty,
    Forwarded,
}

struct Runner<'k, 'a> {
    shape: &'static str,
    receiver: Receiver,
    args: Args,
    generator: &'k Function<'a>,
}

fn receiver(expression: &Expression<'_>, scoping: &Scoping) -> Option<Receiver> {
    match expression.without_parentheses() {
        Expression::ThisExpression(_) => Some(Receiver::This),
        Expression::NullLiteral(_) => Some(Receiver::Inert),
        Expression::Identifier(id) if !is_undefined(expression, scoping) => {
            let symbol = scoping.get_reference(id.reference_id.get()?).symbol_id()?;
            scoping
                .symbol_flags(symbol)
                .contains(SymbolFlags::FunctionScopedVariable)
                .then_some(Receiver::Inert)
        }
        other => is_undefined(other, scoping).then_some(Receiver::Inert),
    }
}

fn args(expression: &Expression<'_>, scoping: &Scoping) -> Option<Args> {
    match expression.without_parentheses() {
        Expression::NullLiteral(_) => Some(Args::Empty),
        Expression::Identifier(id) if id.name.as_str() == "arguments" => {
            let reference = id.reference_id.get()?;
            scoping
                .get_reference(reference)
                .symbol_id()
                .is_none()
                .then_some(Args::Forwarded)
        }
        other => is_undefined(other, scoping).then_some(Args::Empty),
    }
}

fn runner<'k, 'a>(
    call: &'k CallExpression<'a>,
    table: &Table,
    scoping: &Scoping,
) -> Option<Runner<'k, 'a>> {
    if let Some(found) = babel_runner(call, table, scoping) {
        return Some(found);
    }
    let arguments: Vec<&Expression<'a>> = call
        .arguments
        .iter()
        .map(Argument::as_expression)
        .collect::<Option<_>>()?;
    let (shape, this, forwarded, generator) = match (
        table.kind_of_callee(&call.callee, scoping)?,
        arguments.as_slice(),
    ) {
        (Kind::Awaiter, [this, forwarded, promise, generator])
            if is_undefined(promise, scoping) =>
        {
            (AWAITER, *this, *forwarded, *generator)
        }
        (Kind::Async, [this, forwarded, generator]) => (ESBUILD, *this, *forwarded, *generator),
        _ => return None,
    };
    let Expression::FunctionExpression(generator) = generator.without_parentheses() else {
        return None;
    };
    let args = args(forwarded, scoping)?;
    let fits = generator.generator
        && !generator.r#async
        && generator.id.is_none()
        && (args == Args::Forwarded
            || generator.params.items.is_empty() && generator.params.rest.is_none())
        && generator
            .body
            .as_ref()
            .is_some_and(|body| body.directives.is_empty());
    fits.then_some(Runner {
        shape,
        receiver: receiver(this, scoping)?,
        args,
        generator,
    })
}

fn babel_runner<'k, 'a>(
    call: &'k CallExpression<'a>,
    table: &Table,
    scoping: &Scoping,
) -> Option<Runner<'k, 'a>> {
    let Expression::CallExpression(inner) = call.callee.without_parentheses() else {
        return None;
    };
    if !call.arguments.is_empty()
        || call.optional
        || inner.optional
        || table.kind_of_callee(&inner.callee, scoping) != Some(Kind::AsyncToGenerator)
    {
        return None;
    }
    let [Argument::FunctionExpression(generator)] = inner.arguments.as_slice() else {
        return None;
    };
    let unnamed = generator.id.as_ref().is_none_or(|id| {
        id.symbol_id
            .get()
            .is_some_and(|symbol| scoping.get_resolved_reference_ids(symbol).is_empty())
    });
    let fits = generator.generator
        && !generator.r#async
        && unnamed
        && generator.params.items.is_empty()
        && generator.params.rest.is_none()
        && generator
            .body
            .as_ref()
            .is_some_and(|body| body.directives.is_empty());
    fits.then_some(Runner {
        shape: BABEL,
        receiver: Receiver::Inert,
        args: Args::Empty,
        generator,
    })
}

fn simple_params(params: &FormalParameters<'_>) -> bool {
    params.items.iter().all(|param| {
        param.initializer.is_none() && matches!(param.pattern, BindingPattern::BindingIdentifier(_))
    }) && params
        .rest
        .as_ref()
        .is_none_or(|rest| matches!(rest.rest.argument, BindingPattern::BindingIdentifier(_)))
}

fn names_disjoint(inner: ScopeId, outer: ScopeId, scoping: &Scoping) -> bool {
    let outer = scoping.get_bindings(outer);
    scoping
        .get_bindings(inner)
        .keys()
        .all(|name| !outer.contains_key(name))
}

pub(super) fn binds_name(scope: ScopeId, name: &str, scoping: &Scoping) -> bool {
    scoping.get_binding(scope, Ident::from(name)).is_some()
}

fn generator_clean(found: &Runner<'_, '_>, outer: ScopeId, scoping: &Scoping) -> bool {
    let generator = found.generator;
    let Some(inner) = generator.scope_id.get() else {
        return false;
    };
    let mut scan = Scan::default();
    scan.visit_formal_parameters(&generator.params);
    if let Some(body) = &generator.body {
        for statement in &body.statements {
            scan.visit_statement(statement);
        }
    }
    let receiver_ok = found.receiver == Receiver::This || !scan.this;
    let arguments_ok = found.args == Args::Forwarded || !scan.arguments;
    !scan.bad && receiver_ok && arguments_ok && names_disjoint(inner, outer, scoping)
}

pub(super) fn expected_length(params: &FormalParameters<'_>) -> usize {
    params
        .items
        .iter()
        .take_while(|param| param.initializer.is_none())
        .count()
}

fn forwarding_fits(
    outer: &FormalParameters<'_>,
    generator: &Function<'_>,
    scoping: &Scoping,
) -> bool {
    let unread = outer.items.iter().all(|param| match &param.pattern {
        BindingPattern::BindingIdentifier(id) => id
            .symbol_id
            .get()
            .is_some_and(|symbol| scoping.get_resolved_reference_ids(symbol).is_empty()),
        _ => false,
    });
    unread && outer.rest.is_none() && expected_length(&generator.params) == outer.items.len()
}

type Taken<'a> = (
    AVec<'a, Statement<'a>>,
    oxc_allocator::Box<'a, FormalParameters<'a>>,
);

fn take_generator<'a>(
    statements: &mut AVec<'a, Statement<'a>>,
    cx: &Cx<'a, '_>,
) -> Option<Taken<'a>> {
    let Statement::ReturnStatement(it) = statements.last_mut()? else {
        return None;
    };
    take_from_call(it.argument.as_mut()?, cx)
}

fn take_from_call<'a>(expression: &mut Expression<'a>, cx: &Cx<'a, '_>) -> Option<Taken<'a>> {
    let Expression::CallExpression(outer) = expression.without_parentheses_mut() else {
        return None;
    };
    let call = if outer.arguments.is_empty() {
        let Expression::CallExpression(inner) = outer.callee.without_parentheses_mut() else {
            return None;
        };
        inner
    } else {
        outer
    };
    let Expression::FunctionExpression(generator) =
        call.arguments.last_mut()?.as_expression_mut()?
    else {
        return None;
    };
    let mut statements = generator.body.as_mut()?.statements.take_in(cx);
    let mut awaits = Awaits { functions: 0, cx };
    for statement in statements.iter_mut() {
        awaits.visit_statement(statement);
    }
    Some((statements, generator.params.take_in_box(cx)))
}

pub fn is_site(function: &Function<'_>, table: &Table, scoping: &Scoping) -> bool {
    !function.generator
        && !function.r#async
        && returned_call(function).is_some_and(|call| {
            matches!(
                table.kind_of_callee(&call.callee, scoping),
                Some(Kind::Awaiter | Kind::Async)
            ) || matches!(call.callee.without_parentheses(), Expression::CallExpression(inner)
                if table.kind_of_callee(&inner.callee, scoping) == Some(Kind::AsyncToGenerator))
        })
}

pub fn rewrite_function<'a>(
    function: &mut Function<'a>,
    table: &Table,
    cx: &mut Cx<'a, '_>,
) -> Option<&'static str> {
    if function.generator || function.r#async || !plain_kind(cx) || !simple_params(&function.params)
    {
        return None;
    }
    let outer = function.scope_id.get()?;
    let scoping = cx.scoping();
    let named_await = function
        .id
        .as_ref()
        .is_some_and(|id| id.name.as_str() == "await");
    if named_await || binds_name(outer, "await", scoping) {
        return None;
    }
    let found = runner(returned_call(function)?, table, scoping)?;
    let forwarded = found.args == Args::Forwarded;
    if !generator_clean(&found, outer, scoping)
        || forwarded && !forwarding_fits(&function.params, found.generator, scoping)
    {
        return None;
    }
    let shape = found.shape;
    let body = function.body.as_mut()?;
    let (moved, params) = take_generator(&mut body.statements, cx)?;
    body.statements.pop();
    body.statements.extend(moved);
    if forwarded {
        function.params = params;
    }
    function.r#async = true;
    Some(shape)
}

fn arrow_call<'k, 'a>(arrow: &'k ArrowFunctionExpression<'a>) -> Option<&'k CallExpression<'a>> {
    let expression = match &arrow.body {
        ArrowFunctionBody::FunctionBody(body) => match body.statements.as_slice() {
            [Statement::ReturnStatement(it)] if body.directives.is_empty() => {
                it.argument.as_ref()?
            }
            _ => return None,
        },
        body => body.as_expression()?,
    };
    match expression.without_parentheses() {
        Expression::CallExpression(call) if !call.optional => Some(call),
        _ => None,
    }
}

pub fn rewrite_arrow<'a>(
    arrow: &mut ArrowFunctionExpression<'a>,
    table: &Table,
    cx: &mut Cx<'a, '_>,
) -> Option<&'static str> {
    if arrow.r#async || !simple_params(&arrow.params) {
        return None;
    }
    let outer = arrow.scope_id.get()?;
    let scoping = cx.scoping();
    if binds_name(outer, "await", scoping) {
        return None;
    }
    let found = runner(arrow_call(arrow)?, table, scoping)?;
    if found.args == Args::Forwarded || !generator_clean(&found, outer, scoping) {
        return None;
    }
    let shape = found.shape;
    let (moved, _) = match &mut arrow.body {
        ArrowFunctionBody::FunctionBody(body) => take_generator(&mut body.statements, cx)?,
        body => take_from_call(body.as_expression_mut()?, cx)?,
    };
    let body = FunctionBody::boxed(SPAN, AVec::new_in(cx), moved, &*cx);
    arrow.body = ArrowFunctionBody::FunctionBody(body);
    arrow.r#async = true;
    Some(shape)
}
