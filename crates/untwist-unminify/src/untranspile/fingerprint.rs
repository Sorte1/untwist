use oxc_ast::ast::*;
use oxc_ast::{AstKind, AstType};
use oxc_ast_visit::{walk_js, VisitJs};
use oxc_semantic::Scoping;
use oxc_syntax::operator::UnaryOperator;
use oxc_syntax::scope::{ScopeFlags, ScopeId};
use oxc_syntax::symbol::SymbolId;
use sha2::{Digest, Sha256};

pub const RECORDING_CAP: u32 = 12_000;
const CLOSE: u8 = 0xff;
const GLOBAL: u8 = 0xf0;
const OWN: u8 = 0xf1;
const LOCAL: u8 = 0xf2;
const EXTERNAL: u8 = 0xf3;
const LOCAL_SLOTS: usize = 32;
const EXTERNAL_SLOTS: usize = 8;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Symbols<const N: usize> {
    symbols: [Option<SymbolId>; N],
    counts: [u32; N],
    len: usize,
}

pub type Externals = Symbols<EXTERNAL_SLOTS>;

impl<const N: usize> Symbols<N> {
    pub const EMPTY: Self = Self {
        symbols: [None; N],
        counts: [0; N],
        len: 0,
    };

    pub fn count(&self) -> usize {
        self.len
    }

    pub fn iter(&self) -> impl Iterator<Item = (SymbolId, u32)> + '_ {
        self.symbols
            .iter()
            .zip(self.counts)
            .take(self.len)
            .filter_map(|(symbol, count)| symbol.map(|symbol| (symbol, count)))
    }

    fn position(&self, symbol: SymbolId) -> Option<usize> {
        self.symbols
            .iter()
            .take(self.len)
            .position(|slot| *slot == Some(symbol))
    }

    fn seen_again(&mut self, index: usize) {
        if let Some(count) = self.counts.get_mut(index) {
            *count += 1;
        }
    }

    fn push(&mut self, symbol: SymbolId) -> Option<usize> {
        let index = self.len;
        *self.symbols.get_mut(index)? = Some(symbol);
        *self.counts.get_mut(index)? = 1;
        self.len += 1;
        Some(index)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Print {
    pub hash: u128,
    pub tokens: u32,
    pub external: Externals,
    pub own_reads: u32,
    pub own_writes: u32,
}

struct Printer<'s> {
    scoping: &'s Scoping,
    cap: u32,
    outer: ScopeId,
    own: Option<SymbolId>,
    stream: Vec<u8>,
    tokens: u32,
    locals: Symbols<LOCAL_SLOTS>,
    external: Externals,
    overflow: bool,
    own_reads: u32,
    own_writes: u32,
}

pub fn of_function(
    function: &Function<'_>,
    own: Option<SymbolId>,
    outer: ScopeId,
    scoping: &Scoping,
    cap: u32,
) -> Option<Print> {
    let mut printer = Printer::new(scoping, outer, own, cap);
    printer.visit_function(function, ScopeFlags::Function);
    printer.finish()
}

pub fn of_expression(
    expression: &Expression<'_>,
    own: Option<SymbolId>,
    outer: ScopeId,
    scoping: &Scoping,
    cap: u32,
) -> Option<Print> {
    let mut printer = Printer::new(scoping, outer, own, cap);
    printer.visit_expression(expression);
    printer.finish()
}

impl<'s> Printer<'s> {
    fn new(scoping: &'s Scoping, outer: ScopeId, own: Option<SymbolId>, cap: u32) -> Self {
        Self {
            scoping,
            cap,
            outer,
            own,
            stream: Vec::new(),
            tokens: 0,
            locals: Symbols::EMPTY,
            external: Externals::EMPTY,
            overflow: false,
            own_reads: 0,
            own_writes: 0,
        }
    }

    fn finish(self) -> Option<Print> {
        (!self.overflow && self.tokens <= self.cap).then(|| Print {
            hash: digest(&self.stream),
            tokens: self.tokens,
            external: self.external,
            own_reads: self.own_reads,
            own_writes: self.own_writes,
        })
    }

    fn byte(&mut self, byte: u8) {
        self.stream.push(byte);
    }

    fn word(&mut self, word: u64) {
        word.to_le_bytes()
            .into_iter()
            .for_each(|byte| self.byte(byte));
    }

    fn text(&mut self, text: &str) {
        self.bytes(text.as_bytes());
    }

    fn bytes(&mut self, bytes: &[u8]) {
        self.word(bytes.len() as u64);
        bytes.iter().for_each(|&byte| self.byte(byte));
    }

    fn flag(&mut self, flag: bool) {
        self.byte(u8::from(flag));
    }

    fn is_local(&self, symbol: SymbolId) -> bool {
        let mut scope = self.scoping.symbol_scope_id(symbol);
        if scope == self.outer {
            return false;
        }
        while let Some(parent) = self.scoping.scope_parent_id(scope) {
            if parent == self.outer {
                return true;
            }
            scope = parent;
        }
        false
    }

    fn slot(&mut self, tag: u8, index: usize) {
        self.byte(tag);
        self.word(index as u64);
    }

    fn symbol(&mut self, symbol: SymbolId) {
        if let Some(index) = self.locals.position(symbol) {
            return self.slot(LOCAL, index);
        }
        if let Some(index) = self.external.position(symbol) {
            self.external.seen_again(index);
            return self.slot(EXTERNAL, index);
        }
        let (tag, pushed) = if self.is_local(symbol) {
            (LOCAL, self.locals.push(symbol))
        } else {
            (EXTERNAL, self.external.push(symbol))
        };
        match pushed {
            Some(index) => self.slot(tag, index),
            None => self.overflow = true,
        }
    }

    fn reference(&mut self, it: &IdentifierReference<'_>) {
        let reference = it
            .reference_id
            .get()
            .map(|id| self.scoping.get_reference(id));
        match reference.and_then(|reference| reference.symbol_id()) {
            None => {
                self.byte(GLOBAL);
                self.text(it.name.as_str());
            }
            Some(symbol) if Some(symbol) == self.own => {
                self.byte(OWN);
                if reference.is_some_and(|reference| reference.flags().is_write()) {
                    self.own_writes += 1;
                } else {
                    self.own_reads += 1;
                }
            }
            Some(symbol) => self.symbol(symbol),
        }
    }

    fn binding(&mut self, it: &BindingIdentifier<'_>) {
        match it.symbol_id.get() {
            Some(symbol) if Some(symbol) == self.own => self.byte(OWN),
            Some(symbol) => self.symbol(symbol),
            None => self.byte(GLOBAL),
        }
    }

    fn canonical(&mut self, kind: AstType, payload: impl FnOnce(&mut Self)) {
        self.tokens += 1;
        self.byte(kind as u8);
        payload(self);
        self.byte(CLOSE);
    }

    fn details(&mut self, kind: AstKind<'_>) {
        match kind {
            AstKind::IdentifierReference(it) => self.reference(it),
            AstKind::BindingIdentifier(it) => self.binding(it),
            AstKind::IdentifierName(it) => self.text(it.name.as_str()),
            AstKind::PrivateIdentifier(it) => self.text(it.name.as_str()),
            AstKind::StringLiteral(it) => self.bytes(it.value.as_bytes()),
            AstKind::NumericLiteral(it) => self.word(it.value.to_bits()),
            AstKind::BigIntLiteral(it) => self.text(it.value.as_str()),
            AstKind::BooleanLiteral(it) => self.flag(it.value),
            AstKind::RegExpLiteral(it) => {
                self.text(it.regex.pattern.text.as_str());
                self.byte(it.regex.flags.bits());
            }
            AstKind::TemplateElement(it) => match it.value.cooked {
                Some(cooked) => self.bytes(cooked.as_bytes()),
                None => self.text(it.value.raw.as_str()),
            },
            _ => self.operator_details(kind),
        }
    }

    fn operator_details(&mut self, kind: AstKind<'_>) {
        match kind {
            AstKind::BinaryExpression(it) => self.byte(it.operator as u8),
            AstKind::LogicalExpression(it) => self.byte(it.operator as u8),
            AstKind::UnaryExpression(it) => self.byte(it.operator as u8),
            AstKind::UpdateExpression(it) => {
                self.byte(it.operator as u8);
                self.flag(it.prefix);
            }
            AstKind::AssignmentExpression(it) => self.byte(it.operator as u8),
            _ => self.shape_details(kind),
        }
    }

    fn shape_details(&mut self, kind: AstKind<'_>) {
        match kind {
            AstKind::VariableDeclaration(it) => self.byte(it.kind as u8),
            AstKind::Function(it) => {
                self.byte(it.r#type as u8);
                self.flag(it.generator);
                self.flag(it.r#async);
            }
            AstKind::ArrowFunctionExpression(it) => self.flag(it.r#async),
            AstKind::ObjectProperty(it) => {
                self.byte(it.kind as u8);
                self.flag(it.method);
                self.flag(it.shorthand);
                self.flag(it.computed);
            }
            AstKind::ForStatement(it) => {
                self.flag(it.init.is_some());
                self.flag(it.test.is_some());
                self.flag(it.update.is_some());
            }
            AstKind::ForOfStatement(it) => self.flag(it.r#await),
            AstKind::CallExpression(it) => self.flag(it.optional),
            AstKind::StaticMemberExpression(it) => self.flag(it.optional),
            AstKind::ComputedMemberExpression(it) => self.flag(it.optional),
            AstKind::MethodDefinition(it) => {
                self.byte(it.kind as u8);
                self.flag(it.r#static);
                self.flag(it.computed);
            }
            _ => {}
        }
    }
}

fn digest(stream: &[u8]) -> u128 {
    let full = Sha256::digest(stream);
    let mut head = [0u8; 16];
    head.copy_from_slice(&full[..16]);
    u128::from_be_bytes(head)
}

fn canonical_unary(it: &UnaryExpression<'_>) -> Option<Option<bool>> {
    let Expression::NumericLiteral(number) = &it.argument else {
        return None;
    };
    match it.operator {
        UnaryOperator::LogicalNot if number.value == 0.0 => Some(Some(true)),
        UnaryOperator::LogicalNot if number.value == 1.0 => Some(Some(false)),
        UnaryOperator::Void if number.value == 0.0 => Some(None),
        _ => None,
    }
}

impl<'a> VisitJs<'a> for Printer<'_> {
    fn enter_node(&mut self, kind: AstKind<'a>) {
        if matches!(kind, AstKind::ParenthesizedExpression(_)) {
            return;
        }
        self.tokens += 1;
        self.byte(kind.ty() as u8);
        self.details(kind);
    }

    fn leave_node(&mut self, kind: AstKind<'a>) {
        if !matches!(kind, AstKind::ParenthesizedExpression(_)) {
            self.byte(CLOSE);
        }
    }

    fn visit_unary_expression(&mut self, it: &UnaryExpression<'a>) {
        match canonical_unary(it) {
            Some(Some(value)) => self.canonical(AstType::BooleanLiteral, |printer| {
                printer.flag(value);
            }),
            Some(None) => self.canonical(AstType::IdentifierReference, |printer| {
                printer.byte(GLOBAL);
                printer.text("undefined");
            }),
            None => walk_js::walk_unary_expression(self, it),
        }
    }

    fn visit_expression(&mut self, it: &Expression<'a>) {
        if self.tokens > self.cap {
            return;
        }
        walk_js::walk_expression(self, it);
    }

    fn visit_statement(&mut self, it: &Statement<'a>) {
        if self.tokens > self.cap {
            return;
        }
        walk_js::walk_statement(self, it);
    }
}
