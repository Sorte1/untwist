use oxc_ast::ast::*;
use oxc_ast_visit::{walk_js, VisitJs};
use oxc_semantic::Scoping;
use oxc_span::Span;
use oxc_syntax::operator::AssignmentOperator;
use oxc_syntax::scope::ScopeFlags;
use oxc_syntax::symbol::SymbolId;
use rustc_hash::{FxHashMap, FxHashSet};

use crate::collect::reference_symbol;

const FUNCTION_PROTOTYPES: [&str; 2] = ["Function", "Object"];

pub(crate) fn is_instance_prototype(scoping: &Scoping, expr: &Expression<'_>) -> bool {
    let Expression::StaticMemberExpression(member) = expr.without_parentheses() else {
        return false;
    };
    if member.property.name != "prototype" {
        return false;
    }
    match member.object.without_parentheses() {
        Expression::Identifier(id) => {
            !FUNCTION_PROTOTYPES.contains(&id.name.as_str())
                || reference_symbol(id, scoping).is_some()
        }
        Expression::StaticMemberExpression(outer) => {
            !FUNCTION_PROTOTYPES.contains(&outer.property.name.as_str())
        }
        _ => false,
    }
}

fn prototype_slot<'k, 'a>(target: &'k AssignmentTarget<'a>) -> Option<&'k Expression<'a>> {
    match target {
        AssignmentTarget::StaticMemberExpression(member) => Some(&member.object),
        AssignmentTarget::ComputedMemberExpression(member) => Some(&member.object),
        _ => None,
    }
}

pub(crate) fn declared_methods(program: &Program<'_>, scoping: &Scoping) -> FxHashSet<Span> {
    let mut found = DeclaredMethods {
        scoping,
        declared: FxHashMap::default(),
        installs: FxHashMap::default(),
        exported: FxHashSet::default(),
    };
    found.visit_program(program);
    found
        .declared
        .into_iter()
        .filter(|(symbol, _)| !found.exported.contains(symbol))
        .filter(|(symbol, _)| {
            found.installs.get(symbol).is_some_and(|count| {
                let references = scoping.get_resolved_reference_ids(*symbol);
                let only_reads = references
                    .iter()
                    .all(|reference| !scoping.get_reference(*reference).is_write());
                only_reads && references.len() == *count
            })
        })
        .map(|(_, span)| span)
        .collect()
}

struct DeclaredMethods<'s> {
    scoping: &'s Scoping,
    declared: FxHashMap<SymbolId, Span>,
    installs: FxHashMap<SymbolId, usize>,
    exported: FxHashSet<SymbolId>,
}

fn function_symbol(function: &Function<'_>) -> Option<SymbolId> {
    function.id.as_ref().and_then(|id| id.symbol_id.get())
}

impl<'a> VisitJs<'a> for DeclaredMethods<'_> {
    fn visit_export_declaration(&mut self, it: &ExportDeclaration<'a>) {
        if let Declaration::FunctionDeclaration(function) = &it.declaration {
            self.exported.extend(function_symbol(function));
        }
        walk_js::walk_export_declaration(self, it);
    }

    fn visit_export_default_declaration(&mut self, it: &ExportDefaultDeclaration<'a>) {
        if let ExportDefaultDeclarationKind::FunctionDeclaration(function) = &it.declaration {
            self.exported.extend(function_symbol(function));
        }
        walk_js::walk_export_default_declaration(self, it);
    }

    fn visit_function(&mut self, it: &Function<'a>, flags: ScopeFlags) {
        if let (true, Some(symbol)) = (
            it.is_declaration(),
            it.id.as_ref().and_then(|id| id.symbol_id.get()),
        ) {
            self.declared.insert(symbol, it.span);
        }
        walk_js::walk_function(self, it, flags);
    }

    fn visit_assignment_expression(&mut self, it: &AssignmentExpression<'a>) {
        let installed = it.operator == AssignmentOperator::Assign
            && prototype_slot(&it.left)
                .is_some_and(|slot| is_instance_prototype(self.scoping, slot));
        if let (true, Expression::Identifier(id)) = (installed, it.right.without_parentheses()) {
            if let Some(symbol) = reference_symbol(id, self.scoping) {
                *self.installs.entry(symbol).or_default() += 1;
            }
        }
        walk_js::walk_assignment_expression(self, it);
    }
}
