mod events;
mod functions;
mod methods;
mod values;

use std::borrow::Cow;

use oxc_ast::ast::*;
use oxc_ast_visit::{walk_js, VisitJs};
use oxc_semantic::Scoping;
use oxc_syntax::operator::UnaryOperator;
use oxc_syntax::scope::ScopeFlags;
use oxc_syntax::symbol::SymbolId;
use rustc_hash::{FxHashMap, FxHashSet};
use untwist_core::ast_util::prop::member_parts;

use crate::collect::{module_export_name, reference_symbol};
use crate::generate::{is_identifier_name, lower_camel};
use crate::human::looks_human_written;
use events::{
    event_param_name, handler_name, on_property_event, settled_value_name, timer_callback_name,
};
use functions::{arrow_facts, function_facts, ArgName, CallFacts, FunctionFacts};
use methods::MethodSites;
pub use values::is_word_like;
use values::{
    assigned_value_name, declared_value_name, element_name, is_informative_init,
    is_naming_property, is_storage_call, key_name, method_name, string_argument,
};

const ELEMENT_CALLBACKS: [&str; 10] = [
    "forEach",
    "map",
    "filter",
    "find",
    "findIndex",
    "findLast",
    "findLastIndex",
    "some",
    "every",
    "flatMap",
];

const ELEMENT_PARAMS: [&str; 3] = ["item", "index", "array"];

const REDUCERS: [&str; 2] = ["reduce", "reduceRight"];

const REDUCER_PARAMS: [&str; 4] = ["acc", "item", "index", "array"];

const SORTERS: [&str; 2] = ["sort", "toSorted"];

const SORTER_PARAMS: [&str; 2] = ["a", "b"];

const EXECUTOR_PARAMS: [&str; 2] = ["resolve", "reject"];

const EVENT_SUBSCRIBERS: [&str; 3] = ["addEventListener", "on", "once"];

const NAMING_HOLDERS: [&str; 5] = ["exports", "window", "self", "globalThis", "global"];

const FRAME_TIME: &str = "time";

const SETTER_VALUE: &str = "value";

const CALLBACK: &str = "callback";

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Rank {
    Exported,
    Fixed,
    Holder,
    Assigned,
    Written,
    Role,
    CallSite,
    Shape,
}

impl Rank {
    fn needs_agreement(self) -> bool {
        matches!(self, Rank::Assigned | Rank::Written | Rank::CallSite)
    }

    fn yields_to(self, defined: bool, param: bool) -> bool {
        (defined && self >= Rank::Assigned) || (param && self == Rank::Assigned)
    }
}

#[derive(Default)]
pub struct Evidence {
    names: FxHashMap<SymbolId, String>,
    constructors: FxHashSet<SymbolId>,
    flows: FxHashMap<SymbolId, Vec<SymbolId>>,
}

impl Evidence {
    pub fn name(&self, symbol: SymbolId) -> Option<&str> {
        self.names.get(&symbol).map(String::as_str)
    }

    pub fn flows_into(&self, symbol: SymbolId) -> &[SymbolId] {
        self.flows.get(&symbol).map_or(&[], Vec::as_slice)
    }

    pub fn is_constructor(&self, symbol: SymbolId) -> bool {
        self.constructors.contains(&symbol)
    }
}

pub fn collect_evidence(program: &Program<'_>, scoping: &Scoping) -> Evidence {
    let mut walker = EvidenceWalker {
        scoping,
        offers: FxHashMap::default(),
        params: FxHashSet::default(),
        defined: FxHashSet::default(),
        functions: FxHashMap::default(),
        calls: FxHashMap::default(),
        handlers: Vec::new(),
        prototypes: FxHashSet::default(),
        flows: FxHashMap::default(),
        methods: MethodSites::default(),
    };
    walker.visit_program(program);
    walker.finish()
}

struct Handler {
    function: SymbolId,
    name: Option<String>,
    param: &'static str,
}

struct EvidenceWalker<'s> {
    scoping: &'s Scoping,
    offers: FxHashMap<SymbolId, Vec<(Rank, String)>>,
    params: FxHashSet<SymbolId>,
    defined: FxHashSet<SymbolId>,
    functions: FxHashMap<SymbolId, FunctionFacts>,
    calls: FxHashMap<SymbolId, CallFacts>,
    handlers: Vec<Handler>,
    prototypes: FxHashSet<SymbolId>,
    flows: FxHashMap<SymbolId, Vec<SymbolId>>,
    methods: MethodSites,
}

fn simple_symbol(pattern: &BindingPattern<'_>) -> Option<SymbolId> {
    match pattern {
        BindingPattern::BindingIdentifier(id) => id.symbol_id.get(),
        _ => None,
    }
}

fn callback_params<'k, 'a>(argument: &'k Argument<'a>) -> Option<&'k FormalParameters<'a>> {
    match argument {
        Argument::FunctionExpression(function) => Some(&function.params),
        Argument::ArrowFunctionExpression(arrow) => Some(&arrow.params),
        _ => None,
    }
}

fn expression_params<'k, 'a>(expr: &'k Expression<'a>) -> Option<&'k FormalParameters<'a>> {
    match expr.without_parentheses() {
        Expression::FunctionExpression(function) => Some(&function.params),
        Expression::ArrowFunctionExpression(arrow) => Some(&arrow.params),
        _ => None,
    }
}

fn is_naming_holder(object: &Expression<'_>) -> bool {
    match object.without_parentheses() {
        Expression::ThisExpression(_) => true,
        Expression::Identifier(id) => NAMING_HOLDERS.contains(&id.name.as_str()),
        Expression::StaticMemberExpression(member) => {
            member.property.name == "prototype"
                || (member.property.name == "exports"
                    && matches!(member.object.without_parentheses(), Expression::Identifier(id) if id.name == "module"))
        }
        _ => false,
    }
}

fn written_property<'k, 'a>(
    target: &'k AssignmentTarget<'a>,
) -> Option<(&'k Expression<'a>, Cow<'k, str>)> {
    match target {
        AssignmentTarget::StaticMemberExpression(member) => {
            Some((&member.object, Cow::Borrowed(member.property.name.as_str())))
        }
        AssignmentTarget::ComputedMemberExpression(member) => match &member.expression {
            Expression::StringLiteral(literal) => {
                Some((&member.object, Cow::Borrowed(literal.value.as_str()?)))
            }
            _ => None,
        },
        _ => None,
    }
}

fn pascal(name: &str) -> String {
    let mut chars = name.chars();
    match chars.next() {
        Some(first) => format!("{}{}", first.to_ascii_uppercase(), chars.as_str()),
        None => String::new(),
    }
}

impl EvidenceWalker<'_> {
    fn offer(&mut self, symbol: SymbolId, rank: Rank, name: impl Into<String>) {
        self.offers
            .entry(symbol)
            .or_default()
            .push((rank, name.into()));
    }

    fn name_params(&mut self, params: &FormalParameters<'_>, names: &[&str]) {
        for (param, name) in params.items.iter().zip(names) {
            if let Some(symbol) = simple_symbol(&param.pattern) {
                self.offer(symbol, Rank::Fixed, *name);
            }
        }
    }

    fn name_first_param(&mut self, argument: Option<&Argument<'_>>, name: &str) {
        let Some(params) = argument.and_then(callback_params) else {
            return;
        };
        self.name_params(params, &[name]);
    }

    fn argument_symbol(&self, argument: Option<&Argument<'_>>) -> Option<SymbolId> {
        match argument? {
            Argument::Identifier(id) => reference_symbol(id, self.scoping),
            _ => None,
        }
    }

    fn subscribe(&mut self, call: &CallExpression<'_>) {
        let kind = string_argument(&call.arguments);
        let handler = call.arguments.get(1);
        self.name_first_param(handler, event_param_name(kind));
        if let Some(symbol) = self.argument_symbol(handler) {
            self.handlers.push(Handler {
                function: symbol,
                name: kind.and_then(handler_name),
                param: event_param_name(kind),
            });
        }
    }

    fn method_callback(&mut self, call: &CallExpression<'_>) {
        let Some(method) = method_name(&call.callee) else {
            return;
        };
        let method = method.as_ref();
        if ELEMENT_CALLBACKS.contains(&method) {
            if let Some(params) = call.arguments.first().and_then(callback_params) {
                self.name_params(params, &ELEMENT_PARAMS);
            }
        } else if REDUCERS.contains(&method) {
            if let Some(params) = call.arguments.first().and_then(callback_params) {
                self.name_params(params, &REDUCER_PARAMS);
            }
        } else if SORTERS.contains(&method) {
            if let Some(params) = call.arguments.first().and_then(callback_params) {
                self.name_params(params, &SORTER_PARAMS);
            }
        } else if method == "then" {
            let settled = member_parts(call.callee.without_parentheses())
                .and_then(|(receiver, _)| settled_value_name(receiver))
                .unwrap_or("result");
            self.name_first_param(call.arguments.first(), settled);
            self.name_first_param(call.arguments.get(1), "err");
        } else if method == "catch" {
            self.name_first_param(call.arguments.first(), "err");
        } else if EVENT_SUBSCRIBERS.contains(&method) {
            self.subscribe(call);
        } else if method == "setItem" && is_storage_call(call, "setItem") {
            self.stored_value(call);
        }
    }

    fn stored_value(&mut self, call: &CallExpression<'_>) {
        let Some(name) = string_argument(&call.arguments).and_then(key_name) else {
            return;
        };
        if let Some(symbol) = self.argument_symbol(call.arguments.get(1)) {
            self.offer(symbol, Rank::Written, name);
        }
    }

    fn timer_callback(&mut self, call: &CallExpression<'_>) {
        let Some(callback) = timer_callback_name(call) else {
            return;
        };
        let first = call.arguments.first();
        if callback == "onFrame" {
            self.name_first_param(first, FRAME_TIME);
        }
        if let Some(symbol) = self.argument_symbol(first) {
            self.offer(symbol, Rank::Role, callback);
            if callback == "onFrame" {
                self.handlers.push(Handler {
                    function: symbol,
                    name: None,
                    param: FRAME_TIME,
                });
            }
        }
    }

    fn record_call(&mut self, callee: &Expression<'_>, arguments: &[Argument<'_>], new: bool) {
        let Expression::Identifier(id) = callee.without_parentheses() else {
            return;
        };
        if let Some(symbol) = reference_symbol(id, self.scoping) {
            self.calls.entry(symbol).or_default().record(arguments, new);
        }
    }

    fn written(&mut self, it: &AssignmentExpression<'_>) {
        let Some((object, key)) = written_property(&it.left) else {
            return;
        };
        if let Some(kind) = on_property_event(&key) {
            self.on_property(kind, &it.right);
            return;
        }
        let Expression::Identifier(id) = it.right.without_parentheses() else {
            return;
        };
        let Some(symbol) = reference_symbol(id, self.scoping) else {
            return;
        };
        if !is_naming_property(&key) {
            return;
        }
        let rank = if is_naming_holder(object) {
            Rank::Holder
        } else {
            Rank::Written
        };
        self.offer(symbol, rank, lower_camel(&key));
    }

    fn on_property(&mut self, kind: &str, value: &Expression<'_>) {
        if let Some(params) = expression_params(value) {
            if let Some(symbol) = params
                .items
                .first()
                .and_then(|param| simple_symbol(&param.pattern))
            {
                self.offer(symbol, Rank::Fixed, event_param_name(Some(kind)));
            }
        }
        if let Expression::Identifier(id) = value.without_parentheses() {
            if let Some(symbol) = reference_symbol(id, self.scoping) {
                self.handlers.push(Handler {
                    function: symbol,
                    name: handler_name(kind),
                    param: event_param_name(Some(kind)),
                });
            }
        }
    }

    fn assigned(&mut self, it: &AssignmentExpression<'_>) {
        let AssignmentTarget::AssignmentTargetIdentifier(id) = &it.left else {
            return;
        };
        let Some(symbol) = reference_symbol(id, self.scoping) else {
            return;
        };
        if let Some(name) = assigned_value_name(&it.right) {
            self.offer(symbol, Rank::Assigned, name);
        }
        self.flows_into((&id.name, symbol), &it.right);
    }

    fn flows_into(&mut self, (target, into): (&str, SymbolId), value: &Expression<'_>) {
        match value.without_parentheses() {
            Expression::Identifier(id) => {
                let Some(symbol) = reference_symbol(id, self.scoping) else {
                    return;
                };
                if is_naming_property(target) {
                    self.offer(symbol, Rank::Written, target);
                }
                self.flows.entry(symbol).or_default().push(into);
            }
            Expression::ConditionalExpression(conditional) => {
                self.flows_into((target, into), &conditional.consequent);
                self.flows_into((target, into), &conditional.alternate);
            }
            _ => {}
        }
    }

    fn typeof_function_check(&mut self, it: &BinaryExpression<'_>) {
        if !it.operator.is_equality() {
            return;
        }
        let tested = match (&it.left, &it.right) {
            (Expression::UnaryExpression(unary), Expression::StringLiteral(kind))
            | (Expression::StringLiteral(kind), Expression::UnaryExpression(unary))
                if unary.operator == UnaryOperator::Typeof && kind.value == "function" =>
            {
                &unary.argument
            }
            _ => return,
        };
        if let Expression::Identifier(id) = tested.without_parentheses() {
            if let Some(symbol) = reference_symbol(id, self.scoping) {
                self.offer(symbol, Rank::Role, CALLBACK);
            }
        }
    }

    fn name_setter(&mut self, params: &FormalParameters<'_>) {
        if let Some(symbol) = params
            .items
            .first()
            .and_then(|param| simple_symbol(&param.pattern))
        {
            self.offer(symbol, Rank::Role, SETTER_VALUE);
        }
    }

    fn finish(mut self) -> Evidence {
        for (symbol, name) in std::mem::take(&mut self.methods).offers() {
            self.offer(symbol, Rank::CallSite, name);
        }
        self.offer_handlers();
        self.offer_call_sites();
        let constructors = self.constructors();
        let mut names = FxHashMap::default();
        for (symbol, offers) in std::mem::take(&mut self.offers) {
            let held = (
                self.defined.contains(&symbol),
                self.params.contains(&symbol),
            );
            if let Some(name) = chosen(offers, held) {
                let name = if constructors.contains(&symbol) {
                    pascal(&name)
                } else {
                    name
                };
                names.insert(symbol, name);
            }
        }
        Evidence {
            names,
            constructors,
            flows: std::mem::take(&mut self.flows),
        }
    }

    fn offer_handlers(&mut self) {
        for handler in std::mem::take(&mut self.handlers) {
            let param = self
                .functions
                .get(&handler.function)
                .and_then(|facts| facts.params.first().copied().flatten());
            if let Some(param) = param {
                self.offer(param, Rank::Role, handler.param);
            }
            if let Some(name) = handler.name {
                self.offer(handler.function, Rank::Role, name);
            }
        }
    }

    fn offer_call_sites(&mut self) {
        let mut found: Vec<(SymbolId, Rank, String)> = Vec::new();
        for (&symbol, facts) in &self.functions {
            let calls = self.calls.get(&symbol);
            let references = self.scoping.get_resolved_reference_ids(symbol).len();
            if let Some(calls) = calls.filter(|calls| calls.calls == references) {
                for (param, arg) in facts.params.iter().zip(&calls.args) {
                    if let (Some(param), ArgName::Named(name)) = (param, arg) {
                        found.push((*param, Rank::CallSite, name.clone()));
                    }
                }
            }
            let constructed = calls.is_some_and(|calls| calls.constructed > 0);
            if let Some(name) = facts.predicate.as_ref().filter(|_| !constructed) {
                found.push((symbol, Rank::Shape, name.clone()));
            }
        }
        for (symbol, rank, name) in found {
            self.offer(symbol, rank, name);
        }
    }

    fn constructors(&self) -> FxHashSet<SymbolId> {
        self.functions
            .keys()
            .copied()
            .filter(|symbol| {
                let calls = self.calls.get(symbol);
                let plain = calls.map_or(0, |calls| calls.calls - calls.constructed);
                let constructed = calls.is_some_and(|calls| calls.constructed > 0);
                plain == 0 && (constructed || self.prototypes.contains(symbol))
            })
            .collect()
    }
}

fn chosen(mut offers: Vec<(Rank, String)>, (defined, param): (bool, bool)) -> Option<String> {
    offers.sort_by_key(|(rank, _)| *rank);
    let mut rest = offers.as_slice();
    while let Some((rank, first)) = rest.first() {
        let count = rest.iter().take_while(|(other, _)| other == rank).count();
        let (group, tail) = rest.split_at(count);
        rest = tail;
        if rank.yields_to(defined, param) {
            continue;
        }
        if !rank.needs_agreement() || group.iter().all(|(_, name)| name == first) {
            return Some(first.clone());
        }
    }
    None
}

impl<'a> VisitJs<'a> for EvidenceWalker<'_> {
    fn visit_call_expression(&mut self, it: &CallExpression<'a>) {
        self.method_callback(it);
        self.timer_callback(it);
        self.record_call(&it.callee, &it.arguments, false);
        if let Expression::StaticMemberExpression(member) = it.callee.without_parentheses() {
            self.methods.call(&member.property.name, &it.arguments);
        }
        walk_js::walk_call_expression(self, it);
    }

    fn visit_new_expression(&mut self, it: &NewExpression<'a>) {
        let is_promise = matches!(it.callee.without_parentheses(), Expression::Identifier(id) if id.name == "Promise");
        if is_promise {
            if let Some(params) = it.arguments.first().and_then(callback_params) {
                self.name_params(params, &EXECUTOR_PARAMS);
            }
        }
        self.record_call(&it.callee, &it.arguments, true);
        walk_js::walk_new_expression(self, it);
    }

    fn visit_binary_expression(&mut self, it: &BinaryExpression<'a>) {
        self.typeof_function_check(it);
        walk_js::walk_binary_expression(self, it);
    }

    fn visit_catch_clause(&mut self, it: &CatchClause<'a>) {
        if let Some(symbol) = it
            .param
            .as_ref()
            .and_then(|param| simple_symbol(&param.pattern))
        {
            self.offer(symbol, Rank::Fixed, "err");
        }
        walk_js::walk_catch_clause(self, it);
    }

    fn visit_for_of_statement(&mut self, it: &ForOfStatement<'a>) {
        if let ForStatementLeft::VariableDeclaration(declaration) = &it.left {
            let symbol = match declaration.declarations.as_slice() {
                [declarator] => simple_symbol(&declarator.id),
                _ => None,
            };
            if let (Some(symbol), Some(name)) = (symbol, element_name(&it.right)) {
                self.offer(symbol, Rank::Fixed, name);
            }
        }
        walk_js::walk_for_of_statement(self, it);
    }

    fn visit_formal_parameters(&mut self, it: &FormalParameters<'a>) {
        for param in &it.items {
            if let Some(symbol) = simple_symbol(&param.pattern) {
                self.params.insert(symbol);
            }
        }
        walk_js::walk_formal_parameters(self, it);
    }

    fn visit_function(&mut self, it: &Function<'a>, flags: ScopeFlags) {
        if it.is_declaration() {
            if let Some(symbol) = it.id.as_ref().and_then(|id| id.symbol_id.get()) {
                self.functions.insert(symbol, function_facts(it));
            }
        }
        walk_js::walk_function(self, it, flags);
    }

    fn visit_variable_declarator(&mut self, it: &VariableDeclarator<'a>) {
        if let (Some(symbol), Some(init)) = (simple_symbol(&it.id), it.init.as_ref()) {
            if let Some(name) = declared_value_name(init) {
                self.offer(symbol, Rank::Fixed, name);
            }
            if is_informative_init(init) {
                self.defined.insert(symbol);
            }
            if let BindingPattern::BindingIdentifier(id) = &it.id {
                self.flows_into((&id.name, symbol), init);
            }
            match init.without_parentheses() {
                Expression::FunctionExpression(function) => {
                    self.functions.insert(symbol, function_facts(function));
                }
                Expression::ArrowFunctionExpression(arrow) => {
                    self.functions.insert(symbol, arrow_facts(arrow));
                }
                _ => {}
            }
        }
        walk_js::walk_variable_declarator(self, it);
    }

    fn visit_assignment_expression(&mut self, it: &AssignmentExpression<'a>) {
        if it.operator == AssignmentOperator::Assign {
            self.written(it);
            self.assigned(it);
            if let (AssignmentTarget::StaticMemberExpression(member), Some(params)) =
                (&it.left, expression_params(&it.right))
            {
                self.methods.define(&member.property.name, params);
            }
        }
        walk_js::walk_assignment_expression(self, it);
    }

    fn visit_object_property(&mut self, it: &ObjectProperty<'a>) {
        let setter_key = matches!(&it.key, PropertyKey::StaticIdentifier(key) if key.name == "set");
        if it.kind == PropertyKind::Set || setter_key {
            if let Some(params) = expression_params(&it.value) {
                self.name_setter(params);
            }
        } else if let (PropertyKey::StaticIdentifier(key), Some(params), false) =
            (&it.key, expression_params(&it.value), it.computed)
        {
            self.methods.define(&key.name, params);
        }
        walk_js::walk_object_property(self, it);
    }

    fn visit_method_definition(&mut self, it: &MethodDefinition<'a>) {
        if it.kind == MethodDefinitionKind::Set {
            self.name_setter(&it.value.params);
        } else if let (MethodDefinitionKind::Method, PropertyKey::StaticIdentifier(key), false) =
            (it.kind, &it.key, it.computed)
        {
            self.methods.define(&key.name, &it.value.params);
        }
        walk_js::walk_method_definition(self, it);
    }

    fn visit_static_member_expression(&mut self, it: &StaticMemberExpression<'a>) {
        if it.property.name == "prototype" {
            if let Expression::Identifier(id) = it.object.without_parentheses() {
                if let Some(symbol) = reference_symbol(id, self.scoping) {
                    self.prototypes.insert(symbol);
                }
            }
        }
        walk_js::walk_static_member_expression(self, it);
    }

    fn visit_export_named_declaration(&mut self, it: &ExportNamedDeclaration<'a>) {
        for specifier in &it.specifiers {
            let ModuleExportName::IdentifierReference(local) = &specifier.local else {
                continue;
            };
            let exported = module_export_name(&specifier.exported);
            let usable = exported != local.name.as_str()
                && looks_human_written(&exported)
                && is_identifier_name(&exported);
            if !usable {
                continue;
            }
            if let Some(symbol) = reference_symbol(local, self.scoping) {
                self.offer(symbol, Rank::Exported, exported.to_string());
            }
        }
        walk_js::walk_export_named_declaration(self, it);
    }
}
