use oxc_allocator::{Allocator, Vec as AVec};
use oxc_ast::ast::*;
use oxc_ast_visit::{walk_js, VisitJs};
use oxc_semantic::Scoping;
use oxc_span::GetSpan;
use oxc_syntax::scope::{ScopeFlags, ScopeId};
use oxc_syntax::symbol::SymbolId;
use rustc_hash::{FxHashMap, FxHashSet};
use untwist_core::ast_util::respan::SYNTHETIC_BASE;
use untwist_core::semantic::build_scoping;

use crate::canon::statement_shape;
use crate::esbuild::mentions::{mentioned, names_anywhere};
use crate::model::{Bundle, BundleKind, Module};
use crate::rename::{rename_binding, rename_parameters};
use crate::runtime::{declared_names, live_exports, module_of, synthetic_statements};

fn synthetic(statement: &Statement<'_>) -> bool {
    statement.span().end >= SYNTHETIC_BASE
}

fn own_statements<'k, 'a>(module: &'k Module<'a>) -> impl Iterator<Item = &'k Statement<'a>> {
    module
        .program()
        .body
        .iter()
        .filter(|statement| !synthetic(statement))
}

struct CalledOnly<'n> {
    names: &'n FxHashSet<String>,
    read: FxHashSet<String>,
}

impl<'a> VisitJs<'a> for CalledOnly<'_> {
    fn visit_call_expression(&mut self, it: &CallExpression<'a>) {
        let direct = matches!(&it.callee, Expression::Identifier(callee)
            if !it.optional && self.names.contains(callee.name.as_str()));
        if !direct {
            walk_js::walk_call_expression(self, it);
            return;
        }
        for argument in &it.arguments {
            self.visit_argument(argument);
        }
    }

    fn visit_identifier_reference(&mut self, it: &IdentifierReference<'a>) {
        if self.names.contains(it.name.as_str()) {
            self.read.insert(it.name.to_string());
        }
    }
}

fn read_otherwise(
    bundle: &Bundle<'_>,
    (key, runtime): (&str, &Module<'_>),
    names: &FxHashSet<String>,
) -> FxHashSet<String> {
    let mut scan = CalledOnly {
        names,
        read: FxHashSet::default(),
    };
    for (id, module) in &bundle.modules {
        if id != key {
            scan.visit_program(module.program());
        }
    }
    for statement in own_statements(runtime) {
        scan.visit_statement(statement);
    }
    scan.read
}

struct Free<'s> {
    scoping: &'s Scoping,
    scope: ScopeId,
    own: Option<SymbolId>,
    closed: bool,
}

impl Free<'_> {
    fn local(&self, symbol: SymbolId) -> bool {
        Some(symbol) != self.own
            && self
                .scoping
                .scope_ancestors(self.scoping.symbol_scope_id(symbol))
                .any(|scope| scope == self.scope)
    }
}

impl<'a> VisitJs<'a> for Free<'_> {
    fn visit_identifier_reference(&mut self, it: &IdentifierReference<'a>) {
        let Some(reference) = it.reference_id.get() else {
            self.closed = false;
            return;
        };
        let reference = self.scoping.get_reference(reference);
        let fits = match reference.symbol_id() {
            Some(symbol) => self.local(symbol),
            None => it.name != "arguments" && !reference.is_write(),
        };
        self.closed &= fits;
    }
}

fn closed(scoping: &Scoping, statement: &Statement<'_>) -> bool {
    let free = |scope: Option<ScopeId>, own: Option<SymbolId>| {
        scope.map(|scope| Free {
            scoping,
            scope,
            own,
            closed: true,
        })
    };
    match statement {
        Statement::FunctionDeclaration(function) => {
            let Some(mut scan) = free(function.scope_id.get(), None) else {
                return false;
            };
            scan.visit_function(function, ScopeFlags::Function);
            scan.closed
        }
        Statement::VariableDeclaration(declaration) => {
            let [declarator] = declaration.declarations.as_slice() else {
                return false;
            };
            match &declarator.init {
                Some(Expression::FunctionExpression(function)) => {
                    let own = function.id.as_ref().and_then(|id| id.symbol_id.get());
                    let Some(mut scan) = free(function.scope_id.get(), own) else {
                        return false;
                    };
                    scan.visit_function(function, ScopeFlags::Function);
                    scan.closed
                }
                Some(Expression::ArrowFunctionExpression(arrow)) => {
                    let Some(mut scan) = free(arrow.scope_id.get(), None) else {
                        return false;
                    };
                    scan.visit_arrow_function_expression(arrow);
                    scan.closed
                }
                _ => false,
            }
        }
        _ => false,
    }
}

fn closed_helpers(runtime: &Module<'_>, names: &FxHashSet<String>) -> FxHashSet<String> {
    let scoping = build_scoping(runtime.program()).scoping;
    let mut out = FxHashSet::default();
    for statement in own_statements(runtime) {
        let declared = declared_names(std::slice::from_ref(statement));
        if let [name] = declared.as_slice() {
            if names.contains(name) && closed(&scoping, statement) {
                out.insert(name.clone());
            }
        }
    }
    out
}

fn fresh_names(programs: &[&Program<'_>], apart: &[String]) -> Vec<(String, String)> {
    let taken = |candidate: &str| {
        programs
            .iter()
            .any(|program| names_anywhere(program, candidate))
    };
    apart
        .iter()
        .map(|name| {
            let chosen = (1..)
                .map(|index| format!("{name}${index}"))
                .find(|candidate| !taken(candidate))
                .unwrap_or_default();
            (name.clone(), chosen)
        })
        .collect()
}

fn rename_free<'a>(alloc: &'a Allocator, bundle: &mut Bundle<'a>, fresh: &[(String, String)]) {
    let wanted: FxHashMap<String, usize> = fresh
        .iter()
        .enumerate()
        .map(|(index, (name, _))| (name.clone(), index))
        .collect();
    let from: Vec<&str> = fresh.iter().map(|(name, _)| name.as_str()).collect();
    let to: Vec<&str> = fresh.iter().map(|(_, name)| name.as_str()).collect();
    for module in bundle.modules.values_mut() {
        if !mentioned(&wanted, module.program().body.iter()).is_empty() {
            rename_parameters(alloc, module.program_mut(), &from, &to);
        }
    }
}

fn copies<'a>(
    alloc: &'a Allocator,
    (mut runtime, kind): (Module<'a>, BundleKind),
    fresh: &[(String, String)],
) -> Option<Vec<Statement<'a>>> {
    let source_text = runtime.program().source_text;
    let kept = AVec::from_iter_in(
        runtime.program_mut().body.drain(..).filter(|statement| {
            !synthetic(statement)
                && declared_names(std::slice::from_ref(statement))
                    .iter()
                    .any(|name| fresh.iter().any(|(apart, _)| apart == name))
        }),
        &alloc,
    );
    let mut copied = module_of(alloc, source_text, kind, kept)?;
    for (from, to) in fresh {
        rename_binding(alloc, copied.program_mut(), from, to);
    }
    Some(copied.program_mut().body.drain(..).collect())
}

fn append<'a>(
    alloc: &'a Allocator,
    runtime: &mut Module<'a>,
    copied: Vec<Statement<'a>>,
) -> Option<()> {
    let program = runtime.program_mut();
    let mut body: AVec<'a, Statement<'a>> =
        AVec::with_capacity_in(program.body.len() + copied.len(), &alloc);
    body.extend(
        program
            .body
            .drain(..)
            .filter(|statement| !synthetic(statement)),
    );
    body.extend(copied);
    let exports = format!(
        "module.exports = {{ {} }};",
        declared_names(&body).join(", ")
    );
    body.extend(synthetic_statements(alloc, exports)?);
    program.body = body;
    Some(())
}

fn exports_live(bundle: &Bundle<'_>, key: &str) -> bool {
    bundle
        .modules
        .get(key)
        .is_some_and(|module| !live_exports(&module.program().body).is_empty())
}

pub fn fold_runtime<'a>(
    alloc: &'a Allocator,
    keep: &mut Bundle<'a>,
    drop: &mut Bundle<'a>,
    key: &str,
) -> Option<()> {
    if exports_live(keep, key) || exports_live(drop, key) {
        return None;
    }
    let dropped = drop.modules.shift_remove(key)?;
    let kept = keep.modules.get(key)?;
    let names: FxHashSet<String> = declared_names(&dropped.program().body)
        .into_iter()
        .collect();
    let mut apart = read_otherwise(keep, (key, kept), &names);
    apart.extend(read_otherwise(drop, (key, &dropped), &names));
    let open: FxHashSet<String> = names.difference(&apart).cloned().collect();
    let closed = closed_helpers(kept, &open);
    apart.extend(open.into_iter().filter(|name| !closed.contains(name)));
    if apart.is_empty() {
        return Some(());
    }
    let order: Vec<String> = declared_names(&dropped.program().body)
        .into_iter()
        .filter(|name| apart.contains(name))
        .collect();
    let programs: Vec<&Program<'a>> = keep
        .modules
        .values()
        .chain(drop.modules.values())
        .chain(std::iter::once(&dropped))
        .map(Module::program)
        .collect();
    let fresh = fresh_names(&programs, &order);
    let copied = copies(alloc, (dropped, keep.kind), &fresh)?;
    append(alloc, keep.modules.get_mut(key)?, copied)?;
    rename_free(alloc, drop, &fresh);
    Some(())
}

struct Uses<'s, 'n> {
    scoping: &'s Scoping,
    names: &'n FxHashSet<String>,
    runtime: bool,
    otherwise: bool,
}

impl Uses<'_, '_> {
    fn target(&self, id: &IdentifierReference<'_>) -> bool {
        if !self.names.contains(id.name.as_str()) {
            return false;
        }
        let symbol = id
            .reference_id
            .get()
            .and_then(|reference| self.scoping.get_reference(reference).symbol_id());
        match symbol {
            None => true,
            Some(symbol) => {
                self.runtime && self.scoping.symbol_scope_id(symbol) == self.scoping.root_scope_id()
            }
        }
    }
}

impl<'a> VisitJs<'a> for Uses<'_, '_> {
    fn visit_call_expression(&mut self, it: &CallExpression<'a>) {
        let direct = !it.optional
            && matches!(&it.callee, Expression::Identifier(callee) if self.target(callee));
        if !direct {
            walk_js::walk_call_expression(self, it);
            return;
        }
        for argument in &it.arguments {
            self.visit_argument(argument);
        }
    }

    fn visit_identifier_reference(&mut self, it: &IdentifierReference<'a>) {
        self.otherwise |= self.target(it);
    }
}

fn only_called(program: &Program<'_>, names: &FxHashSet<String>, runtime: bool) -> bool {
    let scoping = build_scoping(program).scoping;
    let mut uses = Uses {
        scoping: &scoping,
        names,
        runtime,
        otherwise: false,
    };
    for statement in &program.body {
        let declares = single_name(statement).is_some_and(|name| names.contains(&name));
        match (runtime, declares || synthetic(statement)) {
            (true, true) => {}
            (true, false) => {
                let mut any = Mentions {
                    uses: &uses,
                    found: false,
                };
                any.visit_statement(statement);
                uses.otherwise |= any.found;
            }
            (false, _) => uses.visit_statement(statement),
        }
    }
    !uses.otherwise
}

struct Mentions<'u, 's, 'n> {
    uses: &'u Uses<'s, 'n>,
    found: bool,
}

impl<'a> VisitJs<'a> for Mentions<'_, '_, '_> {
    fn visit_identifier_reference(&mut self, it: &IdentifierReference<'a>) {
        self.found |= self.uses.target(it);
    }
}

fn single_name(statement: &Statement<'_>) -> Option<String> {
    match declared_names(std::slice::from_ref(statement)).as_slice() {
        [name] => Some(name.clone()),
        _ => None,
    }
}

fn closed_shapes(runtime: &Module<'_>) -> Vec<(String, String)> {
    let program = runtime.program();
    let scoping = build_scoping(program).scoping;
    own_statements(runtime)
        .filter(|statement| closed(&scoping, statement))
        .filter_map(|statement| {
            let name = single_name(statement)?;
            Some((name, statement_shape(statement, program.source_text)))
        })
        .collect()
}

fn equal_helpers(kept: &Module<'_>, later: &Module<'_>) -> Vec<(String, String)> {
    let kept_names: FxHashSet<String> = declared_names(&kept.program().body).into_iter().collect();
    let mut by_shape: FxHashMap<String, Option<String>> = FxHashMap::default();
    for (name, shape) in closed_shapes(kept) {
        by_shape
            .entry(shape)
            .and_modify(|seen| *seen = None)
            .or_insert(Some(name));
    }
    closed_shapes(later)
        .into_iter()
        .filter(|(name, _)| !kept_names.contains(name))
        .filter_map(|(name, shape)| Some((name, by_shape.get(&shape)?.clone()?)))
        .collect()
}

fn without_helpers<'a>(
    alloc: &'a Allocator,
    runtime: &mut Module<'a>,
    names: &FxHashSet<String>,
) -> Option<()> {
    let program = runtime.program_mut();
    let mut body: AVec<'a, Statement<'a>> = AVec::with_capacity_in(program.body.len(), &alloc);
    body.extend(program.body.drain(..).filter(|statement| {
        !synthetic(statement) && single_name(statement).is_none_or(|name| !names.contains(&name))
    }));
    let exports = format!(
        "module.exports = {{ {} }};",
        declared_names(&body).join(", ")
    );
    body.extend(synthetic_statements(alloc, exports)?);
    program.body = body;
    Some(())
}

pub fn unify_helpers<'a>(
    alloc: &'a Allocator,
    keep: &Bundle<'a>,
    drop: &mut Bundle<'a>,
    key: &str,
) -> Option<()> {
    let pairs = equal_helpers(keep.modules.get(key)?, drop.modules.get(key)?);
    if pairs.is_empty() {
        return Some(());
    }
    let names: FxHashSet<String> = pairs.iter().map(|(name, _)| name.clone()).collect();
    let later = drop.modules.get(key)?;
    let called_only = drop
        .modules
        .iter()
        .all(|(id, module)| id == key || only_called(module.program(), &names, false));
    if !called_only || !only_called(later.program(), &names, true) {
        return None;
    }
    let clashes = drop.modules.iter().any(|(id, module)| {
        id != key
            && pairs
                .iter()
                .any(|(_, to)| names_anywhere(module.program(), to))
    });
    if clashes {
        return None;
    }
    let from: Vec<&str> = pairs.iter().map(|(name, _)| name.as_str()).collect();
    let to: Vec<&str> = pairs.iter().map(|(_, name)| name.as_str()).collect();
    let wanted: FxHashMap<String, usize> = names.iter().map(|name| (name.clone(), 0)).collect();
    for (id, module) in drop.modules.iter_mut() {
        let named = !mentioned(&wanted, module.program().body.iter()).is_empty();
        if id != key && named && rename_parameters(alloc, module.program_mut(), &from, &to) == 0 {
            return None;
        }
    }
    without_helpers(alloc, drop.modules.get_mut(key)?, &names)
}
