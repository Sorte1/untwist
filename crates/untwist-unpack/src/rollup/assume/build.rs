use oxc_allocator::{Allocator, CloneIn, Vec as AVec};
use oxc_ast::ast::*;
use oxc_ast::builder::AstBuilder;
use oxc_semantic::Scoping;
use oxc_span::SPAN;
use rustc_hash::{FxHashMap, FxHashSet};

use super::outer::assigned_names;
use super::plan::Plan;
#[cfg(feature = "boundaries")]
use super::rewrite::route;
use crate::model::{Bundle, BundleKind, Module};
use crate::rollup::build::{
    comments_of, entry_statements, fresh_creator, head_text, parsed, program, rebuilt_wrapper,
    sanitized, tail_text, wrapper_body, Links, CHUNK_PREFIX, ENTRY_ID,
};
use crate::rollup::scan::Facts;
use crate::rollup::shell::Shell;
use crate::runtime::synthetic_statements;

pub const SHELL_ID: &str = "shell";
const SETTER_PREFIX: &str = "set_";
const SETTER_PARAMETER: &str = "value";

fn fresh(text: &str, base: String, taken: &FxHashSet<String>) -> String {
    let mut name = base;
    while text.contains(name.as_str()) || taken.contains(&name) {
        name.push('_');
    }
    name
}

fn setter_definition(setter: &str, name: &str) -> String {
    let mut parameter = String::from(SETTER_PARAMETER);
    while parameter == name {
        parameter.push('_');
    }
    format!("function {setter}({parameter}) {{\n  return {name} = {parameter};\n}}\n")
}

pub struct Layout {
    pub links: Links,
    pub shell: Option<usize>,
    pub entry: usize,
    pub setters: Vec<(String, String)>,
    pub definitions: Vec<String>,
    pub rewrites: FxHashMap<u32, Rewrite>,
}

pub struct Rewrite {
    pub setter: String,
    pub binding: String,
    pub named: bool,
}

fn ids(plan: &Plan, shell: bool) -> Vec<String> {
    let mut taken: FxHashSet<String> = [ENTRY_ID, SHELL_ID]
        .map(str::to_string)
        .into_iter()
        .collect();
    let mut ids = Vec::new();
    for part in &plan.parts {
        let base = format!("{CHUNK_PREFIX}{}", sanitized(&part.stem));
        let mut id = base.clone();
        let mut suffix = 2usize;
        while !taken.insert(id.to_ascii_lowercase()) {
            id = format!("{base}-{suffix}");
            suffix += 1;
        }
        ids.push(id);
    }
    if shell {
        ids.push(SHELL_ID.to_string());
    }
    for run in plan.runs.iter().rev() {
        let base = sanitized(&run.stem);
        let mut id = base.clone();
        let mut suffix = 2usize;
        while !taken.insert(id.to_ascii_lowercase()) {
            id = format!("{base}-{suffix}");
            suffix += 1;
        }
        ids.push(id);
    }
    ids.push(ENTRY_ID.to_string());
    ids
}

fn grouped(mut edges: Vec<(usize, usize, String)>, count: usize) -> Vec<Vec<(usize, Vec<String>)>> {
    edges.sort_unstable();
    edges.dedup();
    let mut imports: Vec<Vec<(usize, Vec<String>)>> = vec![Vec::new(); count];
    for (to, from, name) in edges {
        match imports[to].last_mut() {
            Some((known, names)) if *known == from => {
                if !name.is_empty() {
                    names.push(name);
                }
            }
            _ => imports[to].push((from, [name].into_iter().filter(|n| !n.is_empty()).collect())),
        }
    }
    imports
}

#[cfg(not(feature = "boundaries"))]
fn route<'a>(
    _alloc: &'a Allocator,
    rewrites: &FxHashMap<u32, Rewrite>,
    _statements: &mut [Statement<'a>],
) -> bool {
    rewrites.is_empty()
}

fn run_slot(first: usize, count: usize, run: usize) -> usize {
    first + count - 1 - run
}

pub fn layout(scoping: &Scoping, facts: &Facts, plan: &Plan, text: &str) -> Layout {
    let shell = (!plan.shell.is_empty()).then_some(plan.parts.len());
    let ids = ids(plan, shell.is_some());
    let entry = ids.len() - 1;
    let first_run = entry - plan.runs.len();
    let at = |item: u32| {
        let item = item as usize;
        plan.part_of[item]
            .or_else(|| plan.run_of[item].map(|run| run_slot(first_run, plan.runs.len(), run)))
            .unwrap_or(entry)
    };
    let mut edges = Vec::new();
    let mut exports: Vec<Vec<String>> = vec![Vec::new(); ids.len()];
    for &(item, symbol) in &facts.uses {
        let owner = facts.owner[symbol.index()];
        let (to, from) = (at(item), at(owner));
        if from != to {
            let name = scoping.symbol_name(symbol).to_string();
            edges.push((to, from, name.clone()));
            exports[from].push(name);
        }
    }
    if let Some(shell) = shell {
        for (item, name, _) in &facts.shell {
            if plan.part_of[*item as usize].is_some() {
                edges.push((at(*item), shell, name.clone()));
            }
        }
    }
    let mut taken: FxHashSet<String> = FxHashSet::default();
    let setters: Vec<(String, String)> = plan
        .setters
        .iter()
        .map(|name| {
            let setter = fresh(text, format!("{SETTER_PREFIX}{name}"), &taken);
            taken.insert(setter.clone());
            (name.clone(), setter)
        })
        .collect();
    let mut definitions = vec![String::new(); ids.len()];
    let mut rewrites = FxHashMap::default();
    for found in &plan.run_setters {
        let owner = run_slot(first_run, plan.runs.len(), found.run);
        let setter = fresh(text, format!("{SETTER_PREFIX}{}", found.name), &taken);
        taken.insert(setter.clone());
        definitions[owner].push_str(&setter_definition(&setter, &found.name));
        exports[owner].push(setter.clone());
        for &(item, at_start, named) in &found.writes {
            edges.push((at(item), owner, setter.clone()));
            rewrites.insert(
                at_start,
                Rewrite {
                    setter: setter.clone(),
                    binding: found.name.clone(),
                    named,
                },
            );
        }
    }
    for (_, setter) in &setters {
        edges.push((entry, shell.unwrap_or(entry), setter.clone()));
    }
    for part in 0..plan.parts.len() {
        edges.push((entry, part, String::new()));
    }
    for run in first_run..entry {
        edges.push((entry, run, String::new()));
    }
    for run in 1..plan.runs.len() {
        let count = plan.runs.len();
        edges.push((
            run_slot(first_run, count, run),
            run_slot(first_run, count, run - 1),
            String::new(),
        ));
    }
    for names in &mut exports {
        names.sort_unstable();
        names.dedup();
    }
    let mut requires = vec![false; ids.len()];
    for (item, &required) in facts.requires.iter().enumerate() {
        requires[at(item as u32)] |= required;
    }
    Layout {
        links: Links {
            ids,
            imports: grouped(edges, entry + 1),
            exports,
            requires,
            dropped: facts.dropped.clone(),
        },
        shell,
        entry,
        setters,
        definitions,
        rewrites,
    }
}

fn shell_text(names: &[String], setters: &[(String, String)]) -> String {
    let mut text = format!("export var {};\n", names.join(", "));
    for (name, setter) in setters {
        text.push_str("export ");
        text.push_str(&setter_definition(setter, name));
    }
    text
}

fn setter_call<'a>(
    alloc: &'a Allocator,
    setter: &str,
    value: Expression<'a>,
) -> Option<Expression<'a>> {
    let mut call = synthetic_statements(alloc, format!("{setter}(0);"))?;
    let Some(Statement::ExpressionStatement(wrapped)) = call.pop() else {
        return None;
    };
    let mut expression = wrapped.unbox().expression;
    let Expression::CallExpression(invoke) = &mut expression else {
        return None;
    };
    invoke.arguments.clear();
    invoke.arguments.push(Argument::from(value));
    Some(expression)
}

fn setter_of<'s>(setters: &'s [(String, String)], name: &str) -> Option<&'s str> {
    setters
        .iter()
        .find(|(known, _)| known == name)
        .map(|(_, setter)| setter.as_str())
}

fn routed<'a>(
    alloc: &'a Allocator,
    mut statement: Statement<'a>,
    setters: &[(String, String)],
    statements: &mut AVec<'a, Statement<'a>>,
) -> Option<()> {
    let routes: Vec<&str> = assigned_names(&statement)
        .into_iter()
        .filter_map(|name| setter_of(setters, name))
        .collect();
    match (&mut statement, &routes[..]) {
        (_, []) => statements.push(statement),
        (Statement::ExpressionStatement(expression), [setter]) => {
            let builder = AstBuilder::new(alloc);
            let assignment = std::mem::replace(
                &mut expression.expression,
                Expression::new_null_literal(SPAN, &builder),
            );
            expression.expression = setter_call(alloc, setter, assignment)?;
            statements.push(statement);
        }
        (Statement::VariableDeclaration(declaration), [setter])
            if declaration.declarations.len() == 1 =>
        {
            let name = declaration.declarations[0]
                .id
                .get_binding_identifier()?
                .name;
            statements.push(statement);
            statements.extend(synthetic_statements(alloc, format!("{setter}({name});"))?);
        }
        _ => return None,
    }
    Some(())
}

fn outer_statements<'a>(
    alloc: &'a Allocator,
    outer_body: &FunctionBody<'a>,
    inner_index: usize,
    inner: AVec<'a, Statement<'a>>,
    setters: &[(String, String)],
) -> Option<AVec<'a, Statement<'a>>> {
    let mut statements = AVec::new_in(&alloc);
    let mut inner = Some(inner);
    for (index, statement) in outer_body.statements.iter().enumerate() {
        let cloned = statement.clone_in(alloc);
        match index.cmp(&inner_index) {
            std::cmp::Ordering::Equal => statements.extend(inner.take()?),
            std::cmp::Ordering::Greater => routed(alloc, cloned, setters, &mut statements)?,
            std::cmp::Ordering::Less => statements.push(cloned),
        }
    }
    Some(statements)
}

fn nested_entry<'a>(
    alloc: &'a Allocator,
    source: &Program<'a>,
    shell: &Shell<'_, 'a>,
    head: AVec<'a, Statement<'a>>,
    inner: AVec<'a, Statement<'a>>,
    setters: &[(String, String)],
) -> Option<AVec<'a, Statement<'a>>> {
    let outer = shell.outer.as_ref()?;
    let text = match outer.arrow {
        true => "(() => {})();",
        false => "(function () {})();",
    };
    let mut rebuilt = synthetic_statements(alloc, text.to_string())?;
    let body = wrapper_body(&mut rebuilt)?;
    body.directives = outer.body.directives.clone_in(alloc);
    body.statements = outer_statements(alloc, outer.body, shell.wrapper.index, inner, setters)?;
    let mut statements = head;
    for (index, statement) in source.body.iter().enumerate() {
        if index == outer.index {
            statements.extend(rebuilt.drain(..));
        } else {
            statements.push(statement.clone_in(alloc));
        }
    }
    Some(statements)
}

struct Pieces<'a> {
    parts: Vec<Vec<Statement<'a>>>,
    runs: Vec<Vec<Statement<'a>>>,
    entry: Vec<Statement<'a>>,
}

fn pieces<'a>(plan: &Plan, layout: &Layout, owned: AVec<'a, Statement<'a>>) -> Pieces<'a> {
    let mut parts: Vec<Vec<Statement<'a>>> = (0..plan.parts.len()).map(|_| Vec::new()).collect();
    let mut runs: Vec<Vec<Statement<'a>>> = (0..plan.runs.len()).map(|_| Vec::new()).collect();
    let mut entry = Vec::new();
    for (index, statement) in owned.into_iter().enumerate() {
        if layout.links.dropped.get(index).copied().unwrap_or(false) {
            continue;
        }
        let part = plan.part_of.get(index).copied().flatten();
        let run = plan.run_of.get(index).copied().flatten();
        match (part, run) {
            (Some(part), _) => parts[part].push(statement),
            (None, Some(run)) => runs[run].push(statement),
            (None, None) => entry.push(statement),
        }
    }
    Pieces { parts, runs, entry }
}

fn module_of<'a>(
    alloc: &'a Allocator,
    source: &Program<'a>,
    layout: &Layout,
    at: usize,
    statements: AVec<'a, Statement<'a>>,
    comments: Vec<Comment>,
) -> Module<'a> {
    let directives = match at == layout.entry {
        true => source.directives.clone_in(alloc),
        false => AVec::new_in(&alloc),
    };
    let program = program(alloc, source, comments, directives, statements);
    let id = layout.links.ids[at].clone();
    let mut module = Module::new(id, program, at == layout.entry);
    module.dependencies = layout.links.imports[at]
        .iter()
        .map(|(from, _)| {
            (
                layout.links.ids[*from].clone(),
                layout.links.ids[*from].clone(),
            )
        })
        .collect();
    module
}

struct Prepared<'a> {
    heads: Vec<AVec<'a, Statement<'a>>>,
    tails: Vec<AVec<'a, Statement<'a>>>,
    shell: Option<AVec<'a, Statement<'a>>>,
    inner: AVec<'a, Statement<'a>>,
}

fn prepared<'a>(
    alloc: &'a Allocator,
    source: &Program<'a>,
    shell: &Shell<'_, 'a>,
    (plan, layout): (&Plan, &Layout),
) -> Option<Prepared<'a>> {
    let creator = fresh_creator(source.source_text);
    let links = &layout.links;
    let mut heads = Vec::with_capacity(links.ids.len());
    let mut tails = Vec::with_capacity(links.ids.len());
    for at in 0..links.ids.len() {
        heads.push(parsed(alloc, head_text(links, at, &creator))?);
        let definitions = layout.definitions.get(at).map_or("", String::as_str);
        tails.push(parsed(
            alloc,
            format!("{definitions}{}", tail_text(links, at)),
        )?);
    }
    let shell_module = match layout.shell {
        Some(_) => Some(parsed(alloc, shell_text(&plan.shell, &layout.setters))?),
        None => None,
    };
    let mut inner = rebuilt_wrapper(alloc, shell)?;
    wrapper_body(&mut inner)?;
    if shell.outer.is_some() {
        let probe = AVec::new_in(&alloc);
        nested_entry(
            alloc,
            source,
            shell,
            probe,
            AVec::new_in(&alloc),
            &layout.setters,
        )?;
    }
    Some(Prepared {
        heads,
        tails,
        shell: shell_module,
        inner,
    })
}

pub fn assemble<'a>(
    alloc: &'a Allocator,
    source: &Program<'a>,
    shell: &Shell<'_, 'a>,
    (plan, layout): (&Plan, &Layout),
    owned: AVec<'a, Statement<'a>>,
) -> Result<Bundle<'a>, AVec<'a, Statement<'a>>> {
    let Some(prepared) = prepared(alloc, source, shell, (plan, layout)) else {
        return Err(owned);
    };
    let Prepared {
        heads,
        tails,
        shell: shell_module,
        mut inner,
    } = prepared;
    let mut owned = owned;
    if !route(alloc, &layout.rewrites, &mut owned) {
        return Err(owned);
    }
    let mut heads = heads.into_iter();
    let mut tails = tails.into_iter();
    let Pieces { parts, runs, entry } = pieces(plan, layout, owned);
    let mut bundle = Bundle::new(BundleKind::Rollup, Some(ENTRY_ID.to_string()));
    let mut emit = |at: usize,
                    body: Vec<Statement<'a>>,
                    heads: &mut std::vec::IntoIter<_>,
                    tails: &mut std::vec::IntoIter<_>| {
        let comments = comments_of(source, &body);
        let mut statements = heads.next().unwrap_or_else(|| AVec::new_in(&alloc));
        statements.extend(body);
        statements.extend(tails.next().unwrap_or_else(|| AVec::new_in(&alloc)));
        let module = module_of(alloc, source, layout, at, statements, comments);
        bundle.modules.insert(module.id.clone(), module);
    };
    for (at, body) in parts.into_iter().enumerate() {
        emit(at, body, &mut heads, &mut tails);
    }
    let shell_module = match (layout.shell, shell_module) {
        (Some(at), Some(statements)) => {
            heads.next();
            tails.next();
            Some(module_of(alloc, source, layout, at, statements, Vec::new()))
        }
        _ => None,
    };
    let first_run = layout.entry - runs.len();
    let count = runs.len();
    let mut ordered: Vec<(usize, Vec<Statement<'a>>)> = runs
        .into_iter()
        .enumerate()
        .map(|(index, body)| (run_slot(first_run, count, index), body))
        .collect();
    ordered.sort_by_key(|(slot, _)| *slot);
    for (slot, body) in ordered {
        emit(slot, body, &mut heads, &mut tails);
    }
    if let Some(module) = shell_module {
        bundle.modules.insert(module.id.clone(), module);
    }
    let comments = comments_of(source, &entry);
    if let Some(body) = wrapper_body(&mut inner) {
        body.statements.extend(entry);
    }
    let head = heads.next().unwrap_or_else(|| AVec::new_in(&alloc));
    let statements = match shell.outer {
        Some(_) => nested_entry(alloc, source, shell, head, inner, &layout.setters)
            .unwrap_or_else(|| AVec::new_in(&alloc)),
        None => entry_statements(alloc, source, shell, head, inner),
    };
    let module = module_of(alloc, source, layout, layout.entry, statements, comments);
    bundle.modules.insert(module.id.clone(), module);
    Ok(bundle)
}
