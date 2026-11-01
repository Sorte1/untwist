use oxc_ast::ast::*;
use oxc_ast_visit::{walk_js, VisitJs};
use oxc_semantic::Scoping;
use oxc_span::{GetSpan, Span};
use oxc_syntax::operator::UnaryOperator;
use oxc_syntax::symbol::SymbolId;
use rustc_hash::FxHashMap;
use untwist_core::ast_util::prop::literal_key;
use untwist_core::ast_util::readonly::{is_readonly_object, ObjectUses, PropRead};
use untwist_core::semantic::is_constant_binding;

use super::spans::{holds_target, is_target, sorted_spans};
use crate::init_order::{reads_are_initialised, InitOrder};

struct ConstantKey {
    member_span: Span,
    object_span: Span,
    key: SymbolId,
    key_span: Span,
    excluded: bool,
}

#[derive(Default)]
struct Keyed {
    found: usize,
    excluded: usize,
    unknown: usize,
    constant: Vec<ConstantKey>,
}

struct KeyedSearch<'s> {
    scoping: &'s Scoping,
    targets: &'s [Span],
    excluded: Vec<Span>,
    deleted: Vec<Span>,
    keyed: FxHashMap<SymbolId, Keyed>,
}

impl KeyedSearch<'_> {
    fn symbol_of(&self, id: &IdentifierReference<'_>) -> Option<SymbolId> {
        self.scoping
            .get_reference(id.reference_id.get()?)
            .symbol_id()
    }

    fn keyed_member<'k, 'a>(
        &self,
        expr: &'k Expression<'a>,
    ) -> Option<(&'k ComputedMemberExpression<'a>, SymbolId)> {
        let Expression::ComputedMemberExpression(member) = expr else {
            return None;
        };
        if member.optional || literal_key(&member.expression).is_some() {
            return None;
        }
        let Expression::Identifier(object) = &member.object else {
            return None;
        };
        if !is_target(self.targets, object.span) {
            return None;
        }
        Some((member, self.symbol_of(object)?))
    }

    fn exclude(&mut self, expr: &Expression<'_>) {
        if let Some((member, _)) = self.keyed_member(expr) {
            self.excluded.push(member.span);
        }
    }

    fn exclude_call(&mut self, callee: &Expression<'_>, arguments: &[Argument<'_>]) {
        self.exclude(callee);
        for argument in arguments {
            match argument {
                Argument::SpreadElement(spread) => self.exclude(&spread.argument),
                other => {
                    if let Some(expr) = other.as_expression() {
                        self.exclude(expr);
                    }
                }
            }
        }
    }

    fn exclude_positions(&mut self, expr: &Expression<'_>) {
        match expr {
            Expression::CallExpression(call) => self.exclude_call(&call.callee, &call.arguments),
            Expression::NewExpression(new) => self.exclude_call(&new.callee, &new.arguments),
            Expression::TaggedTemplateExpression(tagged) => self.exclude(&tagged.tag),
            Expression::UnaryExpression(unary) if unary.operator == UnaryOperator::Delete => {
                if let Some((member, _)) = self.keyed_member(&unary.argument) {
                    self.deleted.push(member.span);
                }
            }
            Expression::ChainExpression(chain) => {
                if let ChainElement::CallExpression(call) = &chain.expression {
                    self.exclude_call(&call.callee, &call.arguments);
                }
            }
            _ => {}
        }
    }

    fn classify(&mut self, member: &ComputedMemberExpression<'_>, symbol: SymbolId) {
        let constant = match &member.expression {
            Expression::Identifier(key) => self
                .symbol_of(key)
                .filter(|key| self.is_local_constant(*key))
                .map(|symbol| (symbol, key.span)),
            _ => None,
        };
        let deleted = self.deleted.contains(&member.span);
        let excluded = self.excluded.contains(&member.span);
        let object_span = member.object.span();
        let entry = self.keyed.entry(symbol).or_default();
        entry.found += 1;
        match (deleted, excluded, constant) {
            (false, _, Some((key, key_span))) => entry.constant.push(ConstantKey {
                member_span: member.span,
                object_span,
                key,
                key_span,
                excluded,
            }),
            (true, _, _) | (false, true, None) => entry.excluded += 1,
            (false, false, None) => entry.unknown += 1,
        }
    }

    fn is_local_constant(&self, symbol: SymbolId) -> bool {
        self.scoping.symbol_scope_id(symbol) != self.scoping.root_scope_id()
            && is_constant_binding(self.scoping, symbol)
    }
}

impl<'a> VisitJs<'a> for KeyedSearch<'_> {
    fn visit_statement(&mut self, it: &Statement<'a>) {
        if holds_target(self.targets, it.span()) {
            walk_js::walk_statement(self, it);
        }
    }

    fn visit_expression(&mut self, it: &Expression<'a>) {
        if !holds_target(self.targets, it.span()) {
            return;
        }
        self.exclude_positions(it);
        if let Some((member, symbol)) = self.keyed_member(it) {
            self.classify(member, symbol);
        }
        walk_js::walk_expression(self, it);
    }

    fn visit_for_in_statement(&mut self, it: &ForInStatement<'a>) {
        self.exclude(&it.right);
        walk_js::walk_for_in_statement(self, it);
    }

    fn visit_for_of_statement(&mut self, it: &ForOfStatement<'a>) {
        self.exclude(&it.right);
        walk_js::walk_for_of_statement(self, it);
    }
}

struct KeyDeclarations<'s> {
    targets: &'s [Span],
    found: FxHashMap<Span, (Span, String)>,
}

impl<'a> VisitJs<'a> for KeyDeclarations<'_> {
    fn visit_statement(&mut self, it: &Statement<'a>) {
        if !holds_target(self.targets, it.span()) {
            return;
        }
        if let Some((binding, name)) = literal_declaration(it) {
            if is_target(self.targets, binding) {
                self.found.insert(binding, (it.span(), name));
                return;
            }
        }
        walk_js::walk_statement(self, it);
    }

    fn visit_expression(&mut self, it: &Expression<'a>) {
        if holds_target(self.targets, it.span()) {
            walk_js::walk_expression(self, it);
        }
    }
}

fn literal_declaration(stmt: &Statement<'_>) -> Option<(Span, String)> {
    let Statement::VariableDeclaration(declaration) = stmt else {
        return None;
    };
    let [declarator] = declaration.declarations.as_slice() else {
        return None;
    };
    let BindingPattern::BindingIdentifier(id) = &declarator.id else {
        return None;
    };
    let name = literal_key(declarator.init.as_ref()?)?;
    Some((id.span, name.into_owned()))
}

fn search_keyed(
    program: &Program<'_>,
    scoping: &Scoping,
    keyed: &FxHashMap<SymbolId, ObjectUses>,
) -> FxHashMap<SymbolId, Keyed> {
    let targets = sorted_spans(
        keyed
            .values()
            .flat_map(|uses| uses.opaque_spans.iter().copied()),
    );
    let mut search = KeyedSearch {
        scoping,
        targets: &targets,
        excluded: Vec::new(),
        deleted: Vec::new(),
        keyed: FxHashMap::default(),
    };
    search.visit_program(program);
    search.keyed
}

fn key_declarations(
    program: &Program<'_>,
    scoping: &Scoping,
    found: &FxHashMap<SymbolId, Keyed>,
) -> FxHashMap<Span, (Span, String)> {
    let targets = sorted_spans(
        found
            .values()
            .flat_map(|keyed| keyed.constant.iter())
            .map(|constant| scoping.symbol_span(constant.key)),
    );
    if targets.is_empty() {
        return FxHashMap::default();
    }
    let mut search = KeyDeclarations {
        targets: &targets,
        found: FxHashMap::default(),
    };
    search.visit_program(program);
    search.found
}

fn resolve_key(
    constant: &ConstantKey,
    declarations: &FxHashMap<Span, (Span, String)>,
    scoping: &Scoping,
    order: &InitOrder,
) -> Option<PropRead> {
    let (statement, name) = declarations.get(&scoping.symbol_span(constant.key))?;
    let key_read = [PropRead {
        member_span: constant.member_span,
        object_span: constant.key_span,
        name: name.clone(),
    }];
    reads_are_initialised(&key_read, *statement, order).ok()?;
    let [key_read] = key_read;
    Some(PropRead {
        object_span: constant.object_span,
        ..key_read
    })
}

fn resolve_into(
    uses: &mut ObjectUses,
    search: &Keyed,
    declarations: &FxHashMap<Span, (Span, String)>,
    scoping: &Scoping,
    order: &InitOrder,
) -> bool {
    let mut unknown = search.unknown;
    for constant in &search.constant {
        match resolve_key(constant, declarations, scoping, order) {
            Some(read) => uses.reads.push(read),
            None if constant.excluded => return false,
            None => unknown += 1,
        }
    }
    uses.keyed = u32::try_from(unknown).unwrap_or(u32::MAX);
    !uses.reads.is_empty()
}

pub(super) fn resolve_keyed(
    program: &Program<'_>,
    scoping: &Scoping,
    uses: &mut FxHashMap<SymbolId, ObjectUses>,
    order: &InitOrder,
    unsafe_transforms: bool,
) {
    let mut found = search_keyed(program, scoping, uses);
    found.retain(|symbol, search| {
        search.excluded == 0
            && uses
                .get(symbol)
                .is_some_and(|uses| usize::try_from(uses.keyed) == Ok(search.found))
    });
    let declarations = key_declarations(program, scoping, &found);
    uses.retain(|symbol, uses| {
        if is_readonly_object(uses) {
            return true;
        }
        let Some(search) = found.get(symbol) else {
            return false;
        };
        resolve_into(uses, search, &declarations, scoping, order)
            && (uses.keyed == 0 || unsafe_transforms)
    });
}
