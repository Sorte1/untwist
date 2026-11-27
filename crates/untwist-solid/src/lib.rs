mod ast;
mod convert;
mod element;
mod fingerprints;
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

fn delegated(
    runtime: &Runtime,
    scoping: &oxc_semantic::Scoping,
    nodes: &oxc_semantic::AstNodes<'_>,
) -> Option<Vec<String>> {
    let known = runtime.esm
        || !runtime.minified
        || runtime.symbols.values().any(|rt| *rt == Rt::DelegateEvents);
    if !known {
        return None;
    }
    let mut names = Vec::new();
    for node in nodes.iter() {
        let AstKind::CallExpression(call) = node.kind() else {
            continue;
        };
        if !runtime.is(&call.callee, scoping, Rt::DelegateEvents) {
            continue;
        }
        let Some(Expression::ArrayExpression(array)) =
            ast::arg(call, 0).map(Expression::without_parentheses)
        else {
            return None;
        };
        for element in &array.elements {
            names.push(element.as_expression().and_then(ast::string)?.to_string());
        }
    }
    Some(names)
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

#[doc(hidden)]
pub use untwist_restore::table::FingerprintRow;

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
    let templates: Vec<SymbolId> = nodes
        .iter()
        .filter_map(|node| match node.kind() {
            AstKind::VariableDeclarator(declarator) => match declarator
                .init
                .as_ref()
                .map(Expression::without_parentheses)
            {
                Some(Expression::CallExpression(call))
                    if matches!(call.callee.without_parentheses(), Expression::Identifier(ident) if runtime::labelled(&ident.name) == Some("template")) =>
                {
                    ast::binding(&declarator.id)
                }
                _ => None,
            },
            _ => None,
        })
        .collect();
    let jsx: Vec<Span> = nodes
        .iter()
        .filter_map(|node| match node.kind() {
            AstKind::CallExpression(call) => match call.callee.without_parentheses() {
                Expression::Identifier(ident)
                    if ast::symbol_of(ident, scoping)
                        .is_some_and(|symbol| templates.contains(&symbol))
                        || runtime::labelled(&ident.name) == Some("createComponent") =>
                {
                    Some(call.span)
                }
                _ => None,
            },
            _ => None,
        })
        .collect();
    let spans: Vec<Span> = nodes
        .iter()
        .filter_map(|node| match node.kind() {
            AstKind::Function(function) => Some(function.span),
            AstKind::ArrowFunctionExpression(arrow) => Some(arrow.span),
            _ => None,
        })
        .filter(|span| {
            jsx.iter()
                .any(|call| span.start <= call.start && call.end <= span.end)
        })
        .filter(|span| {
            !templates.iter().any(|symbol| {
                let at = scoping.symbol_span(*symbol);
                span.start <= at.start && at.end <= span.end
            })
        })
        .collect();
    let marks = untwist_restore::fingerprint::Marks::of(nodes, scoping);
    untwist_restore::fingerprint::bound_functions(program.source_text, &marks, scoping, nodes)
        .into_iter()
        .map(|(symbol, print)| {
            let used_by_component = scoping.get_resolved_references(symbol).any(|reference| {
                let span = nodes.kind(reference.node_id()).span();
                let scope = nodes.get_node(reference.node_id()).scope_id();
                scoping.scope_ancestors(scope).count() <= 2
                    || spans
                        .iter()
                        .any(|outer| outer.start <= span.start && span.end <= outer.end)
            });
            let name = scoping.symbol_name(symbol);
            FingerprintRow {
                hash: print.hash,
                rt: runtime::labelled(name).map(str::to_string),
                nodes: print.nodes,
                name: name.to_string(),
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
    let mut rw = Rewriter::new(code, "Solid", Vec::new(), refs, shorthands);
    rw.keep_lines(
        nodes
            .iter()
            .filter_map(|node| match node.kind() {
                AstKind::TemplateLiteral(template) => Some(template),
                _ => None,
            })
            .flat_map(|template| template.quasis.iter())
            .filter(|quasi| quasi.value.raw.contains('\n'))
            .map(|quasi| quasi.span)
            .collect(),
    );
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
        ignored: Default::default(),
        inline: Default::default(),
        deleted: Default::default(),
        library: Default::default(),
        candidates: convert::candidates(nodes),
        delegated: delegated(&runtime, scoping, nodes),
    };
    let found = module::components(&cx, home);
    let mut restored = module::restore_all(&cx, home, &found);
    for component in &mut restored.components {
        component.text = untwist_restore::text::restore_lines(&component.text);
    }
    restored
}
