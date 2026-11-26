mod component;
mod css;
mod dom;
mod fingerprint;
mod fingerprints;
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

const MARKERS: &[&str] = &["from_html", "svelte/internal", "from_svg", "<!>"];

pub fn may_hold_components(code: &str) -> bool {
    MARKERS.iter().any(|marker| code.contains(marker))
}

#[doc(hidden)]
pub struct FingerprintRow {
    pub hash: u64,
    pub rt: Option<String>,
    pub nodes: usize,
    pub name: String,
    pub used_by_component: bool,
}

#[doc(hidden)]
pub fn fingerprint_rows(code: &str) -> Vec<FingerprintRow> {
    let alloc = Allocator::default();
    let parsed = untwist_core::parse::parse_program(&alloc, code);
    if untwist_core::parse::has_error(&parsed.diagnostics) {
        return Vec::new();
    }
    let program = alloc.alloc(parsed.program);
    let semantic = SemanticBuilder::new()
        .with_build_nodes(true)
        .build(program)
        .semantic;
    let scoping = semantic.scoping();
    let nodes = semantic.nodes();
    let runtime = Runtime::of(program, scoping, nodes);
    let spans: Vec<oxc_span::Span> =
        nodes
            .iter()
            .filter_map(|node| match node.kind() {
                AstKind::Function(function)
                    if function.id.as_ref().is_some_and(|id| {
                        id.name.starts_with(|c: char| c.is_ascii_uppercase())
                    }) || component_params(function, &runtime, scoping).is_some() =>
                {
                    Some(function.span)
                }
                _ => None,
            })
            .collect();
    let marks = fingerprint::Marks::of(nodes, scoping);
    runtime::bound_functions(program.source_text, &marks, scoping, nodes)
        .into_iter()
        .map(|(symbol, print)| {
            let used_by_component = scoping.get_resolved_references(symbol).any(|reference| {
                let span = oxc_span::GetSpan::span(&nodes.kind(reference.node_id()));
                let scope = nodes.get_node(reference.node_id()).scope_id();
                scoping.scope_ancestors(scope).count() <= 2
                    || spans
                        .iter()
                        .any(|outer| outer.start <= span.start && span.end <= outer.end)
            });
            FingerprintRow {
                hash: print.hash,
                rt: runtime::sourced_name(scoping.symbol_name(symbol)).map(|rt| format!("{rt:?}")),
                nodes: print.nodes,
                name: scoping.symbol_name(symbol).to_string(),
                used_by_component,
            }
        })
        .collect()
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
    let rw = Rewriter::new(code, collected.sites, collected.refs, collected.shorthands);
    for symbol in runtime
        .namespaces
        .iter()
        .chain(runtime.symbols.keys())
        .chain(runtime.ambiguous.iter())
        .chain(collected.facts.props_objects.iter())
    {
        rw.forbid(*symbol);
    }
    for (symbol, at) in &runtime.exports {
        let name = runtime::EXPORTS[usize::from(*at)].0;
        if scoping.symbol_name(*symbol) != name {
            rw.rename(*symbol, name.to_string());
        }
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
    let mut done = Vec::new();
    for (function, name, top) in &components {
        let esm = runtime.imports_svelte && *top;
        match shared.restore(function, esm) {
            Ok((text, children)) => done.push((name.clone(), text, children)),
            Err(reason) => out.kept.push((name.clone(), reason)),
        }
    }
    loop {
        let kept: Vec<String> = out.kept.iter().map(|(name, _)| name.clone()).collect();
        let Some(at) = done
            .iter()
            .position(|(_, _, children)| children.iter().any(|child| kept.contains(child)))
        else {
            break;
        };
        let (name, _, children) = done.remove(at);
        let child = children
            .into_iter()
            .find(|child| kept.contains(child))
            .unwrap_or_default();
        out.kept
            .push((name, format!("its child component {child} is kept as JS")));
    }
    out.components = done
        .into_iter()
        .map(|(name, text, _)| Component { name, text })
        .collect();
    out
}
