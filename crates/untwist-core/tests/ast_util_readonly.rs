use oxc_allocator::Allocator;
use oxc_ast::ast::Statement;
use oxc_span::Span;
use oxc_syntax::symbol::SymbolId;
use untwist_core::ast_util::readonly::{
    is_readonly_object, is_readonly_object_outside, scan_object_uses, scan_symbol_uses, ObjectUses,
};
use untwist_core::ast_util::scope::binding_symbol;
use untwist_core::parse::parse_program;
use untwist_core::semantic::build_scoping;

fn first_declarator_symbol(program: &oxc_ast::ast::Program<'_>) -> SymbolId {
    let Statement::VariableDeclaration(declaration) = &program.body[0] else {
        panic!("variable declaration");
    };
    binding_symbol(&declaration.declarations[0].id).expect("bound")
}

fn readonly(source: &str) -> bool {
    is_readonly_object(&uses(source))
}

fn uses(source: &str) -> ObjectUses {
    let alloc = Allocator::default();
    let parsed = parse_program(&alloc, source);
    let sem = build_scoping(&parsed.program);
    let symbol = first_declarator_symbol(&parsed.program);
    scan_object_uses(&parsed.program, &sem.scoping, symbol)
}

#[test]
fn plain_property_reads_are_readonly() {
    assert!(readonly("const a = { x: 1 };\nconsole.log(a.x);"));
    assert!(readonly("const a = { x: 1 };\nconsole.log(a['x'], a.x);"));
}

#[test]
fn every_reference_negative_is_rejected() {
    assert!(!readonly("const a = { x: 1 };\nfn(a);\nconsole.log(a.x);"));
    assert!(!readonly(
        "let a = { x: 1 };\na = { x: 2 };\nconsole.log(a.x);"
    ));
    assert!(!readonly(
        "const a = { x: 1 };\na.x = 2;\nconsole.log(a.x);"
    ));
    assert!(!readonly(
        "let a = { x: 1 };\n[a.x] = [2];\nconsole.log(a.x);"
    ));
    assert!(!readonly(
        "let a = { x: 1 };\n({ x: a.x } = { x: 2 });\nconsole.log(a.x);"
    ));
    assert!(!readonly(
        "const a = { x: 1 };\ndelete a.x;\nconsole.log(a.x);"
    ));
    assert!(!readonly("const a = { x: 1 };\na.x++;\nconsole.log(a.x);"));
    assert!(!readonly("const a = { x: 1 };"));
}

#[test]
fn deleting_a_target_property_is_one_write_and_no_read() {
    let scanned = uses("var a = { x: 1 };\ndelete a.x;");
    assert_eq!(scanned.writes, 1);
    assert!(scanned.reads.is_empty());
    assert_eq!(scanned.opaque, 0);
}

#[test]
fn reads_nested_inside_a_foreign_delete_are_still_counted() {
    let scanned = uses("var a = { p: 1 };\ndelete window[a.p];\nconsole.log(a.p);");
    assert!(is_readonly_object(&scanned));
    let names: Vec<&str> = scanned
        .reads
        .iter()
        .map(|read| read.name.as_str())
        .collect();
    assert_eq!(names, vec!["p", "p"]);
    assert_eq!(scanned.writes, 0);
    assert_eq!(scanned.opaque, 0);
}

#[test]
fn a_dynamic_key_delete_leaves_the_object_opaque() {
    let scanned = uses("var a = { k: \"x\", x: 1 };\ndelete a[a.k];");
    assert_eq!(scanned.writes, 0);
    assert_eq!(scanned.opaque, 1);
    let names: Vec<&str> = scanned
        .reads
        .iter()
        .map(|read| read.name.as_str())
        .collect();
    assert_eq!(names, vec!["k"]);
}

#[test]
fn the_property_names_that_were_read_are_reported() {
    let alloc = Allocator::default();
    let parsed = parse_program(&alloc, "const a = { c: 1, d: 2 };\nf(a.c, a['d'], a.c);");
    let sem = build_scoping(&parsed.program);
    let symbol = first_declarator_symbol(&parsed.program);
    let uses = scan_object_uses(&parsed.program, &sem.scoping, symbol);
    let mut names: Vec<&str> = uses.reads.iter().map(|read| read.name.as_str()).collect();
    names.sort_unstable();
    assert_eq!(names, vec!["c", "c", "d"]);
}

#[test]
fn the_set_form_matches_the_one_symbol_form_for_two_objects() {
    let source = "const a = { x: 1 };\nconst b = { y: 2 };\nconsole.log(a.x, b.y, b);\nb.y = 3;\ndelete a.x;";
    let alloc = Allocator::default();
    let parsed = parse_program(&alloc, source);
    let sem = build_scoping(&parsed.program);
    let Statement::VariableDeclaration(first) = &parsed.program.body[0] else {
        panic!("variable declaration");
    };
    let Statement::VariableDeclaration(second) = &parsed.program.body[1] else {
        panic!("variable declaration");
    };
    let a = binding_symbol(&first.declarations[0].id).expect("bound");
    let b = binding_symbol(&second.declarations[0].id).expect("bound");

    let mut symbols = rustc_hash::FxHashSet::default();
    symbols.insert(a);
    symbols.insert(b);
    let together = scan_symbol_uses(&parsed.program, &sem.scoping, &symbols);

    for symbol in [a, b] {
        let alone = scan_object_uses(&parsed.program, &sem.scoping, symbol);
        let set = together
            .of(symbol)
            .expect("every wanted symbol has an entry");
        assert_eq!(set.writes, alone.writes, "writes");
        assert_eq!(set.opaque, alone.opaque, "opaque");
        assert_eq!(
            set.reads
                .iter()
                .map(|read| read.name.as_str())
                .collect::<Vec<_>>(),
            alone
                .reads
                .iter()
                .map(|read| read.name.as_str())
                .collect::<Vec<_>>(),
            "reads"
        );
    }
    assert_eq!(
        together.references_of(a).len(),
        1,
        "every identifier expression resolving to a, member objects included"
    );
    assert_eq!(
        together.references_of(b).len(),
        3,
        "every identifier expression resolving to b, member objects included"
    );
}

#[test]
fn spans_are_reported_for_every_write_and_opaque_use() {
    let alloc = Allocator::default();
    let parsed = parse_program(
        &alloc,
        "const a = { x: 1 };\na.x = 2;\nfn(a);\nconsole.log(a.x);",
    );
    let sem = build_scoping(&parsed.program);
    let symbol = first_declarator_symbol(&parsed.program);
    let uses = scan_object_uses(&parsed.program, &sem.scoping, symbol);
    assert_eq!(uses.writes as usize, uses.write_spans.len());
    assert_eq!(uses.opaque as usize, uses.opaque_spans.len());
    assert_eq!(uses.writes, 1);
    assert_eq!(uses.opaque, 1);
    assert!(!is_readonly_object_outside(&uses, &[]));
    let everything = [Span::new(0, parsed.program.source_text.len() as u32)];
    assert!(is_readonly_object_outside(&uses, &everything));
}
