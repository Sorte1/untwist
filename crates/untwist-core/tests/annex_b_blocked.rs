use oxc_allocator::Allocator;
use oxc_ast::ast::IdentifierReference;
use oxc_ast_visit::{walk_js, VisitJs};
use oxc_semantic::Scoping;
use untwist_core::parse::parse_program;
use untwist_core::semantic::build_scoping;

struct Targets<'s> {
    scoping: &'s Scoping,
    name: &'s str,
    found: Vec<Option<u32>>,
}

impl<'a> VisitJs<'a> for Targets<'_> {
    fn visit_identifier_reference(&mut self, it: &IdentifierReference<'a>) {
        if it.name.as_str() == self.name {
            let symbol = it
                .reference_id
                .get()
                .and_then(|id| self.scoping.get_reference(id).symbol_id());
            self.found
                .push(symbol.map(|symbol| self.scoping.symbol_span(symbol).start));
        }
        walk_js::walk_identifier_reference(self, it);
    }
}

fn targets(source: &str, name: &str) -> Vec<Option<u32>> {
    let alloc = Allocator::default();
    let parsed = parse_program(&alloc, source);
    let sem = build_scoping(&parsed.program);
    let mut scan = Targets {
        scoping: &sem.scoping,
        name,
        found: Vec::new(),
    };
    scan.visit_program(&parsed.program);
    let listed: usize = sem
        .scoping
        .symbol_ids()
        .map(|symbol| sem.scoping.get_resolved_reference_ids(symbol).len())
        .sum::<usize>()
        + sem
            .scoping
            .root_unresolved_references()
            .values()
            .map(|ids| ids.len())
            .sum::<usize>();
    assert_eq!(listed, sem.scoping.references_len(), "reference lists");
    for symbol in sem.scoping.symbol_ids() {
        for id in sem.scoping.get_resolved_reference_ids(symbol) {
            assert_eq!(sem.scoping.get_reference(*id).symbol_id(), Some(symbol));
        }
    }
    scan.found
}

fn binding(source: &str, marker: &str, offset: usize) -> Option<u32> {
    let at = source.find(marker).expect("marker") + offset;
    Some(u32::try_from(at).expect("offset"))
}

const OUTER: &str = "var o = 0;";

fn outer(source: &str) -> Option<u32> {
    binding(source, OUTER, 4)
}

#[test]
fn a_let_between_blocks_the_hoist() {
    let source = "var o = 0; function f() { { let o = 1; { function o() {} o(); } o; } return o; }";
    assert_eq!(
        targets(source, "o"),
        [
            binding(source, "function o", 9),
            binding(source, "let o", 4),
            outer(source)
        ]
    );
}

#[test]
fn every_lexical_form_between_blocks_the_hoist() {
    for between in [
        "{ const o = 1; if (o) { { function o() {} } } }",
        "{ class o {} { function o() {} } }",
        "for (let o = 0; o < 1; o++) { function o() {} }",
        "for (let o of [1]) { function o() {} }",
        "for (const o in {a: 1}) { function o() {} }",
        "try { throw 1; } catch ({ o }) { { function o() {} } }",
        "try { throw 1; } catch ([o]) { { function o() {} } }",
        "switch (1) { case 1: let o = 1; { function o() {} } }",
        "{ { function o() {} } let o = 1; }",
    ] {
        let source = format!("var o = 0; function f() {{ {between} return o; }}");
        let found = targets(&source, "o");
        assert_eq!(found.last(), Some(&outer(&source)), "{between}");
    }
}

#[test]
fn a_simple_catch_parameter_does_not_block_the_hoist() {
    let source =
        "var o = 0; function f() { try { throw 1; } catch (o) { { function o() {} } } return o; }";
    assert_eq!(targets(source, "o"), [binding(source, "function o", 9)]);
}

#[test]
fn an_unblocked_block_function_still_hoists() {
    let source = "var o = 0; function f() { { function o() {} } return o; }";
    assert_eq!(targets(source, "o"), [binding(source, "function o", 9)]);
}

#[test]
fn a_blocked_function_at_the_top_level_leaves_the_name_global() {
    let source = "{ let o = 1; { function o() {} } } o;";
    assert_eq!(targets(source, "o"), [None]);
}

#[test]
fn another_unblocked_function_takes_over_the_var() {
    let source =
        "var o = 0; function f() { { { function o() { return 1; } } let o; } { function o() { return 2; } } return o; }";
    assert_eq!(
        targets(source, "o"),
        [binding(source, "function o() { return 2", 9)]
    );
}

#[test]
fn a_later_var_of_the_function_keeps_its_own_binding() {
    let source =
        "var o = 0; function f() { { let o = 1; { function o() {} o; } } var o = 5; return o; }";
    let found = targets(source, "o");
    let var = binding(source, "var o = 5", 4);
    assert_eq!(found, [binding(source, "function o", 9), var]);
}

#[test]
fn a_block_function_in_a_block_between_blocks_only_the_inner_one() {
    for source in [
        "var o = 0; function f() { { function o() { return 2; } { function o() { return 1; } } } return o; }",
        "var o = 0; function f() { { { function o() { return 1; } } function o() { return 2; } } return o; }",
    ] {
        let found = targets(source, "o");
        assert_eq!(found, [binding(source, "function o() { return 2", 9)], "{source}");
    }
}
