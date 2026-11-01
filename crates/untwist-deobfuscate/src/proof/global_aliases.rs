use oxc_ast::ast::*;
use oxc_ast_visit::{walk_js, VisitJs};
use oxc_semantic::{Scoping, SymbolId};
use rustc_hash::{FxHashMap, FxHashSet};

use super::GLOBAL_OBJECTS;

fn is_global_object_identifier(scoping: &Scoping, expr: &Expression<'_>) -> bool {
    let Expression::Identifier(id) = expr.without_parentheses() else {
        return false;
    };
    let unresolved = id
        .reference_id
        .get()
        .is_some_and(|r| scoping.get_reference(r).symbol_id().is_none());
    unresolved && GLOBAL_OBJECTS.contains(&id.name.as_str())
}

fn never_written(scoping: &Scoping, symbol: SymbolId) -> bool {
    scoping
        .get_resolved_reference_ids(symbol)
        .iter()
        .all(|r| !scoping.get_reference(*r).is_write())
}

struct GlobalAliasScan<'s> {
    scoping: &'s Scoping,
    found: FxHashSet<SymbolId>,
}

impl<'a> VisitJs<'a> for GlobalAliasScan<'_> {
    fn visit_variable_declarator(&mut self, it: &VariableDeclarator<'a>) {
        if let (BindingPattern::BindingIdentifier(id), Some(init)) = (&it.id, &it.init) {
            if is_global_object_identifier(self.scoping, init) {
                if let Some(symbol) = id.symbol_id.get() {
                    if never_written(self.scoping, symbol) {
                        self.found.insert(symbol);
                    }
                }
            }
        }
        walk_js::walk_variable_declarator(self, it);
    }
}

pub(super) fn scan_global_aliases(program: &Program<'_>, scoping: &Scoping) -> FxHashSet<SymbolId> {
    let mut scan = GlobalAliasScan {
        scoping,
        found: FxHashSet::default(),
    };
    scan.visit_program(program);
    scan.found
}

fn returns_bare_this(function: &Function<'_>) -> bool {
    if function.r#async || function.generator || function.has_use_strict_directive() {
        return false;
    }
    let Some(body) = function.body.as_ref() else {
        return false;
    };
    matches!(
        body.statements.as_slice(),
        [Statement::ReturnStatement(ret)]
            if matches!(ret.argument.as_ref(), Some(Expression::ThisExpression(_)))
    )
}

fn bare_this_property_names<'a>(literal: &ObjectExpression<'a>) -> FxHashSet<&'a str> {
    literal
        .properties
        .iter()
        .filter_map(|property| {
            let ObjectPropertyKind::ObjectProperty(entry) = property else {
                return None;
            };
            let PropertyKey::StaticIdentifier(name) = &entry.key else {
                return None;
            };
            let Expression::FunctionExpression(function) = entry.value.without_parentheses() else {
                return None;
            };
            returns_bare_this(function).then_some(name.name.as_str())
        })
        .collect()
}

struct ThisReturningLiterals<'s, 'a> {
    scoping: &'s Scoping,
    found: FxHashMap<SymbolId, FxHashSet<&'a str>>,
}

impl<'a> VisitJs<'a> for ThisReturningLiterals<'_, 'a> {
    fn visit_variable_declarator(&mut self, it: &VariableDeclarator<'a>) {
        if let (BindingPattern::BindingIdentifier(id), Some(init)) = (&it.id, &it.init) {
            if let Expression::ObjectExpression(literal) = init.without_parentheses() {
                let symbol = id.symbol_id.get();
                let tracked = symbol.is_some_and(|s| never_written(self.scoping, s));
                if let (Some(symbol), true) = (symbol, tracked) {
                    let names = bare_this_property_names(literal);
                    if !names.is_empty() {
                        self.found.insert(symbol, names);
                    }
                }
            }
        }
        walk_js::walk_variable_declarator(self, it);
    }
}

fn extracted_method_name<'k, 'a>(
    expr: &'k Expression<'a>,
) -> Option<(&'k Expression<'a>, &'a str)> {
    match expr.without_parentheses() {
        Expression::StaticMemberExpression(member) => {
            Some((&member.object, member.property.name.as_str()))
        }
        Expression::ComputedMemberExpression(member) => match member
            .expression
            .without_parentheses()
        {
            Expression::StringLiteral(literal) => Some((&member.object, literal.value.as_str())),
            _ => None,
        },
        _ => None,
    }
}

struct BareCallGlobalScan<'s, 'a> {
    scoping: &'s Scoping,
    literals: FxHashMap<SymbolId, FxHashSet<&'a str>>,
    found: FxHashSet<SymbolId>,
}

impl<'a> VisitJs<'a> for BareCallGlobalScan<'_, 'a> {
    fn visit_variable_declarator(&mut self, it: &VariableDeclarator<'a>) {
        if let (BindingPattern::BindingIdentifier(id), Some(init)) = (&it.id, &it.init) {
            if let Some((object, key)) = extracted_method_name(init) {
                if let Expression::Identifier(object_id) = object.without_parentheses() {
                    let owner = object_id
                        .reference_id
                        .get()
                        .and_then(|r| self.scoping.get_reference(r).symbol_id());
                    let names_hold_key = owner
                        .and_then(|s| self.literals.get(&s))
                        .is_some_and(|names| names.contains(key));
                    let symbol = id.symbol_id.get();
                    let tracked = symbol.is_some_and(|s| never_written(self.scoping, s));
                    if names_hold_key {
                        if let (Some(symbol), true) = (symbol, tracked) {
                            self.found.insert(symbol);
                        }
                    }
                }
            }
        }
        walk_js::walk_variable_declarator(self, it);
    }
}

pub(super) fn scan_bare_call_globals(
    program: &Program<'_>,
    scoping: &Scoping,
) -> FxHashSet<SymbolId> {
    let mut literals = ThisReturningLiterals {
        scoping,
        found: FxHashMap::default(),
    };
    literals.visit_program(program);
    let mut scan = BareCallGlobalScan {
        scoping,
        literals: literals.found,
        found: FxHashSet::default(),
    };
    scan.visit_program(program);
    scan.found
}
