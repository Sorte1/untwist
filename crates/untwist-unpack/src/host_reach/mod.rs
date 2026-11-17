use oxc_allocator::{Address, GetAddress};
use oxc_ast::ast::*;
use oxc_ast::AstKind;
use oxc_ast_visit::{walk_js, VisitJs};
use oxc_semantic::Scoping;
use oxc_syntax::operator::{LogicalOperator, UnaryOperator};
use oxc_syntax::symbol::SymbolId;
use rustc_hash::{FxHashMap, FxHashSet};

use crate::host_keys::{const_key, literal_data, runner_key, Keys, Write, Writes, NAME_LISTERS};
use crate::host_text::CONSTRUCTOR;

mod carry;
mod collect;
mod copies;
#[cfg(test)]
mod tests;

const REFLECT: &str = "Reflect";
const DESCRIPTOR: &str = "getOwnPropertyDescriptor";
const DESCRIPTORS: &str = "getOwnPropertyDescriptors";
const GET: &str = "get";
const DEFINE: &str = "defineProperty";
const COPIERS: [&str; 2] = ["defineProperties", "create"];
const OBJECT: &str = "Object";
const DEFINE_OWNERS: [&str; 2] = [OBJECT, REFLECT];
const FORWARDERS: [&str; 3] = ["call", "apply", "bind"];
const DESCRIPTOR_FIELDS: [&str; 3] = ["value", "get", "set"];
const PROTO: &str = "__proto__";
const VALUE_SINKS: [&str; 7] = [
    "String",
    "Number",
    "Boolean",
    "isNaN",
    "isFinite",
    "parseInt",
    "parseFloat",
];
const CONSOLE: &str = "console";
const PROXY: &str = "Proxy";
const UNDEFINED: &str = "undefined";
const PASSES: usize = 6;

const WITH: &str = "a with statement";
const RUNNER_KEY: &str = "a code runner's name as a key";
const LOOSE_KEY: &str = "a computed key that may name a code runner";
const LOOSE_VALUE: &str = "a value read under a key that may name a code runner";
const PATTERN_KEY: &str = "a pattern key that may name a code runner";
const READER_VALUE: &str = "a property reader taken as a value";
const REFLECT_VALUE: &str = "Reflect taken as a value";
const FREE_CONSTRUCTOR: &str = "a free constructor";
const DESCRIPTORS_TAKEN: &str = "property descriptors taken as values";
const TOO_DEEP: &str = "a value held through too many names";

#[derive(Clone, Copy, PartialEq, Eq)]
enum Phase {
    Collect,
    Check,
}

#[derive(Clone, Copy)]
enum Held<'a> {
    Read(Option<&'a Expression<'a>>),
    Descriptor(Option<&'a Expression<'a>>),
}

impl<'a> Held<'a> {
    fn key(self) -> Option<&'a Expression<'a>> {
        match self {
            Held::Read(key) | Held::Descriptor(key) => key,
        }
    }

    fn descriptor(self) -> bool {
        matches!(self, Held::Descriptor(_))
    }

    fn same(self, other: Held<'a>) -> bool {
        let key = match (self.key(), other.key()) {
            (Some(a), Some(b)) => a.address() == b.address(),
            (None, None) => true,
            _ => false,
        };
        key && self.descriptor() == other.descriptor()
    }

    fn joined(self, other: Held<'a>) -> Held<'a> {
        match self.descriptor() || other.descriptor() {
            true => Held::Descriptor(None),
            false => Held::Read(None),
        }
    }
}

struct Reach<'s, 'a> {
    scoping: &'s Scoping,
    phase: Phase,
    stack: Vec<AstKind<'a>>,
    writes: Writes<'a>,
    tainted: FxHashMap<SymbolId, Held<'a>>,
    grew: bool,
    found: Option<&'static str>,
}

fn same(expr: &Expression<'_>, node: Address) -> bool {
    expr.address() == node
}

fn member_address(member: &MemberExpression<'_>) -> Address {
    match member {
        MemberExpression::StaticMemberExpression(m) => m.address(),
        MemberExpression::ComputedMemberExpression(m) => m.address(),
        MemberExpression::PrivateFieldExpression(m) => m.address(),
    }
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

fn member_key(member: &MemberExpression<'_>) -> Option<String> {
    match member {
        MemberExpression::StaticMemberExpression(m) => Some(m.property.name.to_string()),
        MemberExpression::ComputedMemberExpression(m) => const_key(&m.expression),
        MemberExpression::PrivateFieldExpression(_) => None,
    }
}

fn quiet_arguments(arguments: &[Argument<'_>]) -> bool {
    arguments.iter().all(|argument| match argument {
        Argument::NumericLiteral(_)
        | Argument::BooleanLiteral(_)
        | Argument::NullLiteral(_)
        | Argument::FunctionExpression(_)
        | Argument::ArrowFunctionExpression(_)
        | Argument::ClassExpression(_) => true,
        Argument::Identifier(id) => id.name == UNDEFINED,
        _ => false,
    })
}

fn member_ok(held: Held<'_>, key: &str) -> bool {
    let field = matches!(held, Held::Descriptor(_)) && DESCRIPTOR_FIELDS.contains(&key);
    !field && !FORWARDERS.contains(&key) && !runner_key(key)
}

impl<'s, 'a> Reach<'s, 'a> {
    fn new(scoping: &'s Scoping) -> Self {
        Self {
            scoping,
            phase: Phase::Collect,
            stack: Vec::new(),
            writes: Writes::default(),
            tainted: FxHashMap::default(),
            grew: false,
            found: None,
        }
    }

    fn keys(&self) -> Keys<'_, 'a> {
        Keys {
            scoping: self.scoping,
            writes: &self.writes,
        }
    }

    fn symbol(&self, id: &IdentifierReference<'_>) -> Option<SymbolId> {
        let reference = id.reference_id.get()?;
        self.scoping.get_reference(reference).symbol_id()
    }

    fn is_read(&self, id: &IdentifierReference<'_>) -> bool {
        id.reference_id
            .get()
            .is_none_or(|reference| self.scoping.get_reference(reference).is_read())
    }

    fn refuse(&mut self, what: &'static str) {
        self.found.get_or_insert(what);
    }

    fn parent(&self, at: usize) -> Option<(usize, Address)> {
        let mut child = self.stack.get(at)?.address();
        let mut up = at;
        while up > 0 {
            up -= 1;
            if !transparent(&self.stack[up]) {
                return Some((up, child));
            }
            child = self.stack[up].address();
        }
        None
    }

    fn under_typeof(&self, at: usize) -> bool {
        self.parent(at).is_some_and(|(up, _)| {
            matches!(self.stack[up], AstKind::UnaryExpression(unary)
                if unary.operator == UnaryOperator::Typeof)
        })
    }

    fn written(&self, at: usize) -> bool {
        let Some((up, child)) = self.parent(at) else {
            return false;
        };
        match self.stack[up] {
            AstKind::AssignmentExpression(assign) => assign
                .left
                .as_member_expression()
                .is_some_and(|member| member_address(member) == child),
            AstKind::ForInStatement(each) => !same(&each.right, child),
            AstKind::ForOfStatement(each) => !same(&each.right, child),
            AstKind::UpdateExpression(_)
            | AstKind::ArrayAssignmentTarget(_)
            | AstKind::AssignmentTargetWithDefault(_)
            | AstKind::AssignmentTargetPropertyProperty(_)
            | AstKind::AssignmentTargetRest(_) => true,
            _ => false,
        }
    }

    fn called(&self, at: usize) -> bool {
        self.parent(at).is_some_and(|(up, child)| {
            matches!(self.stack[up], AstKind::CallExpression(call) if same(&call.callee, child))
        })
    }

    fn tagged(&self, up: usize) -> bool {
        up >= 1 && matches!(self.stack[up - 1], AstKind::TaggedTemplateExpression(_))
    }
}

impl<'a> Reach<'_, 'a> {
    fn harmless(&mut self, at: usize, held: Held<'a>, can_hold: bool) -> bool {
        let mut child = self.stack[at].address();
        let mut up = at;
        while up > 0 {
            up -= 1;
            let parent = self.stack[up];
            let flows = match parent {
                kind if transparent(&kind) => true,
                AstKind::AwaitExpression(_) => true,
                AstKind::ConditionalExpression(choice) => !same(&choice.test, child),
                AstKind::LogicalExpression(logical) => {
                    logical.operator != LogicalOperator::And || !same(&logical.left, child)
                }
                AstKind::SequenceExpression(sequence) => sequence
                    .expressions
                    .last()
                    .is_some_and(|last| same(last, child)),
                _ => return self.settles(up, child, held, can_hold),
            };
            if !flows {
                return true;
            }
            child = parent.address();
        }
        false
    }

    fn settles(&mut self, up: usize, child: Address, held: Held<'a>, can_hold: bool) -> bool {
        match self.stack[up] {
            AstKind::ExpressionStatement(_) => true,
            AstKind::UnaryExpression(_)
            | AstKind::UpdateExpression(_)
            | AstKind::BinaryExpression(_)
            | AstKind::PrivateInExpression(_)
            | AstKind::IfStatement(_)
            | AstKind::WhileStatement(_)
            | AstKind::DoWhileStatement(_)
            | AstKind::ForStatement(_)
            | AstKind::SwitchStatement(_)
            | AstKind::SwitchCase(_)
            | AstKind::PrivateFieldExpression(_) => true,
            AstKind::ForInStatement(each) => same(&each.right, child),
            AstKind::ForOfStatement(each) => same(&each.right, child),
            AstKind::TemplateLiteral(_) => !self.tagged(up),
            AstKind::ObjectProperty(property) => {
                property.computed
                    && property
                        .key
                        .as_expression()
                        .is_some_and(|key| same(key, child))
            }
            AstKind::StaticMemberExpression(member) => {
                self.member_use(up, held, &member.property.name, can_hold)
            }
            AstKind::ComputedMemberExpression(member) => {
                same(&member.expression, child)
                    || const_key(&member.expression)
                        .is_none_or(|key| self.member_use(up, held, &key, can_hold))
            }
            AstKind::CallExpression(call) if same(&call.callee, child) => {
                quiet_arguments(&call.arguments)
            }
            AstKind::CallExpression(call) => {
                self.sink(call, child, held) || (can_hold && self.passed_on(call, child, held))
            }
            AstKind::NewExpression(call) => {
                same(&call.callee, child) && quiet_arguments(&call.arguments)
            }
            AstKind::AssignmentExpression(assign) => {
                same(&assign.right, child) && self.assigned(assign, held, can_hold)
            }
            AstKind::VariableDeclarator(declarator) => can_hold && self.hold(&declarator.id, held),
            AstKind::ArrowFunctionExpression(_) => self.getter(up, held),
            _ => false,
        }
    }
}

impl<'a> Reach<'_, 'a> {
    fn check(&mut self, kind: AstKind<'a>) {
        let at = self.stack.len() - 1;
        match kind {
            AstKind::WithStatement(_) => self.refuse(WITH),
            AstKind::ComputedMemberExpression(member) => self.check_computed(at, member),
            AstKind::StaticMemberExpression(member) => self.check_static(at, member),
            AstKind::IdentifierReference(id) => self.check_identifier(at, id),
            AstKind::CallExpression(call) => self.check_call(at, call),
            AstKind::BindingProperty(property) => {
                self.check_pattern_key(&property.key, property.computed);
            }
            AstKind::AssignmentTargetPropertyProperty(property) => {
                self.check_pattern_key(&property.name, property.computed);
            }
            AstKind::AssignmentTargetPropertyIdentifier(property)
                if runner_key(&property.binding.name) =>
            {
                self.refuse(PATTERN_KEY);
            }
            _ => {}
        }
    }

    fn check_computed(&mut self, at: usize, member: &'a ComputedMemberExpression<'a>) {
        if self.written(at) {
            return;
        }
        if let Some(key) = const_key(&member.expression) {
            if runner_key(&key) && !self.under_typeof(at) {
                self.refuse(RUNNER_KEY);
            }
            return;
        }
        let keys = self.keys();
        if keys.safe(&member.expression) || keys.data(&member.object) {
            return;
        }
        if !self.harmless(at, Held::Read(Some(&member.expression)), true) {
            self.refuse(LOOSE_KEY);
        }
    }

    fn check_static(&mut self, at: usize, member: &'a StaticMemberExpression<'a>) {
        let name = member.property.name.as_str();
        if self.written(at) {
            return;
        }
        if runner_key(name) && !self.under_typeof(at) {
            self.refuse(RUNNER_KEY);
        }
        if name == REFLECT {
            self.check_reflect(at);
        }
        let reader = name == DESCRIPTOR
            || name == DESCRIPTORS
            || (name == GET && self.reflective(&member.object));
        if reader && !self.called(at) && !(name == DESCRIPTOR && self.aliased_reader(at)) {
            self.refuse(READER_VALUE);
        }
    }

    fn check_identifier(&mut self, at: usize, id: &'a IdentifierReference<'a>) {
        if !self.is_read(id) {
            return;
        }
        match self.symbol(id) {
            None if id.name == CONSTRUCTOR => self.refuse(FREE_CONSTRUCTOR),
            None if id.name == REFLECT => self.check_reflect(at),
            Some(symbol) if self.writes.readers.contains(&symbol) && !self.called(at) => {
                self.refuse(READER_VALUE);
            }
            Some(symbol) => {
                if let Some(held) = self.tainted.get(&symbol).copied() {
                    if !self.harmless(at, held, true) {
                        self.refuse(LOOSE_VALUE);
                    }
                }
            }
            _ => {}
        }
    }

    fn check_reflect(&mut self, at: usize) {
        let member_object = self
            .parent(at)
            .is_some_and(|(up, child)| match self.stack[up] {
                AstKind::StaticMemberExpression(member) => same(&member.object, child),
                AstKind::ComputedMemberExpression(member) => same(&member.object, child),
                _ => false,
            });
        if !member_object && !self.harmless(at, Held::Read(None), false) {
            self.refuse(REFLECT_VALUE);
        }
    }

    fn alias(&self, expr: &Expression<'_>, of: &FxHashSet<SymbolId>) -> bool {
        match expr.without_parentheses() {
            Expression::Identifier(id) => self.symbol(id).is_some_and(|symbol| {
                of.contains(&symbol) && self.writes.single(self.scoping, symbol)
            }),
            _ => false,
        }
    }

    fn builtin_member(&self, expr: &Expression<'_>, owners: &[&str], names: &[&str]) -> bool {
        expr.without_parentheses()
            .as_member_expression()
            .is_some_and(|member| {
                member_key(member).is_some_and(|name| names.contains(&name.as_str()))
                    && matches!(member.object().without_parentheses(), Expression::Identifier(id)
                        if owners.contains(&id.name.as_str()) && self.symbol(id).is_none())
            })
    }

    fn defines(&self, call: &CallExpression<'_>) -> bool {
        let named = self.builtin_member(&call.callee, &DEFINE_OWNERS, &[DEFINE]);
        let spread = call.arguments.iter().any(Argument::is_spread);
        !spread && (named || self.alias(&call.callee, &self.writes.definers))
    }

    fn aliased_reader(&self, at: usize) -> bool {
        self.parent(at)
            .is_some_and(|(up, child)| match self.stack[up] {
                AstKind::VariableDeclarator(declarator) => {
                    declarator
                        .init
                        .as_ref()
                        .is_some_and(|init| same(init, child))
                        && matches!(&declarator.id, BindingPattern::BindingIdentifier(id)
                        if id.symbol_id.get().is_some_and(|symbol| {
                            self.writes.readers.contains(&symbol)
                                && self.writes.single(self.scoping, symbol)
                        }))
                }
                _ => false,
            })
    }

    fn reflective(&self, expr: &Expression<'_>) -> bool {
        match expr.without_parentheses() {
            Expression::Identifier(id) => id.name == REFLECT && self.symbol(id).is_none(),
            other => other
                .as_member_expression()
                .is_some_and(|member| member_key(member).as_deref() == Some(REFLECT)),
        }
    }

    fn check_call(&mut self, at: usize, call: &'a CallExpression<'a>) {
        if self.alias(&call.callee, &self.writes.readers) {
            self.check_read_call(at, call, true);
            return;
        }
        let Some(member) = call.callee.without_parentheses().as_member_expression() else {
            return;
        };
        let Some(name) = member_key(member) else {
            return;
        };
        let reads = name == DESCRIPTOR
            || (name == GET && (call.arguments.len() >= 2 || self.reflective(member.object())));
        if reads {
            self.check_read_call(at, call, name == DESCRIPTOR);
        } else if name == DESCRIPTORS && !self.copied(at, call) {
            self.refuse(DESCRIPTORS_TAKEN);
        }
    }

    fn check_read_call(&mut self, at: usize, call: &'a CallExpression<'a>, descriptor: bool) {
        if call.arguments.iter().any(Argument::is_spread) {
            self.refuse(LOOSE_KEY);
            return;
        }
        let Some(key) = call.arguments.get(1) else {
            return;
        };
        let Some(key) = key.as_expression() else {
            self.refuse(LOOSE_KEY);
            return;
        };
        if let Some(text) = const_key(key) {
            if !runner_key(&text) {
                return;
            }
        } else if self.keys().safe(key) {
            return;
        }
        let held = match descriptor {
            true => Held::Descriptor(Some(key)),
            false => Held::Read(Some(key)),
        };
        if !self.harmless(at, held, true) {
            self.refuse(LOOSE_KEY);
        }
    }

    fn copied(&mut self, at: usize, descriptors: &'a CallExpression<'a>) -> bool {
        let Some((up, child)) = self.parent(at) else {
            return false;
        };
        let AstKind::CallExpression(call) = self.stack[up] else {
            return false;
        };
        let copier = self.builtin_member(&call.callee, &[OBJECT], &COPIERS)
            && !call.arguments.iter().any(Argument::is_spread);
        let second = call
            .arguments
            .get(1)
            .and_then(Argument::as_expression)
            .is_some_and(|argument| same(argument, child));
        copier && second && self.copies_carry(call, descriptors)
    }

    fn check_pattern_key(&mut self, key: &PropertyKey<'a>, computed: bool) {
        let loose = match (computed, key.as_expression()) {
            (true, Some(expr)) => match const_key(expr) {
                Some(text) => runner_key(&text),
                None => !self.keys().safe(expr),
            },
            _ => key
                .static_name()
                .is_none_or(|name| name.as_str().is_none_or(runner_key)),
        };
        if loose {
            self.refuse(PATTERN_KEY);
        }
    }
}

impl<'a> VisitJs<'a> for Reach<'_, 'a> {
    fn enter_node(&mut self, kind: AstKind<'a>) {
        self.stack.push(kind);
        match self.phase {
            Phase::Collect => self.collect(kind),
            Phase::Check => self.check(kind),
        }
    }

    fn leave_node(&mut self, _kind: AstKind<'a>) {
        self.stack.pop();
    }
}

fn run<'a>(scoping: &Scoping, mut walk: impl FnMut(&mut Reach<'_, 'a>)) -> Option<&'static str> {
    let mut reach = Reach::new(scoping);
    let reach = &mut reach;
    walk(reach);
    reach.phase = Phase::Check;
    for _ in 0..PASSES {
        reach.grew = false;
        walk(reach);
        if reach.found.is_some() || !reach.grew {
            return reach.found;
        }
    }
    Some(TOO_DEEP)
}

pub fn program_reach(program: &Program<'_>, scoping: &Scoping) -> Option<&'static str> {
    run(scoping, |reach| reach.visit_program(program))
}

pub fn statements_reach(statements: &[&Statement<'_>], scoping: &Scoping) -> Option<&'static str> {
    run(scoping, |reach| {
        for statement in statements {
            reach.visit_statement(statement);
        }
    })
}

#[derive(Default)]
struct MayReach {
    found: bool,
}

impl<'a> VisitJs<'a> for MayReach {
    fn visit_computed_member_expression(&mut self, it: &ComputedMemberExpression<'a>) {
        self.found |= const_key(&it.expression).is_none_or(|key| runner_key(&key));
        walk_js::walk_computed_member_expression(self, it);
    }

    fn visit_static_member_expression(&mut self, it: &StaticMemberExpression<'a>) {
        let name = it.property.name.as_str();
        self.found |= runner_key(name) || [REFLECT, DESCRIPTOR, DESCRIPTORS, GET].contains(&name);
        walk_js::walk_static_member_expression(self, it);
    }

    fn visit_identifier_reference(&mut self, it: &IdentifierReference<'a>) {
        self.found |= it.name == CONSTRUCTOR || it.name == REFLECT;
    }

    fn visit_with_statement(&mut self, it: &WithStatement<'a>) {
        self.found = true;
        walk_js::walk_with_statement(self, it);
    }

    fn visit_object_pattern(&mut self, it: &ObjectPattern<'a>) {
        self.found = true;
        walk_js::walk_object_pattern(self, it);
    }

    fn visit_object_assignment_target(&mut self, it: &ObjectAssignmentTarget<'a>) {
        self.found = true;
        walk_js::walk_object_assignment_target(self, it);
    }
}

pub fn may_reach(program: &Program<'_>) -> bool {
    let mut may = MayReach::default();
    may.visit_program(program);
    may.found
}

#[derive(Default)]
struct Arguments {
    found: bool,
}

impl<'a> VisitJs<'a> for Arguments {
    fn visit_identifier_reference(&mut self, it: &IdentifierReference<'a>) {
        self.found |= it.name == "arguments";
    }

    fn visit_function(&mut self, _it: &Function<'a>, _flags: oxc_syntax::scope::ScopeFlags) {}
}

fn reads_arguments(function: &Function<'_>) -> bool {
    let mut arguments = Arguments::default();
    if let Some(body) = &function.body {
        arguments.visit_function_body(body);
    }
    arguments.found
}
