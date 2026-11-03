use std::borrow::Cow;

use oxc_ast::ast::*;
use oxc_semantic::Scoping;
use oxc_span::{GetSpan, Span};
use oxc_syntax::symbol::SymbolId;
use rustc_hash::FxHashMap;

use super::funcs::{param_symbol, Func};
use super::{Flow, UNREACHED};
use crate::calls::agnostic::sites::span_key;
use untwist_eval::value::JsString;

const INERT_IDENT_NATIVES: [&str; 6] = [
    "String",
    "Number",
    "Boolean",
    "isNaN",
    "parseInt",
    "parseFloat",
];

const IMPLICIT_CALL_FIELDS: [&str; 11] = [
    "toString",
    "valueOf",
    "then",
    "get",
    "set",
    "next",
    "return",
    "throw",
    "toJSON",
    "handleEvent",
    "constructor",
];

fn is_index_name(name: &str) -> bool {
    !name.is_empty()
        && name.bytes().all(|byte| byte.is_ascii_digit())
        && (name == "0" || !name.starts_with('0'))
}

pub(super) enum Key<'k> {
    Named(Cow<'k, str>),
    Any,
    Index,
}

fn named<'k>(name: Cow<'k, str>) -> Key<'k> {
    if is_index_name(&name) {
        Key::Index
    } else {
        Key::Named(name)
    }
}

pub(super) enum Callee<'k, 'a> {
    Declared(SymbolId),
    Direct(&'k FormalParameters<'a>),
}

pub(super) struct Collector<'s> {
    scoping: &'s Scoping,
    resolve: Option<&'s FxHashMap<Span, &'s JsString>>,
    funcs: FxHashMap<SymbolId, Func>,
    next: u32,
    reads: FxHashMap<u64, u32>,
    binds: Vec<u32>,
    named: FxHashMap<String, u32>,
    any: Option<u32>,
    index: Option<u32>,
    rets: FxHashMap<u32, u32>,
    edges: Vec<(u32, u32)>,
    sinks: Vec<(u32, Span)>,
    pub(super) fn_stack: Vec<u32>,
}

impl<'s> Collector<'s> {
    pub(super) fn new(
        scoping: &'s Scoping,
        resolve: Option<&'s FxHashMap<Span, &'s JsString>>,
        funcs: FxHashMap<SymbolId, Func>,
    ) -> Self {
        Self {
            scoping,
            resolve,
            funcs,
            next: 0,
            reads: FxHashMap::default(),
            binds: vec![UNREACHED; scoping.symbols_len()],
            named: FxHashMap::default(),
            any: None,
            index: None,
            rets: FxHashMap::default(),
            edges: Vec::new(),
            sinks: Vec::new(),
            fn_stack: Vec::new(),
        }
    }

    fn alloc(&mut self) -> u32 {
        let node = self.next;
        self.next += 1;
        node
    }

    pub(super) fn read_node(&mut self, span: Span) -> u32 {
        let key = span_key(span);
        if let Some(&node) = self.reads.get(&key) {
            return node;
        }
        let node = self.alloc();
        self.reads.insert(key, node);
        node
    }

    pub(super) fn bind_node(&mut self, symbol: SymbolId) -> Option<u32> {
        let index = symbol.index();
        let held = *self.binds.get(index)?;
        if held != UNREACHED {
            return Some(held);
        }
        let node = self.alloc();
        self.binds[index] = node;
        Some(node)
    }

    fn any_node(&mut self) -> u32 {
        if let Some(node) = self.any {
            return node;
        }
        let node = self.alloc();
        self.any = Some(node);
        node
    }

    pub(super) fn field_node(&mut self, key: &Key<'_>) -> u32 {
        let found = match key {
            Key::Any => return self.any_node(),
            Key::Index => self.index,
            Key::Named(name) => self.named.get(name.as_ref()).copied(),
        };
        if let Some(node) = found {
            return node;
        }
        let node = self.alloc();
        match key {
            Key::Named(name) => {
                self.named.insert(name.to_string(), node);
            }
            Key::Index | Key::Any => self.index = Some(node),
        }
        let any = self.any_node();
        self.edge(any, node);
        node
    }

    pub(super) fn ret_node(&mut self, span_start: u32) -> u32 {
        if let Some(&node) = self.rets.get(&span_start) {
            return node;
        }
        let node = self.alloc();
        self.rets.insert(span_start, node);
        node
    }

    pub(super) fn edge(&mut self, from: u32, to: u32) {
        self.edges.push((from, to));
    }

    pub(super) fn sink(&mut self, node: u32, span: Span) {
        self.sinks.push((node, span));
    }

    pub(super) fn symbol_of(&self, id: &IdentifierReference<'_>) -> Option<SymbolId> {
        id.reference_id
            .get()
            .and_then(|reference| self.scoping.get_reference(reference).symbol_id())
    }

    pub(super) fn key_of_member<'a>(&self, member: &MemberExpression<'a>) -> Key<'a> {
        match member {
            MemberExpression::StaticMemberExpression(m) => {
                named(Cow::Borrowed(m.property.name.as_str()))
            }
            MemberExpression::ComputedMemberExpression(m) => self.key_of_expr(&m.expression),
            MemberExpression::PrivateFieldExpression(_) => Key::Any,
        }
    }

    fn key_of_expr<'a>(&self, expr: &Expression<'a>) -> Key<'a> {
        match expr.without_parentheses() {
            Expression::StringLiteral(text) => named(Cow::Borrowed(text.value.as_str())),
            Expression::NumericLiteral(_) => Key::Index,
            other => self
                .resolve
                .and_then(|map| map.get(&other.span()))
                .and_then(|text| text.as_string())
                .map_or(Key::Any, |text| named(Cow::Owned(text))),
        }
    }

    pub(super) fn key_of_property<'a>(&self, key: &PropertyKey<'a>) -> Key<'a> {
        match key {
            PropertyKey::StaticIdentifier(name) => named(Cow::Borrowed(name.name.as_str())),
            PropertyKey::PrivateIdentifier(_) => Key::Any,
            other => other
                .as_expression()
                .map_or(Key::Any, |expr| self.key_of_expr(expr)),
        }
    }

    pub(super) fn node_of(&mut self, expr: &Expression<'_>) -> Option<u32> {
        let inner = expr.without_parentheses();
        match inner {
            Expression::Identifier(id) => self.symbol_of(id).and_then(|s| self.bind_node(s)),
            Expression::StaticMemberExpression(_) | Expression::ComputedMemberExpression(_) => {
                Some(self.read_node(inner.span()))
            }
            Expression::CallExpression(call) => self.call_result(call),
            Expression::AssignmentExpression(assign) => self.node_of(&assign.right),
            Expression::SequenceExpression(seq) => {
                seq.expressions.last().and_then(|last| self.node_of(last))
            }
            _ => None,
        }
    }

    fn call_result(&mut self, call: &CallExpression<'_>) -> Option<u32> {
        let start = match call.callee.without_parentheses() {
            Expression::FunctionExpression(func) => Some(func.span.start),
            Expression::ArrowFunctionExpression(arrow) => Some(arrow.span.start),
            Expression::Identifier(id) => self
                .symbol_of(id)
                .and_then(|symbol| self.funcs.get(&symbol))
                .map(|func| func.span_start),
            _ => None,
        };
        start.map(|span_start| self.ret_node(span_start))
    }

    pub(super) fn resolved_callee<'k, 'a>(
        &self,
        callee: &'k Expression<'a>,
    ) -> Option<Callee<'k, 'a>> {
        match callee.without_parentheses() {
            Expression::FunctionExpression(func) => Some(Callee::Direct(&func.params)),
            Expression::ArrowFunctionExpression(arrow) => Some(Callee::Direct(&arrow.params)),
            Expression::Identifier(id) => self
                .symbol_of(id)
                .filter(|symbol| self.funcs.contains_key(symbol))
                .map(Callee::Declared),
            _ => None,
        }
    }

    fn param_at(&self, callee: &Callee<'_, '_>, index: usize) -> Option<SymbolId> {
        match callee {
            Callee::Direct(params) => params.items.get(index).and_then(param_symbol),
            Callee::Declared(symbol) => self
                .funcs
                .get(symbol)
                .and_then(|func| func.params.get(index).copied().flatten()),
        }
    }

    pub(super) fn forwards(&self, member: &MemberExpression<'_>) -> bool {
        match self.key_of_member(member) {
            Key::Named(name) => matches!(name.as_ref(), "call" | "apply" | "bind"),
            Key::Any => true,
            Key::Index => false,
        }
    }

    pub(super) fn inert_native(&self, callee: &Expression<'_>) -> bool {
        match callee.without_parentheses() {
            Expression::Identifier(id) => INERT_IDENT_NATIVES.contains(&id.name.as_str()),
            other => other.as_member_expression().is_some_and(|member| {
                match member.object().without_parentheses() {
                    Expression::Identifier(obj) if obj.name == "console" => true,
                    Expression::Identifier(obj) if obj.name == "JSON" => matches!(
                        self.key_of_member(member),
                        Key::Named(name) if name == "stringify"
                    ),
                    _ => false,
                }
            }),
        }
    }

    pub(super) fn sink_arguments(&mut self, arguments: &[Argument<'_>], span: Span) {
        for argument in arguments {
            if let Some(node) = argument.as_expression().and_then(|e| self.node_of(e)) {
                self.sink(node, span);
            }
        }
    }

    pub(super) fn bind_arguments(&mut self, callee: &Callee<'_, '_>, arguments: &[Argument<'_>]) {
        for (index, argument) in arguments.iter().enumerate() {
            let Some(value) = argument.as_expression().and_then(|e| self.node_of(e)) else {
                continue;
            };
            if let Some(bind) = self
                .param_at(callee, index)
                .and_then(|param| self.bind_node(param))
            {
                self.edge(value, bind);
            }
        }
    }

    pub(super) fn store(&mut self, value: u32, key: &Key<'_>, span: Span) {
        let holder = self.field_node(key);
        self.edge(value, holder);
        let implicit = match key {
            Key::Named(name) => IMPLICIT_CALL_FIELDS.contains(&name.as_ref()),
            Key::Any => true,
            Key::Index => false,
        };
        if implicit {
            self.sink(value, span);
        }
    }

    pub(super) fn solve(self) -> Flow {
        let nodes = self.next as usize;
        let mut starts = vec![0u32; nodes + 1];
        for &(_, to) in &self.edges {
            starts[to as usize + 1] += 1;
        }
        for index in 0..nodes {
            starts[index + 1] += starts[index];
        }
        let mut fill = starts.clone();
        let mut sources = vec![0u32; self.edges.len()];
        for &(from, to) in &self.edges {
            let slot = &mut fill[to as usize];
            sources[*slot as usize] = from;
            *slot += 1;
        }
        let mut reached = vec![UNREACHED; nodes];
        let mut spans = Vec::with_capacity(self.sinks.len());
        let mut stack = Vec::new();
        for (node, span) in self.sinks {
            if reached[node as usize] == UNREACHED {
                reached[node as usize] = spans.len() as u32;
                spans.push(span);
                stack.push(node);
            }
        }
        while let Some(node) = stack.pop() {
            let sink = reached[node as usize];
            let low = starts[node as usize] as usize;
            let high = starts[node as usize + 1] as usize;
            for &source in &sources[low..high] {
                if reached[source as usize] == UNREACHED {
                    reached[source as usize] = sink;
                    stack.push(source);
                }
            }
        }
        Flow {
            reads: self.reads,
            reached,
            sinks: spans,
        }
    }
}
