use oxc_allocator::{CloneIn, GetAllocator, Vec as AVec};
use oxc_ast::ast::*;
use oxc_ast_visit::VisitJs;
use oxc_semantic::Scoping;
use oxc_syntax::scope::ScopeId;
use oxc_syntax::symbol::SymbolId;
use oxc_traverse::{Ancestor, BoundIdentifier};

use untwist_core::pass::Cx;

use super::identify::Table;
use super::machine::{slot_at, statement_at, Machine, Received, Slot, Term};
use super::machine_check::{Reads, Uses};
use super::machine_emit::Emitter;
use super::machine_tslib::{self, Sent, Tslib};
use super::shapes::Kind;
use super::structure;
use super::structure_tidy::tidy;

pub const SHAPE: &str = "generator/tslib/unsafe";

pub fn plain_kind(cx: &Cx<'_, '_>) -> bool {
    match cx.tcx.parent() {
        Ancestor::MethodDefinitionValue(method) => *method.kind() == MethodDefinitionKind::Method,
        Ancestor::ObjectPropertyValue(property) => *property.kind() == PropertyKind::Init,
        _ => true,
    }
}

pub fn eager_free(statement: &Statement<'_>) -> bool {
    let Statement::VariableDeclaration(declaration) = statement else {
        return false;
    };
    declaration.kind == VariableDeclarationKind::Var
        && declaration.declarations.iter().all(|declarator| {
            matches!(declarator.id, BindingPattern::BindingIdentifier(_))
                && match declarator
                    .init
                    .as_ref()
                    .map(Expression::without_parentheses)
                {
                    None | Some(Expression::ThisExpression(_)) => true,
                    Some(Expression::Identifier(id)) => id.name.as_str() == "arguments",
                    Some(_) => false,
                }
        })
}

fn aliases_arguments(statement: &Statement<'_>) -> bool {
    let Statement::VariableDeclaration(declaration) = statement else {
        return false;
    };
    declaration.declarations.iter().any(|declarator| {
        matches!(declarator.init.as_ref().map(Expression::without_parentheses),
            Some(Expression::Identifier(id)) if id.name.as_str() == "arguments")
    })
}

pub fn returned_call<'k, 'a>(function: &'k Function<'a>) -> Option<&'k CallExpression<'a>> {
    let body = function.body.as_ref()?;
    let (last, prefix) = body.statements.split_last()?;
    let Statement::ReturnStatement(it) = last else {
        return None;
    };
    if !prefix.iter().all(eager_free) {
        return None;
    }
    match it.argument.as_ref()?.without_parentheses() {
        Expression::CallExpression(call) if !call.optional => Some(call),
        _ => None,
    }
}

pub fn simple_param(params: &FormalParameters<'_>) -> Option<SymbolId> {
    match params.items.as_slice() {
        [FormalParameter {
            pattern: BindingPattern::BindingIdentifier(id),
            initializer: None,
            ..
        }] if params.rest.is_none() => id.symbol_id.get(),
        _ => None,
    }
}

struct Body<'k, 'a> {
    state: SymbolId,
    scope: ScopeId,
    statements: &'k [Statement<'a>],
}

fn body_of<'k, 'a>(expression: &'k Expression<'a>) -> Option<Body<'k, 'a>> {
    match expression.without_parentheses() {
        Expression::FunctionExpression(function)
            if !function.generator && !function.r#async && function.id.is_none() =>
        {
            let body = function.body.as_ref()?;
            body.directives.is_empty().then_some(())?;
            Some(Body {
                state: simple_param(&function.params)?,
                scope: function.scope_id.get()?,
                statements: &body.statements,
            })
        }
        Expression::ArrowFunctionExpression(arrow) if !arrow.r#async => {
            let ArrowFunctionBody::FunctionBody(body) = &arrow.body else {
                return None;
            };
            body.directives.is_empty().then_some(())?;
            Some(Body {
                state: simple_param(&arrow.params)?,
                scope: arrow.scope_id.get()?,
                statements: &body.statements,
            })
        }
        _ => None,
    }
}

fn generator_body<'k, 'a>(
    call: &'k CallExpression<'a>,
    table: &Table,
    scoping: &Scoping,
) -> Option<Body<'k, 'a>> {
    if table.kind_of_callee(&call.callee, scoping) != Some(Kind::Generator) {
        return None;
    }
    let [Argument::ThisExpression(_), body] = call.arguments.as_slice() else {
        return None;
    };
    body_of(body.as_expression()?)
}

fn scopes_free(body: &Body<'_, '_>, scoping: &Scoping) -> bool {
    let own = scoping.get_bindings(body.scope);
    let only_state = own.len() == 1 && own.values().all(|symbol| *symbol == body.state);
    let switch_free = match body.statements {
        [Statement::SwitchStatement(switch)] => switch
            .scope_id
            .get()
            .is_some_and(|scope| scoping.get_bindings(scope).is_empty()),
        _ => true,
    };
    only_state && switch_free
}

pub fn is_site(function: &Function<'_>, table: &Table, scoping: &Scoping) -> bool {
    !function.generator
        && !function.r#async
        && returned_call(function).is_some_and(|call| {
            table.kind_of_callee(&call.callee, scoping) == Some(Kind::Generator)
        })
}

fn copy_cases<'a>(
    call: &CallExpression<'a>,
    cx: &Cx<'a, '_>,
) -> Option<Vec<AVec<'a, Statement<'a>>>> {
    let allocator = GetAllocator::allocator(cx);
    let statements = match call
        .arguments
        .get(1)?
        .as_expression()?
        .without_parentheses()
    {
        Expression::FunctionExpression(function) => &function.body.as_ref()?.statements,
        Expression::ArrowFunctionExpression(arrow) => match &arrow.body {
            ArrowFunctionBody::FunctionBody(body) => &body.statements,
            _ => return None,
        },
        _ => return None,
    };
    let copy = |list: &AVec<'a, Statement<'a>>| list.clone_in_with_semantic_ids(allocator);
    if let [Statement::SwitchStatement(switch)] = statements.as_slice() {
        return Some(
            switch
                .cases
                .iter()
                .map(|case| copy(&case.consequent))
                .collect(),
        );
    }
    Some(vec![copy(statements)])
}

fn term_slot(term: &Term) -> Option<Slot> {
    match *term {
        Term::Branch { test, .. } => Some(test),
        Term::Return(slot) => slot,
        Term::Yield { value, .. } => value,
        _ => None,
    }
}

fn caught_symbol(
    statement: &Statement<'_>,
    reads: &dyn Reads,
    scoping: &Scoping,
) -> Option<SymbolId> {
    let Statement::ExpressionStatement(it) = statement else {
        return None;
    };
    let Expression::AssignmentExpression(assignment) = &it.expression else {
        return None;
    };
    let AssignmentTarget::AssignmentTargetIdentifier(id) = &assignment.left else {
        return None;
    };
    let plain = assignment.operator == oxc_syntax::operator::AssignmentOperator::Assign
        && reads.is_read(&assignment.right);
    plain.then(|| scoping.get_reference(id.reference_id.get()?).symbol_id())?
}

fn declared_bare<'a>(prefix: &[Statement<'a>], symbol: SymbolId) -> Option<&'a str> {
    prefix.iter().find_map(|statement| {
        let Statement::VariableDeclaration(declaration) = statement else {
            return None;
        };
        declaration
            .declarations
            .iter()
            .find_map(|declarator| match &declarator.id {
                BindingPattern::BindingIdentifier(id)
                    if declarator.init.is_none() && id.symbol_id.get() == Some(symbol) =>
                {
                    Some(id.name.as_str())
                }
                _ => None,
            })
    })
}

fn confined(
    machine: &Machine,
    cases: &[&[Statement<'_>]],
    range: (u32, u32),
    count: &mut Uses<'_>,
) -> Option<()> {
    for node in machine
        .nodes
        .iter()
        .filter(|node| range.0 <= node.label && node.label < range.1)
    {
        for loc in &node.body {
            count.visit_statement(statement_at(cases, loc)?);
        }
        if let Some(slot) = term_slot(&node.term) {
            count.visit_expression(slot_at(cases, &slot)?);
        }
    }
    Some(())
}

fn rebinds<'a>(
    machine: &Machine,
    cases: &[&[Statement<'a>]],
    prefix: &[Statement<'a>],
    reads: &dyn Reads,
    scoping: &Scoping,
) -> Vec<(u32, SymbolId, &'a str)> {
    let mut out = Vec::new();
    for (index, region) in machine.trys.iter().enumerate() {
        let found = (|| {
            let catch = region.catch?;
            let entry = machine.node(machine.entry(catch)?)?;
            if entry.received != Received::Statement {
                return None;
            }
            let symbol = caught_symbol(statement_at(cases, entry.body.first()?)?, reads, scoping)?;
            let name = declared_bare(prefix, symbol)?;
            let mut count = Uses::new(scoping, symbol);
            confined(machine, cases, (catch, region.catch_end()), &mut count)?;
            let total = scoping.get_resolved_reference_ids(symbol).len();
            (count.inside == total && !count.closed_over).then_some((
                u32::try_from(index).ok()?,
                symbol,
                name,
            ))
        })();
        out.extend(found);
    }
    out
}

fn drop_declarators(statements: &mut AVec<'_, Statement<'_>>, symbols: &[SymbolId]) {
    for statement in statements.iter_mut() {
        if let Statement::VariableDeclaration(declaration) = statement {
            declaration.declarations.retain(|declarator| {
                !matches!(&declarator.id, BindingPattern::BindingIdentifier(id)
                    if id.symbol_id.get().is_some_and(|symbol| symbols.contains(&symbol)))
            });
        }
    }
    statements.retain(|statement| {
        !matches!(statement, Statement::VariableDeclaration(declaration) if declaration.declarations.is_empty())
    });
}

fn unbind(generated: &[BoundIdentifier<'_>], cx: &mut Cx<'_, '_>) {
    for bound in generated {
        let scope = cx.scoping().symbol_scope_id(bound.symbol_id);
        cx.scoping_mut().remove_binding(scope, bound.name);
    }
}

fn drop_final_return(statements: &mut AVec<'_, Statement<'_>>) {
    if matches!(statements.last(), Some(Statement::ReturnStatement(it)) if it.argument.is_none()) {
        statements.pop();
    }
}

pub fn rewrite<'a>(
    function: &mut Function<'a>,
    table: &Table,
    cx: &mut Cx<'a, '_>,
) -> Option<&'static str> {
    if function.generator || function.r#async || !plain_kind(cx) {
        return None;
    }
    let scope = function.scope_id.get()?;
    let scoping = cx.scoping();
    let named_yield = function
        .id
        .as_ref()
        .is_some_and(|id| id.name.as_str() == "yield");
    if named_yield || super::async_fn::binds_name(scope, "yield", scoping) {
        return None;
    }
    let call = returned_call(function)?;
    let body = generator_body(call, table, scoping)?;
    if !scopes_free(&body, scoping) {
        return None;
    }
    let values =
        |callee: &Expression<'_>| table.kind_of_callee(callee, scoping) == Some(Kind::Values);
    let prefix = function.body.as_ref()?.statements.split_last()?.1;
    let tslib = Tslib {
        sent: Sent {
            scoping,
            state: body.state,
        },
        home: (!prefix.iter().any(aliases_arguments)).then_some(scope),
        values: &values,
    };
    let loaded = machine_tslib::read(body.statements, &tslib)?;
    let mut structured = structure::structure(&loaded.machine)?;
    tidy(&mut structured.code, &loaded.machine);
    let reads = machine_tslib::SentReads::of(scoping, body.state);
    let rebound = rebinds(&loaded.machine, &loaded.cases, prefix, &reads, scoping);
    let labels: Vec<&'a str> = loaded.labels;
    let machine = loaded.machine;
    let cases = copy_cases(returned_call(function)?, cx)?;
    let mut emitter = Emitter::new(cases, (&machine, &structured.yields), &reads, scope, cx);
    for (region, symbol, name) in &rebound {
        emitter
            .rebound
            .insert(*region, BoundIdentifier::new((*name).into(), *symbol));
    }
    emitter.name_labels(&structured.code, &labels);
    let mut statements = emitter.sequence(std::mem::take(&mut structured.code));
    if emitter.failed() {
        let generated: Vec<BoundIdentifier<'a>> = emitter.generated().cloned().collect();
        unbind(&generated, cx);
        return None;
    }
    drop_final_return(&mut statements);
    let body = function.body.as_mut()?;
    body.statements.pop();
    let symbols: Vec<SymbolId> = rebound.iter().map(|(_, symbol, _)| *symbol).collect();
    drop_declarators(&mut body.statements, &symbols);
    body.statements.extend(statements.drain(..));
    function.generator = true;
    Some(SHAPE)
}
