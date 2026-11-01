use oxc_ast::ast::*;
use oxc_ast_visit::{walk_js, VisitJs};
use oxc_semantic::Scoping;
use oxc_span::Span;
use oxc_syntax::reference::ReferenceId;
use oxc_syntax::scope::ScopeId;
use oxc_syntax::symbol::{SymbolFlags, SymbolId};

fn is_identifier_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_' || c == '$'
}

fn bounded_at(text: &str, name: &str, at: usize) -> bool {
    let before = text.get(..at).and_then(|head| head.chars().next_back());
    let after = text
        .get(at + name.len()..)
        .and_then(|tail| tail.chars().next());
    !before.is_some_and(is_identifier_char) && !after.is_some_and(is_identifier_char)
}

fn token_at(text: &str, name: &str) -> bool {
    let (bytes, wanted) = (text.as_bytes(), name.as_bytes());
    let Some(first) = wanted.first() else {
        return false;
    };
    let mut from = 0;
    while let Some(offset) = bytes
        .get(from..)
        .and_then(|rest| rest.iter().position(|b| b == first))
    {
        let at = from + offset;
        if bytes.get(at..).is_some_and(|rest| rest.starts_with(wanted))
            && bounded_at(text, name, at)
        {
            return true;
        }
        from = at + 1;
    }
    false
}

fn names_in(text: &str, name: &str) -> bool {
    !name.is_empty() && text.len() >= name.len() && token_at(text, name)
}

pub fn names_a_token(text: &str, names: &[&str]) -> bool {
    names.iter().any(|name| names_in(text, name))
}

pub struct Write {
    pub index: usize,
    pub reference: ReferenceId,
    pub span: Span,
}

#[derive(Default)]
pub struct NameScan {
    pub spelled: Vec<Option<Span>>,
    pub writes: Vec<Write>,
}

pub fn mark_spelled(spelled: &mut [Option<Span>], names: &[&str], text: &str, span: Span) {
    for (slot, name) in spelled.iter_mut().zip(names) {
        if slot.is_none() && names_in(text, name) {
            *slot = Some(span);
        }
    }
}

struct Spelled<'n> {
    names: &'n [&'n str],
    scan: NameScan,
}

impl Spelled<'_> {
    fn check(&mut self, text: &str, span: Span) {
        mark_spelled(&mut self.scan.spelled, self.names, text, span);
    }

    fn write(&mut self, id: &IdentifierReference<'_>) {
        let Some(index) = self.names.iter().position(|name| *name == id.name.as_str()) else {
            return;
        };
        if let Some(reference) = id.reference_id.get() {
            self.scan.writes.push(Write {
                index,
                reference,
                span: id.span,
            });
        }
    }
}

impl<'a> VisitJs<'a> for Spelled<'_> {
    fn visit_string_literal(&mut self, it: &StringLiteral<'a>) {
        self.check(it.value.as_str(), it.span);
    }

    fn visit_template_element(&mut self, it: &TemplateElement<'a>) {
        self.check(it.value.raw.as_str(), it.span);
    }

    fn visit_reg_exp_literal(&mut self, it: &RegExpLiteral<'a>) {
        self.check(it.regex.pattern.text.as_str(), it.span);
    }

    fn visit_static_member_expression(&mut self, it: &StaticMemberExpression<'a>) {
        self.check(it.property.name.as_str(), it.property.span);
        walk_js::walk_static_member_expression(self, it);
    }

    fn visit_object_property(&mut self, it: &ObjectProperty<'a>) {
        if it.computed {
            walk_js::walk_object_property(self, it);
        } else {
            self.visit_expression(&it.value);
        }
    }

    fn visit_property_key(&mut self, it: &PropertyKey<'a>) {
        if let PropertyKey::StaticIdentifier(id) = it {
            self.check(id.name.as_str(), id.span);
        }
        walk_js::walk_property_key(self, it);
    }

    fn visit_simple_assignment_target(&mut self, it: &SimpleAssignmentTarget<'a>) {
        if let SimpleAssignmentTarget::AssignmentTargetIdentifier(id) = it {
            self.write(id);
        }
        walk_js::walk_simple_assignment_target(self, it);
    }
}

pub fn scan_names(program: &Program<'_>, names: &[&str]) -> NameScan {
    let mut spelled = Spelled {
        names,
        scan: NameScan {
            spelled: vec![None; names.len()],
            writes: Vec::new(),
        },
    };
    spelled.visit_program(program);
    spelled.scan
}

pub fn spelled_at(program: &Program<'_>, names: &[&str]) -> Vec<Option<Span>> {
    scan_names(program, names).spelled
}

fn is_within(scoping: &Scoping, scope: ScopeId, outer: ScopeId) -> bool {
    scoping
        .scope_ancestors(scope)
        .any(|ancestor| ancestor == outer)
}

fn is_block_level(scoping: &Scoping, candidate: SymbolId, home: ScopeId) -> bool {
    let scope = scoping.symbol_scope_id(candidate);
    !scoping.scope_flags(scope).is_var() && is_within(scoping, scope, home)
}

pub fn redeclared(scoping: &Scoping, symbol: SymbolId) -> Option<Span> {
    if let Some(again) = scoping.symbol_redeclarations(symbol).first() {
        return Some(again.span);
    }
    let name = scoping.symbol_name(symbol);
    let home = scoping.symbol_scope_id(symbol);
    scoping
        .symbol_ids()
        .filter(|candidate| {
            scoping
                .symbol_flags(*candidate)
                .contains(SymbolFlags::Function)
        })
        .filter(|candidate| *candidate != symbol && scoping.symbol_name(*candidate) == name)
        .find(|candidate| is_block_level(scoping, *candidate, home))
        .map(|candidate| scoping.symbol_span(candidate))
}

fn encloses(outer: Span, inner: Span) -> bool {
    outer.start <= inner.start && inner.end <= outer.end
}

pub fn writes_outside(
    scoping: &Scoping,
    symbol: SymbolId,
    statement: Span,
    writes: &[Write],
    index: usize,
) -> bool {
    let total = scoping
        .get_resolved_reference_ids(symbol)
        .iter()
        .filter(|reference| scoping.get_reference(**reference).is_write())
        .count();
    if total == 0 {
        return false;
    }
    let inside = writes
        .iter()
        .filter(|write| write.index == index && encloses(statement, write.span))
        .filter(|write| scoping.get_reference(write.reference).symbol_id() == Some(symbol))
        .count();
    total > inside
}

pub fn written_outside(
    program: &Program<'_>,
    scoping: &Scoping,
    symbol: SymbolId,
    statement: Span,
) -> bool {
    let scan = scan_names(program, &[scoping.symbol_name(symbol)]);
    writes_outside(scoping, symbol, statement, &scan.writes, 0)
}
