use oxc_allocator::Vec as AVec;
use oxc_ast::ast::*;
use oxc_semantic::Scoping;
use oxc_syntax::symbol::SymbolId;
use untwist_core::pass::Cx;

use super::identify::{Definition, Table};

fn function_symbol(statement: &Statement<'_>) -> Option<SymbolId> {
    let Statement::FunctionDeclaration(function) = statement else {
        return None;
    };
    function.id.as_ref()?.symbol_id.get()
}

fn declarator_symbol(declarator: &VariableDeclarator<'_>) -> Option<SymbolId> {
    let BindingPattern::BindingIdentifier(id) = &declarator.id else {
        return None;
    };
    id.symbol_id.get()
}

fn is_var<'k, 'a>(statement: &'k Statement<'a>) -> Option<&'k VariableDeclaration<'a>> {
    match statement {
        Statement::VariableDeclaration(declaration)
            if declaration.kind == VariableDeclarationKind::Var && !declaration.declare =>
        {
            Some(declaration)
        }
        _ => None,
    }
}

fn sweepable(table: &Table, symbol: SymbolId, function: bool) -> bool {
    table
        .get(symbol)
        .is_some_and(|helper| (helper.definition == Definition::Function) == function)
}

fn candidates(
    statements: &AVec<'_, Statement<'_>>,
    table: &Table,
    scoping: &Scoping,
    scope: oxc_syntax::scope::ScopeId,
) -> Vec<SymbolId> {
    let mut found = Vec::new();
    for statement in statements {
        if let Some(symbol) = function_symbol(statement) {
            if sweepable(table, symbol, true) {
                found.push(symbol);
            }
        } else if let Some(declaration) = is_var(statement) {
            found.extend(
                declaration
                    .declarations
                    .iter()
                    .filter_map(declarator_symbol)
                    .filter(|symbol| sweepable(table, *symbol, false)),
            );
        }
    }
    found.retain(|symbol| scoping.symbol_scope_id(*symbol) == scope);
    found
}

fn references_from(table: &Table, removed: &[SymbolId], symbol: SymbolId) -> u32 {
    removed
        .iter()
        .filter_map(|holder| table.get(*holder))
        .flat_map(|holder| holder.external.iter())
        .filter(|(external, _)| *external == symbol)
        .map(|(_, count)| count)
        .sum()
}

fn unreferenced(found: &[SymbolId], table: &Table, scoping: &Scoping) -> Vec<SymbolId> {
    let mut removed: Vec<SymbolId> = Vec::new();
    loop {
        let before = removed.len();
        for symbol in found {
            if removed.contains(symbol) {
                continue;
            }
            let Some(helper) = table.get(*symbol) else {
                continue;
            };
            let total = scoping.get_resolved_reference_ids(*symbol).len();
            let accounted = helper.own_references + references_from(table, &removed, *symbol);
            if total == accounted as usize {
                removed.push(*symbol);
            }
        }
        if removed.len() == before {
            return removed;
        }
    }
}

fn keep_declarators<'a>(
    declaration: &mut VariableDeclaration<'a>,
    removed: &[SymbolId],
    cx: &Cx<'a, '_>,
) -> usize {
    let declarators = std::mem::replace(&mut declaration.declarations, AVec::new_in(cx));
    let before = declarators.len();
    for declarator in declarators {
        if !declarator_symbol(&declarator).is_some_and(|symbol| removed.contains(&symbol)) {
            declaration.declarations.push(declarator);
        }
    }
    before - declaration.declarations.len()
}

pub struct Swept {
    pub functions: usize,
    pub declarators: usize,
}

pub fn sweep<'a>(
    statements: &mut AVec<'a, Statement<'a>>,
    table: &Table,
    scope: oxc_syntax::scope::ScopeId,
    cx: &mut Cx<'a, '_>,
) -> Swept {
    let mut swept = Swept {
        functions: 0,
        declarators: 0,
    };
    let scoping = cx.scoping();
    let here: Vec<SymbolId> = table
        .symbols()
        .filter(|symbol| scoping.symbol_scope_id(*symbol) == scope)
        .collect();
    if unreferenced(&here, table, scoping).is_empty() {
        return swept;
    }
    let found = candidates(statements, table, cx.scoping(), scope);
    if found.is_empty() {
        return swept;
    }
    let removed = unreferenced(&found, table, cx.scoping());
    if removed.is_empty() {
        return swept;
    }
    let old = std::mem::replace(statements, AVec::new_in(cx));
    for mut statement in old {
        if function_symbol(&statement).is_some_and(|symbol| removed.contains(&symbol)) {
            swept.functions += 1;
            continue;
        }
        if let Statement::VariableDeclaration(declaration) = &mut statement {
            swept.declarators += keep_declarators(declaration, &removed, cx);
            if declaration.declarations.is_empty() {
                continue;
            }
        }
        statements.push(statement);
    }
    swept
}
