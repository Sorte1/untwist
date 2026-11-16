use oxc_ast::ast::*;
use oxc_semantic::Scoping;
use oxc_syntax::operator::{BinaryOperator, UnaryOperator};
use oxc_syntax::symbol::SymbolId;
use rustc_hash::{FxHashMap, FxHashSet};

use crate::host_text::{CONSTRUCTOR, EVAL, FUNCTION};

pub const RUNNER_KEYS: [&str; 3] = [CONSTRUCTOR, EVAL, FUNCTION];
const DEPTH: u8 = 12;
const NUMBER_NAMES: [&str; 3] = ["undefined", "NaN", "Infinity"];
const NUMBER_CALLS: [&str; 3] = ["Number", "parseInt", "parseFloat"];
const NUMBER_SPACES: [&str; 2] = ["Math", "Date"];
const SYMBOL: &str = "Symbol";
const OBJECT: &str = "Object";
pub const NAME_LISTERS: [&str; 3] = ["getOwnPropertyNames", "keys", "getOwnPropertySymbols"];

#[derive(Clone, Copy)]
pub enum Write<'a> {
    Value(&'a Expression<'a>),
    Plus(&'a Expression<'a>),
    Numeric,
}

#[derive(Default)]
pub struct Writes<'a> {
    pub declared: FxHashSet<SymbolId>,
    pub loose: FxHashSet<SymbolId>,
    pub values: FxHashMap<SymbolId, Vec<Write<'a>>>,
    pub counted: FxHashMap<SymbolId, usize>,
    pub params: FxHashMap<SymbolId, (SymbolId, usize)>,
    pub calls: FxHashMap<SymbolId, Vec<&'a [Argument<'a>]>>,
    pub readers: FxHashSet<SymbolId>,
    pub definers: FxHashSet<SymbolId>,
    pub passed: FxHashSet<SymbolId>,
    pub escaped: FxHashSet<SymbolId>,
    pub listers: FxHashSet<SymbolId>,
    pub reshaped: FxHashSet<SymbolId>,
    pub stored: FxHashMap<SymbolId, Vec<&'a Expression<'a>>>,
    pub fn_params: FxHashMap<SymbolId, Vec<Option<SymbolId>>>,
}

impl<'a> Writes<'a> {
    pub fn record(&mut self, symbol: SymbolId, write: Write<'a>, counted: bool) {
        self.values.entry(symbol).or_default().push(write);
        if counted {
            *self.counted.entry(symbol).or_default() += 1;
        }
    }

    fn settled(&self, scoping: &Scoping, symbol: SymbolId) -> Option<&[Write<'a>]> {
        if !self.declared.contains(&symbol) || self.loose.contains(&symbol) {
            return None;
        }
        self.counted_writes(scoping, symbol)
    }

    pub fn stable(&self, scoping: &Scoping, symbol: SymbolId) -> bool {
        !self.loose.contains(&symbol)
            && self
                .counted_writes(scoping, symbol)
                .is_some_and(|writes| writes.len() <= 1)
            && (self.declared.contains(&symbol) || !self.values.contains_key(&symbol))
    }

    pub fn single(&self, scoping: &Scoping, symbol: SymbolId) -> bool {
        self.settled(scoping, symbol)
            .is_some_and(|writes| writes.len() == 1)
    }

    fn counted_writes(&self, scoping: &Scoping, symbol: SymbolId) -> Option<&[Write<'a>]> {
        let writes = scoping
            .get_resolved_reference_ids(symbol)
            .iter()
            .filter(|reference| scoping.get_reference(**reference).is_write())
            .count();
        let counted = self.counted.get(&symbol).copied().unwrap_or(0);
        (writes == counted).then(|| self.values.get(&symbol).map_or(&[][..], Vec::as_slice))
    }
}

pub fn runner_key(key: &str) -> bool {
    RUNNER_KEYS.contains(&key)
}

pub fn const_key(expr: &Expression<'_>) -> Option<String> {
    const_text(expr, DEPTH)
}

fn const_text(expr: &Expression<'_>, depth: u8) -> Option<String> {
    let next = depth.checked_sub(1)?;
    match expr.without_parentheses() {
        Expression::StringLiteral(text) => text.value.as_str().map(str::to_string),
        Expression::NumericLiteral(number) => Some(number.value.to_string()),
        Expression::TemplateLiteral(template) => template
            .single_quasi()
            .and_then(|text| text.as_str().map(str::to_string)),
        Expression::BinaryExpression(sum) if sum.operator == BinaryOperator::Addition => {
            Some(const_text(&sum.left, next)? + &const_text(&sum.right, next)?)
        }
        _ => None,
    }
}

pub struct Keys<'k, 'a> {
    pub scoping: &'k Scoping,
    pub writes: &'k Writes<'a>,
}

impl<'a> Keys<'_, 'a> {
    fn symbol(&self, id: &IdentifierReference<'_>) -> Option<SymbolId> {
        let reference = id.reference_id.get()?;
        self.scoping.get_reference(reference).symbol_id()
    }

    fn free(&self, expr: &Expression<'_>, names: &[&str]) -> bool {
        matches!(expr.without_parentheses(), Expression::Identifier(id)
            if names.contains(&id.name.as_str()) && self.symbol(id).is_none())
    }

    pub fn safe(&self, expr: &Expression<'a>) -> bool {
        self.safe_at(expr, DEPTH, &mut Vec::new())
    }

    fn safe_at(&self, expr: &Expression<'a>, depth: u8, seen: &mut Vec<SymbolId>) -> bool {
        let Some(next) = depth.checked_sub(1) else {
            return false;
        };
        if let Some(key) = const_key(expr) {
            return !runner_key(&key);
        }
        match expr.without_parentheses() {
            Expression::BooleanLiteral(_)
            | Expression::NullLiteral(_)
            | Expression::BigIntLiteral(_)
            | Expression::RegExpLiteral(_)
            | Expression::UpdateExpression(_)
            | Expression::UnaryExpression(_) => true,
            Expression::Identifier(id) => match self.symbol(id) {
                None => NUMBER_NAMES.contains(&id.name.as_str()),
                Some(symbol) => self.safe_symbol(symbol, next, seen),
            },
            Expression::BinaryExpression(binary) => self.safe_binary(binary, next),
            Expression::ConditionalExpression(choice) => {
                self.safe_at(&choice.consequent, next, seen)
                    && self.safe_at(&choice.alternate, next, seen)
            }
            Expression::LogicalExpression(logical) => {
                self.safe_at(&logical.left, next, seen) && self.safe_at(&logical.right, next, seen)
            }
            Expression::SequenceExpression(sequence) => sequence
                .expressions
                .last()
                .is_some_and(|last| self.safe_at(last, next, seen)),
            Expression::AssignmentExpression(assign) => self.safe_at(&assign.right, next, seen),
            Expression::TemplateLiteral(template) => outside_runner_keys(template),
            Expression::StaticMemberExpression(member) => self.free(&member.object, &[SYMBOL]),
            Expression::CallExpression(call) => self.numeric_call(&call.callee),
            _ => false,
        }
    }

    fn numeric_call(&self, callee: &Expression<'_>) -> bool {
        match callee.without_parentheses() {
            Expression::Identifier(_) => {
                self.free(callee, &NUMBER_CALLS) || self.free(callee, &[SYMBOL])
            }
            Expression::StaticMemberExpression(member) => {
                self.free(&member.object, &NUMBER_SPACES) || self.free(&member.object, &[SYMBOL])
            }
            _ => false,
        }
    }

    fn safe_binary(&self, binary: &BinaryExpression<'a>, depth: u8) -> bool {
        if binary.operator != BinaryOperator::Addition {
            return true;
        }
        if self.numeric(&binary.left, depth) || self.numeric(&binary.right, depth) {
            return true;
        }
        let prefix = leftmost_text(&binary.left, depth);
        let suffix = rightmost_text(&binary.right, depth);
        prefix.is_some_and(|text| {
            !text.is_empty() && !RUNNER_KEYS.iter().any(|key| key.starts_with(&text))
        }) || suffix.is_some_and(|text| {
            !text.is_empty() && !RUNNER_KEYS.iter().any(|key| key.ends_with(&text))
        })
    }

    fn numeric(&self, expr: &Expression<'a>, depth: u8) -> bool {
        let Some(next) = depth.checked_sub(1) else {
            return false;
        };
        match expr.without_parentheses() {
            Expression::NumericLiteral(_) | Expression::UpdateExpression(_) => true,
            Expression::UnaryExpression(unary) => matches!(
                unary.operator,
                UnaryOperator::UnaryNegation | UnaryOperator::UnaryPlus | UnaryOperator::BitwiseNot
            ),
            Expression::BinaryExpression(binary)
                if binary.operator.is_arithmetic() || binary.operator.is_bitwise() =>
            {
                binary.operator != BinaryOperator::Addition
                    || (self.numeric(&binary.left, next) && self.numeric(&binary.right, next))
            }
            Expression::CallExpression(call) => {
                self.numeric_call(&call.callee) && !self.free(&call.callee, &[SYMBOL])
            }
            Expression::Identifier(id) => self
                .symbol(id)
                .is_some_and(|symbol| self.numeric_symbol(symbol, next)),
            _ => false,
        }
    }

    fn numeric_symbol(&self, symbol: SymbolId, depth: u8) -> bool {
        self.writes
            .settled(self.scoping, symbol)
            .is_some_and(|writes| {
                !writes.is_empty()
                    && writes.iter().all(|write| match write {
                        Write::Numeric => true,
                        Write::Plus(value) | Write::Value(value) => self.numeric(value, depth),
                    })
            })
    }

    fn safe_symbol(&self, symbol: SymbolId, depth: u8, seen: &mut Vec<SymbolId>) -> bool {
        if seen.contains(&symbol) {
            return false;
        }
        let Some(writes) = self.writes.settled(self.scoping, symbol) else {
            return false;
        };
        seen.push(symbol);
        let safe = writes.iter().all(|write| match write {
            Write::Numeric => true,
            Write::Plus(value) => self.numeric(value, depth),
            Write::Value(value) => self.safe_at(value, depth, seen),
        });
        seen.pop();
        safe
    }

    pub fn data(&self, expr: &Expression<'a>) -> bool {
        match expr.without_parentheses() {
            Expression::Identifier(id) => self.symbol(id).is_some_and(|symbol| {
                if self.uncalled_param(symbol) {
                    return true;
                }
                if self.writes.escaped.contains(&symbol) || self.reshaped(symbol) {
                    return false;
                }
                match self.writes.settled(self.scoping, symbol) {
                    Some(writes) => !writes.is_empty() && data_writes(writes),
                    None => self.data_param(symbol),
                }
            }),
            other => literal_data(other),
        }
    }

    fn reshaped(&self, symbol: SymbolId) -> bool {
        self.writes.reshaped.contains(&symbol)
            || self
                .writes
                .stored
                .get(&symbol)
                .is_some_and(|values| values.iter().any(|value| !self.primitive(value)))
    }

    fn primitive(&self, expr: &Expression<'a>) -> bool {
        match expr.without_parentheses() {
            Expression::StringLiteral(_)
            | Expression::NumericLiteral(_)
            | Expression::BooleanLiteral(_)
            | Expression::BigIntLiteral(_)
            | Expression::TemplateLiteral(_)
            | Expression::UnaryExpression(_)
            | Expression::UpdateExpression(_)
            | Expression::BinaryExpression(_) => true,
            other => self.numeric(other, DEPTH),
        }
    }

    fn listed(&self, expr: &Expression<'_>) -> bool {
        let Expression::Identifier(id) = expr.without_parentheses() else {
            return false;
        };
        self.symbol(id)
            .and_then(|symbol| self.writes.settled(self.scoping, symbol))
            .is_some_and(|writes| {
                !writes.is_empty()
                    && writes.iter().all(|write| match write {
                        Write::Value(value) => match value.without_parentheses() {
                            Expression::CallExpression(call) => self.lister(&call.callee, true),
                            _ => false,
                        },
                        _ => false,
                    })
            })
    }

    fn lister(&self, callee: &Expression<'_>, alias: bool) -> bool {
        match callee.without_parentheses() {
            Expression::Identifier(id) if alias => self
                .symbol(id)
                .and_then(|symbol| self.writes.settled(self.scoping, symbol))
                .is_some_and(
                    |writes| matches!(writes, [Write::Value(init)] if self.lister(init, false)),
                ),
            Expression::StaticMemberExpression(member) => {
                self.free(&member.object, &[OBJECT])
                    && NAME_LISTERS.contains(&member.property.name.as_str())
            }
            _ => false,
        }
    }

    fn uncalled_param(&self, symbol: SymbolId) -> bool {
        self.writes
            .params
            .get(&symbol)
            .is_some_and(|(function, _)| {
                self.scoping
                    .get_resolved_reference_ids(*function)
                    .is_empty()
            })
    }

    fn data_param(&self, symbol: SymbolId) -> bool {
        let written = || {
            self.writes
                .counted_writes(self.scoping, symbol)
                .is_some_and(data_writes)
        };
        if self.writes.passed.contains(&symbol) {
            return written();
        }
        let Some(&(function, index)) = self.writes.params.get(&symbol) else {
            return false;
        };
        let calls = self
            .writes
            .calls
            .get(&function)
            .map_or(&[][..], Vec::as_slice);
        let references = self.scoping.get_resolved_reference_ids(function).len();
        let passed = calls.iter().all(|arguments| {
            arguments
                .get(index)
                .and_then(Argument::as_expression)
                .is_some_and(literal_data)
        });
        calls.len() == references && passed && written()
    }

    pub fn same_key(&self, left: &Expression<'_>, right: &Expression<'_>) -> bool {
        match (left.without_parentheses(), right.without_parentheses()) {
            (Expression::Identifier(a), Expression::Identifier(b)) => {
                a.name == b.name && self.symbol(a) == self.symbol(b)
            }
            (Expression::ComputedMemberExpression(a), Expression::ComputedMemberExpression(b)) => {
                self.listed(&a.object)
                    && matches!(&a.expression, Expression::Identifier(_))
                    && self.same_key(&a.object, &b.object)
                    && self.same_key(&a.expression, &b.expression)
            }
            _ => false,
        }
    }
}

fn data_writes(writes: &[Write<'_>]) -> bool {
    writes.iter().all(|write| match write {
        Write::Numeric | Write::Plus(_) => true,
        Write::Value(value) => literal_data(value),
    })
}

pub fn literal_data(expr: &Expression<'_>) -> bool {
    match expr.without_parentheses() {
        Expression::NullLiteral(_) | Expression::ArrayExpression(_)
        | Expression::StringLiteral(_)
        | Expression::NumericLiteral(_)
        | Expression::BooleanLiteral(_)
        | Expression::TemplateLiteral(_) => true,
        Expression::ObjectExpression(object) => !object.properties.iter().any(|property| {
            matches!(property, ObjectPropertyKind::ObjectProperty(property)
                if !property.computed && property.key.static_name().is_some_and(|name| name.as_str() == Some("__proto__")))
        }),
        _ => false,
    }
}

fn outside_runner_keys(template: &TemplateLiteral<'_>) -> bool {
    let texts: Vec<&str> = template
        .quasis
        .iter()
        .map(|quasi| quasi.value.raw.as_str())
        .collect();
    let (Some(first), Some(last)) = (texts.first(), texts.last()) else {
        return false;
    };
    let middle = texts.len() > 2
        && texts[1..texts.len() - 1]
            .iter()
            .any(|text| !text.is_empty() && !RUNNER_KEYS.iter().any(|key| key.contains(text)));
    middle
        || (!first.is_empty() && !RUNNER_KEYS.iter().any(|key| key.starts_with(first)))
        || (!last.is_empty() && !RUNNER_KEYS.iter().any(|key| key.ends_with(last)))
}

fn leftmost_text(expr: &Expression<'_>, depth: u8) -> Option<String> {
    let next = depth.checked_sub(1)?;
    match expr.without_parentheses() {
        Expression::BinaryExpression(sum) if sum.operator == BinaryOperator::Addition => {
            leftmost_text(&sum.left, next)
        }
        Expression::TemplateLiteral(template) => template
            .quasis
            .first()
            .map(|quasi| quasi.value.raw.to_string()),
        other => const_key(other),
    }
}

fn rightmost_text(expr: &Expression<'_>, depth: u8) -> Option<String> {
    let next = depth.checked_sub(1)?;
    match expr.without_parentheses() {
        Expression::BinaryExpression(sum) if sum.operator == BinaryOperator::Addition => {
            rightmost_text(&sum.right, next)
        }
        Expression::TemplateLiteral(template) => template
            .quasis
            .last()
            .map(|quasi| quasi.value.raw.to_string()),
        other => const_key(other),
    }
}
