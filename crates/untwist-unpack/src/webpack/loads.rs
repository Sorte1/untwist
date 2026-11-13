use oxc_ast::ast::*;
use oxc_ast_visit::{walk_js, VisitJs};
use rustc_hash::{FxHashMap, FxHashSet};

use crate::matcher::literal_id;
use crate::model::{Bundle, BundleKind};
use crate::notes::{
    chunk_computed, chunk_missing, chunk_module_computed, chunk_module_early, ensure_escapes,
    module_missing,
};
use crate::webpack::gate::module_require;

const ENSURE: &str = "e";
const BIND: &str = "bind";
const FAKE_NAMESPACE: &str = "t";
const THEN: &str = "then";
const ALL: &str = "all";
const PROMISE: &str = "Promise";

struct Site {
    required: String,
    loaded: Vec<String>,
}

struct Loads<'n> {
    require: &'n str,
    promise_shadowed: bool,
    ids: Vec<Option<String>>,
    ensures: usize,
    sites: Vec<Site>,
    computed: bool,
    loaded: Vec<String>,
}

#[derive(Default)]
struct Bindings {
    names: FxHashSet<String>,
}

impl<'a> VisitJs<'a> for Bindings {
    fn visit_binding_identifier(&mut self, it: &BindingIdentifier<'a>) {
        self.names.insert(it.name.to_string());
    }
}

fn first_literal(arguments: &[Argument<'_>], at: usize) -> Option<String> {
    arguments.get(at)?.as_expression().and_then(literal_id)
}

fn member_of<'k, 'a>(expr: &'k Expression<'a>, name: &str) -> Option<&'k Expression<'a>> {
    match expr.without_parentheses() {
        Expression::StaticMemberExpression(member) if member.property.name == name => {
            Some(member.object.without_parentheses())
        }
        _ => None,
    }
}

impl Loads<'_> {
    fn is_require(&self, expr: &Expression<'_>) -> bool {
        matches!(expr.without_parentheses(), Expression::Identifier(id) if id.name == self.require)
    }

    fn ensured(&self, call: &CallExpression<'_>) -> Option<Option<String>> {
        let object = member_of(&call.callee, ENSURE)?;
        if !self.is_require(object) {
            return None;
        }
        Some(match &call.arguments[..] {
            [only] => only.as_expression().and_then(literal_id),
            _ => None,
        })
    }

    fn load_promise(&self, expr: &Expression<'_>) -> Option<Vec<String>> {
        let Expression::CallExpression(call) = expr.without_parentheses() else {
            return None;
        };
        if let Some(id) = self.ensured(call) {
            return Some(id.into_iter().collect());
        }
        if let Some(object) = member_of(&call.callee, THEN) {
            return self.load_promise(object);
        }
        let object = member_of(&call.callee, ALL)?;
        let global = matches!(object, Expression::Identifier(id) if id.name == PROMISE);
        if !global || self.promise_shadowed {
            return None;
        }
        let [only] = &call.arguments[..] else {
            return None;
        };
        let Some(Expression::ArrayExpression(list)) =
            only.as_expression().map(Expression::without_parentheses)
        else {
            return None;
        };
        Some(
            list.elements
                .iter()
                .filter_map(ArrayExpressionElement::as_expression)
                .filter_map(|element| self.load_promise(element))
                .flatten()
                .collect(),
        )
    }

    fn require_site(&mut self, id: Option<String>) {
        match id {
            Some(required) => self.sites.push(Site {
                required,
                loaded: self.loaded.clone(),
            }),
            None => self.computed = true,
        }
    }

    fn record(&mut self, it: &CallExpression<'_>) {
        if self.is_require(&it.callee) {
            self.require_site(first_literal(&it.arguments, 0));
        }
        let Expression::StaticMemberExpression(member) = it.callee.without_parentheses() else {
            return;
        };
        let name = member.property.name.as_str();
        if name == BIND && self.is_require(&member.object) {
            self.require_site(first_literal(&it.arguments, 1));
        }
        if name == FAKE_NAMESPACE && self.is_require(&member.object) {
            self.require_site(first_literal(&it.arguments, 0));
        }
        let fake_bound = name == BIND
            && member_of(&member.object, FAKE_NAMESPACE)
                .is_some_and(|inner| self.is_require(inner));
        if fake_bound {
            self.require_site(first_literal(&it.arguments, 1));
        }
        if let Some(id) = self.ensured(it) {
            self.ids.push(id);
        }
    }

    fn then_callback(&mut self, it: &CallExpression<'_>) -> Option<Vec<String>> {
        let object = member_of(&it.callee, THEN)?;
        it.arguments.first()?;
        self.load_promise(object)
    }
}

impl<'a> VisitJs<'a> for Loads<'_> {
    fn visit_call_expression(&mut self, it: &CallExpression<'a>) {
        self.record(it);
        let Some(chunks) = self.then_callback(it) else {
            walk_js::walk_call_expression(self, it);
            return;
        };
        self.visit_expression(&it.callee);
        let depth = self.loaded.len();
        for (at, argument) in it.arguments.iter().enumerate() {
            if at == 0 {
                self.loaded.extend(chunks.iter().cloned());
            }
            self.visit_argument(argument);
            self.loaded.truncate(depth);
        }
    }

    fn visit_static_member_expression(&mut self, it: &StaticMemberExpression<'a>) {
        if it.property.name == ENSURE && self.is_require(&it.object) {
            self.ensures += 1;
        }
        walk_js::walk_static_member_expression(self, it);
    }
}

struct ModuleLoads {
    id: String,
    sites: Vec<Site>,
    computed: bool,
}

fn scanned(
    bundle: &Bundle<'_>,
    supplied: impl Fn(&String) -> bool,
) -> Result<Vec<ModuleLoads>, String> {
    let merged = !bundle.chunk_ids.is_empty() && bundle.runtime.is_some();
    let mut all = Vec::new();
    for (id, module) in &bundle.modules {
        let Some(require) = module_require(bundle, id, module) else {
            continue;
        };
        let mut bindings = Bindings::default();
        bindings.visit_program(module.program());
        let mut loads = Loads {
            require,
            promise_shadowed: bindings.names.contains(PROMISE),
            ids: Vec::new(),
            ensures: 0,
            sites: Vec::new(),
            computed: false,
            loaded: Vec::new(),
        };
        loads.visit_program(module.program());
        if loads.ensures != loads.ids.len() {
            return Err(ensure_escapes(id));
        }
        for chunk in &loads.ids {
            let Some(chunk) = chunk else {
                return Err(chunk_computed(id));
            };
            if !supplied(chunk) {
                return Err(chunk_missing(id, chunk));
            }
        }
        if let Some(missing) = loads
            .sites
            .iter()
            .find(|site| merged && !bundle.modules.contains_key(site.required.as_str()))
        {
            return Err(module_missing(id, &missing.required));
        }
        all.push(ModuleLoads {
            id: id.clone(),
            sites: loads.sites,
            computed: loads.computed,
        });
    }
    Ok(all)
}

type Chunks<'b> = FxHashSet<&'b str>;

struct Lazy<'b> {
    eager: Chunks<'b>,
    holders: FxHashMap<&'b str, Vec<&'b [String]>>,
}

impl<'b> Lazy<'b> {
    fn of(bundle: &'b Bundle<'_>) -> Self {
        let mut eager: Chunks<'b> = bundle
            .startup_chunks
            .iter()
            .chain(&bundle.installed_chunks)
            .map(String::as_str)
            .collect();
        for file in bundle.chunk_files.iter().filter(|file| file.eager) {
            eager.extend(file.ids.iter().map(String::as_str));
        }
        let mut holders: FxHashMap<&str, Vec<&[String]>> = FxHashMap::default();
        let mut available: FxHashSet<&str> = FxHashSet::default();
        for file in &bundle.chunk_files {
            let loaded = file.eager || file.ids.iter().any(|id| eager.contains(id.as_str()));
            for module in &file.modules {
                match loaded {
                    true => {
                        available.insert(module);
                    }
                    false => holders.entry(module).or_default().push(&file.ids),
                }
            }
        }
        holders.retain(|module, _| !available.contains(module));
        Self { eager, holders }
    }

    fn base(&self, module: &str) -> Chunks<'b> {
        let mut base = self.eager.clone();
        let mut files = self.holders.get(module).into_iter().flatten();
        if let Some(first) = files.next() {
            let mut common: Chunks<'b> = first.iter().map(String::as_str).collect();
            for ids in files {
                common.retain(|id| ids.iter().any(|other| other == id));
            }
            base.extend(common);
        }
        base
    }

    fn present(&self, module: &str, loaded: &Chunks<'_>) -> bool {
        self.holders.get(module).is_none_or(|files| {
            files
                .iter()
                .any(|ids| ids.iter().any(|id| loaded.contains(id.as_str())))
        })
    }
}

fn with_site<'b>(running: &Chunks<'b>, site: &'b Site) -> Chunks<'b> {
    let mut loaded = running.clone();
    loaded.extend(site.loaded.iter().map(String::as_str));
    loaded
}

fn loaded_when_run<'b>(
    lazy: &Lazy<'b>,
    loads: &'b [ModuleLoads],
) -> FxHashMap<&'b str, Chunks<'b>> {
    let mut known: FxHashMap<&str, Chunks<'b>> = loads
        .iter()
        .filter(|found| !lazy.holders.contains_key(found.id.as_str()))
        .map(|found| (found.id.as_str(), lazy.base(&found.id)))
        .collect();
    let mut incoming: FxHashMap<&str, Vec<(&str, &Site)>> = FxHashMap::default();
    for found in loads {
        for site in &found.sites {
            if lazy.holders.contains_key(site.required.as_str()) {
                incoming
                    .entry(site.required.as_str())
                    .or_default()
                    .push((found.id.as_str(), site));
            }
        }
    }
    let mut targets: Vec<&str> = incoming.keys().copied().collect();
    targets.sort_unstable();
    loop {
        let mut changed = false;
        for &module in &targets {
            let mut meet: Option<Chunks<'b>> = None;
            for &(from, site) in &incoming[module] {
                let Some(running) = known.get(from) else {
                    continue;
                };
                let loaded = with_site(running, site);
                meet = Some(match meet {
                    None => loaded,
                    Some(mut kept) => {
                        kept.retain(|id| loaded.contains(id));
                        kept
                    }
                });
            }
            let Some(meet) = meet else {
                continue;
            };
            let mut next = lazy.base(module);
            next.extend(meet);
            if known.get(module) != Some(&next) {
                known.insert(module, next);
                changed = true;
            }
        }
        if !changed {
            return known;
        }
    }
}

fn loaded_in_order(bundle: &Bundle<'_>, loads: &[ModuleLoads]) -> Result<(), String> {
    if bundle.runtime.is_none() {
        return Ok(());
    }
    let lazy = Lazy::of(bundle);
    if lazy.holders.is_empty() {
        return Ok(());
    }
    if let Some(found) = loads.iter().find(|found| found.computed) {
        return Err(chunk_module_computed(&found.id));
    }
    let known = loaded_when_run(&lazy, loads);
    for found in loads {
        let Some(running) = known.get(found.id.as_str()) else {
            continue;
        };
        if let Some(site) = found
            .sites
            .iter()
            .find(|site| !lazy.present(&site.required, &with_site(running, site)))
        {
            return Err(chunk_module_early(&found.id, &site.required));
        }
    }
    Ok(())
}

pub fn chunks_supplied(bundle: &Bundle<'_>) -> Result<(), String> {
    if bundle.kind != BundleKind::Webpack {
        return Ok(());
    }
    let supplied = |chunk: &String| {
        bundle.chunk_ids.contains(chunk) || bundle.installed_chunks.contains(chunk)
    };
    let started = bundle.runtime.is_some();
    if let Some(chunk) = bundle
        .startup_chunks
        .iter()
        .find(|chunk| started && !supplied(chunk))
    {
        return Err(chunk_missing(
            bundle.entry_id.as_deref().unwrap_or("entry"),
            chunk,
        ));
    }
    let loads = scanned(bundle, supplied)?;
    loaded_in_order(bundle, &loads)
}
