use oxc_allocator::Allocator;
use untwist_core::parse::parse_program;
use untwist_core::semantic::build_scoping;
use untwist_rename::library::shape::UnitKind;
use untwist_rename::library::table::{exact_key, loose_key, Role, RowData, Table};
use untwist_rename::library::{match_with, unit_records, STRONG_LENGTH, STRONG_SPECIFIC};

const FUNCTIONS: [&str; 4] = [
    "function alpha(value) { if (typeof value !== \"string\") throw new TypeError(\"alpha wants a string\"); return value.split(\",\").map(function (part) { return part.trim().toUpperCase(); }).join(\"|\"); }",
    "function beta(list) { var total = 0; for (var i = 0; i < list.length; i++) { total += list[i].weight * Math.pow(2, i); } return total > 1000 ? \"heavy\" : \"light\"; }",
    "function gamma(node) { while (node && node.nodeType !== 1) node = node.parentNode; return node ? node.getAttribute(\"data-gamma\") || \"none\" : \"detached\"; }",
    "function delta(map, key) { if (!Object.prototype.hasOwnProperty.call(map, key)) { map[key] = { count: 0, first: Date.now() }; } map[key].count++; return map[key]; }",
];

fn source(count: usize) -> String {
    let mut text = FUNCTIONS[..count].join("\n");
    text.push_str("\nconsole.log(");
    let calls: Vec<String> = FUNCTIONS[..count]
        .iter()
        .map(|function| function[9..function.find('(').unwrap()].to_string())
        .collect();
    text.push_str(&calls.join(", "));
    text.push_str(");\n");
    text
}

fn rows(source: &str, label: &str) -> Vec<RowData> {
    let alloc = Allocator::default();
    let parsed = parse_program(&alloc, source);
    let sem = build_scoping(&parsed.program);
    unit_records(&parsed.program, &sem.scoping)
        .into_iter()
        .filter(|record| record.kind == UnitKind::Function && record.binding.is_some())
        .filter(|record| record.loose.0 >= STRONG_LENGTH && record.specific >= STRONG_SPECIFIC)
        .map(|record| RowData {
            loose: loose_key(record.loose),
            exact: exact_key(record.exact),
            label: label.to_string(),
            versions: "1.0.0".to_string(),
            module: "index".to_string(),
            role: Role::Function,
            own: record.binding.clone(),
            inner: Vec::new(),
            outer: Vec::new(),
            specific: record.specific,
        })
        .collect()
}

fn labels(table: &Table, source: &str) -> Vec<String> {
    let alloc = Allocator::default();
    let parsed = parse_program(&alloc, source);
    let sem = build_scoping(&parsed.program);
    let mut labels: Vec<String> = match_with(table, &parsed.program, &sem.scoping)
        .hits
        .iter()
        .map(|hit| hit.label.clone())
        .collect();
    labels.sort();
    labels.dedup();
    labels
}

fn collection(rows: &[RowData]) -> Table {
    Table::from_rows(rows).into_collection(&Table::default())
}

#[test]
fn every_test_function_is_a_strong_unit() {
    assert_eq!(rows(&source(4), "pkg").len(), 4);
}

#[test]
fn a_collection_package_with_two_hits_in_a_file_is_not_accepted() {
    let text = source(2);
    let table = collection(&rows(&text, "pkg"));
    assert!(labels(&table, &text).is_empty());
}

#[test]
fn three_hits_of_one_collection_package_are_accepted() {
    let text = source(3);
    let table = collection(&rows(&text, "pkg"));
    assert_eq!(labels(&table, &text), ["pkg"]);
}

#[test]
fn the_cluster_rule_leaves_built_in_tables_alone() {
    let text = source(1);
    let table = Table::from_rows(&rows(&text, "helper"));
    assert_eq!(labels(&table, &text), ["helper"]);
}

#[test]
fn a_shape_three_collection_packages_share_is_not_evidence() {
    let text = source(4);
    let mut all = rows(&text, "own");
    let first = all[0].clone();
    for label in ["one", "two", "three"] {
        all.push(RowData {
            label: label.to_string(),
            ..first.clone()
        });
    }
    let table = collection(&all);
    assert_eq!(labels(&table, &text), ["own"]);
    let found = {
        let alloc = Allocator::default();
        let parsed = parse_program(&alloc, &text);
        let sem = build_scoping(&parsed.program);
        match_with(&table, &parsed.program, &sem.scoping).hits.len()
    };
    assert_eq!(found, 3);
}

#[test]
fn collection_rows_the_built_in_tables_already_hold_are_dropped() {
    let text = source(3);
    let known = Table::from_rows(&rows(&text, "helper")[..1]);
    let table = collection(&rows(&text, "pkg")).into_collection(&known);
    assert_eq!(table.rows.len(), 2);
    let merged = Table::merge(vec![known, table]);
    assert_eq!(labels(&merged, &text), ["helper"]);
}
