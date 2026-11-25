mod component;
mod css;
mod dom;
mod html;
mod markup;
mod runtime;
mod script;
mod text;

use oxc_allocator::Allocator;
use oxc_ast::AstKind;
use oxc_semantic::SemanticBuilder;

use component::Shared;
use runtime::Runtime;
use script::{component_params, Class};
use text::Rewriter;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Component {
    pub name: String,
    pub text: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Restore {
    pub components: Vec<Component>,
    pub kept: Vec<(String, String)>,
}

const MARKERS: &[&str] = &["from_html", "svelte/internal", "from_svg"];

pub fn may_hold_components(code: &str) -> bool {
    MARKERS.iter().any(|marker| code.contains(marker))
}

pub fn restore(code: &str) -> Restore {
    if !may_hold_components(code) {
        return Restore::default();
    }
    let alloc = Allocator::default();
    let parsed = untwist_core::parse::parse_program(&alloc, code);
    if untwist_core::parse::has_error(&parsed.diagnostics) {
        return Restore::default();
    }
    let program = alloc.alloc(parsed.program);
    let semantic = SemanticBuilder::new()
        .with_build_nodes(true)
        .build(program)
        .semantic;
    let scoping = semantic.scoping();
    let runtime = Runtime::of(program, scoping, semantic.nodes());
    if runtime.is_empty() {
        return Restore::default();
    }
    let collected = script::collect(program, &runtime, scoping);
    let rw = Rewriter::new(code, collected.sites, collected.refs);
    for symbol in runtime
        .namespaces
        .iter()
        .chain(runtime.symbols.keys())
        .chain(runtime.ambiguous.iter())
        .chain(collected.facts.props_objects.iter())
    {
        rw.forbid(*symbol);
    }
    for (symbol, class) in &collected.facts.classes {
        if matches!(class, Class::Signal | Class::Getter) {
            rw.forbid(*symbol);
        }
    }
    let nodes = semantic.nodes();
    let mut components = Vec::new();
    let mut taken: Vec<String> = Vec::new();
    for node in nodes.iter() {
        let AstKind::Function(function) = node.kind() else {
            continue;
        };
        if component_params(function, &runtime, scoping).is_none() {
            continue;
        }
        let base = function
            .id
            .as_ref()
            .map_or("Component", |id| id.name.as_str());
        let base = match base.starts_with(|c: char| c.is_ascii_uppercase()) {
            true => base.to_string(),
            false => "Component".to_string(),
        };
        let mut name = base.clone();
        let mut counter = 1;
        while taken.contains(&name) {
            counter += 1;
            name = format!("{base}{counter}");
        }
        taken.push(name.clone());
        let top = matches!(
            nodes.parent_kind(node.id()),
            AstKind::Program(_)
                | AstKind::ExportDefaultDeclaration(_)
                | AstKind::ExportDeclaration(_)
        );
        components.push((function, name, top));
    }
    let shared = Shared {
        program,
        scoping,
        nodes,
        runtime: &runtime,
        facts: &collected.facts,
        rw: &rw,
        keys: &collected.keys,
        components: components
            .iter()
            .map(|(function, name, _)| (*function, name.clone()))
            .collect(),
    };
    let mut out = Restore::default();
    for (function, name, top) in &components {
        let esm = runtime.imports_svelte && *top;
        match shared.restore(function, esm) {
            Ok(text) => out.components.push(Component {
                name: name.clone(),
                text,
            }),
            Err(reason) => out.kept.push((name.clone(), reason)),
        }
    }
    out
}
