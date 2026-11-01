use oxc_ast::ast::Expression;

use rustc_hash::FxHashMap;

use super::{fold_with, FoldOptions, PureHost};
use crate::budget::Budget;
use crate::heap::HeapLedger;
use crate::value::Value;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LayerText {
    Text(String),
    NotAString,
    Bailed,
    Fuel,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LayerFold {
    pub text: LayerText,
    pub steps: u64,
    pub heap: u64,
}

pub fn fold_layer_text(expr: &Expression<'_>, fuel: u64, heap: u64) -> LayerFold {
    let ledger = HeapLedger::new(heap);
    let mut host = PureHost::new();
    host.budget = Budget::new(fuel);
    host.allow_local_mutation = true;
    host.heap = Some(ledger.clone());
    host.realm.set_heap(Some(ledger.clone()));
    let options = FoldOptions {
        allow_pure_calls: true,
        allow_local_mutation: true,
        bindings: FxHashMap::default(),
    };
    let value = fold_with(expr, &options, &mut host);
    let steps = host.budget.steps_used();
    let text = match value {
        Some(Value::Str(text)) => text.as_string().map_or(LayerText::Bailed, LayerText::Text),
        Some(_) => LayerText::NotAString,
        None if host.budget.steps_left() == 0 => LayerText::Fuel,
        None => LayerText::Bailed,
    };
    drop(host);
    LayerFold {
        text,
        steps,
        heap: ledger.peak(),
    }
}
