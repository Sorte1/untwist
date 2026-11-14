use oxc_ast::ast::*;
use oxc_ast_visit::{walk_js, VisitJs};
use oxc_semantic::Scoping;
use oxc_syntax::scope::ScopeFlags;
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
const OBJECT_PROTOTYPE: [&str; 12] = [
    "constructor",
    "hasOwnProperty",
    "isPrototypeOf",
    "propertyIsEnumerable",
    "toLocaleString",
    "toString",
    "valueOf",
    "__proto__",
    "__defineGetter__",
    "__defineSetter__",
    "__lookupGetter__",
    "__lookupSetter__",
];

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
    faithful: bool,
    record_read: bool,
    depth: u32,
    repeated: bool,
    found: Option<String>,
}

pub struct PolyfillUse {
    pub record: bool,
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

    fn silent_call(&self, call: &CallExpression<'_>) -> bool {
        let Some(callee) = const_member(&call.callee, CALL) else {
            return false;
        };
        let Expression::FunctionExpression(function) = callee.without_parentheses() else {
            return false;
        };
        let silent = function.params.items.is_empty() && function.params.rest.is_none();
        silent
            && (!self.legacy || self.strict(function))
            && function
                .body
                .as_ref()
                .is_some_and(|body| !own_arguments(body))
    }

    fn handed_to_helper(&self, call: &CallExpression<'_>, at: usize) -> bool {
        if at > 0 && self.silent_call(call) {
            return call
                .arguments
                .get(at)
                .and_then(Argument::as_expression)
                .is_some_and(|argument| self.held_object(argument).is_some());
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
                        self.repeated |= self.depth > 0;
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

    fn read_through_record(&self, held: Held, name: &str) -> bool {
        self.faithful
            && matches!(held, Held::Polyfilled(_))
            && !ID_MEMBERS.contains(&name)
            && !OBJECT_PROTOTYPE.contains(&name)
    }

    fn visit_test<'a>(&mut self, expr: &Expression<'a>) {
        if self.held_object(expr).is_some() {
            return;
        }
        match expr.without_parentheses() {
            Expression::LogicalExpression(logical) => {
                self.visit_test(&logical.left);
                self.visit_test(&logical.right);
            }
            Expression::UnaryExpression(unary) if unary.operator == UnaryOperator::LogicalNot => {
                self.visit_test(&unary.argument);
            }
            other => self.visit_expression(other),
        }
    }

    fn visit_discarded<'a>(&mut self, expr: &Expression<'a>) {
        if self.held_object(expr).is_some() {
            return;
        }
        match expr.without_parentheses() {
            Expression::LogicalExpression(logical) => {
                self.visit_test(&logical.left);
                self.visit_discarded(&logical.right);
            }
            Expression::ConditionalExpression(conditional) => {
                self.visit_test(&conditional.test);
                self.visit_discarded(&conditional.consequent);
                self.visit_discarded(&conditional.alternate);
            }
            Expression::SequenceExpression(sequence) => {
                for expression in &sequence.expressions {
                    self.visit_discarded(expression);
                }
            }
            other => self.visit_expression(other),
        }
    }

    fn never_read(&self, declarator: &VariableDeclarator<'_>) -> bool {
        let BindingPattern::BindingIdentifier(id) = &declarator.id else {
            return false;
        };
        let Some(symbol) = id.symbol_id.get() else {
            return false;
        };
        let own = self.scoping.symbol_span(symbol) == id.span
            && self.scoping.symbol_redeclarations(symbol).is_empty();
        if !own {
            return false;
        }
        let unread = self
            .scoping
            .get_resolved_references(symbol)
            .all(|reference| !reference.is_read());
        unread
            && !self.scoping.scope_descendants_from_root().any(|scope| {
                self.scoping
                    .scope_flags(scope)
                    .intersects(ScopeFlags::DirectEval | ScopeFlags::With)
            })
    }

    fn receiver(&mut self, callee: &Expression<'_>) {
        let (object, name) = match callee.without_parentheses() {
            Expression::StaticMemberExpression(member) => {
                (&member.object, Some(member.property.name.as_str()))
            }
            Expression::ComputedMemberExpression(member) => {
                (&member.object, self.kept_key(&member.expression))
            }
            Expression::ChainExpression(chain) => match &chain.expression {
                ChainElement::StaticMemberExpression(member) => {
                    (&member.object, Some(member.property.name.as_str()))
                }
                ChainElement::ComputedMemberExpression(member) => {
                    (&member.object, self.kept_key(&member.expression))
                }
                _ => return,
            },
            _ => return,
        };
        let Some(held) = self.held_object(object) else {
            return;
        };
        let copied = name
            .is_some_and(|name| !self.readable(held, name) && self.read_through_record(held, name));
        if !copied {
            self.refuse("a member of module as a call receiver");
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
        if !self.readable(held, name) && self.read_through_record(held, name) {
            self.record_read = true;
            return;
        }
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
        self.receiver(&it.callee);
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

    fn visit_tagged_template_expression(&mut self, it: &TaggedTemplateExpression<'a>) {
        self.receiver(&it.tag);
        walk_js::walk_tagged_template_expression(self, it);
    }

    fn visit_function(&mut self, it: &Function<'a>, flags: ScopeFlags) {
        self.depth += 1;
        walk_js::walk_function(self, it, flags);
        self.depth -= 1;
    }

    fn visit_arrow_function_expression(&mut self, it: &ArrowFunctionExpression<'a>) {
        self.depth += 1;
        walk_js::walk_arrow_function_expression(self, it);
        self.depth -= 1;
    }

    fn visit_class(&mut self, it: &Class<'a>) {
        self.depth += 1;
        walk_js::walk_class(self, it);
        self.depth -= 1;
    }

    fn visit_for_in_statement(&mut self, it: &ForInStatement<'a>) {
        self.depth += 1;
        walk_js::walk_for_in_statement(self, it);
        self.depth -= 1;
    }

    fn visit_for_of_statement(&mut self, it: &ForOfStatement<'a>) {
        self.depth += 1;
        walk_js::walk_for_of_statement(self, it);
        self.depth -= 1;
    }

    fn visit_expression_statement(&mut self, it: &ExpressionStatement<'a>) {
        self.visit_discarded(&it.expression);
    }

    fn visit_variable_declarator(&mut self, it: &VariableDeclarator<'a>) {
        match &it.init {
            Some(init) if self.never_read(it) => self.visit_discarded(init),
            _ => walk_js::walk_variable_declarator(self, it),
        }
    }

    fn visit_if_statement(&mut self, it: &IfStatement<'a>) {
        self.visit_test(&it.test);
        self.visit_statement(&it.consequent);
        if let Some(alternate) = &it.alternate {
            self.visit_statement(alternate);
        }
    }

    fn visit_while_statement(&mut self, it: &WhileStatement<'a>) {
        self.depth += 1;
        self.visit_test(&it.test);
        self.visit_statement(&it.body);
        self.depth -= 1;
    }

    fn visit_do_while_statement(&mut self, it: &DoWhileStatement<'a>) {
        self.depth += 1;
        self.visit_statement(&it.body);
        self.visit_test(&it.test);
        self.depth -= 1;
    }

    fn visit_for_statement(&mut self, it: &ForStatement<'a>) {
        if let Some(init) = &it.init {
            self.visit_for_statement_init(init);
        }
        self.depth += 1;
        if let Some(test) = &it.test {
            self.visit_test(test);
        }
        if let Some(update) = &it.update {
            self.visit_expression(update);
        }
        self.visit_statement(&it.body);
        self.depth -= 1;
    }

    fn visit_conditional_expression(&mut self, it: &ConditionalExpression<'a>) {
        self.visit_test(&it.test);
        self.visit_expression(&it.consequent);
        self.visit_expression(&it.alternate);
    }

    fn visit_logical_expression(&mut self, it: &LogicalExpression<'a>) {
        if it.operator != LogicalOperator::And {
            walk_js::walk_logical_expression(self, it);
            return;
        }
        self.visit_test(&it.left);
        self.visit_expression(&it.right);
    }

    fn visit_unary_expression(&mut self, it: &UnaryExpression<'a>) {
        if it.operator == UnaryOperator::LogicalNot {
            self.visit_test(&it.argument);
            return;
        }
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
    (buildins, record, legacy, trust_builtins): (&FxHashMap<String, Buildin>, Record, bool, bool),
) -> Result<PolyfillUse, String> {
    let Some(symbol) = param_symbol(factory, MODULE_PARAM) else {
        return Ok(PolyfillUse { record: false });
    };
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
        faithful: trust_builtins && record.l,
        record_read: false,
        depth: 0,
        repeated: false,
        found: None,
    };
    uses.visit_function_body(factory.body);
    let mixed = uses.polyfills.contains(&Buildin::Harmony) && uses.polyfills.len() > 1;
    if mixed || uses.record_read && (uses.polyfills.len() != 1 || uses.repeated) {
        uses.refuse("two webpack module polyfills on one module");
    }
    match uses.found {
        Some(found) => Err(found),
        None => Ok(PolyfillUse {
            record: uses.record_read,
        }),
    }
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
        faithful: false,
        record_read: false,
        depth: 0,
        repeated: false,
        found: None,
    };
    uses.visit_function_body(body);
    uses.found
}
