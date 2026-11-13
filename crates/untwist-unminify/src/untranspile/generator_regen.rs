use oxc_allocator::{CloneIn, GetAllocator, Vec as AVec};
use oxc_ast::ast::*;
use oxc_ast_visit::{walk_js, VisitJs};
use oxc_semantic::Scoping;
use oxc_syntax::scope::ScopeFlags;
use oxc_syntax::symbol::SymbolId;
use oxc_traverse::BoundIdentifier;

use untwist_core::pass::Cx;

use super::generator::{
    aliases_arguments, body_of, drop_declarators, drop_final_return, rebinds, returned_call,
    unbind, Body,
};
use super::identify::Table;
use super::machine_check::Reads;
use super::machine_classic;
use super::machine_emit::Emitter;
use super::machine_regenerator::{self, Context, Regenerator, ValueReads};

struct Loaded<'a> {
    machine: super::machine::Machine,
    labels: Vec<&'a str>,
}
use super::shapes::Kind;
use super::structure;
use super::structure_tidy::tidy;

pub const SHAPE: &str = "generator/regenerator/unsafe";

#[derive(Clone, Copy, PartialEq, Eq)]
enum Flavor {
    Inline,
    Classic,
}

impl Flavor {
    fn wrap(self) -> &'static str {
        match self {
            Flavor::Inline => "w",
            Flavor::Classic => "wrap",
        }
    }

    fn mark(self) -> &'static str {
        match self {
            Flavor::Inline => "m",
            Flavor::Classic => "mark",
        }
    }
}

fn runtime_of(
    expression: &Expression<'_>,
    table: &Table,
    s: &Scoping,
) -> Option<(Flavor, &'static str)> {
    let Expression::StaticMemberExpression(member) = expression.without_parentheses() else {
        return None;
    };
    let Expression::CallExpression(runtime) = member.object.without_parentheses() else {
        return None;
    };
    if member.optional || !runtime.arguments.is_empty() || runtime.optional {
        return None;
    }
    let flavor = match table.kind_of_callee(&runtime.callee, s)? {
        Kind::Regenerator => Flavor::Inline,
        Kind::RegeneratorRuntime => Flavor::Classic,
        _ => return None,
    };
    let name = member.property.name.as_str();
    [flavor.wrap(), flavor.mark()]
        .into_iter()
        .find(|method| *method == name)
        .map(|method| (flavor, method))
}

fn runtime_call(expression: &Expression<'_>, method: &str, table: &Table, s: &Scoping) -> bool {
    runtime_of(expression, table, s).is_some_and(|(flavor, found)| {
        found == method
            || (method == "w" && found == flavor.wrap())
            || (method == "m" && found == flavor.mark())
    })
}

fn flavor_of(call: &CallExpression<'_>, table: &Table, s: &Scoping) -> Option<Flavor> {
    runtime_of(&call.callee, table, s).map(|(flavor, _)| flavor)
}

fn state_body<'k, 'a>(
    expression: &'k Expression<'a>,
    scoping: &Scoping,
) -> Option<(Body<'k, 'a>, Option<SymbolId>)> {
    if let Some(body) = body_of(expression) {
        return Some((body, None));
    }
    let Expression::FunctionExpression(function) = expression.without_parentheses() else {
        return None;
    };
    let own = function.id.as_ref()?.symbol_id.get()?;
    if function.generator || function.r#async || !scoping.get_resolved_reference_ids(own).is_empty()
    {
        return None;
    }
    let body = function.body.as_ref()?;
    body.directives.is_empty().then_some(())?;
    Some((
        Body {
            state: super::generator::simple_param(&function.params)?,
            scope: function.scope_id.get()?,
            statements: &body.statements,
        },
        Some(own),
    ))
}

fn wrapped<'k, 'a>(
    function: &'k Function<'a>,
    table: &Table,
    scoping: &Scoping,
) -> Option<(&'k CallExpression<'a>, SymbolId)> {
    if function.generator || function.r#async {
        return None;
    }
    let call = returned_call(function)?;
    if call.optional || !runtime_call(&call.callee, "w", table, scoping) {
        return None;
    }
    let (
        Some(Argument::FunctionExpression(_) | Argument::ArrowFunctionExpression(_)),
        Some(Argument::Identifier(marked)),
    ) = (call.arguments.first(), call.arguments.get(1))
    else {
        return None;
    };
    let extra_fits = match call.arguments.as_slice() {
        [_, _] => true,
        [_, _, Argument::NullLiteral(_), Argument::ArrayExpression(_)] => true,
        _ => false,
    };
    if !extra_fits {
        return None;
    }
    let marked = scoping
        .get_reference(marked.reference_id.get()?)
        .symbol_id()?;
    (scoping.get_resolved_reference_ids(marked).len() == 1).then_some((call, marked))
}

struct ThisFree {
    depth: u32,
    found: bool,
}

impl<'a> VisitJs<'a> for ThisFree {
    fn visit_this_expression(&mut self, _it: &ThisExpression) {
        self.found |= self.depth == 0;
    }

    fn visit_function(&mut self, it: &Function<'a>, flags: ScopeFlags) {
        self.depth += 1;
        walk_js::walk_function(self, it, flags);
        self.depth -= 1;
    }
}

fn this_free(statements: &[Statement<'_>]) -> bool {
    let mut scan = ThisFree {
        depth: 0,
        found: false,
    };
    for statement in statements {
        scan.visit_statement(statement);
    }
    !scan.found
}

fn switch_scopes_free(body: &Body<'_, '_>, named: Option<SymbolId>, scoping: &Scoping) -> bool {
    let own = scoping.get_bindings(body.scope);
    let only_state = own
        .values()
        .all(|symbol| *symbol == body.state || Some(*symbol) == named)
        && own.values().any(|symbol| *symbol == body.state);
    let [Statement::WhileStatement(looped)] = body.statements else {
        return false;
    };
    let Some(switch) = machine_regenerator::switch_of(looped) else {
        return false;
    };
    let block_free = match &looped.body {
        Statement::BlockStatement(block) => block
            .scope_id
            .get()
            .is_some_and(|scope| scoping.get_bindings(scope).is_empty()),
        _ => true,
    };
    only_state
        && block_free
        && switch
            .scope_id
            .get()
            .is_some_and(|scope| scoping.get_bindings(scope).is_empty())
}

fn copy_cases<'a>(
    call: &CallExpression<'a>,
    cx: &Cx<'a, '_>,
) -> Option<Vec<AVec<'a, Statement<'a>>>> {
    let allocator = GetAllocator::allocator(cx);
    let statements = match call.arguments.first()? {
        Argument::FunctionExpression(function) => &function.body.as_ref()?.statements,
        Argument::ArrowFunctionExpression(arrow) => match &arrow.body {
            ArrowFunctionBody::FunctionBody(body) => &body.statements,
            _ => return None,
        },
        _ => return None,
    };
    let [Statement::WhileStatement(looped)] = statements.as_slice() else {
        return None;
    };
    let switch = machine_regenerator::switch_of(looped)?;
    Some(
        switch
            .cases
            .iter()
            .map(|case| case.consequent.clone_in_with_semantic_ids(allocator))
            .collect(),
    )
}

pub fn marks(declarator: &VariableDeclarator<'_>, table: &Table, s: &Scoping) -> Option<SymbolId> {
    let Expression::CallExpression(call) = declarator.init.as_ref()?.without_parentheses() else {
        return None;
    };
    if call.optional || !runtime_call(&call.callee, "m", table, s) {
        return None;
    }
    match call.arguments.as_slice() {
        [Argument::Identifier(id)] => resolved_id(id, s),
        _ => None,
    }
}

fn resolved_id(id: &IdentifierReference<'_>, scoping: &Scoping) -> Option<SymbolId> {
    scoping.get_reference(id.reference_id.get()?).symbol_id()
}

pub fn rewrite<'a>(function: &mut Function<'a>, table: &Table, cx: &mut Cx<'a, '_>) -> Option<()> {
    let scope = function.scope_id.get()?;
    let scoping = cx.scoping();
    let named_yield = function
        .id
        .as_ref()
        .is_some_and(|id| id.name.as_str() == "yield");
    if named_yield || super::async_fn::binds_name(scope, "yield", scoping) {
        return None;
    }
    let (call, _) = wrapped(function, table, scoping)?;
    let flavor = flavor_of(call, table, scoping)?;
    let (body, named) = state_body(call.arguments.first()?.as_expression()?, scoping)?;
    if !switch_scopes_free(&body, named, scoping) || !this_free(body.statements) {
        return None;
    }
    let prefix = function.body.as_ref()?.statements.split_last()?.1;
    let home = (!prefix.iter().any(aliases_arguments)).then_some(scope);
    let (machine, labels, cases_ref, reads): (_, Vec<&'a str>, _, Box<dyn Reads>) = match flavor {
        Flavor::Inline => {
            let context = Context {
                scoping,
                state: body.state,
            };
            let list = call.arguments.get(3).and_then(Argument::as_expression);
            let regions = machine_regenerator::regions(list, body.statements, &context)?;
            let values = |callee: &Expression<'_>| {
                table.kind_of_callee(callee, scoping) == Some(Kind::RegeneratorValues)
            };
            let regenerator = Regenerator {
                context,
                home,
                regions,
                values: &values,
            };
            let loaded = machine_regenerator::read(body.statements, &regenerator)?;
            let (cases_ref, _) = machine_regenerator::cases(body.statements, &regenerator.context)?;
            (
                loaded.machine,
                loaded.labels,
                cases_ref,
                Box::new(ValueReads::of(scoping, body.state)),
            )
        }
        Flavor::Classic => {
            if call.arguments.len() != 2 {
                return None;
            }
            let context = machine_classic::Context {
                scoping,
                state: body.state,
            };
            let found = machine_classic::cases(body.statements, &context)?;
            let classic = machine_classic::Classic {
                context,
                home,
                labels: found.labels.clone(),
            };
            let (machine, labels) = machine_classic::read(body.statements, &classic)?;
            (
                machine,
                labels,
                found.cases,
                Box::new(machine_classic::SentReads::of(scoping, body.state)),
            )
        }
    };
    let loaded = Loaded { machine, labels };
    let mut structured = structure::structure(&loaded.machine)?;
    tidy(&mut structured.code, &loaded.machine);
    let rebound = rebinds(&loaded.machine, &cases_ref, prefix, reads.as_ref(), scoping);
    let labels: Vec<&'a str> = loaded.labels;
    let machine = loaded.machine;
    let cases = copy_cases(returned_call(function)?, cx)?;
    let mut emitter = Emitter::new(
        cases,
        (&machine, &structured.yields),
        reads.as_ref(),
        scope,
        cx,
    );
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
    Some(())
}

fn site_of(
    statement: &Statement<'_>,
    table: &Table,
    scoping: &Scoping,
) -> Option<(SymbolId, SymbolId)> {
    let Statement::FunctionDeclaration(function) = statement else {
        return None;
    };
    let own = function.id.as_ref()?.symbol_id.get()?;
    let (_, marked) = wrapped(function, table, scoping)?;
    Some((own, marked))
}

fn marker_at(
    statements: &[Statement<'_>],
    (own, marked): (SymbolId, SymbolId),
    table: &Table,
    scoping: &Scoping,
) -> bool {
    statements.iter().any(|statement| {
        matches!(statement, Statement::VariableDeclaration(declaration)
        if declaration.declarations.iter().any(|declarator| {
            matches!(&declarator.id, BindingPattern::BindingIdentifier(id)
                if id.symbol_id.get() == Some(marked))
                && marks(declarator, table, scoping) == Some(own)
        }))
    })
}

pub fn has_site(statements: &[Statement<'_>], table: &Table, scoping: &Scoping) -> bool {
    statements.iter().any(|statement| {
        site_of(statement, table, scoping)
            .is_some_and(|site| marker_at(statements, site, table, scoping))
    })
}

fn drop_marker(statements: &mut AVec<'_, Statement<'_>>, marked: SymbolId) {
    for statement in statements.iter_mut() {
        if let Statement::VariableDeclaration(declaration) = statement {
            declaration.declarations.retain(|declarator| {
                !matches!(&declarator.id, BindingPattern::BindingIdentifier(id)
                    if id.symbol_id.get() == Some(marked))
            });
        }
    }
    statements.retain(|statement| {
        !matches!(statement, Statement::VariableDeclaration(declaration)
            if declaration.declarations.is_empty())
    });
}

pub fn rewrite_statements<'a>(
    statements: &mut AVec<'a, Statement<'a>>,
    table: &Table,
    cx: &mut Cx<'a, '_>,
) -> usize {
    let sites: Vec<(usize, SymbolId)> = statements
        .iter()
        .enumerate()
        .filter_map(|(at, statement)| {
            let site = site_of(statement, table, cx.scoping())?;
            marker_at(statements, site, table, cx.scoping()).then_some((at, site.1))
        })
        .collect();
    let mut done = Vec::new();
    for (at, marked) in sites {
        let Some(Statement::FunctionDeclaration(function)) = statements.get_mut(at) else {
            continue;
        };
        if rewrite(function, table, cx).is_some() {
            done.push(marked);
        }
    }
    for marked in &done {
        drop_marker(statements, *marked);
    }
    done.len()
}

fn marked_by_itself(call: &CallExpression<'_>, table: &Table, scoping: &Scoping) -> bool {
    if call.optional || !runtime_call(&call.callee, "m", table, scoping) {
        return false;
    }
    let [Argument::FunctionExpression(function)] = call.arguments.as_slice() else {
        return false;
    };
    let own = function.id.as_ref().and_then(|id| id.symbol_id.get());
    own.is_some()
        && wrapped(function, table, scoping).is_some_and(|(_, marked)| Some(marked) == own)
}

pub fn rewrite_expression<'a>(
    expression: &mut Expression<'a>,
    table: &Table,
    cx: &mut Cx<'a, '_>,
) -> bool {
    let Expression::CallExpression(call) = expression else {
        return false;
    };
    if !marked_by_itself(call, table, cx.scoping()) {
        return false;
    }
    let Some(Argument::FunctionExpression(function)) = call.arguments.first_mut() else {
        return false;
    };
    if rewrite(function, table, cx).is_none() {
        return false;
    }
    let Some(Argument::FunctionExpression(function)) = call.arguments.pop() else {
        return false;
    };
    *expression = Expression::FunctionExpression(function);
    true
}
