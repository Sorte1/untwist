use oxc_allocator::Allocator;
use oxc_semantic::Scoping;
use oxc_syntax::symbol::SymbolId;
use untwist_core::parse::parse_program;
use untwist_core::semantic::build_scoping;
use untwist_rename::usage::{collect_usage, ShorthandKind};

fn symbol(scoping: &Scoping, name: &str) -> SymbolId {
    scoping
        .symbol_ids()
        .find(|&id| scoping.symbol_name(id) == name)
        .expect("symbol")
}

#[test]
fn calls_and_property_reads_are_recorded() {
    let alloc = Allocator::default();
    let parsed = parse_program(
        &alloc,
        "function run(cb, res) { cb(); new run(); res.status(200); res.json({}); }",
    );
    let sem = build_scoping(&parsed.program);
    let usage = collect_usage(&parsed.program, &sem.scoping);

    assert!(usage.called.contains(&symbol(&sem.scoping, "cb")));
    assert!(!usage.called.contains(&symbol(&sem.scoping, "run")));
    assert!(usage.reads_any(symbol(&sem.scoping, "res"), &["status"]));
    assert!(usage.reads_any(symbol(&sem.scoping, "res"), &["json"]));
    assert!(!usage.reads_any(symbol(&sem.scoping, "cb"), &["status"]));
}

#[test]
fn an_object_literal_value_records_its_key() {
    let alloc = Allocator::default();
    let parsed = parse_program(
        &alloc,
        "var t = 1; var n = 2; export default { videoRef: t, cameraRef: n };",
    );
    let sem = build_scoping(&parsed.program);
    let usage = collect_usage(&parsed.program, &sem.scoping);

    assert_eq!(
        usage.object_literal_key(symbol(&sem.scoping, "t")),
        Some("videoRef")
    );
    assert_eq!(
        usage.object_literal_key(symbol(&sem.scoping, "n")),
        Some("cameraRef")
    );
}

#[test]
fn shorthand_hints_cover_patterns_literals_and_import_specifiers() {
    let alloc = Allocator::default();
    let parsed = parse_program(
        &alloc,
        "import { sin as k } from \"m\"; const { cos: v } = Math; const w = 1; use({ abs: w });",
    );
    let sem = build_scoping(&parsed.program);
    let usage = collect_usage(&parsed.program, &sem.scoping);
    let hint = |name: &str| {
        usage
            .hints
            .iter()
            .find(|hint| hint.symbol == symbol(&sem.scoping, name))
            .map(|hint| (hint.key.clone(), hint.kind))
            .expect("hint")
    };

    assert_eq!(
        hint("k"),
        ("sin".to_string(), ShorthandKind::ImportSpecifier)
    );
    assert_eq!(
        hint("v"),
        ("cos".to_string(), ShorthandKind::PatternProperty)
    );
    assert_eq!(
        hint("w"),
        ("abs".to_string(), ShorthandKind::ObjectLiteralValue)
    );
}

#[test]
fn a_computed_key_never_produces_a_hint() {
    let alloc = Allocator::default();
    let parsed = parse_program(&alloc, "const key = 'x'; const w = 1; use({ [key]: w });");
    let sem = build_scoping(&parsed.program);
    let usage = collect_usage(&parsed.program, &sem.scoping);
    assert!(usage.hints.is_empty());
}
