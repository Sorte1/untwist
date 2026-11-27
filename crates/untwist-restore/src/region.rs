use oxc_ast::ast::Statement;
use oxc_ast::AstKind;
use oxc_semantic::{AstNodes, Scoping};
use oxc_span::GetSpan;
use oxc_syntax::scope::ScopeId;
use oxc_syntax::symbol::SymbolId;

pub fn statements<'a>(
    scoping: &Scoping,
    nodes: &'a AstNodes<'a>,
    scope: ScopeId,
) -> &'a [Statement<'a>] {
    match nodes.kind(scoping.get_node_id(scope)) {
        AstKind::Program(program) => &program.body,
        AstKind::Function(function) => function
            .body
            .as_ref()
            .map_or(&[], |body| body.statements.as_slice()),
        AstKind::ArrowFunctionExpression(arrow) => arrow
            .get_function_body()
            .map_or(&[], |body| body.statements.as_slice()),
        _ => &[],
    }
}

fn runs_in(scoping: &Scoping, from: ScopeId, home: ScopeId) -> bool {
    for scope in scoping.scope_ancestors(from) {
        if scope == home {
            return true;
        }
        if scoping.scope_flags(scope).is_function() {
            return false;
        }
    }
    false
}

pub fn template_zones(
    scoping: &Scoping,
    nodes: &AstNodes<'_>,
    callee: SymbolId,
    home: ScopeId,
) -> Vec<(u32, u32)> {
    let statements = statements(scoping, nodes, home);
    let owner = |at: u32| {
        statements
            .iter()
            .map(GetSpan::span)
            .find(|span| span.start <= at && at < span.end)
    };
    let mut zones = Vec::new();
    for reference in scoping.get_resolved_references(callee) {
        let node = reference.node_id();
        if !runs_in(scoping, nodes.get_node(node).scope_id(), home) {
            continue;
        }
        let call = nodes.parent_id(node);
        let AstKind::CallExpression(_) = nodes.kind(call) else {
            continue;
        };
        let AstKind::VariableDeclarator(declarator) = nodes.parent_kind(call) else {
            continue;
        };
        let Some(template) = declarator
            .id
            .get_binding_identifier()
            .and_then(|ident| ident.symbol_id.get())
        else {
            continue;
        };
        let Some(start) = owner(nodes.kind(node).span().start) else {
            continue;
        };
        let end = scoping
            .get_resolved_references(template)
            .filter_map(|used| owner(nodes.kind(used.node_id()).span().start))
            .map(|span| span.end)
            .fold(start.end, u32::max);
        zones.push((start.start, end));
    }
    zones
}

pub fn in_zones(zones: &[(u32, u32)], at: u32) -> bool {
    zones.iter().any(|(start, end)| *start <= at && at < *end)
}

pub fn declared_in(
    scoping: &Scoping,
    nodes: &AstNodes<'_>,
    symbol: SymbolId,
    zones: &[(u32, u32)],
) -> bool {
    in_zones(
        zones,
        nodes.kind(scoping.symbol_declaration(symbol)).span().start,
    )
}
