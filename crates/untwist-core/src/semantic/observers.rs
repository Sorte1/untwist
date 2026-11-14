use std::cell::Cell;

use oxc_allocator::Allocator;
use oxc_ast::ast::{
    Argument, ArrowFunctionExpression, CallExpression, Function, Program, ReturnStatement,
    ThisExpression,
};
use oxc_ast_visit::{walk, Visit};
use oxc_semantic::{Scoping, SemanticBuilder};
use oxc_span::SourceType;
use oxc_str::Ident;
use oxc_syntax::scope::{ScopeFlags, ScopeId};

use super::annex_b::block_function_symbols;
use super::blocked_hoists::repair_blocked_hoists;
use crate::parse::{has_error, parse_with};

const OPAQUE_NAMES: [&str; 2] = ["eval", "arguments"];

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Observers {
    #[default]
    Every,
    Open,
    None,
}

thread_local! {
    static OBSERVERS: Cell<Observers> = const { Cell::new(Observers::Every) };
    static ASSUMED_EVAL: Cell<bool> = const { Cell::new(false) };
}

pub struct ObserversGuard {
    previous: Observers,
}

impl Drop for ObserversGuard {
    fn drop(&mut self) {
        OBSERVERS.with(|cell| cell.set(self.previous));
    }
}

pub fn assume_observers(observers: Observers) -> ObserversGuard {
    ASSUMED_EVAL.with(|cell| cell.set(false));
    let previous = OBSERVERS.with(|cell| cell.replace(observers));
    ObserversGuard { previous }
}

pub fn observers() -> Observers {
    OBSERVERS.with(Cell::get)
}

pub fn observing_open<R>(run: impl FnOnce() -> R) -> R {
    if observers() != Observers::None {
        return run();
    }
    let previous = OBSERVERS.with(|cell| cell.replace(Observers::Open));
    let _restore = ObserversGuard { previous };
    run()
}

pub fn with_observes() -> bool {
    observers() != Observers::None
}

pub fn assumed_hazard(scoping: &Scoping) -> Option<&'static str> {
    if observers() != Observers::None {
        return None;
    }
    if ASSUMED_EVAL.with(Cell::get) {
        return Some("direct eval");
    }
    scoping
        .scope_descendants_from_root()
        .any(|scope| scoping.scope_flags(scope).contains(ScopeFlags::With))
        .then_some("with statement")
}

pub fn closed_eval_names(call: &CallExpression<'_>) -> Option<Vec<String>> {
    if call.optional || !call.callee.is_specific_id("eval") {
        return None;
    }
    let [Argument::StringLiteral(literal)] = call.arguments.as_slice() else {
        return None;
    };
    payload_names(literal.value.as_str()?)
}

const STRICT_PREFIX: &str = "\"use strict\";";

enum Reading {
    Unparsed,
    Open,
    Names(Vec<String>, bool),
}

fn payload_names(text: &str) -> Option<Vec<String>> {
    let Reading::Names(mut names, block_functions) = read_payload(text, false) else {
        return None;
    };
    if !block_functions {
        return Some(names);
    }
    match read_payload(&format!("{STRICT_PREFIX}{text}"), true) {
        Reading::Unparsed => {}
        Reading::Open => return None,
        Reading::Names(strict, _) => {
            for name in strict {
                if !names.contains(&name) {
                    names.push(name);
                }
            }
        }
    }
    Some(names)
}

fn read_payload(text: &str, strict: bool) -> Reading {
    let alloc = Allocator::default();
    let parsed = parse_with(&alloc, text, SourceType::script());
    if has_error(&parsed.diagnostics) {
        return Reading::Unparsed;
    }
    if parsed.program.directives.len() != usize::from(strict) {
        return Reading::Open;
    }
    let mut shape = PayloadShape::default();
    shape.visit_program(&parsed.program);
    if shape.open {
        return Reading::Open;
    }
    let mut scoping = SemanticBuilder::new()
        .build(&parsed.program)
        .semantic
        .into_scoping();
    let block_functions = !block_function_symbols(&scoping).is_empty();
    repair_blocked_hoists(&parsed.program, &mut scoping);
    if declares_outside_functions(&scoping) || has_dynamic_scope(&scoping) {
        return Reading::Open;
    }
    let mut names = Vec::new();
    for (name, references) in scoping.root_unresolved_references() {
        let name = name.as_str();
        let writes = references
            .iter()
            .any(|reference| scoping.get_reference(*reference).is_write());
        if writes || OPAQUE_NAMES.contains(&name) {
            return Reading::Open;
        }
        names.push(name.to_string());
    }
    Reading::Names(names, block_functions)
}

fn declares_outside_functions(scoping: &Scoping) -> bool {
    scoping.symbol_ids().any(|symbol| {
        let home = scoping.symbol_scope_id(symbol);
        let catch = scoping.symbol_flags(symbol).is_catch_variable();
        !catch
            && !scoping
                .scope_ancestors(home)
                .any(|scope| scoping.scope_flags(scope).is_function())
    })
}

fn has_dynamic_scope(scoping: &Scoping) -> bool {
    scoping.scope_descendants_from_root().any(|scope| {
        scoping
            .scope_flags(scope)
            .intersects(ScopeFlags::DirectEval | ScopeFlags::With)
    })
}

#[derive(Default)]
struct PayloadShape {
    depth: u32,
    open: bool,
}

impl<'a> Visit<'a> for PayloadShape {
    fn visit_function(&mut self, it: &Function<'a>, flags: ScopeFlags) {
        self.depth += 1;
        walk::walk_function(self, it, flags);
        self.depth -= 1;
    }

    fn visit_arrow_function_expression(&mut self, it: &ArrowFunctionExpression<'a>) {
        self.depth += 1;
        walk::walk_arrow_function_expression(self, it);
        self.depth -= 1;
    }

    fn visit_this_expression(&mut self, _it: &ThisExpression) {
        self.open = true;
    }

    fn visit_return_statement(&mut self, it: &ReturnStatement<'a>) {
        self.open |= self.depth == 0;
        walk::walk_return_statement(self, it);
    }
}

pub fn settle_eval_scopes(program: &Program<'_>, scoping: &mut Scoping) {
    let observers = observers();
    if observers == Observers::Every {
        return;
    }
    if !scoping
        .scope_flags(scoping.root_scope_id())
        .contains_direct_eval()
    {
        return;
    }
    let mut open = open_eval_sites(program, scoping);
    if observers == Observers::None {
        let assumed = !open.is_empty();
        ASSUMED_EVAL.with(|cell| cell.set(assumed));
        open.clear();
    }
    let scopes: Vec<ScopeId> = scoping.scope_descendants_from_root().collect();
    for scope in scopes {
        scoping
            .scope_flags_mut(scope)
            .remove(ScopeFlags::DirectEval);
    }
    for site in open {
        let chain: Vec<ScopeId> = scoping.scope_ancestors(site).collect();
        for scope in chain {
            scoping
                .scope_flags_mut(scope)
                .insert(ScopeFlags::DirectEval);
        }
    }
}

pub fn open_eval_sites(program: &Program<'_>, scoping: &Scoping) -> Vec<ScopeId> {
    let mut sites = EvalSites {
        scoping,
        scopes: vec![scoping.root_scope_id()],
        open: Vec::new(),
    };
    sites.visit_program(program);
    sites.open
}

struct EvalSites<'s> {
    scoping: &'s Scoping,
    scopes: Vec<ScopeId>,
    open: Vec<ScopeId>,
}

impl EvalSites<'_> {
    fn closed_at(&self, call: &CallExpression<'_>, site: ScopeId) -> bool {
        closed_eval_names(call).is_some_and(|names| {
            names.iter().all(|name| {
                self.scoping
                    .find_binding(site, Ident::from(name.as_str()))
                    .is_none()
            })
        })
    }
}

impl<'a> Visit<'a> for EvalSites<'_> {
    fn enter_scope(&mut self, _flags: ScopeFlags, scope_id: &Cell<Option<ScopeId>>) {
        if let Some(scope) = scope_id.get() {
            self.scopes.push(scope);
        }
    }

    fn leave_scope(&mut self) {
        if self.scopes.len() > 1 {
            self.scopes.pop();
        }
    }

    fn visit_call_expression(&mut self, it: &CallExpression<'a>) {
        if !it.optional && it.callee.is_specific_id("eval") {
            let site = self
                .scopes
                .last()
                .copied()
                .unwrap_or(self.scoping.root_scope_id());
            if !self.closed_at(it, site) {
                self.open.push(site);
            }
        }
        walk::walk_call_expression(self, it);
    }
}

pub fn eval_payload_names(program: &Program<'_>) -> Vec<String> {
    let mut found = PayloadNames::default();
    found.visit_program(program);
    found.names
}

#[derive(Default)]
struct PayloadNames {
    names: Vec<String>,
}

impl<'a> Visit<'a> for PayloadNames {
    fn visit_call_expression(&mut self, it: &CallExpression<'a>) {
        if let Some(names) = closed_eval_names(it) {
            for name in names {
                if !self.names.contains(&name) {
                    self.names.push(name);
                }
            }
        }
        walk::walk_call_expression(self, it);
    }
}
