use oxc_allocator::Allocator;
use oxc_ast::ast::*;
use oxc_ast_visit::{walk_js, VisitJs};
use untwist_core::pass::{Note, NoteKind};

use crate::matcher::named_const_member;
use crate::model::RequireParam;
use crate::model::{Bundle, BundleKind, Module};
use crate::runtime::{runtime_binding_import, runtime_id, runtime_path, synthetic_statements};
use crate::webpack::default_require::convert_default_require;
use crate::webpack::esm::convert_esm;
use crate::webpack::esm_proof::convert_proven;
use crate::webpack::interop::convert_interop;
use crate::webpack::require_paths::{replace_require_paths_with, rewritten, Converted, Routing};
use crate::webpack::runtime_module::{needed, prepend, runtime_module};
use crate::webpack::shield::{restore, shield, Shield};
use crate::webpack::var_injection::{inline_proven_injection, inline_var_injections};
use rustc_hash::FxHashSet;
use untwist_core::semantic::build_scoping;

pub const UNHANDLED_HELPERS: [&str; 6] = ["S", "d", "e", "n", "r", "t"];

const PASS_NAME: &str = "unpack";
const RUNTIME_BINDING: &str = "__webpack_require__";

#[derive(Default)]
struct Helpers {
    found: Vec<&'static str>,
}

impl<'a> VisitJs<'a> for Helpers {
    fn visit_expression(&mut self, it: &Expression<'a>) {
        for helper in UNHANDLED_HELPERS {
            if !self.found.contains(&helper) && named_const_member(it, "require", helper) {
                self.found.push(helper);
            }
        }
        walk_js::walk_expression(self, it);
    }
}

fn helper_notes(id: &str, module: &Module<'_>) -> Vec<Note> {
    let mut helpers = Helpers::default();
    helpers.visit_program(module.program());
    helpers.found.sort_unstable();
    helpers
        .found
        .into_iter()
        .map(|helper| Note {
            pass: PASS_NAME,
            kind: NoteKind::Refusal,
            message: crate::notes::unhandled_helper(id, helper),
            span: None,
        })
        .collect()
}

fn shield_all<'a>(
    alloc: &'a Allocator,
    bundle: &mut Bundle<'a>,
    conservative: bool,
) -> Vec<Option<Shield>> {
    let plain = bundle
        .modules
        .values()
        .all(|module| module.runtime_require == RequireParam::Default);
    if conservative || plain {
        return Vec::new();
    }
    let fresh = std::mem::take(&mut bundle.fresh_requires);
    bundle
        .modules
        .iter_mut()
        .map(|(id, module)| {
            let name = fresh
                .iter()
                .find(|(at, _)| at == id)
                .map(|(_, name)| name.as_str());
            shield(alloc, module, name)
        })
        .collect()
}

fn restore_all<'a>(
    alloc: &'a Allocator,
    bundle: &mut Bundle<'a>,
    shields: &[Option<Shield>],
    converted: &Converted,
) {
    let none = FxHashSet::default();
    for (index, (module, shield)) in bundle.modules.values_mut().zip(shields).enumerate() {
        if let Some(shield) = shield {
            let spans = rewritten(converted, index).map_or(&none, |rewritten| &rewritten.converted);
            restore(alloc, module, shield, spans);
        }
    }
}

#[derive(Default)]
struct NameUse<'n> {
    name: &'n str,
    found: bool,
}

impl<'a> VisitJs<'a> for NameUse<'_> {
    fn visit_identifier_reference(&mut self, it: &IdentifierReference<'a>) {
        self.found |= it.name == self.name;
    }

    fn visit_binding_identifier(&mut self, it: &BindingIdentifier<'a>) {
        self.found |= it.name == self.name;
    }
}

fn name_in_use(program: &Program<'_>, name: &str) -> bool {
    let mut uses = NameUse { name, found: false };
    uses.visit_program(program);
    uses.found
}

fn free_binding(program: &Program<'_>) -> String {
    (1..)
        .map(|index| match index {
            1 => RUNTIME_BINDING.to_string(),
            index => format!("{RUNTIME_BINDING}{index}"),
        })
        .find(|name| !name_in_use(program, name))
        .unwrap_or_default()
}

fn routing(bundle: &Bundle<'_>, conservative: bool) -> Option<Routing> {
    let path = runtime_path(BundleKind::Webpack);
    let taken = bundle
        .modules
        .contains_key(&runtime_id(BundleKind::Webpack));
    if conservative || taken || bundle.modules.values().any(|m| m.path.as_str() == path) {
        return None;
    }
    let runtime = bundle.runtime.as_ref()?;
    let mut available: Vec<String> = Vec::new();
    for helper in &runtime.helpers {
        let routable = !available.contains(&helper.name) && needed(runtime, &helper.name).is_some();
        if routable {
            available.push(helper.name.clone());
        }
    }
    if available.is_empty() {
        return None;
    }
    let mut texts: Vec<(*const u8, bool)> = Vec::new();
    let bindings = bundle
        .modules
        .values()
        .map(|module| {
            let text = module.program().source_text;
            let key = text.as_ptr();
            let clear = match texts.iter().find(|(at, _)| *at == key) {
                Some((_, clear)) => *clear,
                None => {
                    let clear = !text.contains(RUNTIME_BINDING);
                    texts.push((key, clear));
                    clear
                }
            };
            match clear {
                true => RUNTIME_BINDING.to_string(),
                false => free_binding(module.program()),
            }
        })
        .collect();
    Some(Routing {
        available,
        bindings,
    })
}

fn runtime_plan(bundle: &Bundle<'_>, converted: &Converted) -> Option<Vec<usize>> {
    let runtime = bundle.runtime.as_ref()?;
    let mut indices: FxHashSet<usize> = FxHashSet::default();
    for (_, rewritten) in converted {
        for helper in &rewritten.helpers {
            indices.extend(needed(runtime, helper)?);
        }
    }
    if indices.is_empty() {
        return None;
    }
    let mut indices: Vec<usize> = indices.into_iter().collect();
    indices.sort_unstable();
    Some(indices)
}

fn attach_runtime<'a>(
    alloc: &'a Allocator,
    bundle: &mut Bundle<'a>,
    indices: &[usize],
    bindings: &[Option<String>],
) -> Option<()> {
    let runtime = runtime_module(alloc, bundle, indices)?;
    for (module, binding) in bundle.modules.values_mut().zip(bindings) {
        if let Some(binding) = binding {
            let import = runtime_binding_import(alloc, &module.path, BundleKind::Webpack, binding)?;
            let mut first = oxc_allocator::Vec::new_in(&alloc);
            first.push(import);
            prepend(module.program_mut(), first);
        }
    }
    bundle.modules.insert(runtime.id.clone(), runtime);
    Some(())
}

fn reads_free_require(program: &Program<'_>) -> bool {
    build_scoping(program)
        .scoping
        .root_unresolved_references()
        .contains_key("require")
}

fn require_in_esm<'a>(alloc: &'a Allocator, bundle: &mut Bundle<'a>) {
    for module in bundle.modules.values_mut() {
        if !module.is_esm() || !reads_free_require(module.program()) {
            continue;
        }
        let create = free_name(module.program(), "createRequire");
        let text = format!(
            "import {{ createRequire as {create} }} from \"module\";\nconst require = {create}(import.meta.url);"
        );
        if let Some(prologue) = synthetic_statements(alloc, text) {
            prepend(module.program_mut(), prologue);
        }
    }
}

fn free_name(program: &Program<'_>, base: &str) -> String {
    (1..)
        .map(|index| match index {
            1 => base.to_string(),
            index => format!("{base}{index}"),
        })
        .find(|name| !name_in_use(program, name))
        .unwrap_or_default()
}

fn missing_notes(missing: Vec<(String, String)>) -> Vec<Note> {
    missing
        .into_iter()
        .map(|(id, target)| Note {
            pass: PASS_NAME,
            kind: NoteKind::Refusal,
            message: crate::notes::require_missing(id, target),
            span: None,
        })
        .collect()
}

pub fn apply_transforms<'a>(alloc: &'a Allocator, bundle: &mut Bundle<'a>) -> Vec<Note> {
    apply_transforms_with(alloc, bundle, false)
}

fn convert_modules<'a>(alloc: &'a Allocator, bundle: &mut Bundle<'a>, conservative: bool) {
    if conservative {
        for module in bundle.modules.values_mut() {
            convert_esm(alloc, module);
        }
        convert_default_require(alloc, bundle);
        return;
    }
    convert_interop(alloc, bundle);
    convert_proven(alloc, bundle);
}

pub fn apply_transforms_with<'a>(
    alloc: &'a Allocator,
    bundle: &mut Bundle<'a>,
    conservative: bool,
) -> Vec<Note> {
    if bundle.kind != BundleKind::Webpack {
        return Vec::new();
    }
    let shields = shield_all(alloc, bundle, conservative);
    for module in bundle.modules.values_mut() {
        if conservative {
            inline_var_injections(alloc, module);
        } else {
            inline_proven_injection(alloc, module);
        }
    }
    convert_modules(alloc, bundle, conservative);
    let tracked: Vec<bool> = shields.iter().map(Option::is_some).collect();
    let routing = routing(bundle, conservative);
    let (missing, converted) =
        replace_require_paths_with(alloc, bundle, !conservative, &tracked, routing.as_ref());
    let plan = runtime_plan(bundle, &converted);
    let bindings: Vec<Option<String>> = match &routing {
        Some(routing) => (0..bundle.modules.len())
            .map(|index| {
                rewritten(&converted, index)
                    .filter(|rewritten| !rewritten.helpers.is_empty())
                    .map(|_| routing.bindings[index].clone())
            })
            .collect(),
        None => Vec::new(),
    };
    let mut notes = missing_notes(missing);
    for (id, module) in &bundle.modules {
        notes.extend(helper_notes(id, module));
    }
    restore_all(alloc, bundle, &shields, &converted);
    if let Some(indices) = plan.filter(|_| bindings.iter().any(Option::is_some)) {
        attach_runtime(alloc, bundle, &indices, &bindings);
    }
    if !conservative {
        require_in_esm(alloc, bundle);
    }
    notes
}
