use std::cell::RefCell;

pub const VALUE_FLOW_SINKS: &str = "value-flow-sinks";
pub const MACHINERY_SOURCE_BOUND: &str = "machinery-source-bound";
pub const OBSERVER_LIST: &str = "observer-list";
pub const NATIVE_MAP_PENDING: &str = "native-map-pending";
pub const BUILT_LITERAL: &str = "built-literal";
pub const INSTANCE_RECEIVERS: &str = "instance-receivers";
pub const FILTERED_ENUMERATION: &str = "filtered-enumeration";
pub const STORAGE_PROXY: &str = "storage-proxy";
pub const FUNCTION_VALUES: &str = "function-values";
pub const AWAITED_ARRAYS: &str = "awaited-arrays";
pub const FILTERED_REWRITE_ORDER: &str = "filtered-rewrite-order";

const FLOW_RULES: [&str; 11] = [
    VALUE_FLOW_SINKS,
    MACHINERY_SOURCE_BOUND,
    OBSERVER_LIST,
    NATIVE_MAP_PENDING,
    BUILT_LITERAL,
    FILTERED_ENUMERATION,
    INSTANCE_RECEIVERS,
    STORAGE_PROXY,
    FUNCTION_VALUES,
    AWAITED_ARRAYS,
    FILTERED_REWRITE_ORDER,
];

thread_local! {
    static DISABLED: RefCell<Vec<&'static str>> = const { RefCell::new(Vec::new()) };
}

#[must_use]
pub struct FlowRulesGuard {
    previous: Vec<&'static str>,
}

impl Drop for FlowRulesGuard {
    fn drop(&mut self) {
        let restored = std::mem::take(&mut self.previous);
        DISABLED.with(|cell| *cell.borrow_mut() = restored);
    }
}

pub fn assume_disabled_flow_rules(rules: &[&'static str]) -> FlowRulesGuard {
    let wanted: Vec<&'static str> = rules
        .iter()
        .copied()
        .filter(|rule| FLOW_RULES.contains(rule))
        .collect();
    let previous = DISABLED.with(|cell| cell.replace(wanted));
    FlowRulesGuard { previous }
}

pub(crate) fn flow_rule_disabled(rule: &'static str) -> bool {
    DISABLED.with(|cell| cell.borrow().contains(&rule))
}
