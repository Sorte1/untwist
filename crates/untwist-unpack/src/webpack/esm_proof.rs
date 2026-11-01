use oxc_allocator::{Allocator, Vec as AVec};
use oxc_ast::ast::*;
use oxc_semantic::Scoping;
use rustc_hash::{FxHashMap, FxHashSet};
use untwist_core::ast_util::prop::identifier_name;
use untwist_core::semantic::build_scoping;

use crate::matcher::{argument_expression, expression_statement};
use crate::model::{Bundle, Module};
use crate::runtime::synthetic_statements;
use crate::webpack::esm::{
    is_define_esmodule, require_helper_call, zero_param_arrow_body, zero_param_function_return,
};
use crate::webpack::esm_scan::{Facts, Scan};
use crate::webpack::helpers::inert_helpers;

const EXPORTS: &str = "exports";

type Known<'s> = (&'s FxHashSet<String>, &'s FxHashSet<String>);

struct Local {
    scoping: Scoping,
    dropped: FxHashSet<usize>,
    specifiers: Vec<(String, String)>,
}

struct Define<'k, 'a> {
    target: &'k str,
    entries: Vec<(String, &'k Expression<'a>)>,
}

fn bulk_entries<'k, 'a>(
    object: &'k ObjectExpression<'a>,
) -> Option<Vec<(String, &'k Expression<'a>)>> {
    let mut entries = Vec::with_capacity(object.properties.len());
    for property in &object.properties {
        let ObjectPropertyKind::ObjectProperty(property) = property else {
            return None;
        };
        if property.computed || property.method {
            return None;
        }
        let key = match &property.key {
            PropertyKey::StaticIdentifier(key) => key.name.to_string(),
            PropertyKey::StringLiteral(key) if !key.lone_surrogates => key.value.to_string(),
            _ => return None,
        };
        entries.push((key, zero_param_arrow_body(&property.value)?));
    }
    Some(entries)
}

fn define<'k, 'a>(statement: &'k Statement<'a>) -> Option<Option<Define<'k, 'a>>> {
    let arguments = expression_statement(statement).and_then(|e| require_helper_call(e, "d"))?;
    let shape = || -> Option<Define<'k, 'a>> {
        let target = identifier_name(argument_expression(arguments.first()?)?)?;
        let entries = match (arguments.len(), argument_expression(arguments.get(1)?)?) {
            (2, Expression::ObjectExpression(object)) => bulk_entries(object)?,
            (3, Expression::StringLiteral(name)) if !name.lone_surrogates => {
                let value = zero_param_function_return(argument_expression(arguments.get(2)?)?)?;
                vec![(name.value.to_string(), value)]
            }
            _ => return None,
        };
        Some(Define { target, entries })
    };
    Some(shape())
}

fn is_export_name(name: &str) -> bool {
    let mut chars = name.chars();
    let first = chars
        .next()
        .is_some_and(|c| c.is_ascii_alphabetic() || c == '_' || c == '$');
    first && chars.all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '$')
}

fn marks_exports(statement: &Statement<'_>) -> bool {
    is_define_esmodule(statement)
        && expression_statement(statement)
            .and_then(|e| require_helper_call(e, "r"))
            .and_then(|arguments| arguments.first())
            .and_then(argument_expression)
            .and_then(identifier_name)
            == Some(EXPORTS)
}

fn strict(program: &Program<'_>) -> bool {
    program
        .directives
        .iter()
        .any(|directive| directive.directive.as_str() == "use strict")
}

fn root_binding(scoping: &Scoping, value: &Expression<'_>) -> Option<String> {
    let Expression::Identifier(id) = value else {
        return None;
    };
    let symbol = scoping.get_reference(id.reference_id()).symbol_id()?;
    (scoping.symbol_scope_id(symbol) == scoping.root_scope_id()).then(|| id.name.to_string())
}

fn defines_exports(statement: &Statement<'_>) -> bool {
    define(statement).is_some_and(|define| define.is_some_and(|define| define.target == EXPORTS))
}

fn local_exports(program: &Program<'_>) -> Option<Local> {
    let defined = program
        .body
        .iter()
        .find(|statement| !matches!(statement, Statement::FunctionDeclaration(_)))
        .is_some_and(defines_exports);
    if !strict(program) || !(defined || program.body.iter().any(marks_exports)) {
        return None;
    }
    let scoping = build_scoping(program).scoping;
    let mut dropped = FxHashSet::default();
    let mut specifiers = Vec::new();
    for (index, statement) in program.body.iter().enumerate() {
        if marks_exports(statement) {
            dropped.insert(index);
            continue;
        }
        let Some(define) = define(statement) else {
            continue;
        };
        let define = define?;
        if define.target != EXPORTS {
            return None;
        }
        for (name, value) in define.entries {
            if !is_export_name(&name) {
                return None;
            }
            specifiers.push((root_binding(&scoping, value)?, name));
        }
        dropped.insert(index);
    }
    let sorted = specifiers.windows(2).all(|pair| pair[0].1 < pair[1].1);
    sorted.then_some(Local {
        scoping,
        dropped,
        specifiers,
    })
}

fn scan(program: &Program<'_>, local: Option<&Local>, known: Known<'_>) -> Facts {
    let none = FxHashSet::default();
    if let Some(local) = local {
        return Scan::new(Some(&local.scoping), known, &local.dropped).run(program);
    }
    let facts = Scan::new(None, known, &none).run(program);
    if !facts.wants_scoping {
        return facts;
    }
    let scoping = build_scoping(program).scoping;
    Scan::new(Some(&scoping), known, &none).run(program)
}

struct Tarjan<'g> {
    edges: &'g [Vec<usize>],
    index: Vec<Option<usize>>,
    low: Vec<usize>,
    on_stack: Vec<bool>,
    stack: Vec<usize>,
    next: usize,
    cyclic: Vec<bool>,
}

impl Tarjan<'_> {
    fn enter(&mut self, node: usize) {
        self.index[node] = Some(self.next);
        self.low[node] = self.next;
        self.next += 1;
        self.stack.push(node);
        self.on_stack[node] = true;
    }

    fn close(&mut self, node: usize) {
        if Some(self.low[node]) != self.index[node] {
            return;
        }
        let mut members = Vec::new();
        while let Some(member) = self.stack.pop() {
            self.on_stack[member] = false;
            members.push(member);
            if member == node {
                break;
            }
        }
        let looped = members.len() > 1 || self.edges[node].contains(&node);
        for member in members {
            self.cyclic[member] = looped;
        }
    }

    fn visit(&mut self, root: usize) {
        self.enter(root);
        let mut work = vec![(root, 0usize)];
        while let Some((node, edge)) = work.pop() {
            if let Some(&next) = self.edges[node].get(edge) {
                work.push((node, edge + 1));
                match self.index[next] {
                    None => {
                        self.enter(next);
                        work.push((next, 0));
                    }
                    Some(index) if self.on_stack[next] => {
                        self.low[node] = self.low[node].min(index);
                    }
                    Some(_) => {}
                }
                continue;
            }
            self.close(node);
            if let Some(&(parent, _)) = work.last() {
                self.low[parent] = self.low[parent].min(self.low[node]);
            }
        }
    }
}

fn cyclic(edges: &[Vec<usize>]) -> Vec<bool> {
    let count = edges.len();
    let mut tarjan = Tarjan {
        edges,
        index: vec![None; count],
        low: vec![0; count],
        on_stack: vec![false; count],
        stack: Vec::new(),
        next: 0,
        cyclic: vec![false; count],
    };
    for node in 0..count {
        if tarjan.index[node].is_none() {
            tarjan.visit(node);
        }
    }
    tarjan.cyclic
}

fn export_statement<'a>(
    alloc: &'a Allocator,
    specifiers: &[(String, String)],
) -> Option<Statement<'a>> {
    let list: Vec<String> = specifiers
        .iter()
        .map(|(local, name)| match local == name {
            true => local.clone(),
            false => format!("{local} as {name}"),
        })
        .collect();
    synthetic_statements(alloc, format!("export {{ {} }};", list.join(", ")))?.pop()
}

fn emit<'a>(alloc: &'a Allocator, module: &mut Module<'a>, local: &Local) -> Option<()> {
    let export = export_statement(alloc, &local.specifiers)?;
    let program = module.program_mut();
    let old = std::mem::replace(&mut program.body, AVec::new_in(&alloc));
    for (index, statement) in old.into_iter().enumerate() {
        if !local.dropped.contains(&index) {
            program.body.push(statement);
        }
    }
    program.body.push(export);
    program.source_type = program.source_type.with_module(true);
    Some(())
}

pub fn convert_proven<'a>(alloc: &'a Allocator, bundle: &mut Bundle<'a>) -> usize {
    let mut locals: Vec<Option<Local>> = bundle
        .modules
        .values()
        .map(|module| local_exports(module.program()))
        .collect();
    let candidates: FxHashSet<String> = bundle
        .modules
        .keys()
        .zip(&locals)
        .filter(|(_, local)| local.is_some())
        .map(|(id, _)| id.clone())
        .collect();
    if candidates.is_empty() {
        return 0;
    }
    let inert = bundle
        .runtime
        .as_ref()
        .map(inert_helpers)
        .unwrap_or_default();
    let position: FxHashMap<&str, usize> = bundle
        .modules
        .keys()
        .enumerate()
        .map(|(index, id)| (id.as_str(), index))
        .collect();
    let mut refused = vec![false; locals.len()];
    let mut edges = Vec::with_capacity(locals.len());
    for (index, module) in bundle.modules.values().enumerate() {
        let facts = scan(
            module.program(),
            locals[index].as_ref(),
            (&candidates, &inert),
        );
        if facts.global {
            return 0;
        }
        refused[index] |= facts.leftover;
        for id in &facts.refused {
            if let Some(&target) = position.get(id.as_str()) {
                refused[target] = true;
            }
        }
        let targets = facts.targets.iter();
        edges.push(
            targets
                .filter_map(|id| position.get(id.as_str()).copied())
                .collect::<Vec<_>>(),
        );
    }
    for (index, looped) in cyclic(&edges).into_iter().enumerate() {
        refused[index] |= looped;
    }
    let mut converted = 0;
    for (index, module) in bundle.modules.values_mut().enumerate() {
        let local = locals[index].take().filter(|_| !refused[index]);
        if let Some(local) = local {
            converted += usize::from(emit(alloc, module, &local).is_some());
        }
    }
    converted
}

pub fn is_harmony(module: &Module<'_>) -> bool {
    module.is_esm() || module.program().body.iter().any(is_define_esmodule)
}
