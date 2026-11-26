mod ast;
mod convert;
mod element;
mod jsx;
mod module;
mod runtime;

use oxc_allocator::Allocator;
use oxc_ast::ast::Expression;
use oxc_ast::AstKind;
use oxc_semantic::SemanticBuilder;
use oxc_span::{GetSpan, Span};
use oxc_syntax::symbol::SymbolId;
use rustc_hash::FxHashMap;
use untwist_restore::html;
use untwist_restore::text::Rewriter;

pub use untwist_restore::{Component, Restore};

use convert::{Cx, Template};
use runtime::{Rt, Runtime};

const MARKERS: &[&str] = &[
    "solid-js/web",
    "_$DX_DELEGATE",
    "createElement(\"template\")",
    "createElement('template')",
];

pub fn may_hold_components(code: &str) -> bool {
    MARKERS.iter().any(|marker| code.contains(marker))
}

fn templates(
    runtime: &Runtime,
    scoping: &oxc_semantic::Scoping,
    nodes: &oxc_semantic::AstNodes<'_>,
) -> FxHashMap<SymbolId, Template> {
    let mut out = FxHashMap::default();
    for node in nodes.iter() {
        let AstKind::VariableDeclarator(declarator) = node.kind() else {
            continue;
        };
        let (Some(symbol), Some(Expression::CallExpression(call))) = (
            ast::binding(&declarator.id),
            declarator
                .init
                .as_ref()
                .map(Expression::without_parentheses),
        ) else {
            continue;
        };
        if !runtime.is(&call.callee, scoping, Rt::Template) {
            continue;
        }
        let Some(markup) = ast::arg(call, 0).and_then(ast::string) else {
            continue;
        };
        let svg = matches!(
            ast::arg(call, 2).map(Expression::without_parentheses),
            Some(Expression::BooleanLiteral(flag)) if flag.value
        );
        let Some(mut parsed) = html::parse_open_ended(markup) else {
            continue;
        };
        if svg {
            parsed = match parsed.as_slice() {
                [html::Node::Element(wrapper)] if wrapper.name == "svg" => wrapper.children.clone(),
                _ => continue,
            };
        }
        out.insert(symbol, Template { nodes: parsed });
    }
    out
}

fn references(
    nodes: &oxc_semantic::AstNodes<'_>,
    scoping: &oxc_semantic::Scoping,
) -> (Vec<(Span, SymbolId)>, Vec<u32>) {
    let mut refs = Vec::new();
    let mut shorthands = Vec::new();
    for node in nodes.iter() {
        match node.kind() {
            AstKind::IdentifierReference(ident) => {
                if let Some(symbol) = ast::symbol_of(ident, scoping) {
                    refs.push((ident.span, symbol));
                }
            }
            AstKind::BindingIdentifier(ident) => {
                if let Some(symbol) = ident.symbol_id.get() {
                    refs.push((ident.span, symbol));
                }
            }
            AstKind::ObjectProperty(property) if property.shorthand => {
                shorthands.push(property.value.span().start)
            }
            AstKind::BindingProperty(property) if property.shorthand => {
                shorthands.push(property.value.span().start)
            }
            AstKind::AssignmentTargetPropertyIdentifier(property) => {
                shorthands.push(property.binding.span.start)
            }
            _ => {}
        }
    }
    (refs, shorthands)
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
    let nodes = semantic.nodes();
    let runtime = Runtime::of(program, scoping, nodes);
    let Some(home) = runtime.home(scoping) else {
        return Restore::default();
    };
    let templates = templates(&runtime, scoping, nodes);
    if templates.is_empty() {
        return Restore::default();
    }
    let (refs, shorthands) = references(nodes, scoping);
    let mut idents: Vec<(Span, SymbolId)> = nodes
        .iter()
        .filter_map(|node| match node.kind() {
            AstKind::IdentifierReference(ident) => {
                ast::symbol_of(ident, scoping).map(|symbol| (ident.span, symbol))
            }
            _ => None,
        })
        .collect();
    idents.sort_by_key(|(span, _)| span.start);
    let rw = Rewriter::new(code, "Solid", Vec::new(), refs, shorthands);
    for (symbol, rt) in &runtime.symbols {
        match (runtime.esm, rt.spelling()) {
            (false, Some((name, _))) => {
                if scoping.symbol_name(*symbol) != name {
                    rw.rename(*symbol, name.to_string());
                }
            }
            _ => rw.forbid(*symbol),
        }
    }
    for symbol in runtime.ambiguous.iter().chain(templates.keys()) {
        rw.forbid(*symbol);
    }
    let cx = Cx {
        source: code,
        scoping,
        nodes,
        runtime: &runtime,
        templates: &templates,
        rw: &rw,
        roots: Default::default(),
        idents,
    };
    let found = module::components(&cx, home);
    module::restore_all(&cx, home, &found)
}
