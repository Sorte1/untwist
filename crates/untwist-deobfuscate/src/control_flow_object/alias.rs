use std::cell::Cell;

use oxc_allocator::{Allocator, GetAllocator, Vec as AVec};
use oxc_ast::ast::*;
use oxc_ast_visit::{walk_js, walk_mut, VisitJs, VisitMut};
use oxc_semantic::Scoping;
use oxc_span::{GetSpan, Span};
use oxc_str::Ident;
use oxc_syntax::scope::{ScopeFlags, ScopeId};
use oxc_syntax::symbol::SymbolId;
use rustc_hash::{FxHashMap, FxHashSet};
use untwist_core::ast_util::edit::StmtEdits;
use untwist_core::ast_util::prop::member_parts;
use untwist_core::ast_util::shorthand::walk_object_property_keeping_key;
use untwist_core::pass::Cx;
use untwist_core::semantic::is_constant_binding;

use super::keys::key_assignment;
use super::{cff_props, single_declarator};

struct Collapse<'a> {
    target: SymbolId,
    name: Ident<'a>,
    exempt: FxHashSet<usize>,
    removed: Option<Span>,
    initialized_at: u32,
}

fn address_of(id: &IdentifierReference<'_>) -> usize {
    std::ptr::from_ref(id) as usize
}

fn resolved(id: &IdentifierReference<'_>, scoping: &Scoping) -> Option<SymbolId> {
    id.reference_id
        .get()
        .and_then(|reference| scoping.get_reference(reference).symbol_id())
}

fn assigned_object<'k, 'a>(stmt: &'k Statement<'a>) -> Option<&'k IdentifierReference<'a>> {
    let Statement::ExpressionStatement(statement) = stmt else {
        return None;
    };
    let Expression::AssignmentExpression(assignment) = &statement.expression else {
        return None;
    };
    let target = assignment.left.as_member_expression()?.as_expression();
    let (Expression::Identifier(object), name) = member_parts(target)? else {
        return None;
    };
    (name != "__proto__").then_some(&**object)
}

fn const_declarator<'k, 'a>(
    stmt: &'k Statement<'a>,
) -> Option<(&'k BindingIdentifier<'a>, Option<&'k Expression<'a>>)> {
    let Statement::VariableDeclaration(declaration) = stmt else {
        return None;
    };
    if declaration.kind != VariableDeclarationKind::Const {
        return None;
    }
    single_declarator(stmt)
}

struct AliasSearch<'s, 'a> {
    scoping: &'s Scoping,
    extended: bool,
    found: FxHashMap<SymbolId, Collapse<'a>>,
}

impl<'a> AliasSearch<'_, 'a> {
    fn run_at(&mut self, run: &[Statement<'a>]) {
        let Some((first, rest)) = run.split_first() else {
            return;
        };
        let Some((base, Some(object @ Expression::ObjectExpression(literal)))) =
            const_declarator(first)
        else {
            return;
        };
        let mut exempt = FxHashSet::default();
        let mut assigned = 0;
        for stmt in rest {
            if key_assignment(stmt, base.name.as_str()).is_none() {
                break;
            }
            let Some(object) = assigned_object(stmt) else {
                return;
            };
            exempt.insert(address_of(object));
            assigned += 1;
        }
        let Some((alias, Some(Expression::Identifier(init)))) =
            rest.get(assigned).and_then(const_declarator)
        else {
            return;
        };
        let (Some(base_symbol), Some(alias_symbol)) = (base.symbol_id.get(), alias.symbol_id.get())
        else {
            return;
        };
        let constant = is_constant_binding(self.scoping, base_symbol)
            && is_constant_binding(self.scoping, alias_symbol);
        if resolved(init, self.scoping) != Some(base_symbol) || !constant {
            return;
        }
        if assigned == 0 {
            if cff_props(object, self.extended).is_some() {
                self.found.insert(
                    alias_symbol,
                    Collapse {
                        target: base_symbol,
                        name: base.name,
                        exempt: FxHashSet::default(),
                        removed: Some(rest[0].span()),
                        initialized_at: rest[0].span().end,
                    },
                );
            }
            return;
        }
        if !literal.properties.is_empty() && cff_props(object, self.extended).is_none() {
            return;
        }
        exempt.insert(address_of(init));
        self.found.insert(
            base_symbol,
            Collapse {
                target: alias_symbol,
                name: alias.name,
                exempt,
                removed: None,
                initialized_at: rest[assigned].span().end,
            },
        );
    }
}

impl<'a> VisitJs<'a> for AliasSearch<'_, 'a> {
    fn visit_statements(&mut self, it: &AVec<'a, Statement<'a>>) {
        for start in 0..it.len() {
            self.run_at(&it[start..]);
        }
        walk_js::walk_statements(self, it);
    }
}

struct Shadowing<'s, 'k, 'a> {
    scoping: &'s Scoping,
    collapses: &'k FxHashMap<SymbolId, Collapse<'a>>,
    scopes: Vec<ScopeId>,
    refused: FxHashSet<SymbolId>,
    seen: FxHashMap<SymbolId, usize>,
    hoisted_depth: usize,
}

impl<'a> VisitJs<'a> for Shadowing<'_, '_, 'a> {
    fn visit_function(&mut self, it: &Function<'a>, flags: ScopeFlags) {
        let hoisted = usize::from(it.is_declaration());
        self.hoisted_depth += hoisted;
        walk_js::walk_function(self, it, flags);
        self.hoisted_depth -= hoisted;
    }

    fn enter_scope(&mut self, _flags: ScopeFlags, scope_id: &Cell<Option<ScopeId>>) {
        let scope = scope_id
            .get()
            .or_else(|| self.scopes.last().copied())
            .unwrap_or_else(|| self.scoping.root_scope_id());
        self.scopes.push(scope);
    }

    fn leave_scope(&mut self) {
        self.scopes.pop();
    }

    fn visit_identifier_reference(&mut self, it: &IdentifierReference<'a>) {
        let Some(symbol) = resolved(it, self.scoping) else {
            return;
        };
        let Some(collapse) = self.collapses.get(&symbol) else {
            return;
        };
        *self.seen.entry(symbol).or_default() += 1;
        if collapse.exempt.contains(&address_of(it)) {
            return;
        }
        if self.hoisted_depth > 0 || it.span.start < collapse.initialized_at {
            self.refused.insert(symbol);
        }
        let scope = self
            .scopes
            .last()
            .copied()
            .unwrap_or_else(|| self.scoping.root_scope_id());
        if self.scoping.find_binding(scope, collapse.name) != Some(collapse.target) {
            self.refused.insert(symbol);
        }
    }
}

struct Renamer<'s, 'k, 'a> {
    scoping: &'s Scoping,
    collapses: &'k FxHashMap<SymbolId, Collapse<'a>>,
    alloc: &'a Allocator,
    renamed: usize,
}

impl<'a> VisitMut<'a> for Renamer<'_, '_, 'a> {
    fn visit_identifier_reference(&mut self, it: &mut IdentifierReference<'a>) {
        let address = address_of(it);
        if let Some(collapse) =
            resolved(it, self.scoping).and_then(|symbol| self.collapses.get(&symbol))
        {
            if !collapse.exempt.contains(&address) {
                it.name = collapse.name;
                self.renamed += 1;
            }
        }
        walk_mut::walk_identifier_reference(self, it);
    }

    fn visit_object_property(&mut self, it: &mut ObjectProperty<'a>) {
        let alloc = self.alloc;
        walk_object_property_keeping_key(self, it, alloc);
    }
}

fn collapsible(scoping: &Scoping, symbol: SymbolId, seen: usize) -> bool {
    let references = scoping.get_resolved_reference_ids(symbol);
    seen == references.len()
        && references
            .iter()
            .all(|reference| !scoping.get_reference(*reference).is_write())
}

pub(super) fn collapse_object_aliases<'a>(program: &mut Program<'a>, cx: &mut Cx<'a, '_>) -> usize {
    if untwist_core::semantic::scope_hazard(cx.scoping()).is_some() {
        return 0;
    }
    let mut search = AliasSearch {
        scoping: cx.scoping(),
        extended: cx.opts.extended,
        found: FxHashMap::default(),
    };
    search.visit_program(program);
    let mut collapses = search.found;
    if collapses.is_empty() {
        return 0;
    }
    let mut shadowing = Shadowing {
        scoping: cx.scoping(),
        collapses: &collapses,
        scopes: Vec::new(),
        refused: FxHashSet::default(),
        seen: FxHashMap::default(),
        hoisted_depth: 0,
    };
    shadowing.visit_program(program);
    let (refused, seen) = (shadowing.refused, shadowing.seen);
    collapses.retain(|symbol, collapse| {
        let outside = seen.get(symbol).copied().unwrap_or(0);
        let root = cx.scoping().root_scope_id();
        !refused.contains(symbol)
            && cx.scoping().symbol_scope_id(*symbol) != root
            && cx.scoping().symbol_scope_id(collapse.target) != root
            && outside > collapse.exempt.len()
            && collapsible(cx.scoping(), *symbol, outside)
    });
    if collapses.is_empty() {
        return 0;
    }
    let mut renamer = Renamer {
        scoping: cx.scoping(),
        collapses: &collapses,
        alloc: cx.allocator(),
        renamed: 0,
    };
    renamer.visit_program(program);
    let mut removals = StmtEdits::new();
    for collapse in collapses.values() {
        if let Some(statement) = collapse.removed {
            removals.remove(statement);
        }
    }
    renamer.renamed + removals.apply(program, cx.allocator(), cx.scoping_mut())
}
