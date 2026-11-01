use oxc_ast::ast::*;
use oxc_ast_visit::{walk_js, VisitJs};
use oxc_semantic::Scoping;
use oxc_span::Span;
use oxc_syntax::operator::AssignmentOperator;
use oxc_syntax::scope::ScopeFlags;
use oxc_syntax::symbol::SymbolId;
use rustc_hash::FxHashMap;
use std::borrow::Cow;

use super::bind::{function_uses_this, reads_arguments};
use untwist_eval::value::JsString;

pub(super) type Resolve<'k> = Option<&'k FxHashMap<Span, &'k JsString>>;

pub(super) const NOT_A_BOOTSTRAP: &str = "";
pub(crate) const TABLE_NOT_LITERAL: &str =
    "an element of the webpack module table is not a plain function";
pub(crate) const TABLE_ESCAPES: &str =
    "the webpack module table is written, or read other than by the bootstrap's own dispatch";
pub(crate) const DISPATCH_SHAPE: &str =
    "the webpack bootstrap does not dispatch modules once, as table[id].call(...), from one require function";
pub(crate) const REQUIRE_ESCAPES: &str =
    "the webpack require function escapes, or its m, c or d property is used other than as the bootstrap sets and calls it";
pub(crate) const MODULE_ID_UNKNOWN: &str =
    "a webpack require call names a module id that is not a literal key of the module table";
pub(crate) const DEFINER_SHAPE: &str =
    "require.d is not one plain function called directly with every argument spelled out";

#[derive(Default)]
pub(super) struct Proven<'a> {
    pub(super) table: Option<SymbolId>,
    pub(super) params: Vec<(SymbolId, (SymbolId, usize))>,
    pub(super) calls: Vec<(SymbolId, &'a CallExpression<'a>)>,
    pub(super) owners: Vec<SymbolId>,
    pub(super) uncalled: Vec<u32>,
}

fn is_function(expr: &Expression<'_>) -> bool {
    matches!(
        expr.without_parentheses(),
        Expression::FunctionExpression(_) | Expression::ArrowFunctionExpression(_)
    )
}

pub(super) fn table_candidate(call: &CallExpression<'_>) -> bool {
    let Some(first) = call.arguments.first().and_then(Argument::as_expression) else {
        return false;
    };
    match first.without_parentheses() {
        Expression::ArrayExpression(array) => array
            .elements
            .iter()
            .any(|element| element.as_expression().is_some_and(is_function)),
        Expression::ObjectExpression(object) => object.properties.iter().any(|property| {
            matches!(property, ObjectPropertyKind::ObjectProperty(p) if is_function(&p.value))
        }),
        _ => false,
    }
}

fn symbol_of(scoping: &Scoping, id: &IdentifierReference<'_>) -> Option<SymbolId> {
    id.reference_id
        .get()
        .and_then(|reference| scoping.get_reference(reference).symbol_id())
}

fn names(scoping: &Scoping, expr: &Expression<'_>, symbol: SymbolId) -> bool {
    matches!(expr.without_parentheses(), Expression::Identifier(id) if symbol_of(scoping, id) == Some(symbol))
}

fn key_name<'a>(member: &MemberExpression<'a>, resolve: Resolve<'_>) -> Option<Cow<'a, str>> {
    match member {
        MemberExpression::StaticMemberExpression(m) => {
            Some(Cow::Borrowed(m.property.name.as_str()))
        }
        MemberExpression::ComputedMemberExpression(m) => match m.expression.without_parentheses() {
            Expression::StringLiteral(text) => Some(Cow::Borrowed(text.value.as_str())),
            Expression::CallExpression(call) => resolve
                .and_then(|map| map.get(&call.span))
                .and_then(|text| text.as_string())
                .map(Cow::Owned),
            _ => None,
        },
        MemberExpression::PrivateFieldExpression(_) => None,
    }
}

fn function_start(expr: &Expression<'_>) -> Option<u32> {
    match expr.without_parentheses() {
        Expression::FunctionExpression(func) => Some(func.span.start),
        _ => None,
    }
}

fn binding_symbol(param: &FormalParameter<'_>) -> Option<SymbolId> {
    match &param.pattern {
        BindingPattern::BindingIdentifier(id) => id.symbol_id.get(),
        _ => None,
    }
}

fn plain_params<'a>(
    scoping: &Scoping,
    expr: &'a Expression<'a>,
) -> Option<&'a FormalParameters<'a>> {
    match expr.without_parentheses() {
        Expression::FunctionExpression(func) => {
            let unnamed = func.id.as_ref().is_none_or(|id| {
                id.symbol_id
                    .get()
                    .is_some_and(|symbol| scoping.get_resolved_reference_ids(symbol).is_empty())
            });
            let plain = unnamed
                && !func.generator
                && !func.r#async
                && func.params.rest.is_none()
                && !reads_arguments(func);
            plain.then_some(&*func.params)
        }
        Expression::ArrowFunctionExpression(arrow) => {
            (arrow.params.rest.is_none() && !arrow.r#async).then_some(&*arrow.params)
        }
        _ => None,
    }
}

struct Table<'a> {
    functions: Vec<&'a FormalParameters<'a>>,
    keys: Vec<String>,
}

fn index_key(value: f64) -> Option<String> {
    let whole = value.fract() == 0.0 && (0.0..f64::from(u32::MAX)).contains(&value);
    whole.then(|| format!("{}", value as u64))
}

fn table_of<'a>(scoping: &Scoping, table: &'a Expression<'a>) -> Option<Table<'a>> {
    let mut out = Table {
        functions: Vec::new(),
        keys: Vec::new(),
    };
    match table.without_parentheses() {
        Expression::ArrayExpression(array) => {
            for (index, element) in array.elements.iter().enumerate() {
                match element {
                    ArrayExpressionElement::Elision(_) => {}
                    ArrayExpressionElement::SpreadElement(_) => return None,
                    other => {
                        out.functions
                            .push(plain_params(scoping, other.as_expression()?)?);
                        out.keys.push(index.to_string());
                    }
                }
            }
        }
        Expression::ObjectExpression(object) => {
            for property in &object.properties {
                let ObjectPropertyKind::ObjectProperty(p) = property else {
                    return None;
                };
                let key = match &p.key {
                    PropertyKey::StaticIdentifier(id) if !p.computed => id.name.to_string(),
                    PropertyKey::StringLiteral(text) => text.value.to_string(),
                    PropertyKey::NumericLiteral(number) => index_key(number.value)?,
                    _ => return None,
                };
                if p.kind != PropertyKind::Init || key == "__proto__" {
                    return None;
                }
                out.functions.push(plain_params(scoping, &p.value)?);
                out.keys.push(key);
            }
        }
        _ => return None,
    }
    Some(out)
}

struct DispatchScan<'s, 'a> {
    scoping: &'s Scoping,
    resolve: Resolve<'s>,
    table: SymbolId,
    found: Vec<&'a CallExpression<'a>>,
    near: usize,
}

struct Mentions<'s> {
    scoping: &'s Scoping,
    symbol: SymbolId,
    found: bool,
}

impl<'a> VisitJs<'a> for Mentions<'_> {
    fn visit_identifier_reference(&mut self, it: &IdentifierReference<'a>) {
        self.found |= symbol_of(self.scoping, it) == Some(self.symbol);
    }
}

fn mentions(scoping: &Scoping, expr: &Expression<'_>, symbol: SymbolId) -> bool {
    let mut scan = Mentions {
        scoping,
        symbol,
        found: false,
    };
    scan.visit_expression(expr);
    scan.found
}

impl<'a> VisitJs<'a> for DispatchScan<'_, 'a> {
    fn visit_function(&mut self, _it: &Function<'a>, _flags: ScopeFlags) {}
    fn visit_arrow_function_expression(&mut self, _it: &ArrowFunctionExpression<'a>) {}
    fn visit_call_expression(&mut self, it: &CallExpression<'a>) {
        if dispatch_callee(self.scoping, self.resolve, &it.callee, self.table) {
            self.found.push(self.alloc(it));
        } else if let Some(member) = it.callee.without_parentheses().as_member_expression() {
            if key_name(member, self.resolve).as_deref() == Some("call")
                && mentions(self.scoping, member.object(), self.table)
            {
                self.near += 1;
            }
        }
        walk_js::walk_call_expression(self, it);
    }
}

fn dispatch_callee(
    scoping: &Scoping,
    resolve: Resolve<'_>,
    callee: &Expression<'_>,
    table: SymbolId,
) -> bool {
    let Some(member) = callee.without_parentheses().as_member_expression() else {
        return false;
    };
    let Expression::ComputedMemberExpression(element) = member.object().without_parentheses()
    else {
        return false;
    };
    names(scoping, &element.object, table) && key_name(member, resolve).as_deref() == Some("call")
}

struct Shape<'a> {
    table: SymbolId,
    require: SymbolId,
    dispatch: &'a CallExpression<'a>,
    aliases: Vec<usize>,
}

fn shape<'a>(
    scoping: &Scoping,
    resolve: Resolve<'_>,
    outer: &'a Function<'a>,
    table: SymbolId,
) -> Result<Shape<'a>, &'static str> {
    let body = outer.body.as_ref().ok_or(DISPATCH_SHAPE)?;
    let mut found = None;
    let mut count = 0;
    let mut near = 0;
    for statement in &body.statements {
        let Statement::FunctionDeclaration(require) = statement else {
            continue;
        };
        let mut scan = DispatchScan {
            scoping,
            resolve,
            table,
            found: Vec::new(),
            near: 0,
        };
        for inner in require.body.iter().flat_map(|body| body.statements.iter()) {
            scan.visit_statement(inner);
        }
        near += scan.near;
        count += scan.found.len();
        if let Some(call) = scan.found.first() {
            found = Some((&**require, *call));
        }
    }
    let (Some((require, dispatch)), 1) = (found, count) else {
        return Err(if count + near == 0 {
            NOT_A_BOOTSTRAP
        } else {
            DISPATCH_SHAPE
        });
    };
    let require_symbol = require
        .id
        .as_ref()
        .and_then(|id| id.symbol_id.get())
        .filter(|symbol| !scoping.symbol_is_mutated(*symbol))
        .ok_or(DISPATCH_SHAPE)?;
    if dispatch.optional {
        return Err(DISPATCH_SHAPE);
    }
    let mut aliases = Vec::new();
    for (index, argument) in dispatch.arguments.iter().enumerate() {
        let expr = argument.as_expression().ok_or(DISPATCH_SHAPE)?;
        if names(scoping, expr, require_symbol) && index > 0 {
            aliases.push(index - 1);
        }
    }
    Ok(Shape {
        table,
        require: require_symbol,
        dispatch,
        aliases,
    })
}

#[derive(Clone, Copy)]
enum Role {
    Table,
    Require(bool),
}

struct Uses<'s, 'a> {
    scoping: &'s Scoping,
    resolve: Resolve<'s>,
    shape: &'s Shape<'a>,
    aliases: &'s [SymbolId],
    keys: &'s [String],
    failed: Option<&'static str>,
    table_stored: bool,
    cache_stored: bool,
    definer: bool,
    helpers: Vec<(Cow<'a, str>, &'a FormalParameters<'a>, Option<u32>)>,
    helper_calls: Vec<(Cow<'a, str>, &'a CallExpression<'a>)>,
    helper_reads: Vec<Cow<'a, str>>,
    forwarded_ids: Vec<SymbolId>,
}

impl<'a> Uses<'_, 'a> {
    fn role(&self, expr: &Expression<'_>) -> Option<Role> {
        let Expression::Identifier(id) = expr.without_parentheses() else {
            return None;
        };
        self.role_of(symbol_of(self.scoping, id)?)
    }

    fn role_of(&self, symbol: SymbolId) -> Option<Role> {
        if symbol == self.shape.table {
            Some(Role::Table)
        } else if symbol == self.shape.require {
            Some(Role::Require(true))
        } else if self.aliases.contains(&symbol) {
            Some(Role::Require(false))
        } else {
            None
        }
    }

    fn fail(&mut self, reason: &'static str) {
        self.failed.get_or_insert(reason);
    }

    fn table_key(&self, expr: &Expression<'_>) -> bool {
        let key = match expr.without_parentheses() {
            Expression::NumericLiteral(number) => index_key(number.value),
            Expression::StringLiteral(text) => Some(text.value.to_string()),
            _ => None,
        };
        key.is_some_and(|key| self.keys.contains(&key))
    }

    fn module_id(&mut self, call: &CallExpression<'a>) {
        let [argument] = call.arguments.as_slice() else {
            return self.fail(MODULE_ID_UNKNOWN);
        };
        let Some(expr) = argument.as_expression() else {
            return self.fail(MODULE_ID_UNKNOWN);
        };
        let value = match expr.without_parentheses() {
            Expression::AssignmentExpression(assign)
                if assign.operator == AssignmentOperator::Assign
                    && assign.left.as_member_expression().is_some_and(|member| {
                        matches!(self.role(member.object()), Some(Role::Require(true)))
                            && key_name(member, self.resolve).as_deref() == Some("s")
                    }) =>
            {
                &assign.right
            }
            other => other,
        };
        if self.table_key(value) {
            return;
        }
        match value.without_parentheses() {
            Expression::Identifier(id) => match symbol_of(self.scoping, id) {
                Some(symbol) => self.forwarded_ids.push(symbol),
                None => self.fail(MODULE_ID_UNKNOWN),
            },
            _ => self.fail(MODULE_ID_UNKNOWN),
        }
    }

    fn forwarded_ids_known(&self) -> bool {
        self.forwarded_ids.iter().all(|symbol| {
            self.helpers.iter().any(|(name, params, _)| {
                let Some(at) = params
                    .items
                    .iter()
                    .position(|param| binding_symbol(param) == Some(*symbol))
                else {
                    return false;
                };
                let calls = || {
                    self.helper_calls
                        .iter()
                        .filter(|(called, _)| called == name)
                };
                let dead = calls().next().is_none();
                let escapes = self.helper_reads.iter().any(|read| read == name);
                !escapes
                    && (dead
                        || (!self.scoping.symbol_is_mutated(*symbol)
                            && calls().all(|(_, call)| {
                                call.arguments
                                    .get(at)
                                    .and_then(Argument::as_expression)
                                    .is_some_and(|expr| self.table_key(expr))
                            })))
            })
        })
    }

    fn visit_member_key(&mut self, member: &MemberExpression<'a>) {
        if let MemberExpression::ComputedMemberExpression(m) = member {
            self.visit_expression(&m.expression);
        }
    }

    fn visit_arguments(&mut self, arguments: &oxc_allocator::Vec<'a, Argument<'a>>) {
        for argument in arguments {
            self.visit_argument(argument);
        }
    }

    fn require_store(&mut self, own: bool, name: &str, assign: &AssignmentExpression<'a>) -> bool {
        if !matches!(name, "m" | "c" | "d") {
            return false;
        }
        if !own || assign.operator != AssignmentOperator::Assign {
            self.fail(REQUIRE_ESCAPES);
            return true;
        }
        match name {
            "m" if !self.table_stored && matches!(self.role(&assign.right), Some(Role::Table)) => {
                self.table_stored = true;
            }
            "c" if !self.cache_stored => {
                self.cache_stored = true;
                self.visit_expression(&assign.right);
            }
            "d" if !self.definer && is_function(&assign.right) => {
                self.definer = true;
                match plain_params(self.scoping, self.alloc(&assign.right)) {
                    Some(params) => {
                        let start = function_start(&assign.right);
                        self.helpers.push((Cow::Borrowed("d"), params, start));
                    }
                    None => self.fail(DEFINER_SHAPE),
                }
                self.visit_expression(&assign.right);
            }
            _ => self.fail(REQUIRE_ESCAPES),
        }
        true
    }
}

impl<'a> VisitJs<'a> for Uses<'_, 'a> {
    fn visit_call_expression(&mut self, it: &CallExpression<'a>) {
        if it.span == self.shape.dispatch.span {
            return;
        }
        if matches!(self.role(&it.callee), Some(Role::Require(_))) {
            self.module_id(it);
            return self.visit_arguments(&it.arguments);
        }
        let Some(member) = it.callee.without_parentheses().as_member_expression() else {
            return walk_js::walk_call_expression(self, it);
        };
        if !matches!(self.role(member.object()), Some(Role::Require(_))) {
            return walk_js::walk_call_expression(self, it);
        }
        match key_name(member, self.resolve).as_deref() {
            None | Some("m" | "call" | "apply" | "bind") => self.fail(REQUIRE_ESCAPES),
            Some("d") => {
                if it
                    .arguments
                    .iter()
                    .any(|argument| argument.as_expression().is_none())
                {
                    self.fail(DEFINER_SHAPE);
                }
                self.helper_calls.push((Cow::Borrowed("d"), self.alloc(it)));
            }
            Some(name) => {
                let name = Cow::Owned(name.to_string());
                self.helper_calls.push((name, self.alloc(it)));
            }
        }
        self.visit_member_key(member);
        self.visit_arguments(&it.arguments);
    }

    fn visit_assignment_expression(&mut self, it: &AssignmentExpression<'a>) {
        let Some(member) = it.left.as_member_expression() else {
            return walk_js::walk_assignment_expression(self, it);
        };
        match self.role(member.object()) {
            Some(Role::Require(own)) => {
                let Some(name) = key_name(member, self.resolve) else {
                    return self.fail(REQUIRE_ESCAPES);
                };
                if let Expression::FunctionExpression(func) = it.right.without_parentheses() {
                    if function_uses_this(func) {
                        self.fail(REQUIRE_ESCAPES);
                    }
                }
                if self.require_store(own, &name, it) {
                    return;
                }
                let helper =
                    own && it.operator == AssignmentOperator::Assign && is_function(&it.right);
                if helper {
                    if let Some(params) = plain_params(self.scoping, self.alloc(&it.right)) {
                        let start = function_start(&it.right);
                        self.helpers
                            .push((Cow::Owned(name.to_string()), params, start));
                    }
                }
                self.visit_member_key(member);
                self.visit_expression(&it.right);
            }
            Some(Role::Table) => self.fail(TABLE_ESCAPES),
            None => walk_js::walk_assignment_expression(self, it),
        }
    }

    fn visit_member_expression(&mut self, it: &MemberExpression<'a>) {
        match self.role(it.object()) {
            Some(Role::Require(_)) => match key_name(it, self.resolve).as_deref() {
                None | Some("m" | "d" | "call" | "apply" | "bind") => self.fail(REQUIRE_ESCAPES),
                Some(name) => {
                    self.helper_reads.push(Cow::Owned(name.to_string()));
                    self.visit_member_key(it);
                }
            },
            Some(Role::Table) => self.fail(TABLE_ESCAPES),
            None => walk_js::walk_member_expression(self, it),
        }
    }

    fn visit_identifier_reference(&mut self, it: &IdentifierReference<'a>) {
        match symbol_of(self.scoping, it).and_then(|symbol| self.role_of(symbol)) {
            Some(Role::Table) => self.fail(TABLE_ESCAPES),
            Some(Role::Require(_)) => self.fail(REQUIRE_ESCAPES),
            None => {}
        }
    }
}

fn register<'a>(
    proven: &mut Proven<'a>,
    params: &FormalParameters<'a>,
    calls: &[&'a CallExpression<'a>],
) {
    let Some(owner) = params.items.iter().find_map(binding_symbol) else {
        return;
    };
    for (index, param) in params.items.iter().enumerate() {
        if let Some(symbol) = binding_symbol(param) {
            proven.params.push((symbol, (owner, index)));
        }
    }
    proven.calls.extend(calls.iter().map(|call| (owner, *call)));
    proven.owners.push(owner);
}

fn outer_plain(call: &CallExpression<'_>, outer: &Function<'_>) -> bool {
    !call.optional
        && !outer.generator
        && !outer.r#async
        && outer.params.rest.is_none()
        && !reads_arguments(outer)
}

pub(super) fn prove<'a>(
    program: &'a Program<'a>,
    scoping: &Scoping,
    resolve: Resolve<'_>,
    call: &'a CallExpression<'a>,
) -> Result<Proven<'a>, &'static str> {
    let Expression::FunctionExpression(outer) = call.callee.without_parentheses() else {
        return Err(NOT_A_BOOTSTRAP);
    };
    let table_symbol = outer
        .params
        .items
        .first()
        .and_then(binding_symbol)
        .ok_or(NOT_A_BOOTSTRAP)?;
    let shape = shape(scoping, resolve, outer, table_symbol)?;
    let table = call
        .arguments
        .first()
        .and_then(Argument::as_expression)
        .and_then(|table| table_of(scoping, table))
        .ok_or(TABLE_NOT_LITERAL)?;
    if scoping.symbol_is_mutated(table_symbol) || !outer_plain(call, outer) {
        return Err(TABLE_ESCAPES);
    }
    let mut aliases = Vec::new();
    for params in &table.functions {
        for index in &shape.aliases {
            if let Some(symbol) = params.items.get(*index).and_then(binding_symbol) {
                if scoping.symbol_is_mutated(symbol) {
                    return Err(REQUIRE_ESCAPES);
                }
                aliases.push(symbol);
            }
        }
    }
    let mut uses = Uses {
        scoping,
        resolve,
        shape: &shape,
        aliases: &aliases,
        keys: &table.keys,
        failed: None,
        table_stored: false,
        cache_stored: false,
        definer: false,
        helpers: Vec::new(),
        helper_calls: Vec::new(),
        helper_reads: Vec::new(),
        forwarded_ids: Vec::new(),
    };
    uses.visit_program(program);
    if let Some(reason) = uses.failed {
        return Err(reason);
    }
    if !uses.forwarded_ids_known() {
        return Err(MODULE_ID_UNKNOWN);
    }
    let mut proven = Proven {
        table: Some(shape.table),
        ..Proven::default()
    };
    for (name, params, start) in &uses.helpers {
        if uses.helper_reads.contains(name) {
            continue;
        }
        let stored_once = uses
            .helpers
            .iter()
            .filter(|(other, _, _)| other == name)
            .count()
            == 1;
        let calls: Vec<&'a CallExpression<'a>> = uses
            .helper_calls
            .iter()
            .filter(|(called, _)| called == name)
            .map(|(_, call)| *call)
            .collect();
        let spread = calls.iter().any(|call| {
            call.arguments
                .iter()
                .any(|argument| argument.as_expression().is_none())
        });
        if spread {
            continue;
        }
        if calls.is_empty() && stored_once {
            proven.uncalled.extend(*start);
        }
        register(&mut proven, params, &calls);
    }
    Ok(proven)
}
