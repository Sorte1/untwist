use oxc_allocator::Allocator;
use oxc_ast::ast::*;
use untwist_unpack::webpack::container::{
    container_is_empty, is_module_container, module_factories,
};

fn container<'a>(alloc: &'a Allocator, source: &'a str) -> Expression<'a> {
    let parsed = untwist_core::parse::parse_program(alloc, source);
    let Some(Statement::ExpressionStatement(stmt)) = parsed.program.body.into_iter().next() else {
        panic!("expression statement");
    };
    stmt.unbox().expression
}

#[test]
fn array_containers_use_the_index_as_the_id_and_skip_holes() {
    let alloc = Allocator::default();
    let expr = container(&alloc, "[, function (m, e, r) { a(); }, (m) => { b(); }];");
    assert!(is_module_container(&expr));
    let factories = module_factories(&expr);
    assert_eq!(
        factories
            .iter()
            .map(|(id, _)| id.as_str())
            .collect::<Vec<_>>(),
        vec!["1", "2"]
    );
    assert_eq!(factories[0].1.body.statements.len(), 1);
}

#[test]
fn object_containers_accept_identifier_string_and_numeric_keys() {
    let alloc = Allocator::default();
    let expr = container(
        &alloc,
        "({ 386: function () {}, E8gZ: function () {}, '2wwy': function () {} });",
    );
    assert!(is_module_container(&expr));
    assert_eq!(
        module_factories(&expr)
            .iter()
            .map(|(id, _)| id.as_str())
            .collect::<Vec<_>>(),
        vec!["386", "E8gZ", "2wwy"]
    );
}

#[test]
fn object_methods_are_containers_too() {
    let alloc = Allocator::default();
    let expr = container(&alloc, "({ 2(module, exports, require) { x(); } });");
    assert!(is_module_container(&expr));
    assert_eq!(module_factories(&expr).len(), 1);
}

#[test]
fn the_public_path_property_is_accepted_and_dropped() {
    let alloc = Allocator::default();
    let expr = container(&alloc, "({ c: '', 0: function () {} });");
    assert!(is_module_container(&expr));
    assert_eq!(
        module_factories(&expr)
            .iter()
            .map(|(id, _)| id.as_str())
            .collect::<Vec<_>>(),
        vec!["0"]
    );
}

#[test]
fn non_containers_are_rejected() {
    let alloc = Allocator::default();
    for source in [
        "({ a: 1 });",
        "[1, 2];",
        "({ 0: (a) => a });",
        "({ ...spread });",
        "({ 0: function* () {} });",
        "({ [0]: function () {} });",
        "x;",
    ] {
        let expr = container(&alloc, source);
        assert!(!is_module_container(&expr), "{source}");
    }
}

#[test]
fn module_factories_rejects_computed_and_accessor_properties() {
    let alloc = Allocator::default();
    for source in [
        "({ [0]: function () {} });",
        "({ get 0() { return function () {}; } });",
    ] {
        let expr = container(&alloc, source);
        assert!(module_factories(&expr).is_empty(), "{source}");
    }
}

#[test]
fn empty_containers_are_detected() {
    let alloc = Allocator::default();
    assert!(container_is_empty(&container(&alloc, "({});")));
    assert!(container_is_empty(&container(&alloc, "[];")));
    assert!(!container_is_empty(&container(&alloc, "[function () {}];")));
}
