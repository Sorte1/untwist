use oxc_allocator::{TakeIn, Vec as AVec};
use oxc_ast::ast::*;
use oxc_ast_visit::{VisitJs, VisitMut};
use oxc_semantic::Scoping;
use oxc_syntax::operator::AssignmentOperator;
use oxc_syntax::symbol::SymbolId;

use untwist_core::pass::Cx;

use super::async_fn::{binds_name, expected_length, Awaits, Scan};
use super::identify::{resolved, Table};
use super::shapes::Kind;

pub const SHAPE: &str = "async/babel-pair/unsafe";
pub const CLOSURE: &str = "async/babel-closure/unsafe";

const INNER_USES: usize = 3;

fn forwards(statement: &Statement<'_>, inner: SymbolId, scoping: &Scoping) -> bool {
    let Statement::ReturnStatement(it) = statement else {
        return false;
    };
    let Some(Expression::CallExpression(call)) =
        it.argument.as_ref().map(|e| e.without_parentheses())
    else {
        return false;
    };
    let Expression::StaticMemberExpression(member) = call.callee.without_parentheses() else {
        return false;
    };
    let plain_args = matches!(call.arguments.as_slice(),
        [Argument::ThisExpression(_), Argument::Identifier(args)]
            if args.name.as_str() == "arguments"
                && args.reference_id.get().is_some_and(|r| scoping.get_reference(r).symbol_id().is_none()));
    !call.optional
        && !member.optional
        && member.property.name.as_str() == "apply"
        && resolved(&member.object, scoping) == Some(inner)
        && plain_args
}

fn only_statements<'k, 'a>(function: &'k Function<'a>) -> Option<&'k [Statement<'a>]> {
    let body = function.body.as_ref()?;
    let plain = !function.generator
        && !function.r#async
        && body.directives.is_empty()
        && function.params.rest.is_none();
    plain.then_some(body.statements.as_slice())
}

fn outer_target(function: &Function<'_>, scoping: &Scoping) -> Option<SymbolId> {
    let [Statement::ReturnStatement(it)] = only_statements(function)? else {
        return None;
    };
    let Expression::CallExpression(call) = it.argument.as_ref()?.without_parentheses() else {
        return None;
    };
    let Expression::StaticMemberExpression(member) = call.callee.without_parentheses() else {
        return None;
    };
    let inner = resolved(&member.object, scoping)?;
    forwards(&function.body.as_ref()?.statements[0], inner, scoping).then_some(inner)
}

fn generator_of<'k, 'a>(
    function: &'k Function<'a>,
    own: SymbolId,
    table: &Table,
    scoping: &Scoping,
) -> Option<&'k Function<'a>> {
    let [Statement::ExpressionStatement(first), last] = only_statements(function)? else {
        return None;
    };
    if !function.params.items.is_empty() || !forwards(last, own, scoping) {
        return None;
    }
    let Expression::AssignmentExpression(assign) = first.expression.without_parentheses() else {
        return None;
    };
    let AssignmentTarget::AssignmentTargetIdentifier(target) = &assign.left else {
        return None;
    };
    let written = assign.operator == AssignmentOperator::Assign
        && target
            .reference_id
            .get()
            .and_then(|r| scoping.get_reference(r).symbol_id())
            == Some(own);
    let Expression::CallExpression(call) = assign.right.without_parentheses() else {
        return None;
    };
    let runner = written
        && !call.optional
        && table.kind_of_callee(&call.callee, scoping) == Some(Kind::AsyncToGenerator);
    let [Argument::FunctionExpression(generator)] = call.arguments.as_slice() else {
        return None;
    };
    (runner && generator.generator && !generator.r#async).then_some(generator)
}

fn generator_clean(generator: &Function<'_>, scoping: &Scoping) -> bool {
    let mut scan = Scan::default();
    scan.visit_formal_parameters(&generator.params);
    for statement in generator
        .body
        .iter()
        .flat_map(|body| body.statements.iter())
    {
        scan.visit_statement(statement);
    }
    let unnamed = generator.id.as_ref().is_none_or(|id| {
        id.symbol_id
            .get()
            .is_some_and(|symbol| scoping.get_resolved_reference_ids(symbol).is_empty())
    });
    let await_free = generator
        .scope_id
        .get()
        .is_some_and(|scope| !binds_name(scope, "await", scoping));
    !scan.bad
        && unnamed
        && await_free
        && generator
            .body
            .as_ref()
            .is_some_and(|b| b.directives.is_empty())
}

fn placeholders(outer: &Function<'_>, generator: &Function<'_>, scoping: &Scoping) -> bool {
    let unread = outer.params.items.iter().all(|param| match &param.pattern {
        BindingPattern::BindingIdentifier(id) => {
            param.initializer.is_none()
                && id
                    .symbol_id
                    .get()
                    .is_some_and(|symbol| scoping.get_resolved_reference_ids(symbol).is_empty())
        }
        _ => false,
    });
    unread && expected_length(&generator.params) == outer.params.items.len()
}

struct Pair {
    outer: usize,
    inner: usize,
}

fn declared(statement: &Statement<'_>) -> Option<SymbolId> {
    match statement {
        Statement::FunctionDeclaration(function) => function.id.as_ref()?.symbol_id.get(),
        _ => None,
    }
}

fn pair_at(
    statements: &[Statement<'_>],
    outer: usize,
    table: &Table,
    scoping: &Scoping,
) -> Option<Pair> {
    let Statement::FunctionDeclaration(function) = &statements[outer] else {
        return None;
    };
    let named_await = function.id.as_ref()?.name.as_str() == "await";
    let inner_symbol = outer_target(function, scoping)?;
    let inner = statements
        .iter()
        .position(|statement| declared(statement) == Some(inner_symbol))?;
    let Statement::FunctionDeclaration(holder) = &statements[inner] else {
        return None;
    };
    let generator = generator_of(holder, inner_symbol, table, scoping)?;
    let fits = !named_await
        && inner != outer
        && scoping.get_resolved_reference_ids(inner_symbol).len() == INNER_USES
        && scoping.symbol_redeclarations(inner_symbol).is_empty()
        && placeholders(function, generator, scoping)
        && generator_clean(generator, scoping);
    fits.then_some(Pair { outer, inner })
}

fn take_generator<'a>(holder: &mut Function<'a>, cx: &Cx<'a, '_>) -> Option<Function<'a>> {
    let body = holder.body.as_mut()?;
    let Statement::ExpressionStatement(first) = body.statements.first_mut()? else {
        return None;
    };
    let Expression::AssignmentExpression(assign) = &mut first.expression else {
        return None;
    };
    let Expression::CallExpression(call) = &mut assign.right else {
        return None;
    };
    let Some(Argument::FunctionExpression(generator)) = call.arguments.first_mut() else {
        return None;
    };
    Some(generator.as_mut().take_in(cx))
}

fn rewrite<'a>(
    statements: &mut AVec<'a, Statement<'a>>,
    pair: &Pair,
    cx: &mut Cx<'a, '_>,
) -> Option<()> {
    let Statement::FunctionDeclaration(holder) = &mut statements[pair.inner] else {
        return None;
    };
    let mut generator = take_generator(holder, cx)?;
    let mut body = generator.body.take()?;
    let mut awaits = Awaits { functions: 0, cx };
    for statement in body.statements.iter_mut() {
        awaits.visit_statement(statement);
    }
    let Statement::FunctionDeclaration(function) = &mut statements[pair.outer] else {
        return None;
    };
    function.params = generator.params.take_in_box(cx);
    function.body = Some(body);
    function.r#async = true;
    Some(())
}

pub fn has_site(statements: &[Statement<'_>], table: &Table, scoping: &Scoping) -> bool {
    (0..statements.len()).any(|at| pair_at(statements, at, table, scoping).is_some())
}

pub fn rewrite_statements<'a>(
    statements: &mut AVec<'a, Statement<'a>>,
    table: &Table,
    cx: &mut Cx<'a, '_>,
) -> usize {
    let pairs: Vec<Pair> = (0..statements.len())
        .filter_map(|at| pair_at(statements, at, table, cx.scoping()))
        .collect();
    let mut dropped = Vec::new();
    for pair in &pairs {
        if rewrite(statements, pair, cx).is_some() {
            dropped.push(pair.inner);
        }
    }
    if dropped.is_empty() {
        return 0;
    }
    let mut at = 0;
    statements.retain(|_| {
        let keep = !dropped.contains(&at);
        at += 1;
        keep
    });
    cx.binding_moved();
    dropped.len()
}

struct Closure;

fn closure_of(call: &CallExpression<'_>, table: &Table, scoping: &Scoping) -> Option<Closure> {
    let Expression::FunctionExpression(wrapper) = call.callee.without_parentheses() else {
        return None;
    };
    if !call.arguments.is_empty() || call.optional || !wrapper.params.items.is_empty() {
        return None;
    }
    let [Statement::VariableDeclaration(declaration), Statement::ReturnStatement(returned)] =
        only_statements(wrapper)?
    else {
        return None;
    };
    let [declarator] = declaration.declarations.as_slice() else {
        return None;
    };
    let BindingPattern::BindingIdentifier(id) = &declarator.id else {
        return None;
    };
    let runner = id.symbol_id.get()?;
    let Expression::CallExpression(made) = declarator.init.as_ref()?.without_parentheses() else {
        return None;
    };
    let [Argument::FunctionExpression(generator)] = made.arguments.as_slice() else {
        return None;
    };
    let Expression::FunctionExpression(inner) = returned.argument.as_ref()?.without_parentheses()
    else {
        return None;
    };
    let [forwarding] = only_statements(inner)? else {
        return None;
    };
    let name = inner.id.as_ref()?.name.as_str();
    let fits = !made.optional
        && table.kind_of_callee(&made.callee, scoping) == Some(Kind::AsyncToGenerator)
        && generator.generator
        && !generator.r#async
        && name != "await"
        && forwards(forwarding, runner, scoping)
        && scoping.get_resolved_reference_ids(runner).len() == 1
        && placeholders(inner, generator, scoping)
        && generator_clean(generator, scoping)
        && !mentions_name(generator, name);
    fits.then_some(Closure)
}

struct Mentions<'n> {
    name: &'n str,
    hit: bool,
}

impl<'a> VisitJs<'a> for Mentions<'_> {
    fn visit_identifier_reference(&mut self, it: &IdentifierReference<'a>) {
        self.hit |= it.name.as_str() == self.name;
    }
}

fn mentions_name(generator: &Function<'_>, name: &str) -> bool {
    let mut scan = Mentions { name, hit: false };
    scan.visit_formal_parameters(&generator.params);
    for statement in generator
        .body
        .iter()
        .flat_map(|body| body.statements.iter())
    {
        scan.visit_statement(statement);
    }
    scan.hit
}

pub fn rewrite_closure<'a>(
    expression: &mut Expression<'a>,
    table: &Table,
    cx: &mut Cx<'a, '_>,
) -> bool {
    let Expression::CallExpression(call) = expression else {
        return false;
    };
    if closure_of(call, table, cx.scoping()).is_none() {
        return false;
    }
    let Expression::FunctionExpression(wrapper) = call.callee.without_parentheses_mut() else {
        return false;
    };
    let Some(body) = wrapper.body.as_mut() else {
        return false;
    };
    let Some(Statement::ReturnStatement(returned)) = body.statements.pop() else {
        return false;
    };
    let Some(Statement::VariableDeclaration(mut declaration)) = body.statements.pop() else {
        return false;
    };
    let Some(Expression::CallExpression(mut made)) = declaration.declarations[0].init.take() else {
        return false;
    };
    let Some(Argument::FunctionExpression(mut generator)) = made.arguments.pop() else {
        return false;
    };
    let Some(Expression::FunctionExpression(mut inner)) = returned.unbox().argument else {
        return false;
    };
    let Some(mut moved) = generator.body.take() else {
        return false;
    };
    let mut awaits = Awaits { functions: 0, cx };
    for statement in moved.statements.iter_mut() {
        awaits.visit_statement(statement);
    }
    inner.params = generator.params.take_in_box(cx);
    inner.body = Some(moved);
    inner.r#async = true;
    *expression = Expression::FunctionExpression(inner);
    cx.binding_moved();
    true
}
