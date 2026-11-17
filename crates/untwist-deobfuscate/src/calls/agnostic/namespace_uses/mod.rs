mod blocks;
mod flows;
mod holders;
mod shapes;

use std::borrow::Cow;

use oxc_allocator::{Address, GetAddress, UnstableAddress};
use oxc_ast::ast::*;
use oxc_ast::AstKind;
use oxc_ast_visit::{walk_js, VisitJs};
use oxc_semantic::Scoping;
use oxc_span::{GetSpan, Span};
use oxc_syntax::operator::{BinaryOperator, LogicalOperator};
use oxc_syntax::symbol::{SymbolFlags, SymbolId};
use rustc_hash::{FxHashMap, FxHashSet};
use untwist_core::ast_util::js_text::js_text;
use untwist_eval::value::JsString;

use super::roles::{reflective_access, READERS, RUNNERS};
use crate::proof::GLOBAL_OBJECTS;

pub(super) const NAMESPACES: [&str; 2] = ["Reflect", "Object"];

pub(super) const NAMESPACE_KEY: &str = "constructor";

const NAMESPACE: u8 = 1;
const GLOBAL: u8 = 2;
const VALUE: u8 = 4;

const KIND_DEPTH: u8 = 16;

const UNSAFE_READS: [&str; 8] = [
    "apply",
    "call",
    "bind",
    "construct",
    "create",
    "__proto__",
    "Function",
    "constructor",
];

const GLOBAL_RECEIVER_READERS: [&str; 5] = [
    "hasOwnProperty",
    "propertyIsEnumerable",
    "isPrototypeOf",
    "toString",
    "toLocaleString",
];

type Resolve<'r> = Option<&'r FxHashMap<Span, &'r JsString>>;

pub(super) fn handed_back_name(name: &str) -> bool {
    reflective_access(name)
        || READERS.contains(&name)
        || RUNNERS.contains(&name)
        || NAMESPACES.contains(&name)
        || name == NAMESPACE_KEY
}

fn namespace_key(name: &str) -> bool {
    NAMESPACES.contains(&name) || name == NAMESPACE_KEY
}

fn unsafe_read(name: &str) -> bool {
    handed_back_name(name) || UNSAFE_READS.contains(&name)
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Phase {
    Collect,
    Check,
}

struct Candidate<'a> {
    init: &'a Expression<'a>,
    wrapper: Option<&'a Function<'a>>,
    this_global: bool,
    function: Option<Address>,
}

struct Walker<'s, 'a> {
    scoping: &'s Scoping,
    resolve: Resolve<'s>,
    script: bool,
    phase: Phase,
    stack: Vec<AstKind<'a>>,
    this_global: Vec<bool>,
    this_open: Vec<bool>,
    functions: Vec<Address>,
    arguments_used: FxHashSet<Address>,
    candidates: FxHashMap<SymbolId, Candidate<'a>>,
    aliases: FxHashMap<SymbolId, u8>,
    receivers: FxHashMap<SymbolId, usize>,
    protections: FxHashSet<SymbolId>,
    blocks: Vec<(Span, &'static [&'static str])>,
    found: Vec<Span>,
}

#[derive(Default)]
pub(in crate::calls::agnostic) struct Loose {
    pub(in crate::calls::agnostic) spans: Vec<Span>,
    pub(in crate::calls::agnostic) blocks: Vec<(Span, &'static [&'static str])>,
}

pub(super) fn loose_uses<'a>(
    program: &'a Program<'a>,
    scoping: &Scoping,
    resolve: Resolve<'_>,
) -> Loose {
    let mut walker = Walker {
        scoping,
        resolve,
        script: !untwist_core::semantic::has_module_syntax(program),
        phase: Phase::Collect,
        stack: Vec::new(),
        this_global: Vec::new(),
        this_open: Vec::new(),
        functions: Vec::new(),
        arguments_used: FxHashSet::default(),
        candidates: FxHashMap::default(),
        aliases: FxHashMap::default(),
        receivers: FxHashMap::default(),
        protections: FxHashSet::default(),
        blocks: Vec::new(),
        found: Vec::new(),
    };
    walker.walk(program);
    walker.aliases = walker.qualified();
    walker.phase = Phase::Check;
    walker.walk(program);
    let mut loose = Loose {
        spans: walker.found,
        blocks: walker.blocks,
    };
    let kept: Vec<Span> = loose
        .spans
        .iter()
        .copied()
        .filter(|span| !loose.exempt(*span))
        .collect();
    loose.spans = kept;
    loose
}

fn str_const(expr: &Expression<'_>, resolve: Resolve<'_>, depth: u8) -> Option<String> {
    let next = depth.checked_sub(1)?;
    match expr.without_parentheses() {
        Expression::StringLiteral(text) => Some(js_text(text.value).into_owned()),
        Expression::NumericLiteral(number) => Some(number.value.to_string()),
        Expression::TemplateLiteral(t) if t.expressions.is_empty() => t
            .quasis
            .first()?
            .value
            .cooked
            .map(|cooked| js_text(cooked).into_owned()),
        Expression::BinaryExpression(b) if b.operator == BinaryOperator::Addition => Some(format!(
            "{}{}",
            str_const(&b.left, resolve, next)?,
            str_const(&b.right, resolve, next)?
        )),
        Expression::CallExpression(call) => resolve?.get(&call.span)?.as_string(),
        _ => None,
    }
}

fn member_key<'a>(member: &MemberExpression<'a>, resolve: Resolve<'_>) -> Option<Cow<'a, str>> {
    match member {
        MemberExpression::StaticMemberExpression(m) => {
            Some(Cow::Borrowed(m.property.name.as_str()))
        }
        MemberExpression::ComputedMemberExpression(m) => {
            str_const(&m.expression, resolve, KIND_DEPTH).map(Cow::Owned)
        }
        MemberExpression::PrivateFieldExpression(_) => Some(Cow::Borrowed("#")),
    }
}

fn property_name<'a>(key: &PropertyKey<'a>, resolve: Resolve<'_>) -> Option<Cow<'a, str>> {
    match key {
        PropertyKey::StaticIdentifier(name) => Some(Cow::Borrowed(name.name.as_str())),
        PropertyKey::PrivateIdentifier(_) => Some(Cow::Borrowed("#")),
        other => other
            .as_expression()
            .and_then(|expr| str_const(expr, resolve, KIND_DEPTH))
            .map(Cow::Owned),
    }
}

fn key_kind(key: Option<Cow<'_, str>>, base: u8) -> u8 {
    match key {
        Some(key) if namespace_key(&key) => NAMESPACE,
        Some(key) if base & GLOBAL != 0 && GLOBAL_OBJECTS.contains(&key.as_ref()) => GLOBAL,
        None if base & GLOBAL != 0 => NAMESPACE | GLOBAL,
        _ => 0,
    }
}

fn same(expr: &Expression<'_>, node: Address) -> bool {
    expr.address() == node
}

fn transparent(kind: &AstKind<'_>) -> bool {
    matches!(
        kind,
        AstKind::ParenthesizedExpression(_)
            | AstKind::ChainExpression(_)
            | AstKind::TSAsExpression(_)
            | AstKind::TSSatisfiesExpression(_)
            | AstKind::TSNonNullExpression(_)
            | AstKind::TSTypeAssertion(_)
            | AstKind::TSInstantiationExpression(_)
    )
}

fn iife_function<'k, 'a>(
    callee: &'k Expression<'a>,
) -> Option<(&'k FormalParameters<'a>, Option<Address>)> {
    match callee.without_parentheses() {
        Expression::FunctionExpression(func) => Some((&func.params, Some(func.unstable_address()))),
        Expression::ArrowFunctionExpression(arrow) => Some((&arrow.params, None)),
        _ => None,
    }
}

fn function_literal(expr: &Expression<'_>) -> bool {
    matches!(
        expr.without_parentheses(),
        Expression::FunctionExpression(_) | Expression::ArrowFunctionExpression(_)
    )
}

impl<'s, 'a> Walker<'s, 'a> {
    fn walk(&mut self, program: &'a Program<'a>) {
        self.stack.clear();
        self.this_global.clear();
        self.this_open.clear();
        self.functions.clear();
        self.this_global.push(self.script);
        self.this_open.push(self.script);
        self.visit_program(program);
    }

    fn symbol(&self, id: &IdentifierReference<'_>) -> Option<SymbolId> {
        id.reference_id
            .get()
            .and_then(|reference| self.scoping.get_reference(reference).symbol_id())
    }

    fn this_is_global(&self) -> bool {
        self.this_global.last().copied().unwrap_or(false)
    }

    fn symbol_kind(&self, symbol: SymbolId, depth: u8, seen: &mut Vec<SymbolId>) -> u8 {
        if self.phase == Phase::Check {
            return self.aliases.get(&symbol).copied().unwrap_or(0);
        }
        let Some(candidate) = self.candidates.get(&symbol) else {
            return 0;
        };
        if seen.contains(&symbol) {
            return 0;
        }
        seen.push(symbol);
        let kind = self.value_kind(candidate.init, candidate.this_global, depth, seen);
        seen.pop();
        kind
    }

    fn value_kind(
        &self,
        expr: &Expression<'a>,
        this_global: bool,
        depth: u8,
        seen: &mut Vec<SymbolId>,
    ) -> u8 {
        let Some(next) = depth.checked_sub(1) else {
            return 0;
        };
        match expr.without_parentheses() {
            Expression::Identifier(id) => match self.symbol(id) {
                None if NAMESPACES.contains(&id.name.as_str()) => NAMESPACE,
                None if GLOBAL_OBJECTS.contains(&id.name.as_str()) => GLOBAL,
                None => 0,
                Some(symbol) => self.symbol_kind(symbol, next, seen),
            },
            Expression::ThisExpression(_) if this_global => GLOBAL,
            Expression::ConditionalExpression(c) => {
                self.value_kind(&c.consequent, this_global, next, seen)
                    | self.value_kind(&c.alternate, this_global, next, seen)
            }
            Expression::LogicalExpression(l) if l.operator == LogicalOperator::And => {
                self.value_kind(&l.right, this_global, next, seen)
            }
            Expression::LogicalExpression(l) => {
                self.value_kind(&l.left, this_global, next, seen)
                    | self.value_kind(&l.right, this_global, next, seen)
            }
            Expression::SequenceExpression(s) => s
                .expressions
                .last()
                .map_or(0, |last| self.value_kind(last, this_global, next, seen)),
            Expression::AwaitExpression(a) => self.value_kind(&a.argument, this_global, next, seen),
            other => match other.as_member_expression() {
                Some(member) => {
                    let base = self.value_kind(member.object(), this_global, next, seen);
                    self.member_kind(member, base)
                }
                None => 0,
            },
        }
    }

    fn member_kind(&self, member: &MemberExpression<'a>, base: u8) -> u8 {
        match member {
            MemberExpression::ComputedMemberExpression(m) => {
                self.computed_kind(&m.object, &m.expression, base)
            }
            other => key_kind(member_key(other, self.resolve), base),
        }
    }

    fn kind_here(&self, expr: &Expression<'a>) -> u8 {
        self.value_kind(expr, self.this_is_global(), KIND_DEPTH, &mut Vec::new())
    }

    fn qualified(&self) -> FxHashMap<SymbolId, u8> {
        let root = self.scoping.root_scope_id();
        self.candidates
            .iter()
            .filter(|(symbol, candidate)| {
                !self.scoping.symbol_is_mutated(**symbol)
                    && self.scoping.symbol_redeclarations(**symbol).is_empty()
                    && candidate
                        .function
                        .is_none_or(|function| !self.arguments_used.contains(&function))
            })
            .filter_map(|(symbol, _)| {
                let kind = self.symbol_kind(*symbol, KIND_DEPTH, &mut Vec::new());
                let shared = self.script && self.scoping.symbol_scope_id(*symbol) == root;
                (kind != 0 && !(shared && kind & NAMESPACE != 0)).then_some((*symbol, kind))
            })
            .collect()
    }

    fn collect_declarator(&mut self, declarator: &'a VariableDeclarator<'a>) {
        let (BindingPattern::BindingIdentifier(id), Some(init)) =
            (&declarator.id, &declarator.init)
        else {
            return;
        };
        let Some(symbol) = id.symbol_id.get() else {
            return;
        };
        let candidate = Candidate {
            init,
            wrapper: self.unmutated_wrapper(symbol, init),
            this_global: self.this_is_global(),
            function: None,
        };
        self.candidates.insert(symbol, candidate);
    }

    fn unmutated_wrapper(
        &self,
        symbol: SymbolId,
        init: &'a Expression<'a>,
    ) -> Option<&'a Function<'a>> {
        shapes::returned_function(init).filter(|_| !self.scoping.symbol_is_mutated(symbol))
    }

    fn collect_params(&mut self, call: &'a CallExpression<'a>) {
        let Some((params, function)) = iife_function(&call.callee) else {
            return;
        };
        if params.rest.is_some() || call.arguments.iter().any(Argument::is_spread) {
            return;
        }
        let this_global = self.this_is_global();
        for (param, argument) in params.items.iter().zip(&call.arguments) {
            let (BindingPattern::BindingIdentifier(id), None) =
                (&param.pattern, &param.initializer)
            else {
                continue;
            };
            let (Some(symbol), Some(init)) = (id.symbol_id.get(), argument.as_expression()) else {
                continue;
            };
            let candidate = Candidate {
                init,
                wrapper: self.unmutated_wrapper(symbol, init),
                this_global,
                function,
            };
            self.candidates.insert(symbol, candidate);
        }
    }

    fn global_receiver_function(&self) -> bool {
        let Some(top) = self.stack.last() else {
            return false;
        };
        let mut child = top.address();
        let mut at = self.stack.len().saturating_sub(1);
        while at > 0 {
            at -= 1;
            match &self.stack[at] {
                kind if transparent(kind) => child = kind.address(),
                AstKind::StaticMemberExpression(member)
                    if member.object.address() == child
                        && matches!(member.property.name.as_str(), "call" | "apply") =>
                {
                    let member_at = self.stack[at].address();
                    let Some(AstKind::CallExpression(call)) =
                        at.checked_sub(1).map(|up| &self.stack[up])
                    else {
                        return false;
                    };
                    return call.callee.without_parentheses().address() == member_at
                        && call
                            .arguments
                            .first()
                            .and_then(Argument::as_expression)
                            .is_some_and(|receiver| self.kind_here(receiver) & GLOBAL != 0);
                }
                AstKind::CallExpression(call) if self.phase == Phase::Check => {
                    return self.wrapped_by_global(call, child);
                }
                _ => return false,
            }
        }
        false
    }

    fn subject_kind(&self, kind: &AstKind<'a>) -> u8 {
        match kind {
            AstKind::IdentifierReference(id) => {
                let reference = id
                    .reference_id
                    .get()
                    .map(|reference| self.scoping.get_reference(reference));
                if reference.is_some_and(|reference| !reference.is_read()) {
                    return 0;
                }
                match reference.and_then(|reference| reference.symbol_id()) {
                    None if NAMESPACES.contains(&id.name.as_str()) => NAMESPACE,
                    None if GLOBAL_OBJECTS.contains(&id.name.as_str()) => GLOBAL,
                    None => 0,
                    Some(symbol) => self.aliases.get(&symbol).copied().unwrap_or(0),
                }
            }
            AstKind::ThisExpression(_) if self.this_is_global() => GLOBAL,
            AstKind::StaticMemberExpression(m) => key_kind(
                Some(Cow::Borrowed(m.property.name.as_str())),
                self.kind_here(&m.object),
            ),
            AstKind::ComputedMemberExpression(m) => {
                self.computed_kind(&m.object, &m.expression, self.kind_here(&m.object))
            }
            _ => 0,
        }
    }

    fn check(&mut self, kind: AstKind<'a>) {
        let subject = self.subject_kind(&kind);
        if subject != 0 && !self.allowed(self.stack.len() - 1, subject) {
            self.found.push(kind.span());
        }
        match kind {
            AstKind::ObjectProperty(property) => self.check_proto_property(property),
            AstKind::CallExpression(call) => {
                self.check_blocks(call);
                self.check_proto_call(call);
            }
            AstKind::AssignmentExpression(assign) => self.check_proto_write(assign),
            AstKind::BindingProperty(property) => {
                self.check_pattern_key(&property.key, property.span)
            }
            AstKind::AssignmentTargetPropertyProperty(property) => {
                self.check_pattern_key(&property.name, property.span);
            }
            AstKind::AssignmentTargetPropertyIdentifier(property)
                if namespace_key(property.binding.name.as_str()) =>
            {
                self.found.push(property.span);
            }
            _ => {}
        }
    }

    fn check_pattern_key(&mut self, key: &PropertyKey<'a>, span: Span) {
        let taken = property_name(key, self.resolve);
        if taken.as_deref().is_some_and(namespace_key) {
            self.found.push(span);
        }
        if taken.is_none() && key.is_expression() && !self.top_pattern() {
            self.found.push(span);
        }
    }

    fn top_pattern(&self) -> bool {
        let up = |back: usize| self.stack.len().checked_sub(back).map(|at| &self.stack[at]);
        match (up(2), up(3)) {
            (Some(AstKind::ObjectPattern(_)), Some(AstKind::VariableDeclarator(d))) => {
                d.init.is_some()
            }
            (Some(AstKind::ObjectAssignmentTarget(_)), Some(AstKind::AssignmentExpression(_))) => {
                true
            }
            _ => false,
        }
    }

    fn check_proto_property(&mut self, property: &ObjectProperty<'a>) {
        if property.computed || property.shorthand || property.method {
            return;
        }
        let names_proto =
            property_name(&property.key, self.resolve).as_deref() == Some("__proto__");
        if names_proto && !self.proto_literal(&property.value, KIND_DEPTH) {
            self.found.push(property.value.span());
        }
    }

    fn check_proto_write(&mut self, assign: &AssignmentExpression<'a>) {
        let Some(member) = assign.left.as_member_expression() else {
            return;
        };
        let names_proto = member_key(member, self.resolve).as_deref() == Some("__proto__");
        if names_proto && !self.proto_literal(&assign.right, KIND_DEPTH) {
            self.found.push(assign.right.span());
        }
    }

    fn check_proto_call(&mut self, call: &CallExpression<'a>) {
        let Some(member) = call.callee.without_parentheses().as_member_expression() else {
            return;
        };
        let at = match member_key(member, self.resolve).as_deref() {
            Some("setPrototypeOf") => 1,
            Some("create") => 0,
            _ => return,
        };
        if self.kind_here(member.object()) & NAMESPACE == 0 {
            return;
        }
        if call.arguments.iter().take(at + 1).any(Argument::is_spread) {
            self.found.push(call.span);
            return;
        }
        if let Some(proto) = call.arguments.get(at).and_then(Argument::as_expression) {
            if !self.proto_literal(proto, KIND_DEPTH) {
                self.found.push(proto.span());
            }
        }
    }

    fn proto_literal(&self, expr: &Expression<'a>, depth: u8) -> bool {
        let Some(next) = depth.checked_sub(1) else {
            return false;
        };
        match expr.without_parentheses() {
            Expression::NullLiteral(_)
            | Expression::ObjectExpression(_)
            | Expression::ArrayExpression(_)
            | Expression::FunctionExpression(_)
            | Expression::ArrowFunctionExpression(_)
            | Expression::ClassExpression(_) => true,
            Expression::Identifier(id) => match self.symbol(id) {
                None => id.name == "undefined",
                Some(symbol) => {
                    self.scoping
                        .symbol_flags(symbol)
                        .intersects(SymbolFlags::Function | SymbolFlags::Class)
                        && !self.scoping.symbol_is_mutated(symbol)
                }
            },
            Expression::LogicalExpression(l) if l.operator == LogicalOperator::And => {
                self.proto_literal(&l.right, next)
            }
            Expression::LogicalExpression(l) => {
                self.proto_literal(&l.left, next) && self.proto_literal(&l.right, next)
            }
            Expression::ConditionalExpression(c) => {
                self.proto_literal(&c.consequent, next) && self.proto_literal(&c.alternate, next)
            }
            Expression::SequenceExpression(s) => s
                .expressions
                .last()
                .is_some_and(|last| self.proto_literal(last, next)),
            Expression::CallExpression(call) => call
                .callee
                .without_parentheses()
                .as_member_expression()
                .is_some_and(|member| member_key(member, self.resolve).as_deref() == Some("bind")),
            other => other.as_member_expression().is_some_and(|member| {
                member_key(member, self.resolve).as_deref() == Some("prototype")
                    && self.kind_here(member.object()) == 0
            }),
        }
    }

    fn allowed(&self, mut at: usize, kind: u8) -> bool {
        if kind == GLOBAL {
            return self.global_flow_allowed(at);
        }
        let mut child = self.stack[at].address();
        while at > 0 {
            at -= 1;
            let parent = &self.stack[at];
            match parent {
                kind if transparent(kind) => child = kind.address(),
                AstKind::AwaitExpression(_) => child = parent.address(),
                AstKind::ConditionalExpression(c) => {
                    if same(&c.test, child) {
                        return true;
                    }
                    child = parent.address();
                }
                AstKind::LogicalExpression(l) => {
                    if l.operator == LogicalOperator::And && same(&l.left, child) {
                        return true;
                    }
                    child = parent.address();
                }
                AstKind::SequenceExpression(s) => {
                    if !s.expressions.last().is_some_and(|last| same(last, child)) {
                        return true;
                    }
                    child = parent.address();
                }
                AstKind::ExpressionStatement(_) => return true,
                AstKind::UnaryExpression(_)
                | AstKind::UpdateExpression(_)
                | AstKind::BinaryExpression(_)
                | AstKind::PrivateInExpression(_)
                | AstKind::IfStatement(_)
                | AstKind::WhileStatement(_)
                | AstKind::DoWhileStatement(_)
                | AstKind::ForStatement(_)
                | AstKind::SwitchStatement(_)
                | AstKind::SwitchCase(_) => return true,
                AstKind::ForInStatement(f) => return same(&f.right, child),
                AstKind::ForOfStatement(f) => return !same(&f.right, child),
                AstKind::AssignmentExpression(a) => {
                    return a
                        .left
                        .as_member_expression()
                        .is_some_and(|m| member_address(m) == child);
                }
                AstKind::AssignmentTargetWithDefault(t) => {
                    return t
                        .binding
                        .as_member_expression()
                        .is_some_and(|m| member_address(m) == child);
                }
                AstKind::ArrayAssignmentTarget(_)
                | AstKind::AssignmentTargetPropertyProperty(_)
                | AstKind::AssignmentTargetRest(_) => return true,
                AstKind::TemplateLiteral(_) => {
                    return !matches!(
                        at.checked_sub(1).map(|up| &self.stack[up]),
                        Some(AstKind::TaggedTemplateExpression(_))
                    );
                }
                AstKind::ObjectProperty(p) => return p.computed && !same(&p.value, child),
                AstKind::ComputedMemberExpression(m) if same(&m.expression, child) => return true,
                AstKind::StaticMemberExpression(_)
                | AstKind::ComputedMemberExpression(_)
                | AstKind::PrivateFieldExpression(_) => return self.member_use(at, kind),
                AstKind::CallExpression(c) => {
                    if same(&c.callee, child) {
                        return kind & VALUE == 0;
                    }
                    return kind & VALUE == 0 && self.patched_namespace(at, c, child);
                }
                AstKind::NewExpression(n) => return same(&n.callee, child) && kind & VALUE == 0,
                AstKind::VariableDeclarator(d) => {
                    return kind & VALUE == 0
                        && matches!(&d.id, BindingPattern::BindingIdentifier(id)
                            if id.symbol_id.get().is_some_and(|symbol| self.aliases.contains_key(&symbol)));
                }
                _ => return false,
            }
        }
        false
    }
}

fn member_address(member: &MemberExpression<'_>) -> Address {
    match member {
        MemberExpression::StaticMemberExpression(m) => m.address(),
        MemberExpression::ComputedMemberExpression(m) => m.address(),
        MemberExpression::PrivateFieldExpression(m) => m.address(),
    }
}

impl<'s, 'a> VisitJs<'a> for Walker<'s, 'a> {
    fn enter_node(&mut self, kind: AstKind<'a>) {
        self.stack.push(kind);
        match kind {
            AstKind::Function(function) => {
                let global = self.global_receiver_function();
                self.this_global.push(global);
                let strict = function
                    .scope_id
                    .get()
                    .is_some_and(|scope| self.scoping.scope_flags(scope).is_strict_mode());
                self.this_open.push(!strict);
                self.functions.push(function.unstable_address());
            }
            AstKind::StaticBlock(_)
            | AstKind::PropertyDefinition(_)
            | AstKind::AccessorProperty(_) => {
                self.this_global.push(false);
                self.this_open.push(false);
            }
            _ => {}
        }
        match self.phase {
            Phase::Collect => self.collect(kind),
            Phase::Check => self.check(kind),
        }
    }

    fn leave_node(&mut self, kind: AstKind<'a>) {
        match kind {
            AstKind::Function(_) => {
                self.this_global.pop();
                self.this_open.pop();
                self.functions.pop();
            }
            AstKind::StaticBlock(_)
            | AstKind::PropertyDefinition(_)
            | AstKind::AccessorProperty(_) => {
                self.this_global.pop();
                self.this_open.pop();
            }
            _ => {}
        }
        self.stack.pop();
    }

    fn visit_function(&mut self, it: &Function<'a>, flags: oxc_syntax::scope::ScopeFlags) {
        walk_js::walk_function(self, it, flags);
    }
}

impl<'s, 'a> Walker<'s, 'a> {
    fn collect(&mut self, kind: AstKind<'a>) {
        match kind {
            AstKind::VariableDeclarator(declarator) => {
                self.collect_declarator(declarator);
                self.collect_protection(kind);
            }
            AstKind::Function(_) => self.collect_protection(kind),
            AstKind::CallExpression(call) => self.collect_params(call),
            AstKind::IdentifierReference(id) if id.name == "arguments" => {
                if let Some(function) = self.functions.last() {
                    self.arguments_used.insert(*function);
                }
            }
            AstKind::IdentifierReference(id) => self.collect_reference(id),
            _ => {}
        }
    }
}
