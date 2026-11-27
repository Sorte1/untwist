use oxc_allocator::Allocator;
use untwist_core::parse::parse_program;
use untwist_core::semantic::build_scoping;
use untwist_rename::library::shape::statement_shapes;
use untwist_rename::library::table::{loose_key, order_key, Role, RowData, Table};

fn shapes(source: &str) -> Vec<(u32, u64)> {
    let alloc = Allocator::default();
    let parsed = parse_program(&alloc, source);
    let sem = build_scoping(&parsed.program);
    statement_shapes(&parsed.program, &sem.scoping)
        .into_iter()
        .map(|(_, loose)| loose)
        .collect()
}

const FILE: &str = "var Vector = function (x, y) { this.x = x; this.y = y; };\nVector.prototype.add = function (other) { return new Vector(this.x + other.x, this.y + other.y); };\nvar ORIGIN = new Vector(0, 0);\nVector.zero = ORIGIN;\n";

#[test]
fn each_top_level_statement_has_a_shape_that_ignores_names() {
    let renamed = FILE
        .replace("Vector", "a")
        .replace("ORIGIN", "b")
        .replace("other", "c");
    let left = shapes(FILE);
    assert_eq!(left.len(), 4, "{left:?}");
    assert_eq!(left, shapes(&renamed));
}

#[test]
fn a_statement_keeps_its_shape_wherever_it_stands() {
    let shuffled = "var ORIGIN = new Vector(0, 0);\nvar Vector = function (x, y) { this.x = x; this.y = y; };\n";
    let ordered = shapes(FILE);
    let moved = shapes(shuffled);
    assert_eq!(moved[0], ordered[2]);
    assert_eq!(moved[1], ordered[0]);
    assert_ne!(ordered[0], ordered[2]);
}

fn order_row(loose: (u32, u64), position: u32) -> RowData {
    RowData {
        loose: loose_key(loose),
        exact: order_key(loose),
        label: "three".to_string(),
        versions: "0.150.0".to_string(),
        module: "build/three.module.js".to_string(),
        role: Role::Order,
        own: Some("rollup.default".to_string()),
        inner: Vec::new(),
        outer: Vec::new(),
        specific: position,
    }
}

#[test]
fn order_rows_round_trip_apart_from_the_match_rows() {
    let rows: Vec<RowData> = shapes(FILE)
        .into_iter()
        .enumerate()
        .map(|(position, loose)| order_row(loose, position as u32))
        .rev()
        .collect();
    let table = Table::from_rows(&rows);
    assert!(table.rows.is_empty());
    assert!(table.is_empty());
    let positions: Vec<u32> = table.order.iter().map(|row| row.specific).collect();
    assert_eq!(positions, [0, 1, 2, 3]);
    assert!(rows.iter().all(|row| !table.has_loose(row.loose)));
    let parsed = Table::parse(&table.write("three")).expect("the written table parses");
    let back: Vec<RowData> = parsed.order.iter().map(|row| parsed.data(row)).collect();
    let mut expected = rows.clone();
    expected.reverse();
    assert_eq!(back, expected);
    let merged = Table::merge(vec![parsed, Table::default()]);
    assert_eq!(merged.order.len(), 4);
}

#[test]
fn an_iife_bundle_is_read_as_the_statements_inside_it() {
    let wrapped = format!("var lib = (function (exports) {{\n{FILE}return exports;\n}})({{}});\n");
    let inner = shapes(&wrapped);
    assert_eq!(inner.len(), 5, "{inner:?}");
    assert_eq!(&inner[..4], shapes(FILE).as_slice());
    let bang = format!("!function () {{\n{FILE}}}();\n");
    assert_eq!(shapes(&bang), shapes(FILE));
}
