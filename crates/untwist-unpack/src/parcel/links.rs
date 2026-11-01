use oxc_ast::ast::*;
use oxc_span::Span;
use oxc_syntax::operator::AssignmentOperator;
use rustc_hash::{FxHashMap, FxHashSet};

use super::layout::{self, Hoisted, Layout};
use super::single_string;
use crate::frame::Frame;
use crate::matcher::expression_statement;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Form {
    Hoisted,
    Registry,
}

pub struct Loader<'k> {
    pub form: Form,
    pub id: &'k str,
    pub load: Span,
}

#[derive(Clone, Debug, Default)]
pub struct ParcelView {
    pub namespace: String,
    pub foreign: FxHashSet<String>,
    pub loadable: FxHashSet<String>,
    pub chunk: bool,
}

#[derive(PartialEq, Eq)]
enum Role {
    Registered,
    Entry(String),
}

struct Part {
    input: usize,
    global: String,
    form: Form,
    ids: Vec<String>,
    loads: Vec<String>,
    root: bool,
    role: Option<Role>,
}

fn named(expr: &Expression<'_>, name: &str) -> bool {
    matches!(expr.without_parentheses(), Expression::Identifier(id) if id.name == name)
}

fn member_callee(callee: &Expression<'_>, object: &str, property: &str) -> bool {
    let Expression::StaticMemberExpression(member) = callee else {
        return false;
    };
    !member.optional && member.property.name == property && named(&member.object, object)
}

fn relative(call: &CallExpression<'_>) -> bool {
    single_string(call).is_some_and(|spec| spec.starts_with("./") || spec.starts_with("../"))
}

fn exports_assignment<'k, 'a>(statement: &'k Statement<'a>) -> Option<&'k Expression<'a>> {
    let Expression::AssignmentExpression(assign) = expression_statement(statement)? else {
        return None;
    };
    let AssignmentTarget::StaticMemberExpression(target) = &assign.left else {
        return None;
    };
    let fits = assign.operator == AssignmentOperator::Assign
        && !target.optional
        && target.property.name == "exports"
        && named(&target.object, "module");
    fits.then_some(&assign.right)
}

fn arrow_target<'k, 'a>(argument: &'k Argument<'a>) -> Option<&'k Expression<'a>> {
    let Argument::ArrowFunctionExpression(arrow) = argument else {
        return None;
    };
    let empty = arrow.params.items.is_empty() && arrow.params.rest.is_none();
    if !empty || arrow.r#async {
        return None;
    }
    arrow.body.as_expression()
}

fn call_of<'k, 'a>(expr: &'k Expression<'a>) -> Option<&'k CallExpression<'a>> {
    match expr.without_parentheses() {
        Expression::CallExpression(call) if !call.optional => Some(call),
        _ => None,
    }
}

fn hoisted_target<'k>(target: &'k Expression<'_>, require: &str) -> Option<&'k str> {
    let call = call_of(target)?;
    if !named(&call.callee, require) {
        return None;
    }
    single_string(call)
}

fn registry_target<'k>(target: &'k Expression<'_>) -> Option<&'k str> {
    let call = call_of(target)?;
    let Expression::StaticMemberExpression(root) = &call.callee else {
        return None;
    };
    let fits = !root.optional
        && root.property.name == "root"
        && member_callee(&root.object, "module", "bundle");
    if !fits {
        return None;
    }
    single_string(call)
}

pub fn loader<'k>(statements: &'k [Statement<'_>], require: Option<&str>) -> Option<Loader<'k>> {
    let [statement] = statements else {
        return None;
    };
    let then = call_of(exports_assignment(statement)?)?;
    let Expression::StaticMemberExpression(member) = &then.callee else {
        return None;
    };
    if member.optional || member.property.name != "then" {
        return None;
    }
    let resolved = call_of(&member.object)?;
    if !member_callee(&resolved.callee, "Promise", "resolve") {
        return None;
    }
    let ([Argument::CallExpression(load)], [argument]) =
        (resolved.arguments.as_slice(), then.arguments.as_slice())
    else {
        return None;
    };
    let target = arrow_target(argument)?;
    if load.optional || !relative(load) {
        return None;
    }
    let (form, id) = match require {
        Some(require) if named(&load.callee, "require") => {
            (Form::Hoisted, hoisted_target(target, require)?)
        }
        None if member_callee(&load.callee, "module", "require") => {
            (Form::Registry, registry_target(target)?)
        }
        _ => return None,
    };
    Some(Loader {
        form,
        id,
        load: load.span,
    })
}

fn inert_chunk_statement(statement: &Statement<'_>) -> bool {
    match statement {
        Statement::FunctionDeclaration(_) | Statement::EmptyStatement(_) => true,
        Statement::VariableDeclaration(declaration) => {
            declaration.declarations.iter().all(|declarator| {
                declarator.init.as_ref().is_none_or(|init| {
                    matches!(
                        init,
                        Expression::StringLiteral(_)
                            | Expression::NumericLiteral(_)
                            | Expression::BooleanLiteral(_)
                            | Expression::NullLiteral(_)
                            | Expression::FunctionExpression(_)
                            | Expression::ArrowFunctionExpression(_)
                    ) || named(init, "globalThis")
                })
            })
        }
        _ => false,
    }
}

pub fn effect_free_chunk(top: &[Statement<'_>], layout: &Hoisted<'_, '_>) -> bool {
    top.iter().enumerate().all(|(index, statement)| {
        if layout.prelude.contains(&index) {
            return true;
        }
        let registers = layout.defs_at(index).len();
        if registers == 0 {
            return inert_chunk_statement(statement);
        }
        match expression_statement(statement) {
            Some(Expression::SequenceExpression(sequence)) => {
                sequence.expressions.len() == registers
            }
            Some(_) => registers == 1,
            None => false,
        }
    })
}

fn hoisted_part(input: usize, top: &[Statement<'_>], layout: &Hoisted<'_, '_>) -> Part {
    let require = layout.require.name.as_str();
    let loads = layout
        .defs
        .iter()
        .filter_map(|def| loader(&def.body()?.statements, Some(require)))
        .filter(|found| found.form == Form::Hoisted)
        .map(|found| found.id.to_string())
        .collect();
    let chunk = !layout.defined() && effect_free_chunk(top, layout);
    Part {
        input,
        global: layout.global.clone(),
        form: Form::Hoisted,
        ids: layout.defs.iter().map(|def| def.id.clone()).collect(),
        loads,
        root: layout.defined(),
        role: chunk.then_some(Role::Registered),
    }
}

fn part(input: usize, program: &Program<'_>) -> Option<Part> {
    let frame = Frame::of(program);
    match layout::layout(&frame, program)? {
        Layout::Registry(registry) => {
            let loads = registry
                .defs
                .iter()
                .filter_map(|def| loader(&def.body()?.statements, None))
                .filter(|found| found.form == Form::Registry)
                .map(|found| found.id.to_string())
                .collect();
            let role = match (registry.entries.as_slice(), &registry.main) {
                ([only], Some(main)) if only == main => Some(Role::Entry(main.clone())),
                _ => None,
            };
            Some(Part {
                input,
                global: registry.global.clone(),
                form: Form::Registry,
                ids: registry.defs.iter().map(|def| def.id.clone()).collect(),
                loads,
                root: true,
                role,
            })
        }
        Layout::Hoisted(hoisted) => Some(hoisted_part(input, &program.body, &hoisted)),
    }
}

fn owners(parts: &[&Part]) -> Option<FxHashMap<String, usize>> {
    let mut owners = FxHashMap::default();
    for (at, part) in parts.iter().enumerate() {
        for id in &part.ids {
            if owners.insert(id.clone(), at).is_some() {
                return None;
            }
        }
    }
    Some(owners)
}

fn loaded(parts: &[&Part], owners: &FxHashMap<String, usize>) -> Option<Vec<bool>> {
    let mut loaded = vec![false; parts.len()];
    for (at, part) in parts.iter().enumerate() {
        for id in &part.loads {
            let owner = *owners.get(id)?;
            let target = parts[owner];
            let fits = match &target.role {
                Some(Role::Registered) => part.form == Form::Hoisted,
                Some(Role::Entry(main)) => part.form == Form::Registry && main == id,
                None => false,
            };
            if owner == at || !fits {
                return None;
            }
            loaded[owner] = true;
        }
    }
    Some(loaded)
}

fn views(parts: &[&Part]) -> Option<Vec<(usize, ParcelView)>> {
    let form = parts.first()?.form;
    if parts.len() < 2 || parts.iter().any(|part| part.form != form) {
        return None;
    }
    let owners = owners(parts)?;
    let loaded = loaded(parts, &owners)?;
    let roots: Vec<usize> = (0..parts.len()).filter(|&at| !loaded[at]).collect();
    if roots.len() != 1 || !parts[roots[0]].root {
        return None;
    }
    let namespace = format!("parcel:{}", parts[0].global);
    let views = parts
        .iter()
        .enumerate()
        .map(|(at, part)| {
            let foreign = owners
                .iter()
                .filter(|(_, &owner)| owner != at)
                .map(|(id, _)| id.clone())
                .collect();
            let loadable = part.loads.iter().cloned().collect();
            let view = ParcelView {
                namespace: namespace.clone(),
                foreign,
                loadable,
                chunk: loaded[at],
            };
            (part.input, view)
        })
        .collect();
    Some(views)
}

pub fn groups(programs: &[Program<'_>]) -> Vec<Vec<(usize, ParcelView)>> {
    let parts: Vec<Part> = programs
        .iter()
        .enumerate()
        .filter_map(|(input, program)| part(input, program))
        .collect();
    let mut globals: Vec<&str> = parts.iter().map(|part| part.global.as_str()).collect();
    globals.sort_unstable();
    globals.dedup();
    globals
        .into_iter()
        .filter_map(|global| {
            let members: Vec<&Part> = parts.iter().filter(|part| part.global == global).collect();
            views(&members)
        })
        .collect()
}
