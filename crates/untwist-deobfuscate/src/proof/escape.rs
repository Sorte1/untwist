use oxc_ast::ast::*;
use oxc_ast_visit::{walk_js, VisitJs};
use oxc_semantic::{Scoping, SymbolId};
use oxc_span::{GetSpan, Span};
use oxc_syntax::scope::ScopeFlags;
use rustc_hash::{FxHashMap, FxHashSet};

use untwist_core::semantic::build_scoping;
use untwist_eval::object::PROTOTYPE_LINKS;

use super::code_modules::loads_code_module;
use super::definers::{
    assigns_a_definer, destructures_a_definer, direct_definer, is_definer_read, unkeyed,
};
use super::escape_scan::scan_decoded_text;
use super::global_aliases::{scan_bare_call_globals, scan_global_aliases};
use super::pending::{PendingSite, PreScan};
use super::proxy_objects::scan_proxy_functions;
use super::storage::{scan_storage_objects, Storage, StorageObjects};
use super::text_writes::writes_in_text;
use super::unsafe_bindings::scan_unsafe_bindings;
use super::written::{definer_written_keys, member_written_key, text_template, WrittenKey};
use super::GLOBAL_OBJECTS;

pub const SINK_ROOT_NAMES: [&str; 6] = [
    "eval",
    "Function",
    "setTimeout",
    "setInterval",
    "setImmediate",
    "execScript",
];
pub(super) const MAX_SINK_DEPTH: u32 = 4;

const GLOBAL_KEYS: [&str; 5] = ["__proto__", "top", "parent", "opener", "content"];

pub(super) fn is_code_key(key: &str) -> bool {
    key == "constructor"
        || SINK_ROOT_NAMES.contains(&key)
        || GLOBAL_OBJECTS.contains(&key)
        || GLOBAL_KEYS.contains(&key)
}

pub(crate) fn function_source(texts: impl IntoIterator<Item = Option<String>>) -> Option<String> {
    let texts: Vec<String> = texts.into_iter().collect::<Option<_>>()?;
    let (body, params) = texts
        .split_last()
        .map_or(("", &[][..]), |(body, params)| (body.as_str(), params));
    Some(format!(
        "(function ({}\n) {{\n{body}\n}});",
        params.join(",")
    ))
}

fn builds_a_function(sink: &str, args: &[Argument<'_>]) -> bool {
    matches!(sink, "Function" | "constructor")
        && (args.len() > 1
            || args
                .iter()
                .any(|argument| argument.as_expression().is_none_or(is_function_literal)))
}

fn is_function_literal(expr: &Expression<'_>) -> bool {
    matches!(
        expr.without_parentheses(),
        Expression::FunctionExpression(_) | Expression::ArrowFunctionExpression(_)
    )
}

struct Escape<'s> {
    scoping: &'s Scoping,
    globals: &'s FxHashSet<&'static str>,
    found: Option<Span>,
    matched: FxHashSet<&'static str>,
    resolve: Option<&'s dyn Fn(Span) -> Option<String>>,
    depth: u32,
    this_stack: Vec<bool>,
    class_depth: u32,
    pending_binding: Option<SymbolId>,
    context: &'s EscapeContext,
    pending_exempt: Option<&'s FxHashSet<Span>>,
    pending: Vec<PendingSite>,
    defer_object_operand: bool,
    object_operand: Option<Span>,
    written: Vec<WrittenKey>,
    definer_callee: Option<Span>,
}

impl<'s> Escape<'s> {
    fn flag(&mut self, span: Span) {
        self.found.get_or_insert(span);
    }

    fn flag_name(&mut self, name: &str, span: Span) {
        if let Some(&canonical) = self.globals.get(name) {
            self.matched.insert(canonical);
            self.flag(span);
        }
    }

    fn flag_all(&mut self, span: Span) {
        self.flag(span);
        for &name in self.globals.iter() {
            self.matched.insert(name);
        }
    }

    fn flag_code(&mut self, span: Span) {
        self.flag_all(span);
        self.matched.insert(PROTOTYPE_LINKS);
    }

    fn note_definer_read(&mut self, object: &Expression<'_>, key: &str, span: Span) {
        if self.definer_callee != Some(span) && is_definer_read(self.scoping, object, key) {
            self.flag_code(span);
        }
    }

    fn enter_definer_call(&mut self, call: &CallExpression<'_>) -> Option<Span> {
        let previous = self.definer_callee;
        if let Some((method, callee)) = direct_definer(self.scoping, &call.callee) {
            self.definer_callee = Some(callee);
            if unkeyed(method, call) {
                self.flag_all(call.span);
            }
        }
        previous
    }

    fn flag_sink_escape(&mut self, name: &str, span: Span) {
        if SINK_ROOT_NAMES.contains(&name) {
            self.flag_code(span);
        }
    }

    fn global_name<'x>(&self, expr: &Expression<'x>) -> Option<&'x str> {
        let Expression::Identifier(id) = expr.without_parentheses() else {
            return None;
        };
        let resolved = id
            .reference_id
            .get()
            .and_then(|r| self.scoping.get_reference(r).symbol_id());
        resolved.is_none().then_some(id.name.as_str())
    }

    fn is_builtin(&self, expr: &Expression<'_>) -> bool {
        self.global_name(expr)
            .is_some_and(|name| self.globals.contains(name))
    }

    fn is_global_object(&self, expr: &Expression<'_>) -> bool {
        if matches!(expr.without_parentheses(), Expression::ThisExpression(_)) {
            return self.this_stack.last().copied().unwrap_or(false);
        }
        if let Some(symbol) = self.symbol_of(expr) {
            if self.context.global_aliases.contains(&symbol) {
                return true;
            }
        }
        if self.is_bare_call_to_global(expr) {
            return true;
        }
        self.global_name(expr)
            .is_some_and(|name| GLOBAL_OBJECTS.contains(&name))
    }

    fn is_bare_call_to_global(&self, expr: &Expression<'_>) -> bool {
        let Expression::CallExpression(call) = expr.without_parentheses() else {
            return false;
        };
        self.symbol_of(&call.callee)
            .is_some_and(|symbol| self.context.bare_call_globals.contains(&symbol))
    }

    fn is_program_declared_callee(&self, callee: &Expression<'_>) -> bool {
        match callee.without_parentheses() {
            Expression::Identifier(id) => id
                .reference_id
                .get()
                .and_then(|r| self.scoping.get_reference(r).symbol_id())
                .is_some(),
            Expression::FunctionExpression(_) | Expression::ArrowFunctionExpression(_) => true,
            Expression::StaticMemberExpression(member) => self
                .proxy_functions_of(&member.object)
                .is_some_and(|keys| keys.contains(member.property.name.as_str())),
            Expression::ComputedMemberExpression(member) => {
                let Some(functions) = self.proxy_functions_of(&member.object) else {
                    return false;
                };
                match self.resolve_string_expr(&member.expression) {
                    Some(key) => functions.contains(&key),
                    None => self.is_unresolved_call(&member.expression),
                }
            }
            _ => false,
        }
    }

    fn proxy_functions_of(&self, object: &Expression<'_>) -> Option<&FxHashSet<String>> {
        let Expression::Identifier(id) = object.without_parentheses() else {
            return None;
        };
        let symbol_id = id
            .reference_id
            .get()
            .and_then(|r| self.scoping.get_reference(r).symbol_id())?;
        self.context.proxy_functions.get(&symbol_id)
    }

    fn symbol_of(&self, object: &Expression<'_>) -> Option<SymbolId> {
        let Expression::Identifier(id) = object.without_parentheses() else {
            return None;
        };
        id.reference_id
            .get()
            .and_then(|r| self.scoping.get_reference(r).symbol_id())
    }

    fn pending_call_span(&self, expr: &Expression<'_>) -> Option<Span> {
        let exempt = self.pending_exempt?;
        let Expression::CallExpression(call) = expr.without_parentheses() else {
            return None;
        };
        if exempt.contains(&call.span) {
            return None;
        }
        self.is_program_declared_callee(&call.callee)
            .then_some(call.span)
    }

    fn is_unresolved_call(&self, expr: &Expression<'_>) -> bool {
        self.pending_exempt.is_some()
            && matches!(expr.without_parentheses(), Expression::CallExpression(_))
    }

    fn record_proxy_key_pending(
        &mut self,
        callee: &Expression<'_>,
        call_span: Span,
        args: &[Argument<'_>],
    ) {
        let Expression::ComputedMemberExpression(member) = callee.without_parentheses() else {
            return;
        };
        if self.proxy_functions_of(&member.object).is_none() {
            return;
        }
        if self.resolve_string_expr(&member.expression).is_some() {
            return;
        }
        let Expression::CallExpression(key_call) = member.expression.without_parentheses() else {
            return;
        };
        let key_span = key_call.span;
        let forcing = self.pending_call_span(&member.expression).is_some();
        let object_symbol = self.symbol_of(&member.object);
        let mut strict = self.child_scan();
        strict.visit_arguments(args, false);
        self.pending.extend(strict.pending);
        self.pending.push(PendingSite::ProxyKey {
            call_span,
            key_span,
            object_symbol,
            forcing,
            strict_names: strict.matched,
            strict_span: strict.found,
        });
    }

    fn child_scan(&self) -> Escape<'s> {
        let mut child = Escape::new(self.scoping, self.globals, self.context, self.resolve);
        child.depth = self.depth;
        child.this_stack = self.this_stack.clone();
        child.class_depth = self.class_depth;
        child.pending_exempt = self.pending_exempt;
        child
    }

    fn new(
        scoping: &'s Scoping,
        globals: &'s FxHashSet<&'static str>,
        context: &'s EscapeContext,
        resolve: Option<&'s dyn Fn(Span) -> Option<String>>,
    ) -> Self {
        Escape {
            scoping,
            globals,
            found: None,
            matched: FxHashSet::default(),
            resolve,
            depth: 0,
            this_stack: Vec::new(),
            class_depth: 0,
            pending_binding: None,
            context,
            pending_exempt: None,
            pending: Vec::new(),
            defer_object_operand: false,
            object_operand: None,
            written: Vec::new(),
            definer_callee: None,
        }
    }

    fn is_exempt_sink_root_argument(argument: &Argument<'_>) -> bool {
        let Some(expr) = argument.as_expression() else {
            return false;
        };
        matches!(
            expr.without_parentheses(),
            Expression::Identifier(id) if SINK_ROOT_NAMES.contains(&id.name.as_str())
        )
    }

    fn visit_arguments<'a>(&mut self, args: &[Argument<'a>], exemptible: bool)
    where
        Self: VisitJs<'a>,
    {
        for argument in args {
            if exemptible && Self::is_exempt_sink_root_argument(argument) {
                continue;
            }
            self.visit_argument(argument);
        }
    }

    fn constructor_alias(&self, expr: &Expression<'_>) -> Option<&'static str> {
        let Expression::StaticMemberExpression(member) = expr.without_parentheses() else {
            return None;
        };
        if member.property.name.as_str() != "constructor" {
            return None;
        }
        match member.object.without_parentheses() {
            Expression::StringLiteral(_) => Some("String"),
            _ => None,
        }
    }

    fn on_member(&mut self, object: &Expression<'_>, key: Option<&str>, span: Span) -> bool {
        if self.is_global_object(object) {
            match key {
                Some(name) if self.globals.contains(name) => self.flag_name(name, span),
                None => self.flag_code(span),
                Some(_) => {}
            }
            return true;
        }
        if let Some(name) = self.global_name(object) {
            if self.globals.contains(name) {
                if key.is_none() {
                    self.flag_name(name, span);
                }
                return true;
            }
        }
        if let Some(name) = self.constructor_alias(object) {
            if self.globals.contains(name) {
                self.flag_name(name, span);
                return true;
            }
        }
        false
    }

    fn on_decoded_global_key(&mut self, member: &ComputedMemberExpression<'_>) -> bool {
        let Some(key) = self.pending_template(&member.expression) else {
            return false;
        };
        if !self.is_global_object(&member.object) {
            return false;
        }
        if let Some(resolved) = self.resolve_template(&key) {
            self.on_global_key(&resolved, member.span);
            return true;
        }
        if self.pending_exempt.is_none() {
            return false;
        }
        let forcing = self.template_forcing(&member.expression);
        self.pending.push(PendingSite::GlobalKey {
            outer_span: member.span,
            key,
            forcing,
        });
        true
    }

    fn on_decoded_root_key(&mut self, member: &ComputedMemberExpression<'_>) -> bool {
        if self.is_global_object(&member.object) {
            return false;
        }
        let Some(&root) = self
            .global_name(&member.object)
            .and_then(|name| self.globals.get(name))
        else {
            return false;
        };
        let Some(key) = self.pending_template(&member.expression) else {
            return false;
        };
        if let Some(resolved) = self.resolve_template(&key) {
            if is_code_key(&resolved) {
                self.flag_code(member.span);
            }
            return true;
        }
        if self.pending_exempt.is_none() {
            return false;
        }
        let forcing = self.template_forcing(&member.expression);
        self.pending.push(PendingSite::RootKey {
            outer_span: member.span,
            root,
            key,
            forcing,
        });
        true
    }

    fn resolve_template(&self, template: &WrittenKey) -> Option<String> {
        template.resolved(&|span| self.resolve.and_then(|resolve| resolve(span)))
    }

    fn template_forcing(&self, expr: &Expression<'_>) -> bool {
        match expr.without_parentheses() {
            Expression::BinaryExpression(binary) => {
                self.template_forcing(&binary.left) || self.template_forcing(&binary.right)
            }
            Expression::TemplateLiteral(template) => template
                .expressions
                .iter()
                .any(|part| self.template_forcing(part)),
            other => match self.storage_call(other) {
                Some(call) => {
                    call.arguments
                        .iter()
                        .filter_map(Argument::as_expression)
                        .any(|argument| self.template_forcing(argument))
                        || call
                            .callee
                            .without_parentheses()
                            .as_member_expression()
                            .and_then(|member| match member {
                                MemberExpression::ComputedMemberExpression(computed) => {
                                    Some(self.template_forcing(&computed.expression))
                                }
                                _ => None,
                            })
                            .unwrap_or(false)
                }
                None => self.pending_call_span(other).is_some(),
            },
        }
    }

    fn on_global_key(&mut self, key: &str, span: Span) {
        if is_code_key(key) {
            self.flag_code(span);
        } else {
            self.flag_name(key, span);
        }
    }

    fn unwrap_sequence<'e, 'x>(expr: &'e Expression<'x>) -> &'e Expression<'x> {
        let unparenthesized = expr.without_parentheses();
        if let Expression::SequenceExpression(sequence) = unparenthesized {
            if let Some(last) = sequence.expressions.last() {
                return Self::unwrap_sequence(last);
            }
        }
        unparenthesized
    }

    fn member_parts<'e, 'x>(expr: &'e Expression<'x>) -> Option<(&'e Expression<'x>, &'e str)> {
        match expr.without_parentheses() {
            Expression::StaticMemberExpression(member) => {
                Some((&member.object, member.property.name.as_str()))
            }
            _ => None,
        }
    }

    fn computed_global_object_key<'e>(&self, expr: &Expression<'e>) -> Option<&'e str> {
        let Expression::ComputedMemberExpression(member) = expr.without_parentheses() else {
            return None;
        };
        if !self.is_global_object(&member.object) {
            return None;
        }
        match member.expression.without_parentheses() {
            Expression::StringLiteral(literal) => literal.value.as_str(),
            _ => None,
        }
    }

    fn sink_kind(
        &self,
        callee: &Expression<'_>,
        parts: Option<(&Expression<'_>, &str)>,
    ) -> Option<&'static str> {
        if let Some(name) = self.global_name(callee) {
            if let Some(&sink) = SINK_ROOT_NAMES.iter().find(|&&s| s == name) {
                return Some(sink);
            }
        }
        if let Some((object, key)) = parts {
            if self.is_global_object(object) {
                if let Some(&sink) = SINK_ROOT_NAMES.iter().find(|&&s| s == key) {
                    return Some(sink);
                }
            }
            if key == "constructor" {
                return Some("constructor");
            }
            return None;
        }
        let key = self.computed_global_object_key(callee)?;
        SINK_ROOT_NAMES.iter().find(|&&s| s == key).copied()
    }

    fn is_reflect_indirection(parts: Option<(&Expression<'_>, &str)>) -> bool {
        let Some((object, key)) = parts else {
            return false;
        };
        (key == "construct" || key == "apply")
            && matches!(
                object.without_parentheses(),
                Expression::Identifier(id) if id.name.as_str() == "Reflect"
            )
    }

    fn call_apply_bind_receiver<'e, 'x>(
        parts: Option<(&'e Expression<'x>, &'e str)>,
    ) -> Option<&'e Expression<'x>> {
        let (object, key) = parts?;
        matches!(key, "call" | "apply" | "bind").then_some(object)
    }

    fn scan_arguments_for_sink_escape(&mut self, args: &[Argument<'_>], span: Span) {
        for argument in args {
            if let Some(expr) = argument.as_expression() {
                if let Some(name) = self.global_name(expr) {
                    self.flag_sink_escape(name, span);
                }
            }
        }
    }

    fn resolve_string_expr(&self, expr: &Expression<'_>) -> Option<String> {
        self.resolve_template(&self.text_template(expr)?)
    }

    fn text_template(&self, expr: &Expression<'_>) -> Option<WrittenKey> {
        let storage = Storage {
            scoping: self.scoping,
            objects: &self.context.storage,
        };
        text_template(expr, Some(&storage))
    }

    fn pending_template(&self, expr: &Expression<'_>) -> Option<WrittenKey> {
        self.text_template(expr).filter(WrittenKey::is_pending)
    }

    fn storage_call<'e, 'x>(&self, expr: &'e Expression<'x>) -> Option<&'e CallExpression<'x>> {
        let Expression::CallExpression(call) = expr.without_parentheses() else {
            return None;
        };
        let storage = Storage {
            scoping: self.scoping,
            objects: &self.context.storage,
        };
        storage.is_storage_call(call).then_some(call)
    }

    fn walk_call<'a>(&mut self, it: &CallExpression<'a>)
    where
        Self: VisitJs<'a>,
    {
        definer_written_keys(it, &mut self.written);
        if loads_code_module(it, self.scoping) {
            self.flag_code(it.span);
        }
        let callee = Self::unwrap_sequence(&it.callee);
        let parts = Self::member_parts(callee);
        let handled = self.check_sink(callee, parts, &it.arguments, it.span);
        if !handled && !self.is_builtin(&it.callee) {
            self.visit_expression(&it.callee);
        }
        if Self::is_reflect_indirection(parts) {
            self.scan_arguments_for_sink_escape(&it.arguments, it.span);
        }
        if let Some(receiver) = Self::call_apply_bind_receiver(parts) {
            if let Some(name) = self.global_name(receiver) {
                self.flag_sink_escape(name, it.span);
            }
        }
        let exemptible = self.is_program_declared_callee(callee);
        if exemptible {
            self.record_proxy_key_pending(callee, it.span, &it.arguments);
        }
        self.visit_arguments(&it.arguments, exemptible);
    }

    fn check_sink_pending_fallback(&mut self, first: &Expression<'_>, span: Span) -> bool {
        if self.pending_exempt.is_some() {
            if let Some(text) = self.pending_template(first) {
                let forcing = self.template_forcing(first);
                self.pending.push(PendingSite::Sink {
                    outer_span: span,
                    text,
                    forcing,
                });
                return true;
            }
        }
        self.flag_code(span);
        true
    }

    fn check_sink(
        &mut self,
        callee: &Expression<'_>,
        parts: Option<(&Expression<'_>, &str)>,
        args: &[Argument<'_>],
        span: Span,
    ) -> bool {
        let Some(sink) = self.sink_kind(callee, parts) else {
            return false;
        };
        if builds_a_function(sink, args) {
            self.check_built_function(args, span);
        }
        let Some(first) = args.iter().find_map(Argument::as_expression) else {
            return true;
        };
        if is_function_literal(first) {
            return true;
        }
        let Some(text) = self.resolve_string_expr(first) else {
            return self.check_sink_pending_fallback(first, span);
        };
        self.scan_sink_text(&text, span);
        true
    }

    fn check_built_function(&mut self, args: &[Argument<'_>], span: Span) {
        let texts = args.iter().map(|argument| {
            argument
                .as_expression()
                .and_then(|expr| self.resolve_string_expr(expr))
        });
        match function_source(texts) {
            Some(source) => self.scan_sink_text(&source, span),
            None => self.flag_code(span),
        }
    }

    fn scan_sink_text(&mut self, text: &str, span: Span) {
        if self.depth >= MAX_SINK_DEPTH {
            self.flag_code(span);
            return;
        }
        match scan_decoded_text(text, self.globals, self.resolve, self.depth + 1) {
            Some((found, matched)) => {
                if let Some(sub_span) = found {
                    self.flag(sub_span);
                }
                self.matched.extend(matched);
            }
            None => self.flag_code(span),
        }
    }
}

impl<'a> VisitJs<'a> for Escape<'_> {
    fn visit_call_expression(&mut self, it: &CallExpression<'a>) {
        let previous = self.enter_definer_call(it);
        self.walk_call(it);
        self.definer_callee = previous;
    }

    fn visit_new_expression(&mut self, it: &NewExpression<'a>) {
        let callee = Self::unwrap_sequence(&it.callee);
        let parts = Self::member_parts(callee);
        let handled = self.check_sink(callee, parts, &it.arguments, it.span);
        if !handled && !self.is_builtin(&it.callee) {
            self.visit_expression(&it.callee);
        }
        if Self::is_reflect_indirection(parts) {
            self.scan_arguments_for_sink_escape(&it.arguments, it.span);
        }
        let exemptible = self.is_program_declared_callee(callee);
        if exemptible {
            self.record_proxy_key_pending(callee, it.span, &it.arguments);
        }
        self.visit_arguments(&it.arguments, exemptible);
    }

    fn visit_binary_expression(&mut self, it: &BinaryExpression<'a>) {
        let object_operand = self.defer_object_operand
            && it.operator == BinaryOperator::Instanceof
            && self.global_name(&it.right) == Some("Object");
        if object_operand {
            self.object_operand.get_or_insert(it.right.span());
            self.visit_expression(&it.left);
            return;
        }
        walk_js::walk_binary_expression(self, it);
    }

    fn visit_static_member_expression(&mut self, it: &StaticMemberExpression<'a>) {
        let key = it.property.name.as_str();
        self.note_definer_read(&it.object, key, it.span);
        if key == "constructor" {
            self.flag_all(it.span);
        }
        if !self.on_member(&it.object, Some(key), it.span) {
            self.visit_expression(&it.object);
        }
    }

    fn visit_computed_member_expression(&mut self, it: &ComputedMemberExpression<'a>) {
        let key = match it.expression.without_parentheses() {
            Expression::StringLiteral(literal) => literal.value.as_str(),
            _ => None,
        };
        if let Some(key) = it.static_property_name().and_then(|name| name.as_str()) {
            self.note_definer_read(&it.object, key, it.span);
        }
        if key.is_none() && (self.on_decoded_global_key(it) || self.on_decoded_root_key(it)) {
            self.visit_expression(&it.expression);
            return;
        }
        if !self.on_member(&it.object, key, it.span) {
            self.visit_expression(&it.object);
        }
        self.visit_expression(&it.expression);
    }

    fn visit_identifier_reference(&mut self, it: &IdentifierReference<'a>) {
        let resolved = it
            .reference_id
            .get()
            .and_then(|r| self.scoping.get_reference(r).symbol_id());
        if resolved.is_some() {
            return;
        }
        let name = it.name.as_str();
        if SINK_ROOT_NAMES.contains(&name) {
            self.flag_sink_escape(name, it.span);
            return;
        }
        if self.globals.contains(name) {
            self.flag_name(name, it.span);
        }
    }

    fn visit_program(&mut self, it: &Program<'a>) {
        self.this_stack.push(true);
        walk_js::walk_program(self, it);
        self.this_stack.pop();
    }

    fn visit_variable_declarator(&mut self, it: &VariableDeclarator<'a>) {
        if destructures_a_definer(self.scoping, it) {
            self.flag_code(it.span);
        }
        let prev = self.pending_binding;
        if let Some(init) = &it.init {
            if matches!(
                init.without_parentheses(),
                Expression::FunctionExpression(_)
            ) {
                if let BindingPattern::BindingIdentifier(id) = &it.id {
                    self.pending_binding = id.symbol_id.get();
                }
            }
        }
        walk_js::walk_variable_declarator(self, it);
        self.pending_binding = prev;
    }

    fn visit_class(&mut self, it: &Class<'a>) {
        self.class_depth += 1;
        walk_js::walk_class(self, it);
        self.class_depth -= 1;
    }

    fn visit_function(&mut self, it: &Function<'a>, flags: ScopeFlags) {
        let symbol_id = self
            .pending_binding
            .take()
            .or_else(|| it.id.as_ref().and_then(|id| id.symbol_id.get()));
        let mut flags = flags;
        if it.has_use_strict_directive() {
            flags |= ScopeFlags::StrictMode;
        }
        let is_strict = flags.contains(ScopeFlags::StrictMode) || self.class_depth > 0;
        let is_constructor_only =
            symbol_id.is_some_and(|sid| !self.context.unsafe_bindings.contains(&sid));
        let is_global_this = !is_strict && !is_constructor_only;
        self.this_stack.push(is_global_this);
        walk_js::walk_function(self, it, flags);
        self.this_stack.pop();
    }

    fn visit_assignment_expression(&mut self, it: &AssignmentExpression<'a>) {
        if assigns_a_definer(self.scoping, it) {
            self.flag_code(it.span);
        }
        walk_js::walk_assignment_expression(self, it);
    }

    fn visit_import_expression(&mut self, it: &ImportExpression<'a>) {
        self.flag_code(it.span);
        walk_js::walk_import_expression(self, it);
    }

    fn visit_simple_assignment_target(&mut self, it: &SimpleAssignmentTarget<'a>) {
        if let Some(member) = it.as_member_expression() {
            self.written.extend(member_written_key(member));
        }
        walk_js::walk_simple_assignment_target(self, it);
    }

    fn visit_unary_expression(&mut self, it: &UnaryExpression<'a>) {
        if it.operator == oxc_syntax::operator::UnaryOperator::Delete {
            if let Some(member) = it.argument.without_parentheses().as_member_expression() {
                self.written.extend(member_written_key(member));
            }
        }
        walk_js::walk_unary_expression(self, it);
    }

    fn visit_assignment_target(&mut self, it: &AssignmentTarget<'a>) {
        if let AssignmentTarget::StaticMemberExpression(member) = it {
            if let Some(name) = self.global_name(&member.object) {
                if self.globals.contains(name) {
                    self.flag_name(name, member.span());
                }
            }
        }
        walk_js::walk_assignment_target(self, it);
    }
}

pub struct EscapeScan {
    pub names: FxHashSet<&'static str>,
    pub span: Option<Span>,
    pub object_operand: Option<Span>,
}

pub struct EscapeContext {
    unsafe_bindings: FxHashSet<SymbolId>,
    pub(super) proxy_functions: FxHashMap<SymbolId, FxHashSet<String>>,
    storage: StorageObjects,
    global_aliases: FxHashSet<SymbolId>,
    bare_call_globals: FxHashSet<SymbolId>,
}

pub fn build_escape_context(program: &Program<'_>, scoping: &Scoping) -> EscapeContext {
    EscapeContext {
        unsafe_bindings: scan_unsafe_bindings(program, scoping),
        proxy_functions: scan_proxy_functions(program, scoping),
        storage: scan_storage_objects(program, scoping),
        global_aliases: scan_global_aliases(program, scoping),
        bare_call_globals: scan_bare_call_globals(program, scoping),
    }
}

pub fn scan_escaping_roots_resolved(
    program: &Program<'_>,
    scoping: &Scoping,
    globals: &FxHashSet<&'static str>,
    context: &EscapeContext,
    resolve: &dyn Fn(Span) -> Option<String>,
) -> EscapeScan {
    scan_with_resolver(program, scoping, globals, context, Some(resolve), None)
}

pub fn scan_pending(
    program: &Program<'_>,
    scoping: &Scoping,
    globals: &FxHashSet<&'static str>,
    context: &EscapeContext,
    unit_spans: &FxHashSet<Span>,
) -> PreScan {
    let mut scan = Escape::new(scoping, globals, context, None);
    scan.pending_exempt = Some(unit_spans);
    scan.defer_object_operand = true;
    scan.visit_program(program);
    PreScan {
        names: scan.matched,
        span: scan.found,
        pending: scan.pending,
        object_operand: scan.object_operand,
        written: scan.written,
    }
}

pub(super) fn escape_in_decoded_program(
    program: &Program<'_>,
    globals: &FxHashSet<&'static str>,
    resolve: Option<&dyn Fn(Span) -> Option<String>>,
    depth: u32,
) -> (Option<Span>, FxHashSet<&'static str>) {
    let semantics = build_scoping(program);
    let context = build_escape_context(program, &semantics.scoping);
    let mut sub = Escape::new(&semantics.scoping, globals, &context, resolve);
    sub.depth = depth;
    sub.visit_program(program);
    if writes_in_text(program) {
        sub.matched.extend(globals.iter().copied());
        sub.matched.insert(PROTOTYPE_LINKS);
    }
    (sub.found, sub.matched)
}

fn scan_with_resolver(
    program: &Program<'_>,
    scoping: &Scoping,
    globals: &FxHashSet<&'static str>,
    context: &EscapeContext,
    resolve: Option<&dyn Fn(Span) -> Option<String>>,
    pending_exempt: Option<&FxHashSet<Span>>,
) -> EscapeScan {
    let mut scan = Escape::new(scoping, globals, context, resolve);
    scan.pending_exempt = pending_exempt;
    scan.defer_object_operand = true;
    scan.visit_program(program);
    EscapeScan {
        names: scan.matched,
        span: scan.found,
        object_operand: scan.object_operand,
    }
}
