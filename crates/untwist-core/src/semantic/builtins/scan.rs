use oxc_ast::ast::*;
use oxc_ast_visit::{walk_js, VisitJs};
use oxc_semantic::Scoping;
use oxc_span::{GetSpan, Span};
use oxc_syntax::operator::{AssignmentOperator, BinaryOperator, LogicalOperator, UnaryOperator};
use oxc_syntax::scope::ScopeFlags;
use oxc_syntax::symbol::{SymbolFlags, SymbolId};
use rustc_hash::{FxHashMap, FxHashSet};
use std::cell::{Cell, OnceCell, RefCell};

use super::{Builtins, Reliance, Scanned};
use crate::semantic::{every_global_foreign, is_foreign_global, program_is_strict};

mod allow;
mod escapes;
mod functions;
mod keys;
mod modules;
mod numeric;
mod parts;
mod reach;
mod statics;
mod targets;
mod windows;

use allow::Root;

use reach::{
    call_of, key_of, member_of, member_parts, Key, CODE_RUNNERS, CONTEXT_RUNNERS, REACHING_KEYS,
    TIMERS,
};

const GLOBAL_OBJECTS: [&str; 7] = [
    "globalThis",
    "window",
    "self",
    "global",
    "top",
    "parent",
    "frames",
];

const DEFINERS: [(&str, &str); 8] = [
    ("Object", "defineProperty"),
    ("Object", "defineProperties"),
    ("Object", "setPrototypeOf"),
    ("Object", "assign"),
    ("Reflect", "defineProperty"),
    ("Reflect", "set"),
    ("Reflect", "setPrototypeOf"),
    ("Reflect", "deleteProperty"),
];

const KEYED_DEFINERS: [&str; 3] = ["defineProperty", "set", "deleteProperty"];

const LEGACY_DEFINERS: [&str; 2] = ["__defineGetter__", "__defineSetter__"];

const PROMISE: &str = "Promise";

const REFLECT: &str = "Reflect";

fn literal_key<'k>(expression: &'k Expression<'_>) -> Option<&'k str> {
    match expression.without_parentheses() {
        Expression::StringLiteral(literal) => literal.value.as_str(),
        _ => None,
    }
}

fn member_path<'k>(expression: &'k Expression<'_>, path: &mut Vec<&'k str>) -> bool {
    match expression.without_parentheses() {
        Expression::Identifier(id) => {
            path.push(id.name.as_str());
            true
        }
        Expression::StaticMemberExpression(member) => {
            let reached = member_path(&member.object, path);
            path.push(member.property.name.as_str());
            reached
        }
        Expression::ComputedMemberExpression(member) => {
            let reached = member_path(&member.object, path);
            match literal_key(&member.expression) {
                Some(key) => {
                    path.push(key);
                    reached
                }
                None => false,
            }
        }
        _ => false,
    }
}

fn target_path<'k>(target: &'k SimpleAssignmentTarget<'_>, path: &mut Vec<&'k str>) -> bool {
    match target {
        SimpleAssignmentTarget::StaticMemberExpression(member) => {
            let reached = member_path(&member.object, path);
            path.push(member.property.name.as_str());
            reached
        }
        SimpleAssignmentTarget::ComputedMemberExpression(member) => {
            let reached = member_path(&member.object, path);
            match literal_key(&member.expression) {
                Some(key) => {
                    path.push(key);
                    reached
                }
                None => false,
            }
        }
        _ => false,
    }
}

fn installed_value<'k, 'a>(
    read: &Expression<'_>,
    write: &'k Expression<'a>,
) -> Option<&'k Expression<'a>> {
    let Expression::AssignmentExpression(assignment) = write.without_parentheses() else {
        return None;
    };
    if assignment.operator != AssignmentOperator::Assign {
        return None;
    }
    let target = assignment.left.as_simple_assignment_target()?;
    let (mut read_path, mut written) = (Vec::new(), Vec::new());
    let same = member_path(read, &mut read_path)
        && target_path(target, &mut written)
        && read_path.len() > 1
        && read_path == written;
    same.then_some(&assignment.right)
}

fn installs_if_missing<'k, 'a>(logical: &'k LogicalExpression<'a>) -> Option<&'k Expression<'a>> {
    if logical.operator != LogicalOperator::Or {
        return None;
    }
    installed_value(&logical.left, &logical.right)
}

fn only_expression<'k, 'a>(statement: &'k Statement<'a>) -> Option<&'k Expression<'a>> {
    match statement {
        Statement::ExpressionStatement(expression) => Some(&expression.expression),
        Statement::BlockStatement(block) => match block.body.as_slice() {
            [only] => only_expression(only),
            _ => None,
        },
        _ => None,
    }
}

fn guarded_install<'k, 'a>(statement: &'k IfStatement<'a>) -> Option<&'k Expression<'a>> {
    if statement.alternate.is_some() {
        return None;
    }
    let Expression::UnaryExpression(test) = statement.test.without_parentheses() else {
        return None;
    };
    if test.operator != UnaryOperator::LogicalNot {
        return None;
    }
    installed_value(&test.argument, only_expression(&statement.consequent)?)
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum This {
    Global,
    MaybeGlobal,
    NotGlobal,
}

#[derive(Clone, Copy)]
struct Frame {
    strict: bool,
    this: This,
}

impl Frame {
    fn of_program(program: &Program<'_>, module: bool) -> Self {
        Self {
            strict: module || program_is_strict(program),
            this: if module {
                This::NotGlobal
            } else {
                This::Global
            },
        }
    }
}

struct Scan<'s> {
    program: &'s Program<'s>,
    consts: OnceCell<keys::ConstKeys>,
    functions: OnceCell<functions::FunctionBindings>,
    escaping: OnceCell<escapes::Escaping>,
    windows: OnceCell<windows::WindowLocals>,
    numeric: OnceCell<numeric::KeyBindings>,
    reached: RefCell<FxHashMap<usize, bool>>,
    scoping: &'s Scoping,
    reliance: &'s Reliance,
    writes_only: bool,
    unknown_keys_plain: bool,
    first_only: bool,
    statics: bool,
    skipped: Option<&'s FxHashSet<Span>>,
    module: bool,
    frames: Vec<Frame>,
    verdict: Builtins,
    promise_written: bool,
    host_member: Option<usize>,
    window_escaped: bool,
    window_writes: Cell<bool>,
}

fn names_a_foreign_root(scoping: &Scoping, reliance: &Reliance) -> bool {
    scoping.root_unresolved_references().keys().any(|name| {
        let name = name.as_str();
        let relied = reliance.roots.contains(&name)
            || name == REFLECT
            || GLOBAL_OBJECTS.contains(&name)
            || reach::CODE_RUNNERS.contains(&name);
        relied && is_foreign_global(name)
    })
}

fn declares_a_root_in_a_script(scoping: &Scoping, reliance: &Reliance, module: bool) -> bool {
    !module
        && scoping
            .get_bindings(scoping.root_scope_id())
            .iter()
            .any(|(name, symbol)| {
                let name = name.as_str();
                let rootish = reliance.roots.contains(&name)
                    || allow::BUILTIN_ROOTS.contains(&name)
                    || GLOBAL_OBJECTS.contains(&name)
                    || reach::CODE_RUNNERS.contains(&name);
                rootish
                    && scoping
                        .symbol_flags(*symbol)
                        .intersects(SymbolFlags::FunctionScopedVariable | SymbolFlags::Function)
            })
}

fn first_verdict(scoping: &Scoping, reliance: &Reliance, module: bool) -> Builtins {
    if names_a_foreign_root(scoping, reliance) {
        return Builtins::Written;
    }
    match every_global_foreign() || declares_a_root_in_a_script(scoping, reliance, module) {
        true => Builtins::Aliased,
        false => Builtins::Intact,
    }
}

pub fn scan(
    program: &Program<'_>,
    scoping: &Scoping,
    module: bool,
    writes_only: bool,
    reliance: &Reliance,
) -> Scanned {
    let mut scan = Scan::new(program, scoping, module, reliance);
    scan.writes_only = writes_only;
    scan.visit_program(program);
    scan.settle_window_flows();
    Scanned {
        verdict: scan.verdict,
        promise_written: scan.promise_written,
    }
}

pub fn scan_with_unknown_keys_plain(
    program: &Program<'_>,
    scoping: &Scoping,
    module: bool,
    reliance: &Reliance,
) -> Builtins {
    let mut scan = Scan::new(program, scoping, module, reliance);
    scan.unknown_keys_plain = true;
    scan.visit_program(program);
    scan.verdict
}

pub fn scan_outside(
    program: &Program<'_>,
    scoping: &Scoping,
    module: bool,
    reliance: &Reliance,
    skipped: &FxHashSet<Span>,
) -> Builtins {
    outside(program, scoping, module, reliance, skipped, false)
}

pub fn scan_outside_with_regexp_statics(
    program: &Program<'_>,
    scoping: &Scoping,
    module: bool,
    reliance: &Reliance,
    skipped: &FxHashSet<Span>,
) -> Builtins {
    outside(program, scoping, module, reliance, skipped, true)
}

fn outside(
    program: &Program<'_>,
    scoping: &Scoping,
    module: bool,
    reliance: &Reliance,
    skipped: &FxHashSet<Span>,
    statics: bool,
) -> Builtins {
    let mut scan = Scan::new(program, scoping, module, reliance);
    scan.skipped = Some(skipped);
    scan.statics = statics;
    scan.visit_program(program);
    scan.settle_window_flows();
    scan.verdict
}

pub fn intact(program: &Program<'_>, scoping: &Scoping, module: bool, reliance: &Reliance) -> bool {
    let mut scan = Scan::new(program, scoping, module, reliance);
    scan.first_only = true;
    scan.visit_program(program);
    scan.settle_window_flows();
    scan.verdict == Builtins::Intact
}

impl<'s> Scan<'s> {
    fn new(
        program: &'s Program<'s>,
        scoping: &'s Scoping,
        module: bool,
        reliance: &'s Reliance,
    ) -> Self {
        Self {
            program,
            consts: OnceCell::new(),
            functions: OnceCell::new(),
            escaping: OnceCell::new(),
            windows: OnceCell::new(),
            numeric: OnceCell::new(),
            reached: RefCell::new(FxHashMap::default()),
            scoping,
            reliance,
            writes_only: false,
            unknown_keys_plain: false,
            first_only: false,
            statics: false,
            skipped: None,
            module,
            frames: vec![Frame::of_program(program, module)],
            verdict: first_verdict(scoping, reliance, module),
            promise_written: false,
            host_member: None,
            window_escaped: false,
            window_writes: Cell::new(false),
        }
    }
}

impl Scan<'_> {
    pub(super) fn const_key(&self, symbol: SymbolId) -> Option<String> {
        self.consts
            .get_or_init(|| keys::const_keys(self.program, self.scoping))
            .get(&symbol)
            .cloned()
    }

    pub(super) fn numeric_key(&self, key: &Expression<'_>) -> bool {
        let numeric = self
            .numeric
            .get_or_init(|| numeric::key_bindings(self.program, self.scoping, self.module));
        numeric::is_harmless_key(key, self.scoping, numeric)
    }

    pub(super) fn sharpen<'k>(&self, key: Key<'k>) -> Key<'k> {
        match key {
            Key::Unknown(expression) if self.numeric_key(expression) => Key::Other,
            other => other,
        }
    }

    pub(super) fn only_a_function(&self, symbol: SymbolId) -> bool {
        self.functions
            .get_or_init(|| functions::function_bindings(self.program))
            .contains(&symbol)
            && !self.scoping.symbol_is_mutated(symbol)
    }

    pub(super) fn window_local(&self, symbol: SymbolId) -> bool {
        self.windows
            .get_or_init(|| {
                let consts = self
                    .consts
                    .get_or_init(|| keys::const_keys(self.program, self.scoping));
                windows::window_locals(self.program, self.scoping, consts)
            })
            .contains(&symbol)
    }

    fn escapes(&self, function: &Function<'_>) -> bool {
        self.escaping
            .get_or_init(|| escapes::escaping_functions(self.program, self.scoping, self.module))
            .contains(&escapes::function_address(function))
    }

    fn pattern_key_reaches(&self, computed: bool, key: &PropertyKey<'_>) -> bool {
        let name = match (computed, key) {
            (true, PropertyKey::StaticIdentifier(_) | PropertyKey::PrivateIdentifier(_)) => None,
            (true, _) => match key.as_expression().map(|key| self.sharpen(key_of(key))) {
                Some(Key::Name(name)) => Some(name),
                Some(Key::Other) | None => return false,
                Some(Key::Unknown(expression)) => {
                    return self
                        .known_key(expression)
                        .is_none_or(|name| self.pattern_name_reaches(&name))
                }
            },
            (false, PropertyKey::StaticIdentifier(id)) => Some(id.name.as_str()),
            (false, PropertyKey::StringLiteral(literal)) => literal.value.as_str(),
            (false, _) => None,
        };
        name.is_some_and(|name| self.pattern_name_reaches(name))
    }

    fn pattern_name_reaches(&self, name: &str) -> bool {
        REACHING_KEYS.contains(&name)
            || CODE_RUNNERS.contains(&name)
            || CONTEXT_RUNNERS.contains(&name)
            || TIMERS.contains(&name)
            || name == REFLECT
            || modules::LOADERS.contains(&name)
            || self.names_a_root(name)
    }

    fn pattern_read(&mut self, reaches: bool) {
        if reaches && !self.writes_only {
            self.aliased();
        }
    }

    pub(super) fn this_value(&self) -> This {
        self.frame().this
    }

    fn frame(&self) -> Frame {
        self.frames.last().copied().unwrap_or(Frame {
            strict: true,
            this: This::NotGlobal,
        })
    }

    fn within(&mut self, frame: Frame, walk: impl FnOnce(&mut Self)) {
        self.frames.push(frame);
        walk(self);
        self.frames.pop();
    }

    fn settled(&self) -> bool {
        self.verdict == Builtins::Written || (self.first_only && self.verdict != Builtins::Intact)
    }

    fn global_named(&self, id: &IdentifierReference<'_>, names: &[&str]) -> bool {
        if !names.contains(&id.name.as_str()) {
            return false;
        }
        let symbol = id
            .reference_id
            .get()
            .and_then(|reference| self.scoping.get_reference(reference).symbol_id());
        symbol.is_none_or(|symbol| self.is_script_global(symbol))
    }

    fn is_script_global(&self, symbol: SymbolId) -> bool {
        !self.module
            && self.scoping.symbol_scope_id(symbol) == self.scoping.root_scope_id()
            && self
                .scoping
                .symbol_flags(symbol)
                .intersects(SymbolFlags::FunctionScopedVariable | SymbolFlags::Function)
    }

    fn is_global_object(&self, expression: &Expression<'_>) -> bool {
        match expression.without_parentheses() {
            Expression::Identifier(id) => self.global_named(id, &GLOBAL_OBJECTS),
            Expression::ThisExpression(_) => self.frame().this == This::Global,
            _ => false,
        }
    }

    fn is_builtin(&self, expression: &Expression<'_>) -> bool {
        match expression.without_parentheses() {
            Expression::Identifier(id) => self.global_named(id, self.reliance.roots),
            Expression::StaticMemberExpression(member) => {
                let name = member.property.name.as_str();
                (name == "prototype" && self.is_builtin(&member.object))
                    || (self.reliance.roots.contains(&name)
                        && self.is_global_object(&member.object))
            }
            _ => false,
        }
    }

    fn is_root(&self, expression: &Expression<'_>) -> bool {
        self.is_named_root(expression)
            || self.is_code_runner_name(expression)
            || self.is_reached(expression)
    }

    fn is_named_root(&self, expression: &Expression<'_>) -> bool {
        if self.is_builtin(expression) || self.is_global_object(expression) {
            return true;
        }
        matches!(
            self.allow_root(expression),
            Some(Root::Builtin(_) | Root::Prototype(_) | Root::Global | Root::Host(_))
        )
    }

    fn reaches_builtin(&self, expression: &Expression<'_>) -> bool {
        let mut current = expression.without_parentheses();
        loop {
            if self.is_builtin(current) {
                return true;
            }
            current = match current {
                Expression::StaticMemberExpression(member) => member.object.without_parentheses(),
                Expression::ComputedMemberExpression(member) => member.object.without_parentheses(),
                _ => return false,
            };
        }
    }

    fn global_key(&self, object: &Expression<'_>, key: Option<&str>) -> bool {
        let named =
            |name: &str| self.reliance.roots.contains(&name) || GLOBAL_OBJECTS.contains(&name);
        let host = matches!(self.allow_root(object), Some(Root::Host(_)));
        (self.is_global_object(object)
            || self.is_named_root(object) && !self.is_builtin(object) && !host)
            && key.is_none_or(named)
    }

    fn builtin_slot<'e>(&self, object: &'e Expression<'_>) -> Option<(&'e str, bool)> {
        match object.without_parentheses() {
            Expression::Identifier(id) if self.global_named(id, self.reliance.roots) => {
                Some((id.name.as_str(), false))
            }
            Expression::StaticMemberExpression(member)
                if member.property.name.as_str() == "prototype" =>
            {
                match member.object.without_parentheses() {
                    Expression::Identifier(id) if self.global_named(id, self.reliance.roots) => {
                        Some((id.name.as_str(), true))
                    }
                    _ => None,
                }
            }
            _ => None,
        }
    }

    fn writes_member(&self, object: &Expression<'_>, key: Option<&str>) -> bool {
        if let Some((root, on_prototype)) = self.builtin_slot(object) {
            return self.reliance.covers(root, on_prototype, key);
        }
        self.reaches_builtin(object) || self.global_key(object, key)
    }

    fn is_maybe_global_this(&self, expression: &Expression<'_>) -> bool {
        matches!(
            expression.without_parentheses(),
            Expression::ThisExpression(_)
        ) && self.this_value() == This::MaybeGlobal
    }

    fn is_reached_value(&self, expression: &Expression<'_>) -> bool {
        self.is_reached(expression) && !self.is_maybe_global_this(expression)
    }

    fn writes_reached(&self, object: &Expression<'_>, key: Option<&str>) -> bool {
        if self.is_maybe_global_this(object) {
            return key.is_none_or(|key| self.names_a_root(key) || GLOBAL_OBJECTS.contains(&key));
        }
        if key.is_some_and(|key| self.names_a_root(key))
            || (self.is_reached(object) && self.relies_on_key(key))
        {
            return true;
        }
        let mut current = object.without_parentheses();
        while let Some((inner, _)) = member_of(current) {
            current = inner.without_parentheses();
            if self.is_reached_value(current) {
                return true;
            }
        }
        false
    }

    fn judge_write(&mut self, object: &Expression<'_>, key: Option<&str>) -> bool {
        if self.writes_member(object, key) {
            self.written();
            return true;
        }
        if self.writes_reached(object, key) {
            self.aliased();
        }
        false
    }

    fn member_write(&mut self, target: &Expression<'_>) -> bool {
        match target.without_parentheses() {
            Expression::StaticMemberExpression(member) => {
                self.judge_write(&member.object, Some(member.property.name.as_str()))
            }
            Expression::ComputedMemberExpression(member) => {
                self.judge_write(&member.object, literal_key(&member.expression))
            }
            _ => false,
        }
    }

    fn deletes_window_key(&self, target: &Expression<'_>) -> bool {
        match target.without_parentheses() {
            Expression::StaticMemberExpression(member) => {
                self.writes_window_name(&member.object, member.property.name.as_str())
            }
            Expression::ComputedMemberExpression(member) => {
                self.writes_window_key(&member.object, Some(&member.expression))
            }
            _ => false,
        }
    }

    fn assigns_window_key(&self, target: &SimpleAssignmentTarget<'_>) -> bool {
        match target {
            SimpleAssignmentTarget::StaticMemberExpression(member) => {
                self.writes_window_name(&member.object, member.property.name.as_str())
            }
            SimpleAssignmentTarget::ComputedMemberExpression(member) => {
                self.writes_window_key(&member.object, Some(&member.expression))
            }
            _ => false,
        }
    }

    fn judge_source(&mut self, source: Option<&StringLiteral<'_>>) {
        let safe = source
            .and_then(|source| source.value.as_str())
            .is_some_and(modules::is_safe_specifier);
        if !safe {
            self.not_allowed();
        }
    }

    fn visit_require(&mut self, call: &CallExpression<'_>) {
        let first = call.arguments.first().and_then(Argument::as_expression);
        let host_member = self.host_member == Some(std::ptr::from_ref(call).addr());
        if !modules::names_a_safe_module(first) && !(host_member && self.requires_process(call)) {
            self.not_allowed();
        }
        self.visit_checked_member(&call.callee);
        self.visit_arguments(&call.arguments);
    }

    fn written(&mut self) {
        self.verdict = Builtins::Written;
    }

    fn reaches_promise(&self, expression: &Expression<'_>) -> bool {
        let mut current = expression.without_parentheses();
        loop {
            current = match current {
                Expression::Identifier(id) => return self.global_named(id, &[PROMISE]),
                Expression::StaticMemberExpression(member) => {
                    if member.property.name.as_str() == PROMISE
                        && self.is_global_object(&member.object)
                    {
                        return true;
                    }
                    member.object.without_parentheses()
                }
                Expression::ComputedMemberExpression(member) => {
                    if literal_key(&member.expression) == Some(PROMISE)
                        && self.is_global_object(&member.object)
                    {
                        return true;
                    }
                    member.object.without_parentheses()
                }
                _ => return false,
            };
        }
    }

    fn promise_target(&self, target: &SimpleAssignmentTarget<'_>) -> bool {
        match target {
            SimpleAssignmentTarget::AssignmentTargetIdentifier(id) => {
                self.global_named(id, &[PROMISE])
            }
            SimpleAssignmentTarget::StaticMemberExpression(member) => {
                self.reaches_promise(&member.object)
                    || (member.property.name.as_str() == PROMISE
                        && self.is_global_object(&member.object))
            }
            SimpleAssignmentTarget::ComputedMemberExpression(member) => {
                self.reaches_promise(&member.object)
                    || (literal_key(&member.expression) == Some(PROMISE)
                        && self.is_global_object(&member.object))
            }
            _ => false,
        }
    }

    fn defines_on_promise(&self, arguments: &[Argument<'_>]) -> bool {
        let mut parts = arguments.iter().map(Argument::as_expression);
        let Some(Some(target)) = parts.next() else {
            return false;
        };
        let key = parts.next().flatten().and_then(literal_key);
        self.reaches_promise(target) || (key == Some(PROMISE) && self.is_global_object(target))
    }

    fn aliased(&mut self) {
        if self.verdict == Builtins::Intact {
            self.verdict = Builtins::Aliased;
        }
    }

    fn is_definer(&self, callee: &Expression<'_>) -> bool {
        let Expression::StaticMemberExpression(member) = callee.without_parentheses() else {
            return false;
        };
        let name = member.property.name.as_str();
        let Expression::Identifier(root) = member.object.without_parentheses() else {
            return false;
        };
        DEFINERS
            .iter()
            .any(|(owner, method)| *method == name && self.global_named(root, &[owner]))
    }

    fn definer_like(&self, callee: &Expression<'_>) -> Option<&'static str> {
        let (object, Key::Name(name)) = member_of(callee)? else {
            return None;
        };
        let owner = match object.without_parentheses() {
            Expression::Identifier(id) => id.name.as_str(),
            other => match member_of(other)? {
                (_, Key::Name(name)) => name,
                _ => return None,
            },
        };
        DEFINERS
            .iter()
            .find(|(definer, method)| *definer == owner && *method == name)
            .map(|(_, method)| *method)
    }

    fn defines_on_window(&self, callee: &Expression<'_>, arguments: &[Argument<'_>]) -> bool {
        let Some(method) = self.definer_like(callee) else {
            return false;
        };
        let mut parts = arguments.iter().map(Argument::as_expression);
        let Some(Some(target)) = parts.next() else {
            return false;
        };
        let key = match KEYED_DEFINERS.contains(&method) {
            true => parts.next().flatten(),
            false => None,
        };
        self.writes_window_key(target, key)
    }

    fn defines_on_builtin(&mut self, arguments: &[Argument<'_>]) -> bool {
        let mut parts = arguments.iter().map(Argument::as_expression);
        let Some(Some(target)) = parts.next() else {
            return true;
        };
        let key = parts.next().flatten().and_then(literal_key);
        self.judge_write(target, key)
    }

    fn legacy_definition(&mut self, callee: &Expression<'_>, arguments: &[Argument<'_>]) -> bool {
        let Some((object, key)) = member_of(callee) else {
            return false;
        };
        let name = match key {
            Key::Name(name) => Some(name.to_string()),
            Key::Unknown(expression) => self.known_key(expression),
            Key::Other => None,
        };
        if !name.is_some_and(|name| LEGACY_DEFINERS.contains(&name.as_str())) {
            return false;
        }
        let first = arguments.first().and_then(Argument::as_expression);
        if self.writes_window_key(object, first) {
            self.not_allowed();
        }
        self.judge_write(object, first.and_then(literal_key))
    }

    fn visit_unless_root(&mut self, expression: &Expression<'_>) {
        match expression.without_parentheses() {
            Expression::LogicalExpression(logical) => {
                if installs_if_missing(logical).is_some() {
                    self.not_allowed();
                }
                self.visit_unless_root(&logical.left);
                self.visit_unless_root(&logical.right);
                return;
            }
            Expression::ConditionalExpression(conditional) => {
                self.visit_unless_root(&conditional.test);
                self.visit_unless_root(&conditional.consequent);
                self.visit_unless_root(&conditional.alternate);
                return;
            }
            Expression::SequenceExpression(sequence) => {
                sequence
                    .expressions
                    .iter()
                    .for_each(|expression| self.visit_unless_root(expression));
                return;
            }
            _ => {}
        }
        if self.writes_only || !(self.is_root(expression) || self.window_value(expression)) {
            self.visit_expression(expression);
        } else {
            self.visit_root_parts(expression);
        }
    }

    fn visit_local_value(&mut self, symbol: Option<SymbolId>, expression: &Expression<'_>) {
        let local = symbol.is_some_and(|symbol| !self.is_script_global(symbol));
        if local && !self.writes_only && !self.is_root(expression) && self.window_value(expression)
        {
            self.visit_root_parts(expression);
        } else {
            self.visit_expression(expression);
        }
    }

    fn not_allowed(&mut self) {
        if !self.writes_only {
            self.aliased();
        }
    }

    fn visit_member_object(&mut self, object: &Expression<'_>, key: Key<'_>) {
        let stripped = object.without_parentheses();
        if let Some(root) = self.allow_root(stripped) {
            if !self.read_allowed(root, key) {
                self.not_allowed();
            }
            self.visit_checked_member(stripped);
            return;
        }
        if let Some(call) = call_of(stripped) {
            if self.requires_process(call) && self.reads_host_name(key) {
                self.host_member = Some(std::ptr::from_ref(call).addr());
            }
        }
        match stripped {
            Expression::LogicalExpression(_)
            | Expression::ConditionalExpression(_)
            | Expression::SequenceExpression(_)
            | Expression::AssignmentExpression(_) => self.visit_expression(object),
            _ => self.visit_unless_root(object),
        }
    }

    fn calls_through_a_prototype(&self, callee: &Expression<'_>) -> bool {
        member_of(callee.without_parentheses())
            .is_some_and(|(object, _)| matches!(self.allow_root(object), Some(Root::Prototype(_))))
    }

    fn visit_checked_member(&mut self, expression: &Expression<'_>) -> bool {
        let Some((object, key)) = member_of(expression) else {
            return self.visit_member_parts(expression);
        };
        self.visit_member_object(object, key);
        if let Some((_, Some(computed))) = member_parts(expression) {
            self.visit_expression(computed);
        }
        true
    }

    fn visit_timer_member(&mut self, expression: &Expression<'_>) -> bool {
        if self.timer_member(expression).is_none() {
            return false;
        }
        self.visit_checked_member(expression)
    }

    fn visit_callee(&mut self, callee: &Expression<'_>) {
        if !self.is_root(callee) && self.visit_timer_member(callee) {
            return;
        }
        match callee.without_parentheses() {
            Expression::ConditionalExpression(conditional) => {
                self.visit_expression(&conditional.test);
                self.visit_unless_root(&conditional.consequent);
                self.visit_unless_root(&conditional.alternate);
            }
            _ => self.visit_unless_root(callee),
        }
    }
}

impl<'a> VisitJs<'a> for Scan<'_> {
    fn visit_binding_property(&mut self, it: &BindingProperty<'a>) {
        let reaches = self.pattern_key_reaches(it.computed, &it.key);
        self.pattern_read(reaches);
        walk_js::walk_binding_property(self, it);
    }

    fn visit_assignment_target_property_property(
        &mut self,
        it: &AssignmentTargetPropertyProperty<'a>,
    ) {
        let reaches = self.pattern_key_reaches(it.computed, &it.name);
        self.pattern_read(reaches);
        walk_js::walk_assignment_target_property_property(self, it);
    }

    fn visit_assignment_target_property_identifier(
        &mut self,
        it: &AssignmentTargetPropertyIdentifier<'a>,
    ) {
        let reaches = self.pattern_name_reaches(it.binding.name.as_str());
        self.pattern_read(reaches);
        walk_js::walk_assignment_target_property_identifier(self, it);
    }

    fn visit_statement(&mut self, it: &Statement<'a>) {
        if self.settled()
            || self
                .skipped
                .is_some_and(|skipped| skipped.contains(&it.span()))
        {
            return;
        }
        walk_js::walk_statement(self, it);
    }

    fn visit_expression(&mut self, it: &Expression<'a>) {
        if self.settled() {
            return;
        }
        if !self.writes_only && self.timer_member(it).is_some() && !self.is_root(it) {
            self.aliased();
        }
        if !self.writes_only && self.window_value(it) {
            self.window_escaped = true;
        }
        if !self.writes_only && (self.is_root(it) || self.reads_prototypes(it)) {
            self.aliased();
            self.visit_root_parts(it);
            return;
        }
        walk_js::walk_expression(self, it);
    }

    fn visit_static_member_expression(&mut self, it: &StaticMemberExpression<'a>) {
        let key = Key::Name(it.property.name.as_str());
        self.judge_statics(&it.object, key);
        if self.names_a_loader(key) {
            self.not_allowed();
        }
        if !self.reads_off_require(&it.object, key) {
            self.visit_member_object(&it.object, key);
        }
    }

    fn visit_computed_member_expression(&mut self, it: &ComputedMemberExpression<'a>) {
        let key = self.sharpen(key_of(&it.expression));
        self.judge_statics(&it.object, key);
        if self.names_a_loader(key) {
            self.not_allowed();
        }
        if !self.reads_off_require(&it.object, key) {
            self.visit_member_object(&it.object, key);
        }
        self.visit_expression(&it.expression);
    }

    fn visit_identifier_reference(&mut self, it: &IdentifierReference<'a>) {
        let name = it.name.as_str();
        if REACHING_KEYS.contains(&name) && self.global_named(it, &REACHING_KEYS) {
            self.not_allowed();
        }
        if modules::LOADERS.contains(&name) {
            self.not_allowed();
        }
    }

    fn visit_variable_declarator(&mut self, it: &VariableDeclarator<'a>) {
        let (BindingPattern::BindingIdentifier(id), Some(init)) = (&it.id, &it.init) else {
            walk_js::walk_variable_declarator(self, it);
            return;
        };
        self.visit_binding_identifier(id);
        self.visit_local_value(id.symbol_id.get(), init);
    }

    fn visit_with_statement(&mut self, it: &WithStatement<'a>) {
        self.not_allowed();
        walk_js::walk_with_statement(self, it);
    }

    fn visit_import_expression(&mut self, it: &ImportExpression<'a>) {
        if !modules::names_a_safe_module(Some(&it.source)) {
            self.not_allowed();
        }
        walk_js::walk_import_expression(self, it);
    }

    fn visit_import_declaration(&mut self, it: &ImportDeclaration<'a>) {
        self.judge_source(Some(&it.source));
        let process = modules::source_is_process(&it.source);
        for specifier in it.specifiers.iter().flatten() {
            match specifier {
                ImportDeclarationSpecifier::ImportSpecifier(named) => {
                    self.judge_module_name(&named.imported, process);
                }
                ImportDeclarationSpecifier::ImportDefaultSpecifier(_)
                | ImportDeclarationSpecifier::ImportNamespaceSpecifier(_) => {
                    if process {
                        self.not_allowed();
                    }
                }
            }
        }
    }

    fn visit_export_all_declaration(&mut self, it: &ExportAllDeclaration<'a>) {
        self.judge_source(Some(&it.source));
        if modules::source_is_process(&it.source) {
            self.not_allowed();
        }
    }

    fn visit_export_from_declaration(&mut self, it: &ExportFromDeclaration<'a>) {
        self.judge_source(Some(&it.source));
        let process = modules::source_is_process(&it.source);
        for specifier in &it.specifiers {
            self.judge_module_name(&specifier.local, process);
        }
    }

    fn visit_call_expression(&mut self, it: &CallExpression<'a>) {
        if self.calls_require(&it.callee) {
            self.visit_require(it);
            return;
        }
        if self.defines_on_window(&it.callee, &it.arguments) {
            self.not_allowed();
        }
        if self.is_definer(&it.callee) && self.defines_on_promise(&it.arguments) {
            self.promise_written = true;
        }
        let defines = self.is_definer(&it.callee) && self.defines_on_builtin(&it.arguments);
        if defines || self.legacy_definition(&it.callee, &it.arguments) {
            self.written();
            return;
        }
        if self.runs_code(&it.callee, &it.arguments) {
            self.aliased();
        }
        if self.calls_through_a_prototype(&it.callee) {
            self.not_allowed();
        }
        if self.reads_prototypes(&it.callee) {
            self.visit_member_parts(&it.callee);
        } else {
            self.visit_callee(&it.callee);
        }
        self.visit_arguments(&it.arguments);
    }

    fn visit_tagged_template_expression(&mut self, it: &TaggedTemplateExpression<'a>) {
        if self.legacy_definition(&it.tag, &[]) {
            self.written();
            return;
        }
        if self.runs_code(&it.tag, &[]) {
            self.aliased();
        }
        if self.calls_through_a_prototype(&it.tag) {
            self.not_allowed();
        }
        if self.reads_prototypes(&it.tag) {
            self.visit_member_parts(&it.tag);
        } else {
            self.visit_callee(&it.tag);
        }
        self.visit_template_literal(&it.quasi);
    }

    fn visit_expression_statement(&mut self, it: &ExpressionStatement<'a>) {
        self.visit_unless_root(&it.expression);
    }

    fn visit_sequence_expression(&mut self, it: &SequenceExpression<'a>) {
        if let Some((last, discarded)) = it.expressions.split_last() {
            discarded
                .iter()
                .for_each(|expression| self.visit_unless_root(expression));
            self.visit_expression(last);
        }
    }

    fn visit_new_expression(&mut self, it: &NewExpression<'a>) {
        if self.runs_code(&it.callee, &it.arguments) {
            self.aliased();
        }
        self.visit_callee(&it.callee);
        self.visit_arguments(&it.arguments);
    }

    fn visit_unary_expression(&mut self, it: &UnaryExpression<'a>) {
        match it.operator {
            UnaryOperator::Typeof if self.is_require_name(&it.argument) => {}
            UnaryOperator::Typeof if self.visit_timer_member(&it.argument) => {}
            UnaryOperator::Typeof | UnaryOperator::LogicalNot | UnaryOperator::Void => {
                self.visit_unless_root(&it.argument);
            }
            UnaryOperator::Delete => self.visit_delete(it),
            UnaryOperator::UnaryNegation | UnaryOperator::UnaryPlus | UnaryOperator::BitwiseNot => {
                walk_js::walk_unary_expression(self, it)
            }
        }
    }

    fn visit_binary_expression(&mut self, it: &BinaryExpression<'a>) {
        if it.operator.is_equality() {
            self.visit_unless_root(&it.left);
            self.visit_unless_root(&it.right);
            return;
        }
        self.visit_expression(&it.left);
        if matches!(it.operator, BinaryOperator::Instanceof | BinaryOperator::In) {
            self.visit_unless_root(&it.right);
        } else {
            self.visit_expression(&it.right);
        }
    }

    fn visit_assignment_expression(&mut self, it: &AssignmentExpression<'a>) {
        let installs = matches!(
            it.operator,
            AssignmentOperator::LogicalOr | AssignmentOperator::LogicalNullish
        ) && it
            .left
            .as_simple_assignment_target()
            .is_some_and(|target| target_path(target, &mut Vec::new()));
        if installs {
            self.not_allowed();
            self.visit_expression(&it.right);
            return;
        }
        if it.operator != AssignmentOperator::Assign && self.target_reads_a_root(&it.left) {
            self.not_allowed();
        }
        if let AssignmentTarget::AssignmentTargetIdentifier(id) = &it.left {
            let symbol = id
                .reference_id
                .get()
                .and_then(|reference| self.scoping.get_reference(reference).symbol_id());
            self.visit_assignment_target(&it.left);
            self.visit_local_value(symbol, &it.right);
            return;
        }
        walk_js::walk_assignment_expression(self, it);
    }

    fn visit_if_statement(&mut self, it: &IfStatement<'a>) {
        if let Some(value) = guarded_install(it) {
            self.not_allowed();
            self.visit_expression(value);
            return;
        }
        self.visit_unless_root(&it.test);
        self.visit_statement(&it.consequent);
        if let Some(alternate) = &it.alternate {
            self.visit_statement(alternate);
        }
    }

    fn visit_while_statement(&mut self, it: &WhileStatement<'a>) {
        self.visit_unless_root(&it.test);
        self.visit_statement(&it.body);
    }

    fn visit_do_while_statement(&mut self, it: &DoWhileStatement<'a>) {
        self.visit_statement(&it.body);
        self.visit_unless_root(&it.test);
    }

    fn visit_conditional_expression(&mut self, it: &ConditionalExpression<'a>) {
        self.visit_unless_root(&it.test);
        self.visit_expression(&it.consequent);
        self.visit_expression(&it.alternate);
    }

    fn visit_logical_expression(&mut self, it: &LogicalExpression<'a>) {
        if let Some(value) = installs_if_missing(it) {
            self.not_allowed();
            self.visit_expression(value);
            return;
        }
        if it.operator == LogicalOperator::And {
            self.visit_unless_root(&it.left);
        } else {
            self.visit_expression(&it.left);
        }
        self.visit_expression(&it.right);
    }

    fn visit_simple_assignment_target(&mut self, it: &SimpleAssignmentTarget<'a>) {
        if self.promise_target(it) {
            self.promise_written = true;
        }
        let written = match it {
            SimpleAssignmentTarget::AssignmentTargetIdentifier(id) => {
                self.global_named(id, self.reliance.roots) || self.global_named(id, &GLOBAL_OBJECTS)
            }
            SimpleAssignmentTarget::StaticMemberExpression(member) => {
                self.judge_write(&member.object, Some(member.property.name.as_str()))
            }
            SimpleAssignmentTarget::ComputedMemberExpression(member) => {
                self.judge_write(&member.object, literal_key(&member.expression))
            }
            SimpleAssignmentTarget::PrivateFieldExpression(_) => false,
            SimpleAssignmentTarget::TSAsExpression(_)
            | SimpleAssignmentTarget::TSSatisfiesExpression(_)
            | SimpleAssignmentTarget::TSNonNullExpression(_)
            | SimpleAssignmentTarget::TSTypeAssertion(_) => {
                self.not_allowed();
                false
            }
        };
        if written {
            self.written();
            return;
        }
        if self.assigns_window_key(it) {
            self.not_allowed();
        }
        let on_root = it.as_member_expression().is_some_and(|member| {
            self.allow_root(member.object())
                .is_some_and(|root| !matches!(root, Root::Global | Root::Host(_)))
        });
        if on_root {
            self.not_allowed();
        }
        walk_js::walk_simple_assignment_target(self, it);
    }

    fn visit_function(&mut self, it: &Function<'a>, flags: ScopeFlags) {
        let strict = self.frame().strict || it.has_use_strict_directive();
        let frame = Frame {
            strict,
            this: if strict && !self.escapes(it) {
                This::NotGlobal
            } else {
                This::MaybeGlobal
            },
        };
        self.within(frame, |scan| walk_js::walk_function(scan, it, flags));
    }

    fn visit_arrow_function_expression(&mut self, it: &ArrowFunctionExpression<'a>) {
        let outer = self.frame();
        let frame = Frame {
            strict: outer.strict || it.has_use_strict_directive(),
            ..outer
        };
        self.within(frame, |scan| {
            walk_js::walk_arrow_function_expression(scan, it);
        });
    }

    fn visit_super(&mut self, _it: &Super) {
        if !self.writes_only && !self.frame().strict && self.this_value() == This::MaybeGlobal {
            self.aliased();
        }
    }

    fn visit_class(&mut self, it: &Class<'a>) {
        let frame = Frame {
            strict: true,
            ..self.frame()
        };
        self.within(frame, |scan| walk_js::walk_class(scan, it));
    }
}
