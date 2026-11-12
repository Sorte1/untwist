use oxc_ast::ast::*;
use oxc_ast_visit::{walk_js, VisitJs};
use oxc_semantic::Scoping;
use oxc_span::{GetSpan, Span};
use oxc_syntax::scope::ScopeFlags;
use oxc_syntax::symbol::SymbolId;
use rustc_hash::{FxHashMap, FxHashSet};
use std::rc::Rc;

use super::aliased::{is_definer, member_name, BuiltinAliases, DEFINER, GETTER_DEFINER, LOST};
use super::aliased_solve::{solve_bits, solve_strings};
use crate::stamp::{decoder_index, string_array_unit};

#[derive(Clone, Copy)]
pub(super) enum Target {
    Symbol(SymbolId),
    Pattern(SymbolId),
    Lost,
    Return(Span),
    ThisArgument,
}

#[derive(Default)]
struct Bindings {
    found: Vec<SymbolId>,
}

impl<'a> VisitJs<'a> for Bindings {
    fn visit_binding_identifier(&mut self, it: &BindingIdentifier<'a>) {
        self.found.extend(it.symbol_id.get());
    }

    fn visit_expression(&mut self, _it: &Expression<'a>) {}
}

fn pattern_symbols(pattern: &BindingPattern<'_>) -> Vec<SymbolId> {
    let mut bindings = Bindings::default();
    bindings.visit_binding_pattern(pattern);
    bindings.found
}

pub(super) fn is_machinery(stmt: &Statement<'_>) -> bool {
    let machinery = |name: &str| decoder_index(name).is_some() || string_array_unit(name).is_some();
    match stmt {
        Statement::FunctionDeclaration(function) => function
            .id
            .as_ref()
            .is_some_and(|id| machinery(id.name.as_str())),
        Statement::VariableDeclaration(declaration) => {
            declaration.declarations.iter().any(|declarator| {
                matches!(&declarator.id, BindingPattern::BindingIdentifier(id)
                    if machinery(id.name.as_str()))
            })
        }
        _ => false,
    }
}

struct Flows<'p> {
    flows: Vec<(Target, &'p Expression<'p>)>,
    calls: Vec<(&'p Expression<'p>, Vec<&'p Expression<'p>>, bool)>,
    properties: Vec<&'p Expression<'p>>,
    object_functions: FxHashMap<SymbolId, FxHashMap<String, Span>>,
    parameters: FxHashMap<Span, Vec<SymbolId>>,
    uses_arguments: FxHashSet<Span>,
    callees: FxHashSet<Span>,
    escaped_keys: FxHashSet<(SymbolId, Option<String>)>,
    array_inits: Vec<(SymbolId, &'p Expression<'p>)>,
    member_objects: FxHashSet<Span>,
    escaped: FxHashSet<SymbolId>,
    unknown_strings: Vec<SymbolId>,
    installs: Vec<(&'p Expression<'p>, Option<&'p Expression<'p>>)>,
    seeded: Vec<SymbolId>,
    functions: FxHashMap<SymbolId, Span>,
    sloppy_this: FxHashSet<Span>,
    strict: Vec<bool>,
    function: Vec<Span>,
    function_methods_written: bool,
    prototypes_written: bool,
    plain_classes: FxHashSet<SymbolId>,
    function_method_values: Vec<&'p Expression<'p>>,
}

impl<'p> Flows<'p> {
    fn strict(&self) -> bool {
        self.strict.last().copied().unwrap_or(false)
    }

    fn flow(&mut self, target: Target, expr: &'p Expression<'p>) {
        self.flows.push((target, expr));
    }

    fn seed_pattern(&mut self, pattern: &BindingPattern<'_>) {
        self.seeded.extend(pattern_symbols(pattern));
    }

    fn enter_function(&mut self, span: Span, strict: bool) {
        let inherited = self.strict();
        self.strict.push(inherited || strict);
        self.function.push(span);
    }

    fn leave_function(&mut self) {
        self.strict.pop();
        self.function.pop();
    }
}

struct FlowScan<'s, 'p> {
    scoping: &'s Scoping,
    decoded: &'s FxHashMap<Span, Rc<str>>,
    flows: Flows<'p>,
}

fn has_use_strict(directives: &[Directive<'_>]) -> bool {
    directives
        .iter()
        .any(|directive| directive.directive.as_str() == "use strict")
}

impl<'s, 'p> FlowScan<'s, 'p> {
    fn flow(&mut self, target: Target, expr: &Expression<'p>) {
        let expr: &'p Expression<'p> = self.alloc(expr);
        self.flows.flow(target, expr);
    }

    fn escape(&mut self, expr: &Expression<'p>) {
        self.flow(Target::Lost, expr);
    }

    fn install(&mut self, object: &Expression<'p>, value: Option<&Expression<'p>>) {
        let object: &'p Expression<'p> = self.alloc(object);
        let value: Option<&'p Expression<'p>> = value.map(|value| {
            let value: &'p Expression<'p> = self.alloc(value);
            value
        });
        self.flows.installs.push((object, value));
    }

    fn symbol(&self, id: &IdentifierReference<'_>) -> Option<SymbolId> {
        id.reference_id
            .get()
            .and_then(|reference| self.scoping.get_reference(reference).symbol_id())
    }

    fn assign(&mut self, target: &AssignmentTarget<'p>, value: &Expression<'p>) {
        match target {
            AssignmentTarget::AssignmentTargetIdentifier(id) => match self.symbol(id) {
                Some(symbol) => self.flow(Target::Symbol(symbol), value),
                None => self.escape(value),
            },
            AssignmentTarget::ArrayAssignmentTarget(_)
            | AssignmentTarget::ObjectAssignmentTarget(_) => {
                let mut identifiers = TargetIdentifiers {
                    scoping: self.scoping,
                    found: Vec::new(),
                };
                identifiers.visit_assignment_target(target);
                for symbol in identifiers.found {
                    self.flow(Target::Pattern(symbol), value);
                }
                self.escape(value);
            }
            other => {
                if let Some(member) = other.as_member_expression() {
                    self.install(member.object(), Some(value));
                }
                self.escape(value);
            }
        }
    }

    fn arguments(&mut self, callee: &Expression<'p>, arguments: &[Argument<'p>]) {
        let callee: &'p Expression<'p> = self.alloc(callee);
        let mut values = Vec::new();
        let mut spread = false;
        for argument in arguments {
            match argument {
                Argument::SpreadElement(element) => {
                    spread = true;
                    self.escape(&element.argument);
                }
                other => {
                    if let Some(expr) = other.as_expression() {
                        let expr: &'p Expression<'p> = self.alloc(expr);
                        values.push(expr);
                    }
                }
            }
        }
        self.flows.calls.push((callee, values, spread));
    }

    fn record_parameters(&mut self, span: Span, params: &FormalParameters<'_>) {
        let simple: Option<Vec<SymbolId>> = params
            .items
            .iter()
            .map(|param| match &param.pattern {
                BindingPattern::BindingIdentifier(id) => id.symbol_id.get(),
                _ => None,
            })
            .collect();
        if let (Some(simple), None) = (simple, &params.rest) {
            self.flows.parameters.insert(span, simple);
        }
    }

    fn record_object_functions(&mut self, symbol: SymbolId, init: &Expression<'_>) {
        let Expression::ObjectExpression(object) = init.without_parentheses() else {
            return;
        };
        let mut found = FxHashMap::default();
        for property in &object.properties {
            let ObjectPropertyKind::ObjectProperty(property) = property else {
                continue;
            };
            let key = property
                .key
                .static_name()
                .and_then(|name| name.as_str().map(str::to_string));
            if let (Some(key), Some(span), false) =
                (key, function_value(&property.value), property.computed)
            {
                found.insert(key, span);
            }
        }
        if !found.is_empty() {
            self.flows.object_functions.insert(symbol, found);
        }
    }
}

struct TargetIdentifiers<'s> {
    scoping: &'s Scoping,
    found: Vec<SymbolId>,
}

impl<'a> VisitJs<'a> for TargetIdentifiers<'_> {
    fn visit_identifier_reference(&mut self, it: &IdentifierReference<'a>) {
        let symbol = it
            .reference_id
            .get()
            .and_then(|reference| self.scoping.get_reference(reference).symbol_id());
        self.found.extend(symbol);
    }

    fn visit_expression(&mut self, _it: &Expression<'a>) {}
}

impl<'s, 'p> VisitJs<'p> for FlowScan<'s, 'p> {
    fn visit_program(&mut self, it: &Program<'p>) {
        let strict = untwist_core::semantic::program_is_strict(it);
        self.flows.strict.push(strict);
        walk_js::walk_program(self, it);
    }

    fn visit_statement(&mut self, it: &Statement<'p>) {
        if !is_machinery(it) && crate::array_rotator::rotator_unit(it).is_none() {
            walk_js::walk_statement(self, it);
        }
    }

    fn visit_function(&mut self, it: &Function<'p>, flags: ScopeFlags) {
        let strict = it
            .body
            .as_ref()
            .is_some_and(|body| has_use_strict(&body.directives));
        if let Some(symbol) = it.id.as_ref().and_then(|id| id.symbol_id.get()) {
            if it.r#type == FunctionType::FunctionDeclaration {
                self.flows.functions.insert(symbol, it.span);
            }
        }
        self.record_parameters(it.span, &it.params);
        self.flows.enter_function(it.span, strict);
        walk_js::walk_function(self, it, flags);
        self.flows.leave_function();
    }

    fn visit_arrow_function_expression(&mut self, it: &ArrowFunctionExpression<'p>) {
        let strict = self.flows.strict();
        self.record_parameters(it.span, &it.params);
        self.flows.strict.push(strict);
        self.flows.function.push(it.span);
        if let Some(body) = it.body.as_expression() {
            self.flow(Target::Return(it.span), body);
        }
        walk_js::walk_arrow_function_expression(self, it);
        self.flows.strict.pop();
        self.flows.function.pop();
    }

    fn visit_class(&mut self, it: &Class<'p>) {
        let plain = it.r#type == ClassType::ClassDeclaration && plain_class(it);
        if let Some(symbol) = it.id.as_ref().and_then(|id| id.symbol_id.get()) {
            if plain && !written_after_init(self.scoping, symbol) {
                self.flows.plain_classes.insert(symbol);
            }
        }
        self.flows.strict.push(true);
        walk_js::walk_class(self, it);
        self.flows.strict.pop();
    }

    fn visit_formal_parameter(&mut self, it: &FormalParameter<'p>) {
        self.flows.seed_pattern(&it.pattern);
        walk_js::walk_formal_parameter(self, it);
    }

    fn visit_formal_parameters(&mut self, it: &FormalParameters<'p>) {
        if let Some(rest) = &it.rest {
            self.flows.seed_pattern(&rest.rest.argument);
        }
        walk_js::walk_formal_parameters(self, it);
    }

    fn visit_catch_clause(&mut self, it: &CatchClause<'p>) {
        if let Some(param) = &it.param {
            self.flows.seed_pattern(&param.pattern);
        }
        walk_js::walk_catch_clause(self, it);
    }

    fn visit_for_in_statement(&mut self, it: &ForInStatement<'p>) {
        self.for_left(&it.left);
        walk_js::walk_for_in_statement(self, it);
    }

    fn visit_for_of_statement(&mut self, it: &ForOfStatement<'p>) {
        for symbol in self.for_left(&it.left) {
            self.flow(Target::Pattern(symbol), &it.right);
        }
        self.escape(&it.right);
        walk_js::walk_for_of_statement(self, it);
    }

    fn visit_this_expression(&mut self, it: &ThisExpression) {
        if !self.flows.strict() {
            self.flows.sloppy_this.insert(it.span);
        }
    }

    fn visit_variable_declarator(&mut self, it: &VariableDeclarator<'p>) {
        match (&it.id, &it.init) {
            (BindingPattern::BindingIdentifier(id), Some(init)) => {
                if let Some(symbol) = id.symbol_id.get() {
                    if matches!(
                        init.without_parentheses(),
                        Expression::ArrayExpression(_) | Expression::ObjectExpression(_)
                    ) {
                        let init: &'p Expression<'p> = self.alloc(init);
                        self.flows.array_inits.push((symbol, init));
                    }
                    let class = matches!(init.without_parentheses(),
                        Expression::ClassExpression(class) if plain_class(class));
                    if class && !written_after_init(self.scoping, symbol) {
                        self.flows.plain_classes.insert(symbol);
                    }
                    if let Some(span) = function_value(init).or_else(|| returned_function(init)) {
                        self.flows.functions.insert(symbol, span);
                    }
                    self.record_object_functions(symbol, init);
                    self.flow(Target::Symbol(symbol), init);
                }
            }
            (pattern, Some(init)) => {
                for symbol in pattern_symbols(pattern) {
                    self.flow(Target::Pattern(symbol), init);
                }
            }
            _ => {}
        }
        walk_js::walk_variable_declarator(self, it);
    }

    fn visit_member_expression(&mut self, it: &MemberExpression<'p>) {
        if let Expression::Identifier(id) = it.object() {
            self.flows.member_objects.insert(id.span);
            if !self.flows.callees.contains(&it.span()) {
                if let Some(symbol) = self.symbol(id) {
                    let key = member_key(it, self.decoded);
                    self.flows.escaped_keys.insert((symbol, key));
                }
            }
        }
        walk_js::walk_member_expression(self, it);
    }

    fn visit_identifier_reference(&mut self, it: &IdentifierReference<'p>) {
        if it.name == "arguments" {
            if let Some(function) = self.flows.function.last().copied() {
                self.flows.uses_arguments.insert(function);
            }
        }
        if !self.flows.member_objects.contains(&it.span) && !self.flows.callees.contains(&it.span) {
            if let Some(symbol) = self.symbol(it) {
                self.flows.escaped.insert(symbol);
            }
        }
    }

    fn visit_update_expression(&mut self, it: &UpdateExpression<'p>) {
        if let SimpleAssignmentTarget::AssignmentTargetIdentifier(id) = &it.argument {
            if let Some(symbol) = self.symbol(id) {
                self.flows.unknown_strings.push(symbol);
            }
        }
        self.written_object(it.argument.as_member_expression());
        walk_js::walk_update_expression(self, it);
    }

    fn visit_unary_expression(&mut self, it: &UnaryExpression<'p>) {
        if it.operator == oxc_syntax::operator::UnaryOperator::Delete {
            self.written_object(it.argument.without_parentheses().as_member_expression());
        }
        walk_js::walk_unary_expression(self, it);
    }

    fn visit_assignment_expression(&mut self, it: &AssignmentExpression<'p>) {
        if it.operator != oxc_syntax::operator::AssignmentOperator::Assign {
            if let AssignmentTarget::AssignmentTargetIdentifier(id) = &it.left {
                if let Some(symbol) = self.symbol(id) {
                    self.flows.unknown_strings.push(symbol);
                }
            }
        }
        if let Some(member) = it.left.as_member_expression() {
            self.written_object(Some(member));
            let key = member_key(member, self.decoded);
            let computed = matches!(member, MemberExpression::ComputedMemberExpression(_));
            let may_name = match key.as_deref() {
                Some(name) => is_function_method(name),
                None => computed,
            };
            if may_name {
                if it.operator == oxc_syntax::operator::AssignmentOperator::Assign {
                    let value: &'p Expression<'p> = self.alloc(&it.right);
                    self.flows.function_method_values.push(value);
                } else {
                    self.flows.function_methods_written = true;
                }
            }
        }
        self.assign(&it.left, &it.right);
        walk_js::walk_assignment_expression(self, it);
    }

    fn visit_call_expression(&mut self, it: &CallExpression<'p>) {
        let this_argument = it
            .callee
            .without_parentheses()
            .as_member_expression()
            .and_then(member_name)
            .is_some_and(|name| matches!(name, "call" | "apply"));
        if this_argument {
            if let Some(first) = it.arguments.first().and_then(Argument::as_expression) {
                self.flow(Target::ThisArgument, first);
            }
        }
        let defines = it
            .callee
            .without_parentheses()
            .as_member_expression()
            .and_then(member_name)
            .is_some_and(is_definer);
        if defines {
            if let Some(first) = it.arguments.first().and_then(Argument::as_expression) {
                if reaches_a_prototype(first) {
                    self.flows.prototypes_written = true;
                }
                self.install(first, None);
            }
        }
        let reprotos = it
            .callee
            .without_parentheses()
            .as_member_expression()
            .and_then(member_name)
            .is_some_and(|name| name == "setPrototypeOf");
        if reprotos {
            self.flows.prototypes_written = true;
        }
        self.flows
            .callees
            .insert(it.callee.without_parentheses().span());
        self.arguments(&it.callee, &it.arguments);
        walk_js::walk_call_expression(self, it);
    }

    fn visit_new_expression(&mut self, it: &NewExpression<'p>) {
        self.arguments(&it.callee, &it.arguments);
        walk_js::walk_new_expression(self, it);
    }

    fn visit_return_statement(&mut self, it: &ReturnStatement<'p>) {
        if let Some(argument) = &it.argument {
            match self.flows.function.last().copied() {
                Some(function) => self.flow(Target::Return(function), argument),
                None => self.escape(argument),
            }
        }
        walk_js::walk_return_statement(self, it);
    }

    fn visit_array_expression(&mut self, it: &ArrayExpression<'p>) {
        for element in &it.elements {
            match element {
                ArrayExpressionElement::SpreadElement(spread) => self.escape(&spread.argument),
                other => {
                    if let Some(expr) = other.as_expression() {
                        self.escape(expr);
                    }
                }
            }
        }
        walk_js::walk_array_expression(self, it);
    }

    fn visit_object_property(&mut self, it: &ObjectProperty<'p>) {
        let value: &'p Expression<'p> = self.alloc(&it.value);
        self.flows.properties.push(value);
        walk_js::walk_object_property(self, it);
    }

    fn visit_spread_element(&mut self, it: &SpreadElement<'p>) {
        self.escape(&it.argument);
        walk_js::walk_spread_element(self, it);
    }

    fn visit_yield_expression(&mut self, it: &YieldExpression<'p>) {
        if let Some(argument) = &it.argument {
            self.escape(argument);
        }
        walk_js::walk_yield_expression(self, it);
    }

    fn visit_throw_statement(&mut self, it: &ThrowStatement<'p>) {
        self.escape(&it.argument);
        walk_js::walk_throw_statement(self, it);
    }

    fn visit_property_definition(&mut self, it: &PropertyDefinition<'p>) {
        if let Some(value) = &it.value {
            self.escape(value);
        }
        walk_js::walk_property_definition(self, it);
    }

    fn visit_export_default_declaration(&mut self, it: &ExportDefaultDeclaration<'p>) {
        if let Some(expr) = it.declaration.as_expression() {
            self.escape(expr);
        }
        walk_js::walk_export_default_declaration(self, it);
    }

    fn visit_with_statement(&mut self, it: &WithStatement<'p>) {
        self.escape(&it.object);
        walk_js::walk_with_statement(self, it);
    }

    fn visit_tagged_template_expression(&mut self, it: &TaggedTemplateExpression<'p>) {
        for expr in &it.quasi.expressions {
            self.escape(expr);
        }
        walk_js::walk_tagged_template_expression(self, it);
    }

    fn visit_import_declaration(&mut self, it: &ImportDeclaration<'p>) {
        let mut bindings = Bindings::default();
        bindings.visit_import_declaration(it);
        self.flows.seeded.extend(bindings.found);
    }
}

#[derive(Default)]
struct ReturnsValue {
    found: bool,
}

impl<'a> VisitJs<'a> for ReturnsValue {
    fn visit_function(&mut self, _it: &Function<'a>, _flags: ScopeFlags) {}

    fn visit_arrow_function_expression(&mut self, _it: &ArrowFunctionExpression<'a>) {}

    fn visit_return_statement(&mut self, it: &ReturnStatement<'a>) {
        self.found |= it.argument.is_some();
    }
}

fn plain_class(class: &Class<'_>) -> bool {
    class.heritage.is_none() && !constructor_returns(class)
}

fn constructor_returns(class: &Class<'_>) -> bool {
    class.body.body.iter().any(|element| match element {
        ClassElement::MethodDefinition(method)
            if method.kind == MethodDefinitionKind::Constructor =>
        {
            let mut returns = ReturnsValue::default();
            if let Some(body) = &method.value.body {
                for stmt in &body.statements {
                    returns.visit_statement(stmt);
                }
            }
            returns.found
        }
        _ => false,
    })
}

fn reaches_a_prototype(expr: &Expression<'_>) -> bool {
    let Some(member) = expr.without_parentheses().as_member_expression() else {
        return false;
    };
    let named = !matches!(member, MemberExpression::StaticMemberExpression(_))
        || matches!(member_name(member), Some("prototype" | "__proto__"));
    named || reaches_a_prototype(member.object())
}

impl FlowScan<'_, '_> {
    fn written_object(&mut self, member: Option<&MemberExpression<'_>>) {
        if member.is_some_and(|member| reaches_a_prototype(member.object())) {
            self.flows.prototypes_written = true;
        }
        let Some(Expression::Identifier(id)) = member.map(MemberExpression::object) else {
            return;
        };
        if let Some(symbol) = self.symbol(id) {
            self.flows.escaped.insert(symbol);
        }
    }

    fn for_left(&mut self, left: &ForStatementLeft<'_>) -> Vec<SymbolId> {
        let symbols: Vec<SymbolId> = match left {
            ForStatementLeft::VariableDeclaration(declaration) => declaration
                .declarations
                .iter()
                .flat_map(|declarator| pattern_symbols(&declarator.id))
                .collect(),
            other => match other.as_assignment_target() {
                Some(target) => {
                    let mut identifiers = TargetIdentifiers {
                        scoping: self.scoping,
                        found: Vec::new(),
                    };
                    identifiers.visit_assignment_target(target);
                    identifiers.found
                }
                None => Vec::new(),
            },
        };
        self.flows.seeded.extend(symbols.iter().copied());
        symbols
    }
}

#[derive(Default)]
struct TopReturns<'k, 'a> {
    found: Vec<Option<&'k Expression<'a>>>,
}

impl<'k, 'a> TopReturns<'k, 'a> {
    fn scan(&mut self, statements: &'k [Statement<'a>]) {
        for stmt in statements {
            self.statement(stmt);
        }
    }

    fn statement(&mut self, stmt: &'k Statement<'a>) {
        match stmt {
            Statement::ReturnStatement(ret) => self.found.push(ret.argument.as_ref()),
            Statement::BlockStatement(block) => self.scan(&block.body),
            Statement::IfStatement(node) => {
                self.statement(&node.consequent);
                if let Some(alternate) = &node.alternate {
                    self.statement(alternate);
                }
            }
            Statement::FunctionDeclaration(_)
            | Statement::ExpressionStatement(_)
            | Statement::VariableDeclaration(_)
            | Statement::EmptyStatement(_) => {}
            _ => self.found.push(None),
        }
    }
}

fn unnamed_function(expr: &Expression<'_>) -> Option<Span> {
    match expr.without_parentheses() {
        Expression::FunctionExpression(function) if function.id.is_none() => Some(function.span),
        Expression::ArrowFunctionExpression(arrow) => Some(arrow.span),
        _ => None,
    }
}

fn returned_function(init: &Expression<'_>) -> Option<Span> {
    let Expression::CallExpression(call) = init.without_parentheses() else {
        return None;
    };
    let Expression::FunctionExpression(outer) = call.callee.without_parentheses() else {
        return None;
    };
    if outer.r#async || outer.generator || !call.arguments.is_empty() {
        return None;
    }
    let mut returns = TopReturns::default();
    returns.scan(&outer.body.as_ref()?.statements);
    match returns.found.as_slice() {
        [Some(value)] => unnamed_function(value),
        _ => None,
    }
}

fn function_value(expr: &Expression<'_>) -> Option<Span> {
    match expr.without_parentheses() {
        Expression::FunctionExpression(function) => Some(function.span),
        Expression::ArrowFunctionExpression(arrow) => Some(arrow.span),
        _ => None,
    }
}

pub fn builtin_aliases<'s>(
    program: &Program<'_>,
    scoping: &'s Scoping,
    decoded: FxHashMap<Span, Rc<str>>,
) -> BuiltinAliases<'s> {
    let mut scan = FlowScan {
        scoping,
        decoded: &decoded,
        flows: Flows {
            flows: Vec::new(),
            calls: Vec::new(),
            properties: Vec::new(),
            object_functions: FxHashMap::default(),
            parameters: FxHashMap::default(),
            uses_arguments: FxHashSet::default(),
            callees: FxHashSet::default(),
            escaped_keys: FxHashSet::default(),
            array_inits: Vec::new(),
            member_objects: FxHashSet::default(),
            escaped: FxHashSet::default(),
            unknown_strings: Vec::new(),
            installs: Vec::new(),
            seeded: Vec::new(),
            functions: FxHashMap::default(),
            sloppy_this: FxHashSet::default(),
            strict: Vec::new(),
            function: Vec::new(),
            function_methods_written: false,
            prototypes_written: false,
            plain_classes: FxHashSet::default(),
            function_method_values: Vec::new(),
        },
    };
    scan.visit_program(program);
    let bound = bind_calls(&mut scan.flows, (scoping, &decoded));
    let Flows {
        mut flows,
        array_inits,
        escaped,
        unknown_strings,
        installs,
        seeded,
        functions,
        sloppy_this,
        object_functions,
        function_methods_written,
        prototypes_written,
        plain_classes,
        function_method_values,
        parameters,
        ..
    } = scan.flows;
    flows.extend(bound.flows);
    let seeded: Vec<SymbolId> = seeded
        .into_iter()
        .filter(|symbol| !bound.parameters.contains(symbol))
        .collect();
    let mut aliases = BuiltinAliases {
        scoping,
        values: seeded.into_iter().map(|symbol| (symbol, LOST)).collect(),
        parameters: parameters.into_values().flatten().collect(),
        returns: FxHashMap::default(),
        functions,
        sloppy_this,
        this_bits: 0,
        lost: 0,
        decoded,
        object_functions,
        function_methods_written,
        prototypes_written,
        plain_classes,
        strings: FxHashMap::default(),
        unknown_strings: FxHashSet::default(),
        string_arrays: FxHashMap::default(),
        string_objects: FxHashMap::default(),
        recording: std::cell::Cell::new(false),
        reads: std::cell::RefCell::default(),
    };
    aliases.unknown_strings.extend(unknown_strings);
    aliases
        .unknown_strings
        .extend(aliases.values.keys().copied());
    solve_strings(&mut aliases, &flows, (&array_inits, &escaped));
    solve_bits(&mut aliases, &flows, &installs);
    let definer_bits = DEFINER | GETTER_DEFINER | LOST;
    aliases.function_methods_written |= function_method_values
        .iter()
        .any(|value| aliases.value(value) & definer_bits != 0);
    aliases
}

pub(super) fn written_after_init(scoping: &Scoping, symbol: SymbolId) -> bool {
    scoping
        .get_resolved_reference_ids(symbol)
        .iter()
        .any(|reference| scoping.get_reference(*reference).is_write())
}

pub(super) fn string_array(
    aliases: &BuiltinAliases<'_>,
    init: &Expression<'_>,
) -> Option<Vec<String>> {
    let Expression::ArrayExpression(array) = init.without_parentheses() else {
        return None;
    };
    array
        .elements
        .iter()
        .try_fold(Vec::new(), |found, element| {
            let names = aliases.key_set(element.as_expression()?)?;
            merge_names(found, names)
        })
}

pub(super) fn string_object(
    aliases: &BuiltinAliases<'_>,
    init: &Expression<'_>,
) -> Option<FxHashMap<String, Vec<String>>> {
    let Expression::ObjectExpression(object) = init.without_parentheses() else {
        return None;
    };
    let mut found = FxHashMap::default();
    for property in &object.properties {
        let ObjectPropertyKind::ObjectProperty(property) = property else {
            return None;
        };
        if property.computed || property.kind != PropertyKind::Init {
            return None;
        }
        let key = property.key.static_name()?.as_str()?.to_string();
        if let Some(names) = aliases.key_set(&property.value) {
            found.insert(key, names);
        }
    }
    Some(found)
}

pub(super) fn merge_names(mut left: Vec<String>, right: Vec<String>) -> Option<Vec<String>> {
    for name in right {
        if !left.contains(&name) {
            left.push(name);
        }
    }
    (left.len() <= 64).then_some(left)
}

struct Bound<'p> {
    flows: Vec<(Target, &'p Expression<'p>)>,
    parameters: FxHashSet<SymbolId>,
}

pub(super) fn is_function_method(name: &str) -> bool {
    matches!(name, "apply" | "call" | "bind")
}

fn member_key(member: &MemberExpression<'_>, decoded: &FxHashMap<Span, Rc<str>>) -> Option<String> {
    match member {
        MemberExpression::ComputedMemberExpression(m) => match m.expression.without_parentheses() {
            Expression::StringLiteral(text) => text.value.as_str().map(str::to_string),
            Expression::CallExpression(call) => {
                decoded.get(&call.span).map(|text| text.to_string())
            }
            _ => None,
        },
        other => member_name(other).map(str::to_string),
    }
}

fn callee_function(
    flows: &Flows<'_>,
    (scoping, decoded): (&Scoping, &FxHashMap<Span, Rc<str>>),
    callee: &Expression<'_>,
) -> Option<(Span, Option<SymbolId>, Option<String>)> {
    let symbol_of = |id: &IdentifierReference<'_>| {
        id.reference_id
            .get()
            .and_then(|reference| scoping.get_reference(reference).symbol_id())
    };
    match callee.without_parentheses() {
        Expression::Identifier(id) => {
            let symbol = symbol_of(id)?;
            Some((*flows.functions.get(&symbol)?, Some(symbol), None))
        }
        other => {
            let member = other.as_member_expression()?;
            let Expression::Identifier(id) = member.object().without_parentheses() else {
                return None;
            };
            let symbol = symbol_of(id)?;
            let key = member_key(member, decoded)?;
            let span = *flows.object_functions.get(&symbol)?.get(&key)?;
            Some((span, Some(symbol), Some(key)))
        }
    }
}

fn stays_local(
    flows: &Flows<'_>,
    scoping: &Scoping,
    (symbol, key): (SymbolId, Option<&String>),
) -> bool {
    let sealed = !flows.escaped.contains(&symbol) && !written_after_init(scoping, symbol);
    let read = key.is_some_and(|key| {
        flows.escaped_keys.contains(&(symbol, Some(key.clone())))
            || flows.escaped_keys.contains(&(symbol, None))
    });
    sealed && !read
}

fn bind_calls<'p>(
    flows: &mut Flows<'p>,
    (scoping, decoded): (&Scoping, &FxHashMap<Span, Rc<str>>),
) -> Bound<'p> {
    let mut bound = Bound {
        flows: Vec::new(),
        parameters: FxHashSet::default(),
    };
    let mut local: FxHashSet<Span> = FxHashSet::default();
    let mut nonlocal: FxHashSet<Span> = FxHashSet::default();
    for (symbol, span) in &flows.functions {
        if stays_local(flows, scoping, (*symbol, None)) {
            local.insert(*span);
        } else {
            nonlocal.insert(*span);
        }
    }
    for (symbol, functions) in &flows.object_functions {
        for (key, span) in functions {
            if stays_local(flows, scoping, (*symbol, Some(key))) {
                local.insert(*span);
            } else {
                nonlocal.insert(*span);
            }
        }
    }
    local.retain(|span| !nonlocal.contains(span) && !flows.uses_arguments.contains(span));
    let calls = std::mem::take(&mut flows.calls);
    for (callee, arguments, spread) in calls {
        let params = callee_function(flows, (scoping, decoded), callee)
            .map(|(span, _, _)| span)
            .filter(|span| local.contains(span) && !spread)
            .and_then(|span| flows.parameters.get(&span));
        match params {
            Some(params) => {
                bound.parameters.extend(params.iter().copied());
                for (index, argument) in arguments.into_iter().enumerate() {
                    let target = params
                        .get(index)
                        .map_or(Target::Lost, |p| Target::Symbol(*p));
                    bound.flows.push((target, argument));
                }
            }
            None => {
                for argument in arguments {
                    bound.flows.push((Target::Lost, argument));
                }
            }
        }
    }
    for value in std::mem::take(&mut flows.properties) {
        let local_function = function_value(value).is_some_and(|span| local.contains(&span));
        if !local_function {
            bound.flows.push((Target::Lost, value));
        }
    }
    for (span, params) in &flows.parameters {
        if !local.contains(span) {
            for param in params {
                bound.parameters.remove(param);
            }
        }
    }
    bound
}
