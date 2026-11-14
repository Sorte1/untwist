use oxc_ast::ast::*;
use oxc_ast_visit::{walk_js, VisitJs};
use oxc_semantic::Scoping;
use oxc_syntax::symbol::SymbolId;
use rustc_hash::FxHashMap;

use crate::matcher::{const_member, literal_id, Factory};
use crate::webpack::buildin::Buildin;
use crate::webpack::require_form::Record;
use crate::webpack::runtime_module::own_arguments;

const MODULE_PARAM: usize = 0;
const REQUIRE_PARAM: usize = 2;
const EXPORTS: &str = "exports";
const LOADED: &str = "loaded";
const KEPT_MEMBERS: [&str; 2] = [EXPORTS, LOADED];
const ID_MEMBERS: [&str; 2] = ["id", "i"];
const ASYNC_MODULE: &str = "a";
const WASM_MODULE: &str = "v";
const WASM_EXPORTS: &str = "w";
const CALL: &str = "call";

#[derive(Clone, Copy, PartialEq, Eq)]
enum Held {
    Record,
    Polyfilled(Buildin),
}

struct ModuleUses<'s> {
    scoping: &'s Scoping,
    symbol: SymbolId,
    require: Option<SymbolId>,
    buildins: &'s FxHashMap<String, Buildin>,
    record: Record,
    legacy: bool,
    aliases: FxHashMap<SymbolId, Buildin>,
    polyfills: Vec<Buildin>,
    kept: &'s [&'s str],
    computed_kept: bool,
    found: Option<String>,
}

fn immediate<'k, 'a>(call: &'k CallExpression<'a>) -> Option<(&'k Function<'a>, usize)> {
    if call.optional {
        return None;
    }
    if let Expression::FunctionExpression(function) = call.callee.without_parentheses() {
        return Some((function, 0));
    }
    let callee = const_member(&call.callee, CALL)?;
    match callee.without_parentheses() {
        Expression::FunctionExpression(function) => Some((function, 1)),
        _ => None,
    }
}

fn plain_params(function: &Function<'_>) -> bool {
    function.params.rest.is_none()
        && function.params.items.iter().all(|item| {
            item.initializer.is_none()
                && matches!(item.pattern, BindingPattern::BindingIdentifier(_))
        })
}

fn simple_param(function: &Function<'_>, at: usize) -> Option<SymbolId> {
    match &function.params.items.get(at)?.pattern {
        BindingPattern::BindingIdentifier(id) => id.symbol_id.get(),
        _ => None,
    }
}

impl ModuleUses<'_> {
    fn resolves_to(&self, expr: &Expression<'_>, symbol: Option<SymbolId>) -> bool {
        let Expression::Identifier(id) = expr.without_parentheses() else {
            return false;
        };
        symbol.is_some()
            && id
                .reference_id
                .get()
                .and_then(|reference| self.scoping.get_reference(reference).symbol_id())
                == symbol
    }

    fn is_module(&self, expr: &Expression<'_>) -> bool {
        self.resolves_to(expr, Some(self.symbol))
    }

    fn helper(&self, expr: &Expression<'_>, name: &str) -> bool {
        matches!(expr.without_parentheses(), Expression::StaticMemberExpression(member)
            if member.property.name == name && self.resolves_to(&member.object, self.require))
    }

    fn module_id(&self, expr: &Expression<'_>) -> bool {
        matches!(expr.without_parentheses(), Expression::StaticMemberExpression(member)
            if ID_MEMBERS.contains(&member.property.name.as_str()) && self.is_module(&member.object))
    }

    fn silent_call(call: &CallExpression<'_>) -> bool {
        let Some(callee) = const_member(&call.callee, CALL) else {
            return false;
        };
        let Expression::FunctionExpression(function) = callee.without_parentheses() else {
            return false;
        };
        let silent = function.params.items.is_empty() && function.params.rest.is_none();
        silent
            && function
                .body
                .as_ref()
                .is_some_and(|body| !own_arguments(body))
    }

    fn handed_to_helper(&self, call: &CallExpression<'_>, at: usize) -> bool {
        if at > 0 && Self::silent_call(call) {
            return call
                .arguments
                .get(at)
                .and_then(Argument::as_expression)
                .is_some_and(|argument| self.is_module(argument));
        }
        let Some(argument) = call.arguments.get(at).and_then(Argument::as_expression) else {
            return false;
        };
        match at {
            0 => self.helper(&call.callee, ASYNC_MODULE) && self.is_module(argument),
            1 => self.helper(&call.callee, WASM_MODULE) && self.module_id(argument),
            _ => false,
        }
    }

    fn refuse(&mut self, what: impl Into<String>) {
        self.found.get_or_insert_with(|| what.into());
    }

    fn alias(&self, expr: &Expression<'_>) -> Option<Buildin> {
        let Expression::Identifier(id) = expr.without_parentheses() else {
            return None;
        };
        let symbol = id
            .reference_id
            .get()
            .and_then(|reference| self.scoping.get_reference(reference).symbol_id())?;
        self.aliases.get(&symbol).copied()
    }

    fn polyfill_call(&self, expr: &Expression<'_>) -> Option<Buildin> {
        let Expression::CallExpression(call) = expr.without_parentheses() else {
            return None;
        };
        let Expression::CallExpression(inner) = call.callee.without_parentheses() else {
            return None;
        };
        let [argument] = call.arguments.as_slice() else {
            return None;
        };
        let [id] = inner.arguments.as_slice() else {
            return None;
        };
        let shaped = !call.optional
            && !inner.optional
            && self.resolves_to(&inner.callee, self.require)
            && argument
                .as_expression()
                .is_some_and(|it| self.is_module(it));
        let id = literal_id(id.as_expression()?)?;
        shaped.then(|| self.buildins.get(&id).copied()).flatten()
    }

    fn held<'k, 'a>(
        &self,
        expr: &'k Expression<'a>,
    ) -> Option<(Buildin, Option<&'k Expression<'a>>, bool)> {
        if let Some(kind) = self.polyfill_call(expr) {
            return Some((kind, None, true));
        }
        if let Some(kind) = self.alias(expr) {
            return Some((kind, None, false));
        }
        let Expression::LogicalExpression(logical) = expr.without_parentheses() else {
            return None;
        };
        let (kind, _, call) = self.held(&logical.right)?;
        let plain = logical.operator == LogicalOperator::Or && self.held(&logical.left).is_none();
        plain.then_some((kind, Some(&logical.left), call))
    }

    fn handed_on<'a>(&mut self, call: &CallExpression<'a>) -> bool {
        let Some((function, offset)) = immediate(call) else {
            return false;
        };
        let body = function.body.as_deref();
        if body.is_none_or(own_arguments) || !plain_params(function) {
            return false;
        }
        if self.legacy && !self.sealed(function) {
            return false;
        }
        let mut handed = Vec::new();
        for (at, argument) in call.arguments.iter().enumerate() {
            let held = at
                .checked_sub(offset)
                .and_then(|index| Some((simple_param(function, index)?, argument.as_expression()?)))
                .and_then(|(symbol, expr)| self.held(expr).map(|held| (symbol, held)));
            handed.push(held);
        }
        if handed.iter().all(Option::is_none) {
            return false;
        }
        for (argument, held) in call.arguments.iter().zip(handed) {
            match held {
                Some((symbol, (kind, left, call))) => {
                    self.aliases.insert(symbol, kind);
                    if call {
                        self.polyfills.push(kind);
                    }
                    if let Some(left) = left {
                        self.visit_expression(left);
                    }
                }
                None => self.visit_argument(argument),
            }
        }
        self.visit_expression(&call.callee);
        true
    }

    fn strict(&self, function: &Function<'_>) -> bool {
        function
            .scope_id
            .get()
            .is_some_and(|scope| self.scoping.scope_flags(scope).is_strict_mode())
    }

    fn inert(&self, expr: &Expression<'_>, params: &[SymbolId]) -> bool {
        match expr.without_parentheses() {
            Expression::ThisExpression(_)
            | Expression::BooleanLiteral(_)
            | Expression::NumericLiteral(_)
            | Expression::StringLiteral(_)
            | Expression::NullLiteral(_) => true,
            Expression::Identifier(id) => id
                .reference_id
                .get()
                .and_then(|reference| self.scoping.get_reference(reference).symbol_id())
                .is_some_and(|symbol| params.contains(&symbol)),
            Expression::LogicalExpression(logical) => {
                self.inert(&logical.left, params) && self.inert(&logical.right, params)
            }
            other => self.held(other).is_some(),
        }
    }

    fn sealed(&self, function: &Function<'_>) -> bool {
        if self.strict(function) {
            return true;
        }
        let Some(body) = function.body.as_deref() else {
            return false;
        };
        let params: Vec<SymbolId> = (0..function.params.items.len())
            .filter_map(|at| simple_param(function, at))
            .collect();
        body.statements.iter().all(|statement| {
            let Statement::ExpressionStatement(statement) = statement else {
                return false;
            };
            let Expression::CallExpression(call) = statement.expression.without_parentheses()
            else {
                return false;
            };
            immediate(call).is_some_and(|(callee, _)| {
                self.strict(callee)
                    && call.arguments.iter().all(|argument| {
                        argument
                            .as_expression()
                            .is_some_and(|it| self.inert(it, &params))
                    })
            })
        })
    }

    fn held_object(&self, expr: &Expression<'_>) -> Option<Held> {
        if self.is_module(expr) {
            return Some(Held::Record);
        }
        self.alias(expr).map(Held::Polyfilled)
    }

    fn readable(&self, held: Held, name: &str) -> bool {
        if held == Held::Record && !self.kept.contains(&name) {
            return false;
        }
        match (held, name) {
            (Held::Record | Held::Polyfilled(Buildin::Module), EXPORTS) => true,
            (Held::Record, LOADED) => self.record.loaded,
            (Held::Polyfilled(_), LOADED) => self.record.l,
            _ => false,
        }
    }

    fn writable(&self, held: Held, name: &str) -> bool {
        name == EXPORTS
            && match held {
                Held::Record => self.kept.contains(&EXPORTS),
                Held::Polyfilled(kind) => kind == Buildin::Module,
            }
    }

    fn kept_key<'k>(&self, key: &'k Expression<'_>) -> Option<&'k str> {
        let Expression::StringLiteral(key) = key.without_parentheses() else {
            return None;
        };
        let key = key.value.as_str()?;
        (self.computed_kept && self.kept.contains(&key)).then_some(key)
    }

    fn member_of_held(&self, target: &SimpleAssignmentTarget<'_>) -> Option<(Held, String)> {
        match target {
            SimpleAssignmentTarget::StaticMemberExpression(member) => self
                .held_object(&member.object)
                .map(|held| (held, member.property.name.to_string())),
            SimpleAssignmentTarget::ComputedMemberExpression(member) => {
                self.held_object(&member.object).map(|held| {
                    let name = self.kept_key(&member.expression).unwrap_or("[computed]");
                    (held, name.to_string())
                })
            }
            _ => None,
        }
    }

    fn assigned_exports(&self, it: &AssignmentExpression<'_>) -> bool {
        let (object, name) = match &it.left {
            AssignmentTarget::StaticMemberExpression(member) => {
                (&member.object, Some(member.property.name.as_str()))
            }
            AssignmentTarget::ComputedMemberExpression(member) => {
                (&member.object, self.kept_key(&member.expression))
            }
            _ => return false,
        };
        let Some(name) = name else {
            return false;
        };
        it.operator == AssignmentOperator::Assign
            && self
                .held_object(object)
                .is_some_and(|held| self.writable(held, name))
    }
}

impl<'a> VisitJs<'a> for ModuleUses<'_> {
    fn visit_assignment_expression(&mut self, it: &AssignmentExpression<'a>) {
        if self.assigned_exports(it) {
            self.visit_expression(&it.right);
            return;
        }
        walk_js::walk_assignment_expression(self, it);
    }

    fn visit_simple_assignment_target(&mut self, it: &SimpleAssignmentTarget<'a>) {
        if let Some((_, name)) = self.member_of_held(it) {
            self.refuse(format!("a write to module.{name}"));
            return;
        }
        walk_js::walk_simple_assignment_target(self, it);
    }

    fn visit_static_member_expression(&mut self, it: &StaticMemberExpression<'a>) {
        let Some(held) = self.held_object(&it.object) else {
            walk_js::walk_static_member_expression(self, it);
            return;
        };
        let name = it.property.name.as_str();
        if !self.readable(held, name) {
            let what = match held {
                Held::Record => format!("module.{name}"),
                Held::Polyfilled(_) => format!("module.{name} of a webpack module polyfill"),
            };
            self.refuse(what);
        }
    }

    fn visit_call_expression(&mut self, it: &CallExpression<'a>) {
        if self.handed_on(it) {
            return;
        }
        self.visit_expression(&it.callee);
        for (at, argument) in it.arguments.iter().enumerate() {
            if !self.handed_to_helper(it, at) {
                self.visit_argument(argument);
            }
        }
    }

    fn visit_computed_member_expression(&mut self, it: &ComputedMemberExpression<'a>) {
        if self.helper(&it.object, WASM_EXPORTS) && self.module_id(&it.expression) {
            return;
        }
        let held = self.held_object(&it.object);
        let kept = self.kept_key(&it.expression);
        if let (Some(held), Some(name)) = (held, kept) {
            if !self.readable(held, name) {
                self.refuse(format!("module.{name}"));
            }
            return;
        }
        if held.is_some() {
            self.refuse("a computed member of module");
        }
        walk_js::walk_computed_member_expression(self, it);
    }

    fn visit_unary_expression(&mut self, it: &UnaryExpression<'a>) {
        let held = self.held_object(&it.argument).is_some();
        if it.operator == UnaryOperator::Typeof && held {
            return;
        }
        let member = match it.argument.without_parentheses() {
            Expression::StaticMemberExpression(member) => Some(&member.object),
            Expression::ComputedMemberExpression(member) => Some(&member.object),
            _ => None,
        };
        let deleted = it.operator == UnaryOperator::Delete
            && member.is_some_and(|object| self.held_object(object).is_some());
        if deleted {
            self.refuse("a delete of a member of module");
            return;
        }
        walk_js::walk_unary_expression(self, it);
    }

    fn visit_identifier_reference(&mut self, it: &IdentifierReference<'a>) {
        let symbol = it
            .reference_id
            .get()
            .and_then(|reference| self.scoping.get_reference(reference).symbol_id());
        if symbol == Some(self.symbol) {
            self.refuse("module as a value");
        }
        if symbol.is_some_and(|symbol| self.aliases.contains_key(&symbol)) {
            self.refuse("a webpack module polyfill as a value");
        }
    }
}

fn param_symbol(factory: &Factory<'_, '_>, at: usize) -> Option<SymbolId> {
    match &factory.params.items.get(at)?.pattern {
        BindingPattern::BindingIdentifier(id) => id.symbol_id.get(),
        _ => None,
    }
}

pub fn module_object_use(
    scoping: &Scoping,
    factory: &Factory<'_, '_>,
    (buildins, record, legacy): (&FxHashMap<String, Buildin>, Record, bool),
) -> Option<String> {
    let symbol = param_symbol(factory, MODULE_PARAM)?;
    let mut uses = ModuleUses {
        scoping,
        symbol,
        require: param_symbol(factory, REQUIRE_PARAM),
        buildins,
        record,
        legacy,
        aliases: FxHashMap::default(),
        polyfills: Vec::new(),
        kept: &KEPT_MEMBERS,
        computed_kept: false,
        found: None,
    };
    uses.visit_function_body(factory.body);
    let mixed = uses.polyfills.contains(&Buildin::Harmony) && uses.polyfills.len() > 1;
    if mixed {
        uses.refuse("two webpack module polyfills on one module");
    }
    uses.found
}

pub fn module_param_use(
    scoping: &Scoping,
    body: &FunctionBody<'_>,
    module: SymbolId,
    kept: &[&str],
) -> Option<String> {
    let buildins = FxHashMap::default();
    let mut uses = ModuleUses {
        scoping,
        symbol: module,
        require: None,
        buildins: &buildins,
        record: Record::default(),
        legacy: false,
        aliases: FxHashMap::default(),
        polyfills: Vec::new(),
        kept,
        computed_kept: true,
        found: None,
    };
    uses.visit_function_body(body);
    uses.found
}
