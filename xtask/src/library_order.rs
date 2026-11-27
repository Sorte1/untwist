use std::collections::BTreeMap;

use oxc_span::Span;

pub(crate) const MIN_ORDER_STATEMENTS: usize = 16;

pub(crate) fn order_family(
    bundler: &str,
    minifier: &str,
    obfuscated: bool,
) -> Option<&'static str> {
    if obfuscated {
        return None;
    }
    match (bundler, minifier) {
        (bundler, "none") if bundler.starts_with("rollup") => Some("rollup"),
        (bundler, _) if bundler.starts_with("vite") => Some("vite"),
        _ => None,
    }
}

pub(crate) fn statement_modules(
    spans: &[Span],
    contexts: &[(Span, String)],
) -> Vec<Option<String>> {
    let direct: Vec<Option<&str>> = spans
        .iter()
        .map(|span| {
            let mut inside = contexts
                .iter()
                .filter(|(context, _)| span.start <= context.start && context.end <= span.end)
                .map(|(_, module)| module.as_str());
            let first = inside.next()?;
            inside.all(|module| module == first).then_some(first)
        })
        .collect();
    let mut filled: Vec<Option<String>> = direct
        .iter()
        .map(|module| module.map(str::to_string))
        .collect();
    let mut last: Option<(usize, &str)> = None;
    for (index, module) in direct.iter().enumerate() {
        let Some(module) = module else {
            continue;
        };
        if let Some((previous, known)) = last {
            if known == *module {
                for slot in &mut filled[previous + 1..index] {
                    *slot = Some(module.to_string());
                }
            }
        }
        last = Some((index, module));
    }
    filled
}

pub(crate) fn first_lane_per_file<T>(
    lanes: Vec<Vec<T>>,
    file: impl Fn(&T) -> (String, String),
) -> Vec<T> {
    let mut taken: BTreeMap<(String, String), usize> = BTreeMap::new();
    let mut kept = Vec::new();
    for (lane, rows) in lanes.into_iter().enumerate() {
        for row in rows {
            let owner = *taken.entry(file(&row)).or_insert(lane);
            if owner == lane {
                kept.push(row);
            }
        }
    }
    kept
}

pub(crate) struct OrderRow {
    pub(crate) module: String,
    pub(crate) loose: (u32, u64),
    pub(crate) position: u32,
}

pub(crate) fn order_rows(
    shapes: &[(Span, (u32, u64))],
    contexts: &[(Span, String)],
) -> Vec<OrderRow> {
    let spans: Vec<Span> = shapes.iter().map(|(span, _)| *span).collect();
    let modules = statement_modules(&spans, contexts);
    let mut counts: BTreeMap<&str, usize> = BTreeMap::new();
    for module in modules.iter().flatten() {
        *counts.entry(module.as_str()).or_default() += 1;
    }
    let mut positions: BTreeMap<&str, u32> = BTreeMap::new();
    let mut rows = Vec::new();
    for ((_, loose), module) in shapes.iter().zip(&modules) {
        let Some(module) = module.as_deref() else {
            continue;
        };
        if counts[module] < MIN_ORDER_STATEMENTS {
            continue;
        }
        let position = positions.entry(module).or_default();
        rows.push(OrderRow {
            module: module.to_string(),
            loose: *loose,
            position: *position,
        });
        *position += 1;
    }
    rows
}

#[cfg(test)]
mod tests {
    use super::*;

    fn span(start: u32) -> Span {
        Span::new(start, start + 1)
    }

    #[test]
    fn a_statement_holding_declarations_of_two_modules_belongs_to_neither() {
        let spans = vec![Span::new(0, 10), Span::new(10, 20)];
        let contexts = vec![
            (Span::new(1, 3), "a".to_string()),
            (Span::new(4, 6), "b".to_string()),
            (Span::new(12, 14), "b".to_string()),
        ];
        let modules = statement_modules(&spans, &contexts);
        assert_eq!(modules, [None, Some("b".to_string())]);
    }

    #[test]
    fn statements_between_two_of_one_module_belong_to_it() {
        let spans: Vec<Span> = (0..6).map(span).collect();
        let contexts = vec![
            (span(0), "a".to_string()),
            (span(2), "a".to_string()),
            (span(4), "b".to_string()),
        ];
        let modules = statement_modules(&spans, &contexts);
        let names: Vec<Option<&str>> = modules.iter().map(Option::as_deref).collect();
        assert_eq!(
            names,
            [Some("a"), Some("a"), Some("a"), None, Some("b"), None]
        );
    }

    #[test]
    fn a_file_keeps_the_rows_of_the_first_lane_that_has_it() {
        let lanes = vec![vec![("a", 1), ("a", 2)], vec![("a", 3), ("b", 4)]];
        let kept = first_lane_per_file(lanes, |row| (row.0.to_string(), String::new()));
        assert_eq!(kept, [("a", 1), ("a", 2), ("b", 4)]);
    }

    #[test]
    fn only_hoisting_builds_that_keep_their_statements_give_order_rows() {
        assert_eq!(order_family("rollup-cjs", "none", false), Some("rollup"));
        assert_eq!(order_family("rollup-cjs", "terser", false), None);
        assert_eq!(order_family("vite8", "oxc", false), Some("vite"));
        assert_eq!(order_family("vite8", "oxc", true), None);
        assert_eq!(order_family("webpack5", "none", false), None);
        assert_eq!(order_family("esbuild", "none", false), None);
    }
}
