use std::collections::{BTreeMap, BTreeSet};

use untwist_rename::library::table::{Role, RowData as Row};

pub(crate) const VENDORED_SHARE: f64 = 0.5;
const RUNTIME_PREFIX: &str = "__";

pub(crate) fn runtime_named(row: &Row) -> bool {
    row.own
        .as_deref()
        .is_some_and(|own| own.starts_with(RUNTIME_PREFIX))
}

fn label_keys(rows: &[Row]) -> BTreeMap<&str, BTreeSet<u64>> {
    let mut keys: BTreeMap<&str, BTreeSet<u64>> = BTreeMap::new();
    for row in rows.iter().filter(|row| row.role != Role::Order) {
        keys.entry(row.label.as_str())
            .or_default()
            .insert(row.exact);
    }
    keys
}

fn owns(small: (&str, usize), large: (&str, usize)) -> bool {
    (small.1, small.0) < (large.1, large.0)
}

pub(crate) fn vendored_copies(rows: &[Row]) -> BTreeSet<(String, u64)> {
    let keys = label_keys(rows);
    let mut holders: BTreeMap<u64, Vec<&str>> = BTreeMap::new();
    for (label, exacts) in &keys {
        for exact in exacts {
            holders.entry(*exact).or_default().push(label);
        }
    }
    let mut shared: BTreeMap<(&str, &str), usize> = BTreeMap::new();
    for labels in holders.values().filter(|labels| labels.len() > 1) {
        for a in labels {
            for b in labels.iter().filter(|b| *b != a) {
                *shared.entry((a, b)).or_default() += 1;
            }
        }
    }
    let mut copies = BTreeSet::new();
    for ((owner, copy), count) in shared {
        let owner_size = keys[owner].len();
        let vendored = count as f64 >= owner_size as f64 * VENDORED_SHARE;
        if !vendored || !owns((owner, owner_size), (copy, keys[copy].len())) {
            continue;
        }
        for exact in keys[owner].intersection(&keys[copy]) {
            copies.insert((copy.to_string(), *exact));
        }
    }
    copies
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(exact: u64, label: &str) -> Row {
        Row {
            loose: 1,
            exact,
            label: label.to_string(),
            versions: "1.0.0".to_string(),
            module: "index.js".to_string(),
            role: Role::Function,
            own: None,
            inner: Vec::new(),
            outer: Vec::new(),
            specific: 3,
        }
    }

    #[test]
    fn a_package_that_vendors_most_of_another_gives_up_the_shared_rows() {
        let mut rows: Vec<Row> = (1..=4).map(|exact| row(exact, "eventemitter3")).collect();
        rows.extend((1..=3).map(|exact| row(exact, "phaser")));
        rows.extend((10..=40).map(|exact| row(exact, "phaser")));
        rows.extend([row(10, "other"), row(50, "other"), row(51, "other")]);
        let copies = vendored_copies(&rows);
        let expected: BTreeSet<(String, u64)> =
            (1..=3).map(|exact| ("phaser".to_string(), exact)).collect();
        assert_eq!(copies, expected);
    }

    #[test]
    fn a_double_underscore_name_is_a_bundler_runtime_row() {
        let mut runtime = row(1, "phaser");
        runtime.own = Some("__webpack_require__".to_string());
        assert!(runtime_named(&runtime));
        assert!(!runtime_named(&row(2, "phaser")));
    }
}
