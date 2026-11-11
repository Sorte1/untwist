use oxc_ast::ast::*;
use oxc_ast_visit::{walk_js, VisitJs};
use oxc_semantic::Scoping;
use oxc_span::{GetSpan, Span};
use oxc_syntax::scope::ScopeFlags;
use oxc_syntax::symbol::SymbolId;
use rustc_hash::FxHashSet;

use super::{Refusal, STANDARD_GLOBALS};
use crate::mutants::{nested_rotator_builtins_trusted, reused_array_aliases_trusted};
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
    alias_reads: Vec<Span>,
    alias_refs: Vec<(SymbolId, Span)>,
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
            alias_reads: Vec::new(),
            alias_refs: Vec::new(),
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
            self.alias_reads
                .push(it.object.without_parentheses().span());
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

    fn visit_unary_expression(&mut self, it: &UnaryExpression<'a>) {
        if it.operator == UnaryOperator::Delete {
            if let Some(member) = it.argument.without_parentheses().as_member_expression() {
                let object = member.object();
                if self.is_alias(object) || self.array_call(object).is_some() {
                    self.refuse(notes::DECODER_NOT_READ_ONLY, it.span);
                }
            }
        }
        walk_js::walk_unary_expression(self, it);
    }

    fn visit_identifier_reference(&mut self, it: &IdentifierReference<'a>) {
        if let Some(symbol) = self.resolve(it).flatten() {
            if self.aliases.contains(&symbol) {
                self.alias_refs.push((symbol, it.span));
            }
        }
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

fn writes_of(scoping: &Scoping, symbol: SymbolId) -> usize {
    scoping
        .get_resolved_reference_ids(symbol)
        .iter()
        .filter(|reference| scoping.get_reference(**reference).is_write())
        .count()
}

struct MentionsArray<'d, 's> {
    scan: &'d DecoderScan<'s>,
    found: bool,
}

impl<'a> VisitJs<'a> for MentionsArray<'_, '_> {
    fn visit_identifier_reference(&mut self, it: &IdentifierReference<'a>) {
        let symbol = self.scan.resolve(it).flatten();
        self.found |= symbol.is_some_and(|symbol| {
            (self.scan.aliases.contains(&symbol) && !self.scan.alias_reads.contains(&it.span))
                || self.scan.function_aliases.contains(&symbol)
                || Some(symbol) == self.scan.array_symbol
        });
    }
}

fn kill_of<'k, 'a>(
    expr: &'k Expression<'a>,
    scan: &DecoderScan<'_>,
    symbol: SymbolId,
) -> Option<&'k AssignmentExpression<'a>> {
    match expr.without_parentheses() {
        Expression::SequenceExpression(sequence) => sequence
            .expressions
            .iter()
            .find_map(|part| kill_of(part, scan, symbol)),
        Expression::AssignmentExpression(assigned)
            if assigned.operator == AssignmentOperator::Assign =>
        {
            let AssignmentTarget::AssignmentTargetIdentifier(target) = &assigned.left else {
                return None;
            };
            if scan.resolve(target).flatten() != Some(symbol) {
                return None;
            }
            (!mentions_array(scan, &assigned.right)).then_some(&**assigned)
        }
        _ => None,
    }
}

fn mentions_array(scan: &DecoderScan<'_>, expr: &Expression<'_>) -> bool {
    let mut mentions = MentionsArray { scan, found: false };
    mentions.visit_expression(expr);
    mentions.found
}

#[derive(Clone, Copy, PartialEq)]
enum AliasUse {
    Index,
    Other,
    Write { clean: bool },
}

struct AliasOrder<'d, 's, 'k, 'a> {
    scan: &'d DecoderScan<'s>,
    symbol: SymbolId,
    kill: &'k AssignmentExpression<'a>,
    depth: u32,
    position: u32,
    in_target: bool,
    nested: bool,
    declared: Option<u32>,
    killed: Option<u32>,
    uses: Vec<(u32, AliasUse)>,
}

impl AliasOrder<'_, '_, '_, '_> {
    fn is_symbol(&self, expr: &Expression<'_>) -> bool {
        matches!(expr.without_parentheses(), Expression::Identifier(id)
            if self.scan.resolve(id).flatten() == Some(self.symbol))
    }

    fn note(&mut self, kind: AliasUse) {
        self.nested |= self.depth > 0;
        self.position += 1;
        self.uses.push((self.position, kind));
    }

    fn nest(&mut self, walk: impl FnOnce(&mut Self)) {
        self.depth += 1;
        walk(self);
        self.depth -= 1;
    }

    fn proves_read_only(&self) -> bool {
        let (Some(declared), Some(killed)) = (self.declared, self.killed) else {
            return false;
        };
        !self.nested
            && declared < killed
            && self.uses.iter().all(|(position, kind)| match kind {
                AliasUse::Write { clean } => *clean && *position > killed,
                AliasUse::Index => true,
                AliasUse::Other => *position > killed,
            })
    }
}

impl<'a> VisitJs<'a> for AliasOrder<'_, '_, '_, 'a> {
    fn visit_function(&mut self, it: &Function<'a>, flags: ScopeFlags) {
        self.nest(|order| walk_js::walk_function(order, it, flags));
    }

    fn visit_arrow_function_expression(&mut self, it: &ArrowFunctionExpression<'a>) {
        self.nest(|order| walk_js::walk_arrow_function_expression(order, it));
    }

    fn visit_class(&mut self, it: &Class<'a>) {
        self.nest(|order| walk_js::walk_class(order, it));
    }

    fn visit_variable_declarator(&mut self, it: &VariableDeclarator<'a>) {
        walk_js::walk_variable_declarator(self, it);
        let BindingPattern::BindingIdentifier(id) = &it.id else {
            return;
        };
        if id.symbol_id.get() == Some(self.symbol) {
            self.nested |= self.depth > 0;
            self.position += 1;
            self.declared.get_or_insert(self.position);
        }
    }

    fn visit_assignment_expression(&mut self, it: &AssignmentExpression<'a>) {
        let AssignmentTarget::AssignmentTargetIdentifier(target) = &it.left else {
            walk_js::walk_assignment_expression(self, it);
            return;
        };
        if self.scan.resolve(target).flatten() != Some(self.symbol) {
            walk_js::walk_assignment_expression(self, it);
            return;
        }
        self.visit_expression(&it.right);
        if std::ptr::eq(it, self.kill) && self.depth == 0 {
            self.position += 1;
            self.killed.get_or_insert(self.position);
            return;
        }
        let clean =
            it.operator == AssignmentOperator::Assign && !mentions_array(self.scan, &it.right);
        self.note(AliasUse::Write { clean });
    }

    fn visit_simple_assignment_target(&mut self, it: &SimpleAssignmentTarget<'a>) {
        self.in_target = it.as_member_expression().is_some();
        walk_js::walk_simple_assignment_target(self, it);
        self.in_target = false;
    }

    fn visit_unary_expression(&mut self, it: &UnaryExpression<'a>) {
        self.in_target = it.operator == UnaryOperator::Delete;
        self.visit_expression(&it.argument);
        self.in_target = false;
    }

    fn visit_computed_member_expression(&mut self, it: &ComputedMemberExpression<'a>) {
        let target = std::mem::take(&mut self.in_target);
        if self.is_symbol(&it.object) {
            self.note(if target {
                AliasUse::Other
            } else {
                AliasUse::Index
            });
        } else {
            self.visit_expression(&it.object);
        }
        self.visit_expression(&it.expression);
    }

    fn visit_static_member_expression(&mut self, it: &StaticMemberExpression<'a>) {
        self.in_target = false;
        walk_js::walk_static_member_expression(self, it);
    }

    fn visit_identifier_reference(&mut self, it: &IdentifierReference<'a>) {
        self.in_target = false;
        if self.scan.resolve(it).flatten() == Some(self.symbol) {
            self.note(AliasUse::Other);
        }
    }
}

fn top_level_kill<'k, 'a>(
    function: &'k Function<'a>,
    scan: &DecoderScan<'_>,
    symbol: SymbolId,
) -> Option<&'k AssignmentExpression<'a>> {
    function
        .body
        .as_ref()?
        .statements
        .iter()
        .find_map(|stmt| match stmt {
            Statement::ExpressionStatement(node) => kill_of(&node.expression, scan, symbol),
            _ => None,
        })
}

fn reused_alias_is_read_only(
    scoping: &Scoping,
    scan: &DecoderScan<'_>,
    function: &Function<'_>,
    symbol: SymbolId,
) -> bool {
    if reused_array_aliases_trusted() {
        return true;
    }
    let Some(kill) = top_level_kill(function, scan, symbol) else {
        return false;
    };
    let Some(body) = function.body.as_ref() else {
        return false;
    };
    let mut order = AliasOrder {
        scan,
        symbol,
        kill,
        depth: 0,
        position: 0,
        in_target: false,
        nested: false,
        declared: None,
        killed: None,
        uses: Vec::new(),
    };
    order.visit_formal_parameters(&function.params);
    order.visit_function_body(body);
    let writes = order
        .uses
        .iter()
        .filter(|(_, kind)| matches!(kind, AliasUse::Write { .. }))
        .count();
    let seen = scan
        .alias_refs
        .iter()
        .filter(|(alias, _)| *alias == symbol)
        .count();
    writes + 1 == writes_of(scoping, symbol)
        && seen == scoping.get_resolved_reference_ids(symbol).len()
        && order.proves_read_only()
}

fn aliases_only_read(scoping: &Scoping, scan: &DecoderScan<'_>, function: &Function<'_>) -> bool {
    let (reused, plain): (Vec<SymbolId>, Vec<SymbolId>) = scan
        .aliases
        .iter()
        .copied()
        .partition(|symbol| writes_of(scoping, *symbol) > 0);
    let references = plain
        .iter()
        .map(|symbol| scoping.get_resolved_reference_ids(*symbol).len())
        .sum::<usize>();
    let plain_reads = scan
        .alias_refs
        .iter()
        .filter(|(alias, span)| plain.contains(alias) && scan.alias_reads.contains(span))
        .count();
    references == plain_reads
        && reused
            .iter()
            .all(|symbol| reused_alias_is_read_only(scoping, scan, function, *symbol))
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
    if !aliases_only_read(scoping, &scan, decoder.function) {
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

pub fn statement_used_names<'a>(scoping: &Scoping, statement: &Statement<'a>) -> UsedNames<'a> {
    let mut scan = UsedScan {
        scoping,
        used: UsedNames::default(),
    };
    scan.visit_statement(statement);
    scan.used
}
