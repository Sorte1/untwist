use std::cell::Cell;

use oxc_ast::ast::*;
use untwist_core::pass::{Cx, Safety};

use super::identify::Table;
use super::shapes::{is_consumer, Kind};
use super::{spread_array, spread_object};

#[derive(Clone, Copy, Debug)]
pub struct Gate<'g> {
    pub safe: bool,
    pub unsafe_: bool,
    pub asked: Option<&'g Cell<bool>>,
    pub asks_unsafe: bool,
}

impl<'g> Gate<'g> {
    pub const SHUT: Gate<'static> = Gate {
        safe: false,
        unsafe_: false,
        asked: None,
        asks_unsafe: false,
    };

    pub fn asking(asked: &'g Cell<bool>, asks_unsafe: bool) -> Self {
        Gate {
            safe: false,
            unsafe_: false,
            asked: Some(asked),
            asks_unsafe,
        }
    }

    pub fn allows(&self, safety: Safety) -> bool {
        if safety == Safety::Safe || self.asks_unsafe {
            if let Some(asked) = self.asked {
                asked.set(true);
            }
        }
        match safety {
            Safety::Safe => self.safe,
            Safety::Unsafe => self.unsafe_,
        }
    }

    pub fn shut(&self) -> bool {
        !self.safe && !self.unsafe_
    }
}

pub fn shape(kind: &'static str, safety: Safety) -> &'static str {
    match (kind, safety) {
        ("to-consumable-array", Safety::Safe) => "array/to-consumable-array/safe",
        ("to-consumable-array", Safety::Unsafe) => "array/to-consumable-array/unsafe",
        ("spread-array", Safety::Safe) => "array/spread-array/safe",
        ("spread-array-read", _) => "array/spread-array-read/unsafe",
        ("spread-array", Safety::Unsafe) => "array/spread-array/unsafe",
        ("concat", _) => "array/concat/safe",
        ("assign", Safety::Safe) => "object/assign/safe",
        ("assign", Safety::Unsafe) => "object/assign/unsafe",
        ("object-spread2", Safety::Safe) => "object/object-spread2/safe",
        ("object-spread2", Safety::Unsafe) => "object/object-spread2/unsafe",
        ("object-spread-each", Safety::Safe) => "object/object-spread-each/safe",
        ("object-spread-each", Safety::Unsafe) => "object/object-spread-each/unsafe",
        ("spread-values", Safety::Safe) => "object/spread-values/safe",
        ("spread-values", Safety::Unsafe) => "object/spread-values/unsafe",
        ("spread-props", _) => "object/spread-props/safe",
        _ => "unnamed",
    }
}

fn callable(call: &CallExpression<'_>) -> bool {
    !call.optional
        && call
            .arguments
            .iter()
            .all(|argument| !matches!(argument, Argument::SpreadElement(_)))
}

pub fn rewrite<'a>(
    node: &mut Expression<'a>,
    table: &Table,
    gate: &Gate<'_>,
    cx: &mut Cx<'a, '_>,
) -> Option<&'static str> {
    let Expression::CallExpression(call) = &*node else {
        return None;
    };
    let kind = match &call.callee {
        Expression::Identifier(_) => table.kind_of_callee(&call.callee, cx.scoping())?,
        Expression::StaticMemberExpression(member) if member.property.name == "concat" => {
            return callable(call)
                .then(|| spread_array::concat(node, gate, cx))
                .flatten();
        }
        _ => return None,
    };
    if !is_consumer(kind) || !callable(call) {
        return None;
    }
    match kind {
        Kind::ToConsumableArray => spread_array::to_consumable(node, gate, cx),
        Kind::SpreadArray => spread_array::spread_array(node, table, gate, cx),
        Kind::Assign
        | Kind::ObjectSpread2
        | Kind::ObjectSpreadEach
        | Kind::ObjectSpreadProps
        | Kind::SpreadValues
        | Kind::SpreadProps => spread_object::spread(node, kind, table, gate, cx),
        _ => None,
    }
}
