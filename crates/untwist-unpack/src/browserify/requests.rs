use oxc_ast::ast::*;
use oxc_ast_visit::{walk_js, VisitJs};
use oxc_semantic::Scoping;
use oxc_syntax::symbol::SymbolId;
use rustc_hash::FxHashSet;
use untwist_core::ast_util::prop::number_key;
use untwist_core::host_globals::NODE_BUILTIN_MODULES;

use crate::browserify::detect::{module_entries, runs_like_a_file, ModuleEntry, INHERITED_KEYS};
use crate::browserify::link::FALSY_IDS;

const NODE_SCHEME: &str = "node:";
const SCOPE_PREFIX: char = '@';

pub struct Unloaded {
    pub id: String,
    pub what: String,
}

fn unloaded(id: &str, what: String) -> Unloaded {
    Unloaded {
        id: id.to_string(),
        what,
    }
}

fn is_builtin(request: &str) -> bool {
    request.starts_with(NODE_SCHEME) || NODE_BUILTIN_MODULES.contains(&request)
}

fn plain_segment(segment: &str) -> bool {
    !segment.is_empty() && segment != "." && segment != ".."
}

fn package_name(request: &str) -> Option<&str> {
    if request.starts_with(['.', '/']) || request.contains('\\') {
        return None;
    }
    if !request.split('/').all(plain_segment) {
        return None;
    }
    let segments = match request.starts_with(SCOPE_PREFIX) {
        true => 2,
        false => 1,
    };
    let end = request
        .match_indices('/')
        .nth(segments - 1)
        .map_or(request.len(), |(at, _)| at);
    Some(&request[..end])
}

struct Tree<'s> {
    ids: FxHashSet<&'s str>,
    packages: FxHashSet<&'s str>,
    every_module_reached: bool,
}

impl Tree<'_> {
    fn leaves_the_tree(&self, request: &str) -> bool {
        self.every_module_reached
            && package_name(request).is_some_and(|name| !self.packages.contains(name))
    }
}

fn reached_from<'s>(entry: &'s str, modules: &'s [ModuleEntry<'_, '_>]) -> FxHashSet<&'s str> {
    let mut reached: FxHashSet<&str> = FxHashSet::default();
    let mut stack = vec![entry];
    while let Some(id) = stack.pop() {
        if !reached.insert(id) {
            continue;
        }
        let dependencies = modules
            .iter()
            .filter(|module| module.id == id)
            .flat_map(|module| module.dependencies.iter());
        stack.extend(dependencies.map(|(dependency, _)| dependency.as_str()));
    }
    reached
}

fn tree<'s>(entry: &'s str, modules: &'s [ModuleEntry<'_, '_>]) -> Tree<'s> {
    let reached = reached_from(entry, modules);
    Tree {
        ids: modules.iter().map(|module| module.id.as_str()).collect(),
        packages: modules
            .iter()
            .flat_map(|module| module.dependencies.iter())
            .filter_map(|(_, request)| package_name(request))
            .collect(),
        every_module_reached: modules
            .iter()
            .all(|module| reached.contains(module.id.as_str())),
    }
}

struct Calls<'s> {
    scoping: &'s Scoping,
    require: SymbolId,
    dependencies: &'s [(String, String)],
    tree: &'s Tree<'s>,
    found: Option<String>,
}

impl Calls<'_> {
    fn is_require(&self, callee: &Expression<'_>) -> bool {
        let Expression::Identifier(id) = callee else {
            return false;
        };
        id.reference_id
            .get()
            .and_then(|reference| self.scoping.get_reference(reference).symbol_id())
            == Some(self.require)
    }

    fn judge(&self, request: Option<String>) -> Option<String> {
        let Some(request) = request else {
            return Some("it calls require with a request that is not a literal".to_string());
        };
        if self.dependencies.iter().any(|(_, key)| *key == request) {
            return None;
        }
        if INHERITED_KEYS.contains(&request.as_str()) {
            return Some(format!(
                "it requires {request:?}, a name its dependency map inherits"
            ));
        }
        let outside = is_builtin(&request) || self.tree.leaves_the_tree(&request);
        if outside || self.tree.ids.contains(request.as_str()) {
            return None;
        }
        Some(format!(
            "it requires {request:?}, which neither its dependency map nor the bundle holds"
        ))
    }
}

fn request(arguments: &[Argument<'_>]) -> Option<String> {
    match arguments.first()? {
        Argument::StringLiteral(literal) => literal.value.as_str().map(str::to_string),
        Argument::NumericLiteral(literal) => Some(number_key(literal.value)),
        _ => None,
    }
}

impl<'a> VisitJs<'a> for Calls<'_> {
    fn visit_call_expression(&mut self, it: &CallExpression<'a>) {
        if self.found.is_none() && self.is_require(&it.callee) {
            self.found = self.judge(request(&it.arguments));
        }
        walk_js::walk_call_expression(self, it);
    }
}

fn require_symbol(module: &ModuleEntry<'_, '_>) -> Option<SymbolId> {
    match &module.params.items.first()?.pattern {
        BindingPattern::BindingIdentifier(id) => id.symbol_id.get(),
        _ => None,
    }
}

fn map_fault(module: &ModuleEntry<'_, '_>, ids: &FxHashSet<&str>) -> Option<String> {
    module.dependencies.iter().find_map(|(id, request)| {
        if FALSY_IDS.contains(&id.as_str()) {
            return Some(format!(
                "its dependency map sends {request:?} to {id:?}, which browserify reads as no id"
            ));
        }
        (!ids.contains(id.as_str())).then(|| {
            format!(
                "its dependency map sends {request:?} to {id:?}, which is not a module of the bundle"
            )
        })
    })
}

fn module_fault(
    scoping: &Scoping,
    module: &ModuleEntry<'_, '_>,
    tree: &Tree<'_>,
) -> Option<String> {
    if INHERITED_KEYS.contains(&module.id.as_str()) {
        return Some("its id is a name every object inherits".to_string());
    }
    if !runs_like_a_file(module.function) {
        return Some(
            "its factory is async, a generator or takes a pattern or default parameter".to_string(),
        );
    }
    if let Some(fault) = map_fault(module, &tree.ids) {
        return Some(fault);
    }
    let mut calls = Calls {
        scoping,
        require: require_symbol(module)?,
        dependencies: &module.dependencies,
        tree,
        found: None,
    };
    calls.visit_function_body(module.body);
    calls.found
}

pub fn unloaded_module(scoping: &Scoping, expr: &Expression<'_>) -> Option<Unloaded> {
    let (modules, (entry, later)) = module_entries(expr)?;
    let tree = tree(&entry, &modules);
    let entries = std::iter::once(Some(&entry)).chain(later.iter().map(Option::as_ref));
    for id in entries {
        let Some(id) = id else {
            return Some(unloaded("?", "an entry is not a literal id".to_string()));
        };
        if !tree.ids.contains(id.as_str()) {
            let what = "it is an entry but not a module of the bundle".to_string();
            return Some(unloaded(id, what));
        }
    }
    modules.iter().find_map(|module| {
        module_fault(scoping, module, &tree).map(|what| unloaded(&module.id, what))
    })
}
