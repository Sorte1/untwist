use oxc_ast::ast::{Expression, Statement};
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

pub fn first_top_level_call(
    scoping: &Scoping,
    nodes: &AstNodes<'_>,
    callee: SymbolId,
    home: ScopeId,
) -> Option<u32> {
    scoping
        .get_resolved_references(callee)
        .filter(|reference| {
            let node = reference.node_id();
            let called = match nodes.parent_kind(node) {
                AstKind::CallExpression(call) => {
                    matches!(call.callee.without_parentheses(), Expression::Identifier(ident) if ident.span == nodes.kind(node).span())
                }
                _ => false,
            };
            called && runs_in(scoping, nodes.get_node(node).scope_id(), home)
        })
        .map(|reference| nodes.kind(reference.node_id()).span().start)
        .min()
}

pub fn declared_before(
    scoping: &Scoping,
    nodes: &AstNodes<'_>,
    symbol: SymbolId,
    end: u32,
) -> bool {
    nodes.kind(scoping.symbol_declaration(symbol)).span().start < end
}
