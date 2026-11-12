use oxc_ast::ast::*;
use oxc_semantic::Scoping;
use oxc_span::GetSpan;
use oxc_syntax::operator::{AssignmentOperator, BinaryOperator, UnaryOperator};
use oxc_syntax::symbol::SymbolId;
use rustc_hash::FxHashMap;
use std::borrow::Cow;
use untwist_core::ast_util::js_text::js_text;

use super::assembled::literal_fields;
use super::bind::{
    class_constructs_fresh, function_constructs_fresh, function_uses_this, Bindings, Defs,
};

type Fields<'a> = Option<FxHashMap<Cow<'a, str>, &'a Expression<'a>>>;

pub(super) struct Fresh<'s, 'a> {
    pub(super) scoping: &'s Scoping,
    pub(super) bindings: &'s Bindings<'s, 'a>,
    pub(super) sealed: std::cell::RefCell<FxHashMap<SymbolId, Fields<'a>>>,
    pub(super) memo: Memo,
    pub(super) resolve: super::bootstrap::Resolve<'s>,
}

type AssumedKey = [Option<SymbolId>; ASSUMED_KEY_LEN];

#[derive(Default)]
pub(super) struct Memo {
    numeric: std::cell::RefCell<FxHashMap<(SymbolId, u8, AssumedKey), bool>>,
    fresh: std::cell::RefCell<FxHashMap<(SymbolId, u8), bool>>,
}

const PRIMITIVE_GLOBALS: [&str; 3] = ["undefined", "NaN", "Infinity"];
const DEPTH: u8 = 6;
const ASSUMED_KEY_LEN: usize = DEPTH as usize / 2;

pub(super) fn array_element<'k, 'a>(
    array: &'k ArrayExpression<'a>,
    index: f64,
) -> Option<&'k Expression<'a>> {
    let whole = index.fract() == 0.0 && (0.0..f64::from(u32::MAX)).contains(&index);
    let elements = array.elements.get(..=whole.then_some(index as usize)?)?;
    if elements
        .iter()
        .any(|element| matches!(element, ArrayExpressionElement::SpreadElement(_)))
    {
        return None;
    }
    elements.last()?.as_expression()
}

impl<'s, 'a> Fresh<'s, 'a> {
    pub(super) fn symbol(&self, id: &IdentifierReference<'_>) -> Option<SymbolId> {
        id.reference_id
            .get()
            .and_then(|reference| self.scoping.get_reference(reference).symbol_id())
    }

    fn defs(&self, id: &IdentifierReference<'_>) -> Option<Defs<'_, 'a>> {
        let symbol = self.symbol(id)?;
        self.bindings
            .defs
            .get(symbol)
            .filter(|defs| !defs.opaque && !defs.mutated && self.bindings.stable.holds(symbol))
    }

    pub(super) fn inlinable_helper(
        &self,
        id: &IdentifierReference<'_>,
    ) -> Option<&'a Function<'a>> {
        let symbol = self.symbol(id)?;
        let func = self.bindings.defs.get(symbol)?.func?;
        self.bindings
            .inlinable
            .contains(&func.span.start)
            .then_some(func)
    }

    pub(super) fn is_fresh(&self, expr: &Expression<'_>) -> bool {
        self.fresh_at(expr, DEPTH)
    }

    fn fresh_at(&self, expr: &Expression<'_>, depth: u8) -> bool {
        if depth == 0 {
            return false;
        }
        let next = depth - 1;
        match expr.without_parentheses() {
            Expression::StringLiteral(_)
            | Expression::NumericLiteral(_)
            | Expression::BooleanLiteral(_)
            | Expression::NullLiteral(_)
            | Expression::BigIntLiteral(_)
            | Expression::RegExpLiteral(_)
            | Expression::TemplateLiteral(_)
            | Expression::ObjectExpression(_)
            | Expression::ArrayExpression(_)
            | Expression::FunctionExpression(_)
            | Expression::ArrowFunctionExpression(_)
            | Expression::ClassExpression(_)
            | Expression::UnaryExpression(_)
            | Expression::BinaryExpression(_)
            | Expression::UpdateExpression(_) => true,
            Expression::LogicalExpression(logical) => {
                self.fresh_at(&logical.left, next) && self.fresh_at(&logical.right, next)
            }
            Expression::ConditionalExpression(cond) => {
                self.fresh_at(&cond.consequent, next) && self.fresh_at(&cond.alternate, next)
            }
            Expression::SequenceExpression(seq) => seq
                .expressions
                .last()
                .is_some_and(|last| self.fresh_at(last, next)),
            Expression::AssignmentExpression(assign) => {
                assign.operator.is_arithmetic()
                    || assign.operator.is_bitwise()
                    || (assign.operator == AssignmentOperator::Assign
                        && self.fresh_at(&assign.right, next))
            }
            Expression::NewExpression(new) => self.constructs_fresh(&new.callee, next),
            Expression::Identifier(id) => self.identifier_fresh(id, next),
            _ => false,
        }
    }

    fn identifier_fresh(&self, id: &IdentifierReference<'_>, depth: u8) -> bool {
        let Some(symbol) = self.symbol(id) else {
            return PRIMITIVE_GLOBALS.contains(&id.name.as_str());
        };
        if let Some(&known) = self.memo.fresh.borrow().get(&(symbol, depth)) {
            return known;
        }
        let fresh = self.identifier_fresh_uncached(id, depth);
        self.memo.fresh.borrow_mut().insert((symbol, depth), fresh);
        fresh
    }

    fn identifier_fresh_uncached(&self, id: &IdentifierReference<'_>, depth: u8) -> bool {
        if self.numeric_binding(id, depth, None) {
            return true;
        }
        let Some(defs) = self
            .symbol(id)
            .and_then(|symbol| self.bindings.defs.get(symbol))
        else {
            return false;
        };
        if defs.opaque {
            return false;
        }
        let inits_fresh = (defs.callable || !defs.inits.is_empty())
            && defs
                .inits
                .iter()
                .all(|init| init.is_none_or(|init| within_budget() && self.fresh_at(init, depth)));
        inits_fresh && (!defs.mutated || self.writes_fresh(defs.writes, depth))
    }

    fn writes_fresh(&self, writes: &[Option<&Expression<'_>>], depth: u8) -> bool {
        writes
            .iter()
            .all(|value| value.is_some_and(|value| within_budget() && self.fresh_at(value, depth)))
    }

    fn constructs_fresh(&self, callee: &Expression<'_>, depth: u8) -> bool {
        match callee.without_parentheses() {
            Expression::FunctionExpression(function) => function_constructs_fresh(function),
            Expression::ClassExpression(class) => class_constructs_fresh(class),
            Expression::Identifier(id) if depth > 0 => {
                let Some(defs) = self.defs(id) else {
                    return false;
                };
                let inits_fresh = defs
                    .inits
                    .iter()
                    .all(|init| init.is_some_and(|init| self.constructs_fresh(init, depth - 1)));
                (defs.callable || !defs.inits.is_empty())
                    && (!defs.callable || defs.constructs_fresh)
                    && inits_fresh
            }
            _ => false,
        }
    }

    pub(super) fn is_user_function(&self, callee: &Expression<'_>) -> bool {
        match callee.without_parentheses() {
            Expression::FunctionExpression(_)
            | Expression::ArrowFunctionExpression(_)
            | Expression::ClassExpression(_) => true,
            Expression::StaticMemberExpression(_) | Expression::ComputedMemberExpression(_) => {
                self.sealed_method(callee).is_some_and(|value| {
                    matches!(
                        value.without_parentheses(),
                        Expression::FunctionExpression(_) | Expression::ArrowFunctionExpression(_)
                    )
                })
            }
            Expression::Identifier(id) => self.defs(id).is_some_and(|defs| {
                (defs.callable || !defs.inits.is_empty())
                    && defs.inits.iter().all(|init| {
                        init.is_some_and(|init| {
                            matches!(
                                init.without_parentheses(),
                                Expression::FunctionExpression(_)
                                    | Expression::ArrowFunctionExpression(_)
                                    | Expression::ClassExpression(_)
                            )
                        })
                    })
            }),
            _ => false,
        }
    }

    pub(super) fn holds_function_or_primitive(&self, id: &IdentifierReference<'_>) -> bool {
        self.defs(id).is_some_and(|defs| {
            !defs.fields_written
                && match defs.inits {
                    [] => defs.callable,
                    [Some(init)] => {
                        function_valued(init)
                            || super::live::iife_returns_only(init, |value| {
                                function_valued(value)
                                    || super::holds::primitive_result(value.without_parentheses())
                            })
                    }
                    _ => false,
                }
        })
    }

    pub(super) fn callee_function<'k>(
        &self,
        callee: &'k Expression<'a>,
    ) -> Option<&'k Function<'a>> {
        match callee.without_parentheses() {
            Expression::FunctionExpression(func) => Some(&**func),
            Expression::Identifier(id) => self.defs(id).and_then(|defs| defs.func),
            _ => match self.sealed_method(callee)?.without_parentheses() {
                Expression::FunctionExpression(func) => Some(&**func),
                _ => None,
            },
        }
    }

    pub(super) fn bound_value(&self, id: &IdentifierReference<'_>) -> Option<Bound<'a>> {
        let defs = self.defs(id)?;
        if defs.callable {
            return (defs.inits.is_empty() && !defs.fields_written).then_some(Bound::Function);
        }
        let [Some(init)] = defs.inits else {
            return None;
        };
        let init = init.without_parentheses();
        if super::holds::primitive_result(init) {
            return Some(Bound::Primitive);
        }
        if defs.fields_written {
            return None;
        }
        if function_valued(init) {
            return Some(Bound::Function);
        }
        match init {
            Expression::ObjectExpression(_)
            | Expression::ArrayExpression(_)
            | Expression::RegExpLiteral(_)
                if !defs.method_called =>
            {
                Some(Bound::Container(init))
            }
            _ => None,
        }
    }

    pub(super) fn holds_fresh_sequence(&self, expr: &Expression<'_>) -> bool {
        match expr.without_parentheses() {
            Expression::Identifier(id) => self.defs(id).is_some_and(|defs| {
                !defs.escapes
                    && !defs.callable
                    && defs.writes.is_empty()
                    && matches!(defs.inits, [Some(init)] if is_sequence_literal(init))
            }),
            other => is_sequence_literal(other),
        }
    }

    pub(super) fn sealed_function(&self, callee: &Expression<'_>) -> Option<&'a Function<'a>> {
        match self.sealed_method(callee)?.without_parentheses() {
            Expression::FunctionExpression(func) => Some(&**func),
            _ => None,
        }
    }

    fn sealed_method(&self, callee: &Expression<'_>) -> Option<&'a Expression<'a>> {
        let member = callee.without_parentheses().as_member_expression()?;
        let Expression::Identifier(obj) = member.object().without_parentheses() else {
            return None;
        };
        let key = match member {
            MemberExpression::StaticMemberExpression(m) => m.property.name.as_str(),
            MemberExpression::ComputedMemberExpression(m) => {
                match m.expression.without_parentheses() {
                    Expression::StringLiteral(text) => text.value.as_str()?,
                    _ => return None,
                }
            }
            MemberExpression::PrivateFieldExpression(_) => return None,
        };
        self.sealed_field(obj, key)
    }

    pub(super) fn keyed_source(
        &self,
        id: &IdentifierReference<'_>,
        aliased: bool,
    ) -> Option<KeyedSource<'a>> {
        let defs = self.defs(id)?;
        if defs.fields_written
            || defs.method_called
            || defs.callable
            || defs.refs != defs.read_through + u32::from(aliased)
        {
            return None;
        }
        let [Some(init)] = defs.inits else {
            return None;
        };
        let init: &'a Expression<'a> = init;
        match init.without_parentheses() {
            Expression::ObjectExpression(object) => Some(KeyedSource::Literal(object)),
            Expression::Identifier(source) => match self.built_fields(id) {
                Some(fields) => Some(KeyedSource::Built(fields)),
                None => Some(KeyedSource::Alias(source)),
            },
            _ => None,
        }
    }

    fn built_fields(&self, alias: &IdentifierReference<'_>) -> Option<Vec<Cow<'a, str>>> {
        let table = self.bindings.assembled.get(&self.symbol(alias)?)?;
        Some(table.fields.iter().map(|(name, _)| name.clone()).collect())
    }

    pub(super) fn literal_init(&self, id: &IdentifierReference<'_>) -> Option<&'a Expression<'a>> {
        let defs = self.defs(id)?;
        match defs.inits {
            [Some(init)] if !defs.callable => Some(*init),
            _ => None,
        }
    }

    pub(super) fn sealed_field(
        &self,
        id: &IdentifierReference<'_>,
        key: &str,
    ) -> Option<&'a Expression<'a>> {
        let symbol = self.symbol(id)?;
        self.with_sealed(symbol, |fields| fields.get(key).copied())
    }

    pub(super) fn is_sealed(&self, symbol: SymbolId) -> bool {
        self.with_sealed(symbol, |_| Some(())).is_some()
    }

    fn with_sealed<R>(
        &self,
        symbol: SymbolId,
        read: impl Fn(&FxHashMap<Cow<'a, str>, &'a Expression<'a>>) -> Option<R>,
    ) -> Option<R> {
        if let Some(fields) = self.sealed.borrow().get(&symbol) {
            return fields.as_ref().and_then(&read);
        }
        let fields = self.sealed_fields(symbol);
        let found = fields.as_ref().and_then(&read);
        self.sealed.borrow_mut().insert(symbol, fields);
        found
    }

    pub(super) fn sealed_element(
        &self,
        id: &IdentifierReference<'_>,
        index: f64,
    ) -> Option<&'a Expression<'a>> {
        array_element(self.sealed_array(id)?, index)
    }

    fn sealed_array(&self, id: &IdentifierReference<'_>) -> Option<&'a ArrayExpression<'a>> {
        let defs = self.defs(id)?;
        if defs.escapes || defs.fields_written || defs.method_called || defs.callable {
            return None;
        }
        let [Some(init)] = defs.inits else {
            return None;
        };
        match init.without_parentheses() {
            Expression::ArrayExpression(array) => Some(array),
            _ => None,
        }
    }

    pub(super) fn written_array(
        &self,
        id: &IdentifierReference<'_>,
    ) -> Option<(&'a ArrayExpression<'a>, &'s [&'a Expression<'a>])> {
        let symbol = self.symbol(id)?;
        let defs = self.defs(id)?;
        if defs.escapes
            || defs.method_called
            || defs.callable
            || self.bindings.other_fields_written.contains(&symbol)
        {
            return None;
        }
        let [Some(init)] = defs.inits else {
            return None;
        };
        let Expression::ArrayExpression(array) = init.without_parentheses() else {
            return None;
        };
        let written = self
            .bindings
            .element_values
            .get(&symbol)
            .map_or(&[][..], Vec::as_slice);
        Some((array, written))
    }

    fn field_keeps_this(&self, value: &Expression<'_>) -> bool {
        match value.without_parentheses() {
            Expression::Identifier(id) => match self.defs(id) {
                Some(defs) if defs.callable => {
                    defs.inits.is_empty() && defs.func.is_some_and(|func| !function_uses_this(func))
                }
                Some(defs) => {
                    !defs.inits.is_empty()
                        && defs.inits.iter().all(|init| init.is_some_and(inert_field))
                }
                None => self.symbol(id).is_none() && PRIMITIVE_GLOBALS.contains(&id.name.as_str()),
            },
            Expression::CallExpression(call) => self
                .resolve
                .is_some_and(|resolved| resolved.contains_key(&call.span)),
            other => inert_field(other),
        }
    }

    fn sealed_fields(&self, symbol: SymbolId) -> Fields<'a> {
        let defs = self
            .bindings
            .defs
            .get(symbol)
            .filter(|defs| !defs.opaque && !defs.mutated)?;
        if defs.escapes || defs.fields_written || defs.callable {
            return None;
        }
        let [Some(init)] = defs.inits else {
            return None;
        };
        let entries = match init.without_parentheses() {
            Expression::ObjectExpression(object) => literal_fields(object)?,
            Expression::Identifier(_) => self.bindings.assembled.get(&symbol)?.fields.clone(),
            _ => return None,
        };
        let fields: FxHashMap<Cow<'a, str>, &'a Expression<'a>> = entries.into_iter().collect();
        let own = |member: &&MemberExpression<'_>| {
            let Some(value) = self
                .accessed_key(member)
                .and_then(|key| fields.get(key.as_ref()).copied())
            else {
                return false;
            };
            !self.bindings.called.contains(&member.span().start) || self.field_keeps_this(value)
        };
        let accessed = self.bindings.accessed.get(&symbol);
        accessed
            .is_none_or(|members| members.iter().all(own))
            .then_some(fields)
    }

    fn accessed_key(&self, member: &MemberExpression<'_>) -> Option<Cow<'a, str>> {
        match member {
            MemberExpression::StaticMemberExpression(m) => {
                Some(Cow::Owned(m.property.name.to_string()))
            }
            MemberExpression::ComputedMemberExpression(m) => {
                self.key_text(&m.expression, DEPTH).map(Cow::Owned)
            }
            MemberExpression::PrivateFieldExpression(_) => None,
        }
    }

    fn key_text(&self, expr: &Expression<'_>, depth: u8) -> Option<String> {
        let next = depth.checked_sub(1)?;
        match expr.without_parentheses() {
            Expression::StringLiteral(text) => Some(js_text(text.value).into_owned()),
            Expression::TemplateLiteral(template) if template.expressions.is_empty() => {
                let cooked = template.quasis.first()?.value.cooked?;
                Some(js_text(cooked).into_owned())
            }
            Expression::CallExpression(call) => self.resolve?.get(&call.span)?.as_string(),
            Expression::BinaryExpression(binary) if binary.operator == BinaryOperator::Addition => {
                Some(self.key_text(&binary.left, next)? + &self.key_text(&binary.right, next)?)
            }
            Expression::Identifier(id) => self.key_text(self.literal_init(id)?, next),
            _ => None,
        }
    }

    pub(super) fn is_numeric_identifier(&self, id: &IdentifierReference<'_>) -> bool {
        self.numeric_binding(id, DEPTH, None)
    }

    pub(super) fn is_numeric(&self, expr: &Expression<'_>) -> bool {
        self.numeric_at(expr, DEPTH, None)
    }

    fn numeric_at(&self, expr: &Expression<'_>, depth: u8, assumed: Option<&Assumed<'_>>) -> bool {
        if depth == 0 {
            return false;
        }
        let next = depth - 1;
        match expr.without_parentheses() {
            Expression::NumericLiteral(_) | Expression::UpdateExpression(_) => true,
            Expression::UnaryExpression(unary) => matches!(
                unary.operator,
                UnaryOperator::UnaryNegation | UnaryOperator::UnaryPlus | UnaryOperator::BitwiseNot
            ),
            Expression::BinaryExpression(binary) => match binary.operator {
                BinaryOperator::Addition => {
                    self.numeric_at(&binary.left, next, assumed)
                        && self.numeric_at(&binary.right, next, assumed)
                }
                operator => operator.is_arithmetic() || operator.is_bitwise(),
            },
            Expression::Identifier(id) => self.numeric_binding(id, next, assumed),
            _ => false,
        }
    }

    fn numeric_binding(
        &self,
        id: &IdentifierReference<'_>,
        depth: u8,
        assumed: Option<&Assumed<'_>>,
    ) -> bool {
        if depth == 0 {
            return false;
        }
        let Some(symbol) = self.symbol(id) else {
            return false;
        };
        if Assumed::holds(assumed, symbol) {
            return true;
        }
        let Some(key) = Assumed::key(assumed) else {
            return self.numeric_binding_uncached(symbol, depth, assumed);
        };
        if let Some(&known) = self.memo.numeric.borrow().get(&(symbol, depth, key)) {
            return known;
        }
        let numeric = self.numeric_binding_uncached(symbol, depth, assumed);
        self.memo
            .numeric
            .borrow_mut()
            .insert((symbol, depth, key), numeric);
        numeric
    }

    fn numeric_binding_uncached(
        &self,
        symbol: SymbolId,
        depth: u8,
        assumed: Option<&Assumed<'_>>,
    ) -> bool {
        let Some(defs) = self.bindings.defs.get(symbol) else {
            return false;
        };
        let assumed = Assumed {
            symbol,
            outer: assumed,
        };
        !defs.opaque
            && !defs.callable
            && !defs.inits.is_empty()
            && defs.inits.iter().chain(defs.writes).all(|value| {
                value.is_some_and(|value| {
                    within_budget() && self.numeric_at(value, depth - 1, Some(&assumed))
                })
            })
    }
}

fn within_budget() -> bool {
    untwist_core::budget::check().is_ok()
}

struct Assumed<'s> {
    symbol: SymbolId,
    outer: Option<&'s Assumed<'s>>,
}

impl Assumed<'_> {
    fn holds(mut at: Option<&Assumed<'_>>, symbol: SymbolId) -> bool {
        while let Some(assumed) = at {
            if assumed.symbol == symbol {
                return true;
            }
            at = assumed.outer;
        }
        false
    }

    fn key(mut at: Option<&Assumed<'_>>) -> Option<AssumedKey> {
        let mut key = [None; ASSUMED_KEY_LEN];
        let mut len = 0;
        while let Some(assumed) = at {
            *key.get_mut(len)? = Some(assumed.symbol);
            len += 1;
            at = assumed.outer;
        }
        key.sort_unstable();
        Some(key)
    }
}

pub(super) enum KeyedSource<'a> {
    Literal(&'a ObjectExpression<'a>),
    Built(Vec<Cow<'a, str>>),
    Alias(&'a IdentifierReference<'a>),
}

pub(super) enum Bound<'a> {
    Primitive,
    Function,
    Container(&'a Expression<'a>),
}

fn function_valued(expr: &Expression<'_>) -> bool {
    match expr.without_parentheses() {
        Expression::FunctionExpression(_) | Expression::ArrowFunctionExpression(_) => true,
        Expression::ConditionalExpression(cond) => {
            function_valued(&cond.consequent) && function_valued(&cond.alternate)
        }
        _ => false,
    }
}

fn inert_field(value: &Expression<'_>) -> bool {
    match value.without_parentheses() {
        Expression::FunctionExpression(function) => !function_uses_this(function),
        Expression::ArrowFunctionExpression(_)
        | Expression::ObjectExpression(_)
        | Expression::ArrayExpression(_)
        | Expression::RegExpLiteral(_) => true,
        other => super::holds::primitive_result(other),
    }
}

fn is_sequence_literal(expr: &Expression<'_>) -> bool {
    matches!(
        expr.without_parentheses(),
        Expression::ArrayExpression(_)
            | Expression::StringLiteral(_)
            | Expression::TemplateLiteral(_)
    )
}
