use oxc_ast::ast::*;
use oxc_ast_visit::{walk_js, VisitJs};
use oxc_span::{GetSpan, Span};
use oxc_syntax::scope::ScopeFlags;
use oxc_syntax::symbol::SymbolId;

use super::live::opaque_branch;
use super::sites::Sites;
use super::AKey;
use oxc_syntax::operator::BinaryOperator;
use untwist_core::semantic::{observers, Observers};
use untwist_eval::coerce::number_to_string;

#[derive(Default)]
struct ReturnExprs<'a> {
    exprs: Vec<&'a Expression<'a>>,
}

impl<'a> VisitJs<'a> for ReturnExprs<'a> {
    fn visit_function(&mut self, _it: &Function<'a>, _flags: ScopeFlags) {}
    fn visit_arrow_function_expression(&mut self, _it: &ArrowFunctionExpression<'a>) {}
    fn visit_class(&mut self, _it: &Class<'a>) {}
    fn visit_if_statement(&mut self, it: &IfStatement<'a>) {
        match opaque_branch(it) {
            Some(taken) => {
                if let Some(branch) = taken {
                    self.visit_statement(branch);
                }
            }
            None => walk_js::walk_if_statement(self, it),
        }
    }
    fn visit_return_statement(&mut self, it: &ReturnStatement<'a>) {
        if let Some(argument) = &it.argument {
            self.exprs.push(self.alloc(argument));
        }
        walk_js::walk_return_statement(self, it);
    }
}

pub(super) const VALUE_DEPTH: u8 = 6;

const VALUE_ENUMERATORS: [&str; 3] = ["values", "entries", "getOwnPropertyDescriptors"];

const VALUE_KEYED: [&str; 5] = [
    "get",
    "getOwnPropertyDescriptor",
    "set",
    "defineProperty",
    "deleteProperty",
];

const OWN_WRITERS: [&str; 2] = ["__defineGetter__", "__defineSetter__"];

const BATCH_WRITERS: [&str; 2] = ["assign", "defineProperties"];

enum KeyedArgument<'k, 'a> {
    Given(&'k Expression<'a>),
    Absent,
    Spread,
}

fn keyed_argument<'k, 'a>(call: &'k CallExpression<'a>, at: usize) -> KeyedArgument<'k, 'a> {
    if call.arguments.iter().take(at + 1).any(Argument::is_spread) {
        return KeyedArgument::Spread;
    }
    match call.arguments.get(at).and_then(Argument::as_expression) {
        Some(expr) => KeyedArgument::Given(expr),
        None => KeyedArgument::Absent,
    }
}

fn open(key: &AKey<'_>) -> bool {
    matches!(key, AKey::Pending(_) | AKey::Filtered(_) | AKey::Unknown) && dynamic_paths()
}

fn dynamic_paths() -> bool {
    observers() != Observers::None
}

fn names_one_of(keys: &[AKey<'_>], list: &[&str]) -> bool {
    keys.iter()
        .any(|key| matches!(key, AKey::Named(name) if list.contains(&name.as_ref())))
}

impl<'a> Sites<'_, 'a> {
    pub(super) fn watching(&self, span: Span) -> bool {
        !self.watch.names.is_empty() && self.out.reach.is_none() && !self.in_machinery(span)
    }

    pub(super) fn watched_name(&self, name: &str) -> bool {
        self.watch.has_prop(name)
    }

    pub(super) fn hit(&mut self, span: Span) {
        self.out.reach.get_or_insert(span);
    }

    fn hit_dynamic(&mut self, span: Span) {
        if dynamic_paths() {
            self.hit(span);
        }
    }

    fn may_hit(&self, key: &AKey<'_>) -> bool {
        match key {
            AKey::Named(name) => self.watched_name(name),
            AKey::Index => false,
            AKey::Filtered(filter) => self.filter_hits(*filter),
            AKey::Pending(_) | AKey::Unknown => dynamic_paths(),
        }
    }

    fn reach_base(&mut self, base: &Expression<'a>, span: Span) {
        if let Some(name) = self.ambient_base(base) {
            self.out.ambient.push((span, name, false));
        } else if !self.proven_non_global(base) && !self.watched_element(base) {
            self.hit(span);
        }
    }

    pub(super) fn ambient_base(&self, base: &Expression<'a>) -> Option<&'a str> {
        let Expression::Identifier(id) = base.without_parentheses() else {
            return None;
        };
        let name = id.name.as_str();
        (self.fresh.symbol(id).is_none() && super::source::is_ambient_intrinsic(name))
            .then_some(name)
    }

    pub(super) fn holds_array(&self, expr: &Expression<'a>) -> bool {
        let inner = expr.without_parentheses();
        match inner {
            Expression::Identifier(id) => self
                .fresh
                .literal_init(id)
                .is_some_and(|init| self.is_array_result(init)),
            other => self.is_array_result(other),
        }
    }

    fn is_array_result(&self, expr: &Expression<'a>) -> bool {
        let Expression::CallExpression(call) = expr.without_parentheses() else {
            return false;
        };
        let Expression::Identifier(id) = call.callee.without_parentheses() else {
            return false;
        };
        let Some(symbol) = self.fresh.symbol(id) else {
            return false;
        };
        self.watch.has_fn(symbol)
            || (call.arguments.is_empty()
                && crate::stamp::string_array_unit(id.name.as_str()).is_some()
                && self.fresh.bindings.is_array_function(symbol))
    }

    fn ambient_creation(&self, expr: &Expression<'a>) -> bool {
        if !self.extended.get() {
            return false;
        }
        let Expression::CallExpression(call) = expr.without_parentheses() else {
            return false;
        };
        let Some(member) = call.callee.without_parentheses().as_member_expression() else {
            return false;
        };
        let Expression::Identifier(root) = member.object().without_parentheses() else {
            return false;
        };
        if call.optional
            || member.optional()
            || root.name != "Object"
            || self.fresh.symbol(root).is_some()
        {
            return false;
        }
        let created =
            matches!(self.member_keys(member).as_slice(), [AKey::Named(name)] if name == "create");
        if created {
            self.created.borrow_mut().push(call.span);
        }
        created
    }

    fn resolvable_param(
        &self,
        id: &IdentifierReference<'a>,
    ) -> Option<(SymbolId, SymbolId, usize)> {
        let symbol = self.fresh.symbol(id)?;
        let bindings = self.fresh.bindings;
        let (of, index) = bindings.param_of(symbol)?;
        bindings
            .never_reassigned(symbol)
            .then_some((symbol, of, index))
    }

    pub(super) fn proven_non_global(&self, expr: &Expression<'a>) -> bool {
        self.either_pass(|| self.non_global(expr, VALUE_DEPTH))
    }

    pub(super) fn either_pass(&self, judge: impl Fn() -> bool) -> bool {
        if self.extended.get() || judge() {
            return true;
        }
        self.extended.set(true);
        let found = judge();
        self.extended.set(false);
        found
    }

    pub(super) fn non_global(&self, expr: &Expression<'a>, depth: u8) -> bool {
        if depth == 0 {
            return false;
        }
        let inner = expr.without_parentheses();
        if self.fresh.is_fresh(inner)
            || self.is_array_result(inner)
            || self.collection_primitive(inner)
            || self.ambient_creation(inner)
        {
            return true;
        }
        match inner {
            Expression::Identifier(id) => {
                if let Some(arg) = self.substituted(id) {
                    return self.non_global(arg, depth - 1);
                }
                self.identifier_non_global(id, depth)
            }
            Expression::CallExpression(call) => self.call_non_global(call, depth),
            Expression::StaticMemberExpression(member) => {
                member.property.name == "length" && self.fresh.holds_fresh_sequence(&member.object)
            }
            _ => false,
        }
    }

    fn call_non_global(&self, call: &CallExpression<'a>, depth: u8) -> bool {
        if depth == 0 {
            return false;
        }
        if self.resolve.is_some_and(|map| map.contains_key(&call.span)) {
            return true;
        }
        let Some(func) = self.resolve_called_function(&call.callee) else {
            return false;
        };
        self.returns_non_global(func, call, depth - 1)
    }

    pub(super) fn returned_exprs(&self, func: &Function<'a>) -> Option<Vec<&'a Expression<'a>>> {
        if func.generator || func.r#async {
            return None;
        }
        let body = func.body.as_ref()?;
        let mut scan = ReturnExprs::default();
        for statement in &body.statements {
            scan.visit_statement(statement);
        }
        Some(scan.exprs)
    }

    fn returns_non_global(
        &self,
        func: &Function<'a>,
        call: &CallExpression<'a>,
        depth: u8,
    ) -> bool {
        let Some(exprs) = self.returned_exprs(func) else {
            return false;
        };
        self.with_params(func, call, || {
            exprs.iter().all(|expr| self.non_global(expr, depth))
        })
        .unwrap_or(false)
    }

    fn identifier_non_global(&self, id: &IdentifierReference<'a>, depth: u8) -> bool {
        if self.identifier_shown_non_global(id, depth) {
            return true;
        }
        self.extended.get()
            && self
                .fresh
                .symbol(id)
                .is_some_and(|symbol| self.fresh.bindings.holds_dispatched_object(symbol))
    }

    fn identifier_shown_non_global(&self, id: &IdentifierReference<'a>, depth: u8) -> bool {
        if self
            .fresh
            .literal_init(id)
            .is_some_and(|init| self.is_array_result(init) || self.ambient_creation(init))
        {
            return true;
        }
        let Some((symbol, func, index)) = self.resolvable_param(id) else {
            return false;
        };
        let memo = (symbol, self.extended.get());
        if let Some(known) = self.param_values.borrow().get(&memo) {
            return *known;
        }
        self.param_values.borrow_mut().insert(memo, false);
        let known =
            self.fresh
                .bindings
                .call_sites(func)
                .all(|call| match call.arguments.get(index) {
                    None => true,
                    Some(argument) => argument
                        .as_expression()
                        .is_some_and(|value| self.site_non_global(value, depth - 1)),
                });
        self.param_values.borrow_mut().insert(memo, known);
        known
    }

    fn site_non_global(&self, expr: &Expression<'a>, depth: u8) -> bool {
        if depth == 0 {
            return false;
        }
        let inner = expr.without_parentheses();
        if self.fresh.is_fresh(inner) || self.is_array_result(inner) || self.ambient_creation(inner)
        {
            return true;
        }
        match inner {
            Expression::Identifier(id) => self.identifier_non_global(id, depth),
            _ => false,
        }
    }

    fn symbol_valued(&self, expr: &Expression<'a>) -> bool {
        let Some(member) = expr.without_parentheses().as_member_expression() else {
            return false;
        };
        let Expression::Identifier(id) = member.object().without_parentheses() else {
            return false;
        };
        id.name == "Symbol" && self.fresh.symbol(id).is_none() && self.symbol_safe
    }

    pub(super) fn key_hits(&self, expr: &Expression<'a>) -> bool {
        let keys = self.key_set(expr);
        keys.iter().any(|key| self.may_hit(key)) && self.refined_hit(expr)
    }

    fn refined_hit(&self, expr: &Expression<'a>) -> bool {
        let inner = expr.without_parentheses();
        if self.symbol_valued(inner) {
            return false;
        }
        match inner {
            Expression::Identifier(id) if self.substituted(id).is_none() => {
                self.param_key_hits(id, VALUE_DEPTH).unwrap_or(true)
            }
            Expression::BinaryExpression(binary) => self.sum_key_hits(binary).unwrap_or(true),
            _ => true,
        }
    }

    fn sum_key_hits(&self, binary: &BinaryExpression<'a>) -> Option<bool> {
        if binary.operator != BinaryOperator::Addition {
            return None;
        }
        let digits = |side: &Expression<'a>| match side.without_parentheses() {
            Expression::NumericLiteral(number) => Some(number_to_string(number.value)),
            _ => None,
        };
        let numeric = |name: &str| name == "NaN" || name == "Infinity";
        if let Some(text) = digits(&binary.right) {
            let ends = |name: &&str| numeric(name) || name.ends_with(text.as_str());
            return Some(self.watch.props.iter().any(ends));
        }
        let text = digits(&binary.left)?;
        let starts = |name: &&str| numeric(name) || name.starts_with(text.as_str());
        Some(self.watch.props.iter().any(starts))
    }

    fn param_key_hits(&self, id: &IdentifierReference<'a>, depth: u8) -> Option<bool> {
        let (symbol, func, index) = self.resolvable_param(id)?;
        if let Some(known) = self.param_keys.borrow().get(&symbol) {
            return Some(*known);
        }
        self.param_keys.borrow_mut().insert(symbol, true);
        let hit =
            depth == 0
                || self.fresh.bindings.call_sites(func).any(|call| {
                    match call.arguments.get(index) {
                        None => self.watched_name("undefined"),
                        Some(argument) => argument
                            .as_expression()
                            .is_none_or(|value| self.site_key_hits(value, depth - 1)),
                    }
                });
        self.param_keys.borrow_mut().insert(symbol, hit);
        Some(hit)
    }

    fn site_key_hits(&self, expr: &Expression<'a>, depth: u8) -> bool {
        let inner = expr.without_parentheses();
        if self.symbol_valued(inner) {
            return false;
        }
        if let Expression::Identifier(id) = inner {
            if let Some(hit) = self.param_key_hits(id, depth) {
                return hit;
            }
        }
        if let Expression::BinaryExpression(binary) = inner {
            if let Some(hit) = self.sum_key_hits(binary) {
                return hit;
            }
        }
        self.key_set(inner).iter().any(|key| self.may_hit(key))
    }

    pub(super) fn reach_member(&mut self, member: &MemberExpression<'a>, keys: &[AKey<'a>]) {
        if self.watch.names.is_empty() || self.out.reach.is_some() {
            return;
        }
        let candidate = match member {
            MemberExpression::StaticMemberExpression(m) => self.watched_name(&m.property.name),
            MemberExpression::ComputedMemberExpression(m) => {
                keys.iter().any(|key| self.may_hit(key)) && self.refined_hit(&m.expression)
            }
            MemberExpression::PrivateFieldExpression(_) => false,
        };
        if candidate && !self.in_machinery(member.span()) {
            self.reach_base(member.object(), member.span());
        }
    }

    pub(super) fn reach_enumerated(&mut self, base: &Expression<'a>, span: Span) {
        if dynamic_paths() && self.watching(span) {
            self.reach_base(base, span);
        }
    }

    pub(super) fn reach_call(
        &mut self,
        call: &CallExpression<'a>,
        member: &MemberExpression<'a>,
        keys: &[AKey<'a>],
    ) {
        if !self.watching(call.span) || self.table_dispatch(&call.callee) {
            return;
        }
        let unknown = keys.iter().any(open);
        if unknown || (dynamic_paths() && names_one_of(keys, &VALUE_ENUMERATORS)) {
            self.reach_arguments(call);
        }
        if dynamic_paths() && names_one_of(keys, &["stringify"]) && call.arguments.len() > 1 {
            if let Some(expr) = call.arguments.first().and_then(Argument::as_expression) {
                self.reach_enumerated(expr, call.span);
            }
        }
        if unknown || names_one_of(keys, &VALUE_KEYED) {
            let base = call.arguments.first().and_then(Argument::as_expression);
            self.reach_base_keyed(base, keyed_argument(call, 1), call.span);
        }
        if unknown || names_one_of(keys, &OWN_WRITERS) {
            self.reach_base_keyed(Some(member.object()), keyed_argument(call, 0), call.span);
        }
        if unknown || names_one_of(keys, &BATCH_WRITERS) {
            self.reach_batch(call);
        }
    }

    fn reach_arguments(&mut self, call: &CallExpression<'a>) {
        for argument in &call.arguments {
            match argument.as_expression() {
                Some(expr) => self.reach_enumerated(expr, call.span),
                None => self.hit_dynamic(call.span),
            }
        }
    }

    fn reach_base_keyed(
        &mut self,
        base: Option<&Expression<'a>>,
        key: KeyedArgument<'_, 'a>,
        span: Span,
    ) {
        let Some(base) = base else {
            return;
        };
        if let KeyedArgument::Given(key) = key {
            if self.paired_sites_safe(base, key) {
                return;
            }
        }
        let hit = match key {
            KeyedArgument::Given(expr) => self.key_hits(expr),
            KeyedArgument::Absent => self.watched_name("undefined"),
            KeyedArgument::Spread => dynamic_paths(),
        };
        if hit {
            self.reach_base(base, span);
        }
    }

    fn paired_sites_safe(&self, base: &Expression<'a>, key: &Expression<'a>) -> bool {
        let (Expression::Identifier(base), Expression::Identifier(key)) =
            (base.without_parentheses(), key.without_parentheses())
        else {
            return false;
        };
        if self.substituted(base).is_some() || self.substituted(key).is_some() {
            return false;
        }
        let (Some((_, func, base_at)), Some((_, key_func, key_at))) =
            (self.resolvable_param(base), self.resolvable_param(key))
        else {
            return false;
        };
        func == key_func
            && self.fresh.bindings.call_sites(func).all(|call| {
                let value = |at: usize| call.arguments.get(at).map(Argument::as_expression);
                let key_hits = match value(key_at) {
                    None => self.watched_name("undefined"),
                    Some(None) => true,
                    Some(Some(expr)) => self.site_key_hits(expr, VALUE_DEPTH),
                };
                !key_hits
                    || match value(base_at) {
                        None => true,
                        Some(None) => false,
                        Some(Some(expr)) => self.either_pass(|| {
                            self.site_non_global(expr, VALUE_DEPTH) || self.target_fresh(expr, 4)
                        }),
                    }
            })
    }

    fn reach_batch(&mut self, call: &CallExpression<'a>) {
        let mut arguments = call.arguments.iter();
        let Some(target) = arguments.next().and_then(Argument::as_expression) else {
            return;
        };
        for source in arguments {
            let Some(expr) = source.as_expression() else {
                return self.hit_dynamic(call.span);
            };
            self.reach_enumerated(expr, call.span);
            if self.source_may_hit(expr) {
                self.reach_base(target, call.span);
            }
        }
    }

    fn source_may_hit(&self, source: &Expression<'a>) -> bool {
        let Expression::ObjectExpression(object) = source.without_parentheses() else {
            return dynamic_paths();
        };
        object.properties.iter().any(|property| match property {
            ObjectPropertyKind::ObjectProperty(p) => {
                if p.computed {
                    p.key.as_expression().is_none_or(|expr| self.key_hits(expr))
                } else {
                    p.key
                        .static_name()
                        .and_then(|name| name.into_cow_str())
                        .is_none_or(|name| self.watched_name(&name))
                }
            }
            ObjectPropertyKind::SpreadProperty(_) => dynamic_paths(),
        })
    }

    pub(super) fn reach_binding(&mut self, pattern: &BindingPattern<'a>, base: &Expression<'a>) {
        let BindingPattern::ObjectPattern(object) = pattern else {
            return;
        };
        if !self.watching(object.span) {
            return;
        }
        let hit = (object.rest.is_some() && dynamic_paths())
            || object
                .properties
                .iter()
                .any(|property| self.binding_property_hits(property));
        if hit {
            self.reach_base(base, object.span);
        }
    }

    fn binding_property_hits(&self, property: &BindingProperty<'a>) -> bool {
        if property.computed {
            return property
                .key
                .as_expression()
                .is_none_or(|expr| self.key_hits(expr));
        }
        property
            .key
            .static_name()
            .and_then(|name| name.into_cow_str())
            .is_some_and(|name| self.watched_name(&name))
    }

    pub(super) fn reach_assignment_target(
        &mut self,
        target: &ObjectAssignmentTarget<'a>,
        base: &Expression<'a>,
    ) {
        if !self.watching(target.span) {
            return;
        }
        let hit = (target.rest.is_some() && dynamic_paths())
            || target
                .properties
                .iter()
                .any(|property| self.target_property_hits(property));
        if hit {
            self.reach_base(base, target.span);
        }
    }

    fn target_property_hits(&self, property: &AssignmentTargetProperty<'a>) -> bool {
        match property {
            AssignmentTargetProperty::AssignmentTargetPropertyIdentifier(id) => {
                self.watched_name(&id.binding.name)
            }
            AssignmentTargetProperty::AssignmentTargetPropertyProperty(p) if p.computed => p
                .name
                .as_expression()
                .is_none_or(|expr| self.key_hits(expr)),
            AssignmentTargetProperty::AssignmentTargetPropertyProperty(p) => p
                .name
                .static_name()
                .and_then(|name| name.into_cow_str())
                .is_some_and(|name| self.watched_name(&name)),
        }
    }

    pub(super) fn reach_object(&mut self, object: &ObjectExpression<'a>) {
        if !self.watching(object.span) {
            return;
        }
        for property in &object.properties {
            if let ObjectPropertyKind::SpreadProperty(spread) = property {
                self.reach_enumerated(&spread.argument, spread.span());
            }
        }
    }
}
