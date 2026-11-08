use oxc_ast::ast::*;
use oxc_ast_visit::{walk_js, VisitJs};
use oxc_semantic::Scoping;
use oxc_span::{GetSpan, Span};
use oxc_syntax::scope::ScopeFlags;
use oxc_syntax::symbol::SymbolId;
use rustc_hash::FxHashSet;

use super::{Refusal, STANDARD_GLOBALS};
use crate::mutants::nested_rotator_builtins_trusted;
use crate::notes;
use crate::stamp::StampedFn;

struct DecoderScan<'s> {
    scoping: &'s Scoping,
    decoder: Span,
    decoder_symbol: Option<SymbolId>,
    array_symbol: Option<SymbolId>,
    allowed_calls: FxHashSet<Span>,
    function_aliases: FxHashSet<SymbolId>,
    aliases: FxHashSet<SymbolId>,
    alias_reads: usize,
    fail: Option<Refusal>,
}

impl<'s> DecoderScan<'s> {
    fn new(scoping: &'s Scoping, decoder: &StampedFn<'_>, array_symbol: Option<SymbolId>) -> Self {
        Self {
            scoping,
            decoder: decoder.statement,
            decoder_symbol: decoder.symbol,
            array_symbol,
            allowed_calls: FxHashSet::default(),
            function_aliases: FxHashSet::default(),
            aliases: FxHashSet::default(),
            alias_reads: 0,
            fail: None,
        }
    }

    fn resolve(&self, id: &IdentifierReference<'_>) -> Option<Option<SymbolId>> {
        Some(
            self.scoping
                .get_reference(id.reference_id.get()?)
                .symbol_id(),
        )
    }

    fn is_array_function(&self, id: &IdentifierReference<'_>) -> bool {
        self.resolve(id)
            .flatten()
            .is_some_and(|s| Some(s) == self.array_symbol)
    }

    fn is_array(&self, id: &IdentifierReference<'_>) -> bool {
        self.resolve(id).flatten().is_some_and(|symbol| {
            Some(symbol) == self.array_symbol || self.function_aliases.contains(&symbol)
        })
    }

    fn array_call(&self, expr: &Expression<'_>) -> Option<Span> {
        let Expression::CallExpression(call) = expr.without_parentheses() else {
            return None;
        };
        let Expression::Identifier(id) = &call.callee else {
            return None;
        };
        (self.is_array(id) && call.arguments.is_empty()).then_some(call.span)
    }

    fn is_array_reference(&self, expr: &Expression<'_>) -> bool {
        matches!(expr.without_parentheses(), Expression::Identifier(id) if self.is_array(id))
    }

    fn is_alias(&self, expr: &Expression<'_>) -> bool {
        let Expression::Identifier(id) = expr.without_parentheses() else {
            return false;
        };
        self.resolve(id)
            .flatten()
            .is_some_and(|symbol| self.aliases.contains(&symbol))
    }

    fn refuse(&mut self, note: &'static str, span: Span) {
        self.fail.get_or_insert(Refusal::at(note, span));
    }
}

struct FunctionAliases<'s, 'd> {
    scan: &'d mut DecoderScan<'s>,
}

impl<'a> VisitJs<'a> for FunctionAliases<'_, '_> {
    fn visit_variable_declarator(&mut self, it: &VariableDeclarator<'a>) {
        if let (BindingPattern::BindingIdentifier(id), Some(Expression::Identifier(init))) = (
            &it.id,
            it.init.as_ref().map(Expression::without_parentheses),
        ) {
            if self.scan.is_array_function(init) {
                if let Some(symbol) = id.symbol_id.get() {
                    self.scan.function_aliases.insert(symbol);
                }
            }
        }
        walk_js::walk_variable_declarator(self, it);
    }
}

impl<'a> VisitJs<'a> for DecoderScan<'_> {
    fn visit_variable_declarator(&mut self, it: &VariableDeclarator<'a>) {
        if let (BindingPattern::BindingIdentifier(id), Some(init)) = (&it.id, &it.init) {
            if let Some(call) = self.array_call(init) {
                self.allowed_calls.insert(call);
                if let Some(symbol) = id.symbol_id.get() {
                    self.aliases.insert(symbol);
                }
            }
        }
        walk_js::walk_variable_declarator(self, it);
    }

    fn visit_static_member_expression(&mut self, it: &StaticMemberExpression<'a>) {
        if self.is_array_reference(&it.object) {
            self.refuse(notes::DECODER_NOT_READ_ONLY, it.span);
        }
        walk_js::walk_static_member_expression(self, it);
    }

    fn visit_computed_member_expression(&mut self, it: &ComputedMemberExpression<'a>) {
        if self.is_array_reference(&it.object) {
            self.refuse(notes::DECODER_NOT_READ_ONLY, it.span);
        }
        if let Some(call) = self.array_call(&it.object) {
            self.allowed_calls.insert(call);
        }
        if self.is_alias(&it.object) {
            self.alias_reads += 1;
        }
        walk_js::walk_computed_member_expression(self, it);
    }

    fn visit_call_expression(&mut self, it: &CallExpression<'a>) {
        if let Some(member) = it.callee.without_parentheses().as_member_expression() {
            if self.is_alias(member.object()) || self.array_call(member.object()).is_some() {
                self.refuse(notes::DECODER_NOT_READ_ONLY, it.span);
            }
        }
        let bare_array_call = matches!(&it.callee, Expression::Identifier(id) if self.is_array(id));
        if bare_array_call && !self.allowed_calls.contains(&it.span) {
            self.refuse(notes::DECODER_NOT_READ_ONLY, it.span);
        }
        walk_js::walk_call_expression(self, it);
    }

    fn visit_simple_assignment_target(&mut self, it: &SimpleAssignmentTarget<'a>) {
        if let Some(member) = it.as_member_expression() {
            if self.is_alias(member.object()) {
                self.refuse(notes::DECODER_NOT_READ_ONLY, member.span());
            }
        }
        walk_js::walk_simple_assignment_target(self, it);
    }

    fn visit_identifier_reference(&mut self, it: &IdentifierReference<'a>) {
        let ok = match self.resolve(it) {
            None => false,
            Some(None) => STANDARD_GLOBALS.contains(&it.name.as_str()),
            Some(Some(symbol)) => {
                let declared = self.scoping.symbol_span(symbol);
                let inside =
                    declared.start >= self.decoder.start && declared.end <= self.decoder.end;
                inside || Some(symbol) == self.decoder_symbol || Some(symbol) == self.array_symbol
            }
        };
        if !ok {
            self.refuse(notes::DECODER_READS_OUTER_BINDING, it.span);
        }
    }
}

fn aliases_only_read(scoping: &Scoping, scan: &DecoderScan<'_>) -> bool {
    let references: Vec<_> = scan
        .aliases
        .iter()
        .flat_map(|symbol| scoping.get_resolved_reference_ids(*symbol))
        .collect();
    let writes = references
        .iter()
        .filter(|reference| scoping.get_reference(***reference).is_write())
        .count();
    writes == 0 && references.len() == scan.alias_reads
}

pub fn decoder_refusal(
    scoping: &Scoping,
    decoder: &StampedFn<'_>,
    array_symbol: Option<SymbolId>,
) -> Option<Refusal> {
    let mut scan = DecoderScan::new(scoping, decoder, array_symbol);
    FunctionAliases { scan: &mut scan }.visit_function(decoder.function, ScopeFlags::Function);
    scan.visit_function(decoder.function, ScopeFlags::Function);
    if scan.fail.is_some() {
        return scan.fail;
    }
    if !aliases_only_read(scoping, &scan) {
        return Some(Refusal::at(notes::DECODER_NOT_READ_ONLY, decoder.statement));
    }
    None
}

#[derive(Clone, Debug, Default)]
pub struct UsedNames<'a> {
    pub names: Vec<&'a str>,
}

impl<'a> UsedNames<'a> {
    pub fn uses(&self, name: &str) -> bool {
        self.names.contains(&name)
    }

    fn note(&mut self, name: &'a str) {
        if !self.names.contains(&name) {
            self.names.push(name);
        }
    }
}

struct UsedScan<'s, 'a> {
    scoping: &'s Scoping,
    used: UsedNames<'a>,
}

impl<'a> VisitJs<'a> for UsedScan<'_, 'a> {
    fn visit_static_member_expression(&mut self, it: &StaticMemberExpression<'a>) {
        self.used.note(it.property.name.as_str());
        walk_js::walk_static_member_expression(self, it);
    }

    fn visit_computed_member_expression(&mut self, it: &ComputedMemberExpression<'a>) {
        if let Expression::StringLiteral(key) = it.expression.without_parentheses() {
            if let Some(key) = key.value.as_str() {
                self.used.note(key);
            }
        }
        walk_js::walk_computed_member_expression(self, it);
    }

    fn visit_identifier_reference(&mut self, it: &IdentifierReference<'a>) {
        let global = it
            .reference_id
            .get()
            .is_none_or(|r| self.scoping.get_reference(r).symbol_id().is_none());
        if global {
            self.used.note(it.name.as_str());
        }
    }
}

pub fn used_names<'a>(
    scoping: &Scoping,
    decoders: &[&StampedFn<'a>],
    nested_rotator: Option<&Expression<'a>>,
) -> UsedNames<'a> {
    let mut scan = UsedScan {
        scoping,
        used: UsedNames::default(),
    };
    for decoder in decoders {
        scan.visit_function(decoder.function, ScopeFlags::Function);
    }
    if let Some(rotator) = nested_rotator.filter(|_| !nested_rotator_builtins_trusted()) {
        scan.visit_expression(rotator);
    }
    scan.used.names.shrink_to_fit();
    scan.used
}
