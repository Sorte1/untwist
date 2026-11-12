use oxc_allocator::{Allocator, CloneIn};
use oxc_ast::ast::*;
use oxc_ast::builder::AstBuilder;
use oxc_span::{GetSpan, Span};
use rustc_hash::FxHashSet;

use crate::model::{body_key, Bodies, Helper, Runtime};
use crate::webpack::guard::{dirname_helper, dirname_movable, dirname_suffix, guarded, DIRNAME};
use crate::webpack::locals::{
    closed_value, expression_names, local_unit, mentions, scoped, statement_names, writes,
    FreeNames,
};

type Wrapped<'k, 'a> = (
    &'k Statement<'a>,
    &'k FunctionBody<'a>,
    Option<(String, bool)>,
);

const HOST_LOCALS: [&str; 5] = ["__dirname", "__filename", "module", "exports", "require"];

struct Prelude<'k> {
    require_name: &'k str,
    declared: FxHashSet<&'k str>,
    skip: Option<Span>,
}

pub(crate) fn pattern_names<'k, E: Extend<&'k str>>(
    pattern: &'k BindingPattern<'_>,
    names: &mut E,
) {
    names.extend(
        pattern
            .get_binding_identifiers()
            .into_iter()
            .map(|id| id.name.as_str()),
    );
}

pub(crate) fn declared_in<'k, E: Extend<&'k str>>(statements: &'k [Statement<'_>], names: &mut E) {
    for statement in statements {
        declared_at(statement, names);
    }
}

fn declared_at<'k, E: Extend<&'k str>>(statement: &'k Statement<'_>, names: &mut E) {
    match statement {
        Statement::VariableDeclaration(declaration) => {
            for declarator in &declaration.declarations {
                pattern_names(&declarator.id, names);
            }
        }
        Statement::FunctionDeclaration(function) => {
            names.extend(function.id.as_ref().map(|id| id.name.as_str()));
        }
        Statement::ClassDeclaration(class) => {
            names.extend(class.id.as_ref().map(|id| id.name.as_str()));
        }
        Statement::BlockStatement(block) => declared_in(&block.body, names),
        Statement::IfStatement(branch) => {
            declared_at(&branch.consequent, names);
            if let Some(alternate) = &branch.alternate {
                declared_at(alternate, names);
            }
        }
        Statement::ForStatement(each) => {
            if let Some(ForStatementInit::VariableDeclaration(declaration)) = &each.init {
                for declarator in &declaration.declarations {
                    pattern_names(&declarator.id, names);
                }
            }
            declared_at(&each.body, names);
        }
        Statement::ForInStatement(each) => {
            if let ForStatementLeft::VariableDeclaration(declaration) = &each.left {
                for declarator in &declaration.declarations {
                    pattern_names(&declarator.id, names);
                }
            }
            declared_at(&each.body, names);
        }
        Statement::ForOfStatement(each) => {
            if let ForStatementLeft::VariableDeclaration(declaration) = &each.left {
                for declarator in &declaration.declarations {
                    pattern_names(&declarator.id, names);
                }
            }
            declared_at(&each.body, names);
        }
        Statement::WhileStatement(each) => declared_at(&each.body, names),
        Statement::WithStatement(each) => declared_at(&each.body, names),
        Statement::DoWhileStatement(each) => declared_at(&each.body, names),
        Statement::LabeledStatement(labeled) => declared_at(&labeled.body, names),
        Statement::TryStatement(attempt) => {
            declared_in(&attempt.block.body, names);
            if let Some(handler) = &attempt.handler {
                if let Some(param) = &handler.param {
                    pattern_names(&param.pattern, names);
                }
                declared_in(&handler.body.body, names);
            }
            if let Some(finalizer) = &attempt.finalizer {
                declared_in(&finalizer.body, names);
            }
        }
        Statement::SwitchStatement(switch) => {
            for case in &switch.cases {
                declared_in(&case.consequent, names);
            }
        }
        _ => {}
    }
}

fn iife_function<'k, 'a>(
    statement: &'k Statement<'a>,
    skip: Option<Span>,
) -> Option<&'k FunctionBody<'a>> {
    let Statement::ExpressionStatement(statement) = statement else {
        return None;
    };
    if skip == Some(statement.expression.span()) {
        return None;
    }
    let Expression::CallExpression(call) = statement.expression.without_parentheses() else {
        return None;
    };
    if !call.arguments.is_empty() {
        return None;
    }
    match call.callee.without_parentheses() {
        Expression::ArrowFunctionExpression(arrow) if arrow.params.items.is_empty() => {
            match &arrow.body {
                ArrowFunctionBody::FunctionBody(body) => Some(&**body),
                _ => None,
            }
        }
        Expression::FunctionExpression(function) if function.params.items.is_empty() => {
            function.body.as_deref()
        }
        _ => None,
    }
}

fn iife_function_mut<'k, 'a>(statement: &'k mut Statement<'a>) -> Option<&'k mut FunctionBody<'a>> {
    let Statement::ExpressionStatement(statement) = statement else {
        return None;
    };
    let mut expression = &mut statement.expression;
    while let Expression::ParenthesizedExpression(inner) = expression {
        expression = &mut inner.expression;
    }
    let Expression::CallExpression(call) = expression else {
        return None;
    };
    let mut callee = &mut call.callee;
    while let Expression::ParenthesizedExpression(inner) = callee {
        callee = &mut inner.expression;
    }
    match callee {
        Expression::ArrowFunctionExpression(arrow) => match &mut arrow.body {
            ArrowFunctionBody::FunctionBody(body) => Some(&mut **body),
            _ => None,
        },
        Expression::FunctionExpression(function) => function.body.as_deref_mut(),
        _ => None,
    }
}

fn is_declaration(statement: &Statement<'_>) -> bool {
    matches!(
        statement,
        Statement::VariableDeclaration(_)
            | Statement::FunctionDeclaration(_)
            | Statement::ClassDeclaration(_)
    )
}

fn peek<'k, 'a>(body: &'k FunctionBody<'a>, bodies: &'k Bodies<'a>) -> &'k [Statement<'a>] {
    match bodies {
        Bodies::Taken(pool) => pool
            .get(&body_key(body))
            .map_or(&body.statements, |taken| taken),
        _ => &body.statements,
    }
}

fn unit_helper(statements: &[Statement<'_>], require_name: &str) -> Option<(String, bool)> {
    let mut name = None;
    for statement in statements {
        if is_declaration(statement) {
            continue;
        }
        let Statement::ExpressionStatement(expression) = statement else {
            return None;
        };
        let Expression::AssignmentExpression(assignment) = &expression.expression else {
            return None;
        };
        let (helper, _, plain) = helper_parts(assignment, require_name)?;
        if name.replace((helper.to_string(), plain)).is_some() {
            return None;
        }
    }
    name
}

fn unit<'a>(
    alloc: &'a Allocator,
    statement: &Statement<'a>,
    body: &FunctionBody<'a>,
    bodies: &mut Bodies<'a>,
) -> Statement<'a> {
    let taken = match &bodies {
        Bodies::Taken(_) => Some(bodies.statements(alloc, body)),
        Bodies::Planned(_) => {
            bodies.statements(alloc, body);
            None
        }
        Bodies::Cloned => None,
    };
    let mut copy = statement.clone_in(alloc);
    if let (Some(statements), Some(body)) = (taken, iife_function_mut(&mut copy)) {
        body.statements = statements;
    }
    copy
}

fn is_literal(expr: &Expression<'_>) -> bool {
    matches!(
        expr,
        Expression::StringLiteral(_)
            | Expression::NumericLiteral(_)
            | Expression::BooleanLiteral(_)
            | Expression::NullLiteral(_)
    )
}

fn is_empty_object(expr: &Expression<'_>) -> bool {
    matches!(expr.without_parentheses(), Expression::ObjectExpression(object) if object.properties.is_empty())
}

fn is_closed_call(expr: &Expression<'_>) -> bool {
    let Expression::CallExpression(call) = expr.without_parentheses() else {
        return false;
    };
    call.arguments.is_empty()
        && matches!(
            call.callee.without_parentheses(),
            Expression::FunctionExpression(_) | Expression::ArrowFunctionExpression(_)
        )
}

fn helper_parts<'k, 'a>(
    assignment: &'k AssignmentExpression<'a>,
    require_name: &str,
) -> Option<(&'k str, Option<&'k FunctionBody<'a>>, bool)> {
    if assignment.operator != AssignmentOperator::Assign {
        return None;
    }
    let AssignmentTarget::StaticMemberExpression(member) = &assignment.left else {
        return None;
    };
    let Expression::Identifier(object) = &member.object else {
        return None;
    };
    if object.name != require_name {
        return None;
    }
    let (body, plain) = match &assignment.right {
        Expression::FunctionExpression(function) => (function.body.as_deref(), true),
        Expression::ArrowFunctionExpression(arrow) => match &arrow.body {
            ArrowFunctionBody::FunctionBody(body) => (Some(&**body), true),
            _ => (None, true),
        },
        right => (
            None,
            is_literal(right) || is_empty_object(right) || is_closed_call(right),
        ),
    };
    Some((member.property.name.as_str(), body, plain))
}

fn elements<'k, 'a>(statement: &'k Statement<'a>) -> Vec<&'k Expression<'a>> {
    let Statement::ExpressionStatement(statement) = statement else {
        return Vec::new();
    };
    match &statement.expression {
        Expression::SequenceExpression(sequence) => sequence.expressions.iter().collect(),
        expression => vec![expression],
    }
}

struct Collected<'a> {
    helper: Helper<'a>,
    free: FreeNames,
    offending: Vec<String>,
    plain: bool,
    origin: Option<(usize, usize)>,
    guarded: bool,
    unit_at: Option<usize>,
}

fn movable<'a>(
    alloc: &'a Allocator,
    prelude: &Prelude<'_>,
    statement: Statement<'a>,
) -> (Statement<'a>, FreeNames, Vec<String>) {
    let (statement, free) = scoped(alloc, statement);
    let mut offending: Vec<String> = free
        .iter()
        .map(|(name, _)| name)
        .filter(|name| {
            name.as_str() != prelude.require_name
                && (prelude.declared.contains(name.as_str())
                    || HOST_LOCALS.contains(&name.as_str()))
        })
        .cloned()
        .collect();
    offending.sort_unstable();
    (statement, free, offending)
}

fn helper<'a>(
    alloc: &'a Allocator,
    assignment: &AssignmentExpression<'a>,
    body: Option<&FunctionBody<'a>>,
    bodies: &mut Bodies<'a>,
) -> Statement<'a> {
    let taken = match (body, &bodies) {
        (Some(body), Bodies::Taken(_)) => Some(bodies.statements(alloc, body)),
        (Some(body), Bodies::Planned(_)) => {
            bodies.statements(alloc, body);
            None
        }
        _ => None,
    };
    let mut copy = assignment.clone_in(alloc);
    if let Some(statements) = taken {
        match &mut copy.right {
            Expression::FunctionExpression(function) => {
                if let Some(body) = function.body.as_deref_mut() {
                    body.statements = statements;
                }
            }
            Expression::ArrowFunctionExpression(arrow) => {
                if let ArrowFunctionBody::FunctionBody(body) = &mut arrow.body {
                    body.statements = statements;
                }
            }
            _ => {}
        }
    }
    let builder = AstBuilder::new(alloc);
    let expression = Expression::AssignmentExpression(oxc_allocator::Box::new_in(copy, &alloc));
    Statement::new_expression_statement(assignment.span, expression, &builder)
}

struct Site {
    origin: Option<(usize, usize)>,
    guarded: bool,
}

fn push_helper<'a>(
    alloc: &'a Allocator,
    prelude: &Prelude<'_>,
    assignment: &AssignmentExpression<'a>,
    bodies: &mut Bodies<'a>,
    site: Site,
    out: &mut Vec<Collected<'a>>,
) {
    let Some((name, body, plain)) = helper_parts(assignment, prelude.require_name) else {
        return;
    };
    if let Some(suffix) = dirname_suffix(assignment) {
        if let Some(statement) = dirname_helper(alloc, prelude.require_name, name, suffix) {
            out.push(Collected {
                helper: Helper {
                    name: name.to_string(),
                    statement,
                    movable: dirname_movable(prelude.require_name, &prelude.declared),
                },
                free: vec![(DIRNAME.to_string(), false)],
                offending: Vec::new(),
                plain: false,
                origin: None,
                guarded: site.guarded,
                unit_at: None,
            });
        }
        return;
    }
    let built = helper(alloc, assignment, body, bodies);
    let (statement, free, offending) = movable(alloc, prelude, built);
    out.push(Collected {
        helper: Helper {
            name: name.to_string(),
            statement,
            movable: offending.is_empty() && plain,
        },
        free,
        offending,
        plain,
        origin: site.origin,
        guarded: site.guarded,
        unit_at: None,
    });
}

fn collect<'a>(
    alloc: &'a Allocator,
    prelude: &Prelude<'_>,
    (statements, top): (&[Statement<'a>], bool),
    bodies: &mut Bodies<'a>,
    out: &mut Vec<Collected<'a>>,
) {
    for (index, statement) in statements.iter().enumerate() {
        if let Some((_, assignment)) = guarded(statement, prelude.require_name).filter(|_| top) {
            let site = Site {
                origin: None,
                guarded: true,
            };
            push_helper(alloc, prelude, assignment, bodies, site, out);
            continue;
        }
        for (element, expression) in elements(statement).into_iter().enumerate() {
            let Expression::AssignmentExpression(assignment) = expression else {
                continue;
            };
            let site = Site {
                origin: top.then_some((index, element)),
                guarded: false,
            };
            push_helper(alloc, prelude, assignment, bodies, site, out);
        }
    }
}

struct Part {
    statement: usize,
    element: Option<usize>,
    free: FreeNames,
}

fn is_helper_element(expression: &Expression<'_>, require: &str) -> bool {
    matches!(expression, Expression::AssignmentExpression(assignment) if helper_parts(assignment, require).is_some())
}

fn parts<'a>(
    alloc: &'a Allocator,
    (require, skip): (&str, Option<Span>),
    statements: &[Statement<'a>],
    collected: &[Collected<'a>],
) -> Vec<Part> {
    let mut parts = Vec::new();
    for (index, statement) in statements.iter().enumerate() {
        if collected.iter().any(|c| c.unit_at == Some(index)) {
            continue;
        }
        if let Some((test, _)) = guarded(statement, require) {
            parts.push(Part {
                statement: index,
                element: None,
                free: expression_names(alloc, test),
            });
            continue;
        }
        let Statement::ExpressionStatement(_) = statement else {
            parts.push(Part {
                statement: index,
                element: None,
                free: statement_names(alloc, statement),
            });
            continue;
        };
        for (element, expression) in elements(statement).into_iter().enumerate() {
            if !is_helper_element(expression, require) && skip != Some(expression.span()) {
                parts.push(Part {
                    statement: index,
                    element: Some(element),
                    free: expression_names(alloc, expression),
                });
            }
        }
    }
    parts
}

fn top_declarations<'k>(
    params: Option<&'k FormalParameters<'_>>,
    statements: &'k [Statement<'_>],
) -> Vec<&'k str> {
    let mut all = Vec::new();
    for item in params.iter().flat_map(|params| &params.items) {
        pattern_names(&item.pattern, &mut all);
    }
    declared_in(statements, &mut all);
    all
}

fn guard_holds(
    prelude: &Prelude<'_>,
    all: &[&str],
    statements: &[Statement<'_>],
    (parts, collected): (&[Part], &[Collected<'_>]),
) -> bool {
    let require = prelude.require_name;
    let function = statements.iter().any(|statement| {
        matches!(statement, Statement::FunctionDeclaration(f) if f.id.as_ref().is_some_and(|id| id.name == require))
    });
    let once = all.iter().filter(|name| **name == require).count() == 1;
    let written = parts.iter().any(|part| writes(&part.free, require))
        || collected.iter().any(|c| writes(&c.free, require));
    function && once && !written && !prelude.declared.contains("undefined")
}

fn bare_vars<'k>(statements: &'k [Statement<'_>], all: &[&str]) -> Vec<&'k str> {
    let mut bare = Vec::new();
    for statement in statements {
        let Statement::VariableDeclaration(declaration) = statement else {
            continue;
        };
        if declaration.kind != VariableDeclarationKind::Var {
            continue;
        }
        for declarator in &declaration.declarations {
            if let (BindingPattern::BindingIdentifier(id), None) =
                (&declarator.id, &declarator.init)
            {
                bare.push(id.name.as_str());
            }
        }
    }
    bare.retain(|name| all.iter().filter(|other| *other == name).count() == 1);
    bare
}

struct Inits<'k, 'a> {
    sites: Vec<(usize, usize)>,
    values: Vec<&'k Expression<'a>>,
    assignments: Vec<&'k Expression<'a>>,
}

fn init_of<'k, 'a>(
    expression: &'k Expression<'a>,
    locals: &[String],
) -> Option<(&'k str, &'k Expression<'a>)> {
    let Expression::AssignmentExpression(assignment) = expression else {
        return None;
    };
    if assignment.operator != AssignmentOperator::Assign {
        return None;
    }
    match &assignment.left {
        AssignmentTarget::AssignmentTargetIdentifier(id)
            if locals.iter().any(|local| local == id.name.as_str()) =>
        {
            Some((id.name.as_str(), &assignment.right))
        }
        _ => None,
    }
}

fn inits<'k, 'a>(
    statements: &'k [Statement<'a>],
    (statement_at, helper_at): (usize, usize),
    locals: &[String],
    require: &str,
) -> Option<Inits<'k, 'a>> {
    let mut found = Inits {
        sites: Vec::new(),
        values: Vec::new(),
        assignments: Vec::new(),
    };
    let mut named: Vec<&str> = Vec::new();
    for (index, statement) in statements.iter().enumerate().take(statement_at + 1) {
        let list = elements(statement);
        if list.is_empty() && !found.sites.is_empty() {
            return None;
        }
        let end = if index == statement_at {
            helper_at
        } else {
            list.len()
        };
        for (element, expression) in list.into_iter().enumerate().take(end) {
            match init_of(expression, locals) {
                Some((name, value)) => {
                    if named.contains(&name) {
                        return None;
                    }
                    named.push(name);
                    found.sites.push((index, element));
                    found.values.push(value);
                    found.assignments.push(expression);
                }
                None if !found.sites.is_empty() && !is_helper_element(expression, require) => {
                    return None;
                }
                None => {}
            }
        }
    }
    Some(found)
}

fn closed_init<'a>(alloc: &'a Allocator, prelude: &Prelude<'_>, value: &Expression<'a>) -> bool {
    closed_value(value)
        && expression_names(alloc, value).iter().all(|(name, _)| {
            name.as_str() == prelude.require_name
                || !(prelude.declared.contains(name.as_str())
                    || HOST_LOCALS.contains(&name.as_str()))
        })
}

fn local_helper<'a>(
    alloc: &'a Allocator,
    prelude: &Prelude<'_>,
    context: (&[Statement<'a>], &[Part], &[&str]),
    collected: &[Collected<'a>],
    index: usize,
) -> Option<Statement<'a>> {
    let (statements, parts, bare) = context;
    let candidate = &collected[index];
    let (statement, helper_at) = candidate.origin?;
    if candidate.helper.movable || !candidate.plain || candidate.offending.is_empty() {
        return None;
    }
    let locals = &candidate.offending;
    if !locals.iter().all(|local| bare.contains(&local.as_str())) {
        return None;
    }
    let found = inits(
        statements,
        (statement, helper_at),
        locals,
        prelude.require_name,
    )?;
    if !found
        .values
        .iter()
        .all(|value| closed_init(alloc, prelude, value))
    {
        return None;
    }
    let outside = parts
        .iter()
        .filter(|part| {
            !part
                .element
                .is_some_and(|element| found.sites.contains(&(part.statement, element)))
        })
        .map(|part| &part.free)
        .chain(
            collected
                .iter()
                .enumerate()
                .filter(|(other, _)| *other != index)
                .map(|(_, c)| &c.free),
        );
    let mut outside = outside;
    if outside.any(|free| locals.iter().any(|local| mentions(free, local))) {
        return None;
    }
    let names: Vec<&str> = locals.iter().map(String::as_str).collect();
    let inits = found
        .assignments
        .iter()
        .map(|assignment| assignment.clone_in(alloc))
        .collect();
    let unit = local_unit(
        alloc,
        &names,
        inits,
        candidate.helper.statement.clone_in(alloc),
    )?;
    let (unit, _, offending) = movable(alloc, prelude, unit);
    offending.is_empty().then_some(unit)
}

fn refine<'a>(
    alloc: &'a Allocator,
    prelude: &Prelude<'_>,
    (params, statements): (Option<&FormalParameters<'a>>, &[Statement<'a>]),
    collected: &mut [Collected<'a>],
) {
    let any_guarded = collected.iter().any(|c| c.guarded);
    let any_local = collected
        .iter()
        .any(|c| c.origin.is_some() && c.plain && !c.offending.is_empty());
    if !any_guarded && !any_local {
        return;
    }
    let parts = parts(
        alloc,
        (prelude.require_name, prelude.skip),
        statements,
        collected,
    );
    let all = top_declarations(params, statements);
    if any_guarded && !guard_holds(prelude, &all, statements, (&parts, collected)) {
        for c in collected.iter_mut().filter(|c| c.guarded) {
            c.helper.movable = false;
        }
    }
    if !any_local {
        return;
    }
    let bare = bare_vars(statements, &all);
    let units: Vec<(usize, Statement<'a>)> = (0..collected.len())
        .filter_map(|index| {
            local_helper(
                alloc,
                prelude,
                (statements, &parts, &bare),
                collected,
                index,
            )
            .map(|unit| (index, unit))
        })
        .collect();
    for (index, unit) in units {
        collected[index].helper.statement = unit;
        collected[index].helper.movable = true;
    }
}

pub fn prelude_runtime<'a>(
    alloc: &'a Allocator,
    prelude: (Option<&FormalParameters<'a>>, &[Statement<'a>]),
    require_name: &str,
    bodies: &mut Bodies<'a>,
) -> Option<Runtime<'a>> {
    prelude_runtime_skipping(alloc, prelude, (require_name, None), bodies)
}

pub fn prelude_runtime_skipping<'a>(
    alloc: &'a Allocator,
    (params, statements): (Option<&FormalParameters<'a>>, &[Statement<'a>]),
    (require_name, skip): (&str, Option<Span>),
    bodies: &mut Bodies<'a>,
) -> Option<Runtime<'a>> {
    let mut declared = FxHashSet::default();
    for item in params.iter().flat_map(|params| &params.items) {
        pattern_names(&item.pattern, &mut declared);
    }
    declared_in(statements, &mut declared);
    let wrapped: Vec<(usize, Wrapped<'_, 'a>)> = statements
        .iter()
        .enumerate()
        .filter_map(|(index, statement)| {
            let body = iife_function(statement, skip)?;
            Some((
                index,
                (
                    statement,
                    body,
                    unit_helper(peek(body, bodies), require_name),
                ),
            ))
        })
        .collect();
    for (_, (_, body, unit)) in &wrapped {
        if unit.is_none() {
            declared_in(&body.statements, &mut declared);
        }
    }
    let prelude = Prelude {
        require_name,
        declared,
        skip,
    };
    let mut collected = Vec::new();
    collect(alloc, &prelude, (statements, true), bodies, &mut collected);
    for (index, (statement, body, name)) in wrapped {
        let Some((name, plain)) = name else {
            collect(
                alloc,
                &prelude,
                (&body.statements, false),
                bodies,
                &mut collected,
            );
            continue;
        };
        let built = unit(alloc, statement, body, bodies);
        let (statement, free, offending) = movable(alloc, &prelude, built);
        collected.push(Collected {
            helper: Helper {
                name,
                statement,
                movable: offending.is_empty() && plain,
            },
            free,
            offending,
            plain,
            origin: None,
            guarded: false,
            unit_at: Some(index),
        });
    }
    refine(alloc, &prelude, (params, statements), &mut collected);
    Some(Runtime {
        require_name: require_name.to_string(),
        helpers: collected.into_iter().map(|c| c.helper).collect(),
    })
}

fn inert_value(statement: &Statement<'_>) -> bool {
    let Statement::ExpressionStatement(statement) = statement else {
        return false;
    };
    let Expression::AssignmentExpression(assignment) = &statement.expression else {
        return false;
    };
    match &assignment.right {
        Expression::BinaryExpression(sum) => {
            sum.operator == BinaryOperator::Addition
                && matches!(sum.right, Expression::StringLiteral(_))
                && matches!(&sum.left, Expression::CallExpression(call)
                    if matches!(&call.callee, Expression::StaticMemberExpression(member) if member.property.name == "dirname"))
        }
        right => is_literal(right),
    }
}

pub fn inert_helpers(runtime: &Runtime<'_>) -> FxHashSet<String> {
    let mut inert = FxHashSet::default();
    for (index, helper) in runtime.helpers.iter().enumerate() {
        let last = !runtime.helpers[index + 1..]
            .iter()
            .any(|later| later.name == helper.name);
        if last && helper.movable && inert_value(&helper.statement) {
            inert.insert(helper.name.clone());
        }
    }
    inert
}
