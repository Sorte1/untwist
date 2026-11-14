use std::borrow::Cow;

use oxc_ast::ast::*;
use oxc_ast_visit::{walk_js, VisitJs};
use oxc_semantic::Scoping;
use oxc_span::Span;
use oxc_str::Ident;
use oxc_syntax::scope::{ScopeFlags, ScopeId};
use oxc_syntax::symbol::SymbolId;
use rustc_hash::{FxHashMap, FxHashSet};
use untwist_core::ast_util::js_text::js_text;
use untwist_core::ast_util::prop::member_parts;
use untwist_core::machinery_name::is_machinery_name;
use untwist_core::semantic::{
    closed_eval_names, eval_payload_names, observers, Observers, Strictness,
};
use untwist_eval::coerce::number_to_string;

use crate::hazards::{is_direct_eval, Hazards};
use crate::oracle::OuterRule;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum InitKind {
    #[default]
    Unknown,
    Absent,
    Identifier,
    Function,
    Class,
    Call,
    New,
    Require,
    Member,
    This,
    Number,
    String,
    Boolean,
    Object,
    Array,
    Regex,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct InitShape {
    pub kind: InitKind,
    pub name: Option<String>,
    pub root: Option<String>,
    pub symbol: Option<SymbolId>,
    pub callee: Option<Box<InitShape>>,
}

impl InitShape {
    fn of(kind: InitKind) -> Self {
        Self {
            kind,
            ..Self::default()
        }
    }

    fn named(kind: InitKind, name: impl Into<String>) -> Self {
        Self {
            kind,
            name: Some(name.into()),
            ..Self::default()
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum BindingRole {
    Unknown,
    Function,
    Class,
    Param,
    CatchParam,
    Variable(InitShape),
    PatternProperty(String),
    PatternElement,
    ImportDefault(String),
    ImportNamespace(String),
    ImportNamed { source: String, imported: String },
}

#[derive(Clone, Debug)]
pub struct BindingInfo {
    pub symbol: SymbolId,
    pub name: String,
    pub span: Span,
    pub scope: ScopeId,
    pub role: BindingRole,
}

#[derive(Clone, Debug)]
pub struct BindingSite {
    pub symbol: SymbolId,
    pub role: BindingRole,
}

#[derive(Default)]
pub struct Collected {
    pub bindings: Vec<BindingInfo>,
    pub by_symbol: FxHashMap<SymbolId, usize>,
    pub sites: Vec<BindingSite>,
    pub free: FxHashMap<ScopeId, FxHashSet<SymbolId>>,
    pub pinned: FxHashSet<SymbolId>,
    pub name_pinned: FxHashSet<SymbolId>,
    pub loop_counters: FxHashMap<SymbolId, usize>,
    pub labels: FxHashSet<String>,
    pub program_references: FxHashSet<String>,
    pub block_function_names: FxHashSet<String>,
    pub eval_names: Vec<String>,
    pub jsx_tags: FxHashSet<SymbolId>,
    pub hazard: Option<&'static str>,
    pub observed: FxHashSet<SymbolId>,
    pub assumed_unobserved: bool,
    pub classic_script: bool,
    pub library: std::sync::Arc<crate::library::LibraryNames>,
    pub(crate) rule: OuterRule,
}

pub const PROTO_KEY: &str = "__proto__";

const OPAQUE_BINDING_NAMES: [&str; 2] = ["arguments", "eval"];

pub fn is_proto_key(key: &PropertyKey<'_>) -> bool {
    matches!(key, PropertyKey::StaticIdentifier(id) if id.name.as_str() == PROTO_KEY)
}

pub fn reference_symbol(id: &IdentifierReference<'_>, scoping: &Scoping) -> Option<SymbolId> {
    id.reference_id
        .get()
        .and_then(|reference_id| scoping.get_reference(reference_id).symbol_id())
}

pub fn module_export_name<'n>(name: &'n ModuleExportName<'_>) -> Cow<'n, str> {
    match name {
        ModuleExportName::IdentifierName(id) => Cow::Borrowed(id.name.as_str()),
        ModuleExportName::IdentifierReference(id) => Cow::Borrowed(id.name.as_str()),
        ModuleExportName::StringLiteral(literal) => js_text(literal.value),
    }
}

pub fn collect(program: &Program<'_>, scoping: &Scoping, rule: OuterRule) -> Collected {
    let mut collector = Collector {
        scoping,
        scopes: Vec::new(),
        role: BindingRole::Unknown,
        loop_depth: 0,
        strict: Strictness::default(),
        recorded: FxHashMap::default(),
        withs: Vec::new(),
        with_observed: FxHashSet::default(),
        hazards: Hazards::default(),
        out: Collected::default(),
    };
    collector.visit_program(program);
    let Collector {
        recorded,
        hazards,
        with_observed,
        mut out,
        ..
    } = collector;
    out.by_symbol = recorded;
    out.hazard = hazards.name();
    out.assumed_unobserved = observers() == Observers::None && with_observed.is_empty();
    if observers() == Observers::None {
        out.observed.clear();
    }
    out.observed.extend(with_observed);
    out.pinned.extend(out.observed.iter().copied());
    if scoping.root_unresolved_references().contains_key("eval") {
        out.eval_names = eval_payload_names(program);
    }
    out.rule = rule;
    if rule.needs_program_references() {
        for symbol in scoping.symbol_ids() {
            out.program_references
                .insert(scoping.symbol_name(symbol).to_string());
        }
    }
    out
}

struct Collector<'s> {
    scoping: &'s Scoping,
    scopes: Vec<ScopeId>,
    role: BindingRole,
    loop_depth: usize,
    strict: Strictness,
    recorded: FxHashMap<SymbolId, usize>,
    withs: Vec<Option<ScopeId>>,
    with_observed: FxHashSet<SymbolId>,
    hazards: Hazards,
    out: Collected,
}

impl Collector<'_> {
    fn push(&mut self, scope: Option<ScopeId>) {
        if let Some(scope) = scope {
            self.scopes.push(scope);
        }
    }

    fn pop(&mut self, scope: Option<ScopeId>) {
        if scope.is_some() {
            self.scopes.pop();
        }
    }

    fn current(&self) -> ScopeId {
        self.scopes
            .last()
            .copied()
            .unwrap_or(self.scoping.root_scope_id())
    }

    fn record(&mut self, id: &BindingIdentifier<'_>, role: BindingRole) {
        let Some(symbol) = id.symbol_id.get() else {
            return;
        };
        self.observe_through_with(symbol);
        if is_machinery_name(id.name.as_str()) || OPAQUE_BINDING_NAMES.contains(&id.name.as_str()) {
            self.out.pinned.insert(symbol);
        }
        self.mark_hoisted_site(symbol);
        self.out.sites.push(BindingSite {
            symbol,
            role: role.clone(),
        });
        if let Some(&index) = self.recorded.get(&symbol) {
            if let Some(existing) = self.out.bindings.get_mut(index) {
                existing.role = role;
            }
            return;
        }
        self.recorded.insert(symbol, self.out.bindings.len());
        self.out.bindings.push(BindingInfo {
            symbol,
            name: id.name.as_str().to_string(),
            span: id.span,
            scope: self.scoping.symbol_scope_id(symbol),
            role,
        });
    }

    fn mark_hoisted_site(&mut self, symbol: SymbolId) {
        let declared = self.scoping.symbol_scope_id(symbol);
        let current = self.current();
        if declared == current || !self.scoping.scope_is_descendant_of(current, declared) {
            return;
        }
        self.mark_free_below(symbol, current, declared);
    }

    fn mark_free_below(&mut self, symbol: SymbolId, from: ScopeId, declared: ScopeId) {
        let scoping = self.scoping;
        for scope in scoping
            .scope_ancestors(from)
            .take_while(|&scope| scope != declared)
        {
            self.out.free.entry(scope).or_default().insert(symbol);
        }
    }

    fn observe_through_with(&mut self, symbol: SymbolId) {
        let Some(&with) = self.withs.last() else {
            return;
        };
        let declared = self.scoping.symbol_scope_id(symbol);
        let inside = with.is_some_and(|with| {
            self.scoping
                .scope_ancestors(declared)
                .any(|scope| scope == with)
        });
        if !inside {
            self.with_observed.insert(symbol);
        }
    }

    fn observe_eval(&mut self, call: &CallExpression<'_>) {
        self.hazards.eval = true;
        let site = self.current();
        let scoping = self.scoping;
        let Some(names) = closed_eval_names(call) else {
            for scope in scoping.scope_ancestors(site) {
                for (_, symbol) in scoping.get_bindings(scope).iter() {
                    self.out.observed.insert(*symbol);
                }
            }
            return;
        };
        for name in names {
            if let Some(symbol) = scoping.find_binding(site, Ident::from(name.as_str())) {
                self.out.observed.insert(symbol);
            }
        }
    }

    fn pin(&mut self, id: Option<&BindingIdentifier<'_>>) {
        if let Some(symbol) = id.and_then(|id| id.symbol_id.get()) {
            self.out.pinned.insert(symbol);
        }
    }

    fn pin_pattern(&mut self, pattern: &BindingPattern<'_>) {
        match pattern {
            BindingPattern::BindingIdentifier(id) => self.pin(Some(id)),
            BindingPattern::ObjectPattern(object) => {
                for property in &object.properties {
                    self.pin_pattern(&property.value);
                }
                if let Some(rest) = &object.rest {
                    self.pin_pattern(&rest.argument);
                }
            }
            BindingPattern::ArrayPattern(array) => {
                for element in array.elements.iter().flatten() {
                    self.pin_pattern(element);
                }
                if let Some(rest) = &array.rest {
                    self.pin_pattern(&rest.argument);
                }
            }
            BindingPattern::AssignmentPattern(assignment) => self.pin_pattern(&assignment.left),
        }
    }

    fn pin_block_functions(&mut self, statements: &[Statement<'_>]) {
        if self.strict.is_strict() {
            return;
        }
        for statement in statements {
            let Some(id) = declared_function(statement).and_then(|function| function.id.as_ref())
            else {
                continue;
            };
            self.pin(Some(id));
            self.out
                .block_function_names
                .insert(id.name.as_str().to_string());
        }
    }

    fn mark_loop_counters(&mut self, init: &ForStatementInit<'_>) {
        let ForStatementInit::VariableDeclaration(declaration) = init else {
            return;
        };
        for declarator in &declaration.declarations {
            let BindingPattern::BindingIdentifier(id) = &declarator.id else {
                continue;
            };
            if !counts_upward(declarator.init.as_ref(), self.scoping) {
                continue;
            }
            if let Some(symbol) = id.symbol_id.get() {
                self.out.loop_counters.insert(symbol, self.loop_depth);
            }
        }
    }
}

fn counts_upward(init: Option<&Expression<'_>>, scoping: &Scoping) -> bool {
    matches!(
        init_shape(init, scoping).kind,
        InitKind::Number | InitKind::Absent
    )
}

impl<'a> VisitJs<'a> for Collector<'_> {
    fn visit_program(&mut self, it: &Program<'a>) {
        let scope = it.scope_id.get();
        self.strict = Strictness::of_program(it);
        self.push(scope);
        walk_js::walk_program(self, it);
        self.pop(scope);
    }

    fn visit_block_statement(&mut self, it: &BlockStatement<'a>) {
        let scope = it.scope_id.get();
        self.push(scope);
        self.pin_block_functions(&it.body);
        walk_js::walk_block_statement(self, it);
        self.pop(scope);
    }

    fn visit_switch_case(&mut self, it: &SwitchCase<'a>) {
        self.pin_block_functions(&it.consequent);
        walk_js::walk_switch_case(self, it);
    }

    fn visit_static_block(&mut self, it: &StaticBlock<'a>) {
        let scope = it.scope_id.get();
        self.push(scope);
        walk_js::walk_static_block(self, it);
        self.pop(scope);
    }

    fn visit_switch_statement(&mut self, it: &SwitchStatement<'a>) {
        let scope = it.scope_id.get();
        self.push(scope);
        walk_js::walk_switch_statement(self, it);
        self.pop(scope);
    }

    fn visit_with_statement(&mut self, it: &WithStatement<'a>) {
        self.hazards.with = true;
        self.visit_expression(&it.object);
        let scope = it.scope_id.get();
        self.push(scope);
        self.withs.push(scope);
        self.visit_statement(&it.body);
        self.withs.pop();
        self.pop(scope);
    }

    fn visit_call_expression(&mut self, it: &CallExpression<'a>) {
        if is_direct_eval(it) {
            self.observe_eval(it);
        }
        walk_js::walk_call_expression(self, it);
    }

    fn visit_labeled_statement(&mut self, it: &LabeledStatement<'a>) {
        self.out.labels.insert(it.label.name.as_str().to_string());
        walk_js::walk_labeled_statement(self, it);
    }

    fn visit_for_in_statement(&mut self, it: &ForInStatement<'a>) {
        let scope = it.scope_id.get();
        self.push(scope);
        walk_js::walk_for_in_statement(self, it);
        self.pop(scope);
    }

    fn visit_for_of_statement(&mut self, it: &ForOfStatement<'a>) {
        let scope = it.scope_id.get();
        self.push(scope);
        walk_js::walk_for_of_statement(self, it);
        self.pop(scope);
    }

    fn visit_for_statement(&mut self, it: &ForStatement<'a>) {
        let scope = it.scope_id.get();
        self.push(scope);
        if let Some(init) = &it.init {
            self.mark_loop_counters(init);
            self.visit_for_statement_init(init);
        }
        if let Some(test) = &it.test {
            self.visit_expression(test);
        }
        if let Some(update) = &it.update {
            self.visit_expression(update);
        }
        self.loop_depth += 1;
        self.visit_statement(&it.body);
        self.loop_depth -= 1;
        self.pop(scope);
    }

    fn visit_catch_clause(&mut self, it: &CatchClause<'a>) {
        let scope = it.scope_id.get();
        self.push(scope);
        if let Some(param) = &it.param {
            let previous = std::mem::replace(&mut self.role, BindingRole::CatchParam);
            self.visit_binding_pattern(&param.pattern);
            self.role = previous;
        }
        self.visit_block_statement(&it.body);
        self.pop(scope);
    }

    fn visit_function(&mut self, it: &Function<'a>, _flags: ScopeFlags) {
        if let Some(id) = &it.id {
            self.record(id, BindingRole::Function);
        }
        let scope = it.scope_id.get();
        let outer_strict = self.strict;
        self.strict = outer_strict.in_function(it);
        self.push(scope);
        self.visit_formal_parameters(&it.params);
        if let Some(body) = &it.body {
            self.visit_function_body(body);
        }
        self.pop(scope);
        self.strict = outer_strict;
    }

    fn visit_arrow_function_expression(&mut self, it: &ArrowFunctionExpression<'a>) {
        let scope = it.scope_id.get();
        let outer_strict = self.strict;
        self.strict = outer_strict.in_arrow(it);
        self.push(scope);
        walk_js::walk_arrow_function_expression(self, it);
        self.pop(scope);
        self.strict = outer_strict;
    }

    fn visit_class(&mut self, it: &Class<'a>) {
        for decorator in &it.decorators {
            self.visit_decorator(decorator);
        }
        if let Some(id) = &it.id {
            self.record(id, BindingRole::Class);
        }
        let scope = it.scope_id.get();
        let outer_strict = self.strict;
        self.strict = outer_strict.in_class();
        self.push(scope);
        if let Some(heritage) = &it.heritage {
            self.visit_expression(&heritage.expression);
        }
        self.visit_class_body(&it.body);
        self.pop(scope);
        self.strict = outer_strict;
    }

    fn visit_formal_parameter(&mut self, it: &FormalParameter<'a>) {
        let previous = std::mem::replace(&mut self.role, BindingRole::Param);
        self.visit_binding_pattern(&it.pattern);
        self.role = previous;
        if let Some(initializer) = &it.initializer {
            self.visit_expression(initializer);
        }
    }

    fn visit_variable_declarator(&mut self, it: &VariableDeclarator<'a>) {
        let role = if matches!(it.id, BindingPattern::BindingIdentifier(_)) {
            BindingRole::Variable(init_shape(it.init.as_ref(), self.scoping))
        } else {
            BindingRole::Unknown
        };
        let previous = std::mem::replace(&mut self.role, role);
        self.visit_binding_pattern(&it.id);
        self.role = previous;
        if let Some(init) = &it.init {
            self.visit_expression(init);
        }
    }

    fn visit_binding_property(&mut self, it: &BindingProperty<'a>) {
        if !it.computed {
            if let PropertyKey::StaticIdentifier(key) = &it.key {
                let role = BindingRole::PatternProperty(key.name.as_str().to_string());
                let previous = std::mem::replace(&mut self.role, role);
                self.visit_binding_pattern(&it.value);
                self.role = previous;
                return;
            }
        }
        let previous = std::mem::replace(&mut self.role, BindingRole::Unknown);
        walk_js::walk_binding_property(self, it);
        self.role = previous;
    }

    fn visit_object_property(&mut self, it: &ObjectProperty<'a>) {
        if it.shorthand && is_proto_key(&it.key) {
            if let Expression::Identifier(id) = it.value.without_parentheses() {
                if let Some(symbol) = reference_symbol(id, self.scoping) {
                    self.out.pinned.insert(symbol);
                }
            }
        }
        walk_js::walk_object_property(self, it);
    }

    fn visit_assignment_target_property_identifier(
        &mut self,
        it: &AssignmentTargetPropertyIdentifier<'a>,
    ) {
        if let Some(symbol) = reference_symbol(&it.binding, self.scoping) {
            self.out.pinned.insert(symbol);
        }
        walk_js::walk_assignment_target_property_identifier(self, it);
    }

    fn visit_array_pattern(&mut self, it: &ArrayPattern<'a>) {
        let previous = std::mem::replace(&mut self.role, BindingRole::Unknown);
        for element in it.elements.iter().flatten() {
            if let BindingPattern::BindingIdentifier(id) = element {
                self.record(id, BindingRole::PatternElement);
            } else {
                self.visit_binding_pattern(element);
            }
        }
        if let Some(rest) = &it.rest {
            self.visit_binding_rest_element(rest);
        }
        self.role = previous;
    }

    fn visit_binding_rest_element(&mut self, it: &BindingRestElement<'a>) {
        let previous = std::mem::replace(&mut self.role, BindingRole::Unknown);
        walk_js::walk_binding_rest_element(self, it);
        self.role = previous;
    }

    fn visit_import_declaration(&mut self, it: &ImportDeclaration<'a>) {
        let source = js_text(it.source.value).into_owned();
        let Some(specifiers) = &it.specifiers else {
            return;
        };
        for specifier in specifiers {
            match specifier {
                ImportDeclarationSpecifier::ImportSpecifier(spec) => {
                    let imported = module_export_name(&spec.imported).to_string();
                    let role = BindingRole::ImportNamed {
                        source: source.clone(),
                        imported,
                    };
                    self.record(&spec.local, role);
                }
                ImportDeclarationSpecifier::ImportDefaultSpecifier(spec) => {
                    self.record(&spec.local, BindingRole::ImportDefault(source.clone()));
                }
                ImportDeclarationSpecifier::ImportNamespaceSpecifier(spec) => {
                    self.record(&spec.local, BindingRole::ImportNamespace(source.clone()));
                }
            }
        }
    }

    fn visit_export_declaration(&mut self, it: &ExportDeclaration<'a>) {
        walk_js::walk_export_declaration(self, it);
        match &it.declaration {
            Declaration::FunctionDeclaration(function) => self.pin(function.id.as_ref()),
            Declaration::ClassDeclaration(class) => self.pin(class.id.as_ref()),
            Declaration::VariableDeclaration(declaration) => {
                for declarator in &declaration.declarations {
                    self.pin_pattern(&declarator.id);
                }
            }
            _ => {}
        }
    }

    fn visit_export_named_declaration(&mut self, it: &ExportNamedDeclaration<'a>) {
        for specifier in &it.specifiers {
            if module_export_name(&specifier.local) != module_export_name(&specifier.exported) {
                continue;
            }
            if let ModuleExportName::IdentifierReference(id) = &specifier.local {
                if let Some(symbol) = reference_symbol(id, self.scoping) {
                    self.out.pinned.insert(symbol);
                }
            }
        }
        walk_js::walk_export_named_declaration(self, it);
    }

    fn visit_jsx_element_name(&mut self, it: &JSXElementName<'a>) {
        if let JSXElementName::IdentifierReference(id) = it {
            if let Some(symbol) = reference_symbol(id, self.scoping) {
                self.out.jsx_tags.insert(symbol);
            }
        }
        walk_js::walk_jsx_element_name(self, it);
    }

    fn visit_binding_identifier(&mut self, it: &BindingIdentifier<'a>) {
        let role = self.role.clone();
        self.record(it, role);
    }

    fn visit_identifier_reference(&mut self, it: &IdentifierReference<'a>) {
        let Some(symbol) = reference_symbol(it, self.scoping) else {
            return;
        };
        self.observe_through_with(symbol);
        let declared = self.scoping.symbol_scope_id(symbol);
        self.mark_free_below(symbol, self.current(), declared);
    }
}

pub fn init_shape(init: Option<&Expression<'_>>, scoping: &Scoping) -> InitShape {
    let Some(expr) = init else {
        return InitShape::of(InitKind::Absent);
    };
    shape_of(expr.without_parentheses(), scoping)
}

fn shape_of(expr: &Expression<'_>, scoping: &Scoping) -> InitShape {
    match expr {
        Expression::Identifier(id) => InitShape {
            kind: InitKind::Identifier,
            name: Some(id.name.as_str().to_string()),
            symbol: reference_symbol(id, scoping),
            ..InitShape::default()
        },
        Expression::FunctionExpression(function) => {
            own_name_shape(InitKind::Function, function.id.as_ref())
        }
        Expression::ArrowFunctionExpression(_) => InitShape::of(InitKind::Function),
        Expression::ClassExpression(class) => own_name_shape(InitKind::Class, class.id.as_ref()),
        Expression::CallExpression(call) => call_shape(call, scoping),
        Expression::NewExpression(new) => callee_shape(InitKind::New, &new.callee, scoping),
        Expression::ThisExpression(_) => InitShape::named(InitKind::This, "this"),
        Expression::NumericLiteral(literal) => {
            InitShape::named(InitKind::Number, number_to_string(literal.value))
        }
        Expression::StringLiteral(literal) => {
            InitShape::named(InitKind::String, js_text(literal.value))
        }
        Expression::BooleanLiteral(_) => InitShape::of(InitKind::Boolean),
        Expression::ObjectExpression(_) => InitShape::of(InitKind::Object),
        Expression::ArrayExpression(_) => InitShape::of(InitKind::Array),
        Expression::RegExpLiteral(_) => InitShape::of(InitKind::Regex),
        other => member_shape(other),
    }
}

fn own_name_shape(kind: InitKind, id: Option<&BindingIdentifier<'_>>) -> InitShape {
    InitShape {
        kind,
        name: id.map(|id| id.name.as_str().to_string()),
        symbol: id.and_then(|id| id.symbol_id.get()),
        ..InitShape::default()
    }
}

fn member_shape(expr: &Expression<'_>) -> InitShape {
    let Some((object, property)) = member_parts(expr) else {
        return InitShape::of(InitKind::Unknown);
    };
    InitShape {
        kind: InitKind::Member,
        name: Some(property.into_owned()),
        root: member_root(object),
        ..InitShape::default()
    }
}

fn call_shape(call: &CallExpression<'_>, scoping: &Scoping) -> InitShape {
    let callee = Box::new(shape_of(call.callee.without_parentheses(), scoping));
    if let Some(source) = require_source(call) {
        return InitShape {
            kind: InitKind::Require,
            name: Some(source),
            callee: Some(callee),
            ..InitShape::default()
        };
    }
    InitShape {
        kind: InitKind::Call,
        name: callee.name.clone(),
        root: callee.root.clone(),
        symbol: callee.symbol,
        callee: Some(callee),
    }
}

fn callee_shape(kind: InitKind, callee: &Expression<'_>, scoping: &Scoping) -> InitShape {
    let callee = Box::new(shape_of(callee.without_parentheses(), scoping));
    InitShape {
        kind,
        name: callee.name.clone(),
        root: callee.root.clone(),
        symbol: callee.symbol,
        callee: Some(callee),
    }
}

fn member_root(expr: &Expression<'_>) -> Option<String> {
    let expr = expr.without_parentheses();
    if let Expression::Identifier(id) = expr {
        return Some(id.name.as_str().to_string());
    }
    let (object, _) = member_parts(expr)?;
    member_root(object)
}

fn require_source(call: &CallExpression<'_>) -> Option<String> {
    let Expression::Identifier(callee) = call.callee.without_parentheses() else {
        return None;
    };
    if callee.name.as_str() != "require" || call.arguments.len() != 1 {
        return None;
    }
    let Argument::StringLiteral(literal) = &call.arguments[0] else {
        return None;
    };
    Some(js_text(literal.value).into_owned())
}

fn declared_function<'s, 'a>(statement: &'s Statement<'a>) -> Option<&'s Function<'a>> {
    match statement {
        Statement::FunctionDeclaration(function) => Some(function),
        Statement::LabeledStatement(labeled) => declared_function(&labeled.body),
        _ => None,
    }
}
