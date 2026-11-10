mod common;

use common::parse;
use oxc_allocator::Allocator;
use untwist_testing::read_unpack_gap_fixture;
use untwist_unpack::webpack::require_fn::{
    find_assigned_entry_id, find_required_entry_id, require_function, Dispatchers,
};

const WEBPACK4: &str = "function n(t) {\n  if (a[t]) { return a[t].exports; }\n  var r = a[t] = {};\n  e[t].call(r.exports, r, r.exports, n);\n  return r.exports;\n}\n";
const WEBPACK5: &str = "function __webpack_require__(moduleId) {\n  var module = {};\n  __webpack_modules__[moduleId](module, module.exports, __webpack_require__);\n  return module.exports;\n}\n";

#[test]
fn matches_the_call_form_and_the_direct_form() {
    let alloc = Allocator::default();
    let four = parse(&alloc, WEBPACK4);
    let five = parse(&alloc, WEBPACK5);
    let matched = require_function(&four.body[0], "e").expect("webpack 4 shape");
    assert_eq!(matched.name, "n");
    let matched = require_function(&five.body[0], "__webpack_modules__").expect("webpack 5 shape");
    assert_eq!(matched.name, "__webpack_require__");
}

#[test]
fn the_container_name_must_match() {
    let alloc = Allocator::default();
    let program = parse(&alloc, WEBPACK4);
    assert!(require_function(&program.body[0], "installedModules").is_none());
}

#[test]
fn rejects_var_functions_arrows_and_multi_param_functions() {
    let alloc = Allocator::default();
    for source in [
        "var n = function (t) { e[t].call(x); };",
        "const n = (t) => { e[t].call(x); };",
        "function n(t, u) { e[t].call(x); }",
        "function n() { e[t].call(x); }",
        "function n(t) { var q = e[t]; }",
    ] {
        let program = parse(&alloc, source);
        assert!(
            require_function(&program.body[0], "e").is_none(),
            "{source}"
        );
    }
}

#[test]
fn assigned_entry_id_accepts_both_member_forms() {
    let alloc = Allocator::default();
    let program = parse(&alloc, "n.p = '';\nn(n.s = 2);\n");
    assert_eq!(
        find_assigned_entry_id(&program.body, "n", None),
        Some("2".to_string())
    );
    let program = parse(&alloc, "n(n['s'] = '2wwy');\n");
    assert_eq!(
        find_assigned_entry_id(&program.body, "n", None),
        Some("2wwy".to_string())
    );
    let program = parse(&alloc, "n.x = 2;\n");
    assert_eq!(find_assigned_entry_id(&program.body, "n", None), None);
}

#[test]
fn required_entry_id_finds_the_first_literal_call() {
    let alloc = Allocator::default();
    let program = parse(&alloc, "require.e = function () {};\nreturn require(0);\n");
    assert_eq!(
        find_required_entry_id(&program.body, "require", None),
        Some("0".to_string())
    );
    let program = parse(&alloc, "require(a);\n");
    assert_eq!(find_required_entry_id(&program.body, "require", None), None);
}

#[test]
fn the_skipped_statement_is_not_scanned() {
    let alloc = Allocator::default();
    let program = parse(
        &alloc,
        "var modules = { 0: function () { require(9); } };\nrequire(require.s = 3);\n",
    );
    assert_eq!(
        find_required_entry_id(&program.body, "require", Some(0)),
        None
    );
    assert_eq!(
        find_assigned_entry_id(&program.body, "require", Some(0)),
        Some("3".to_string())
    );
}

#[test]
fn a_chunk_id_that_collides_with_a_module_id_is_never_read_as_an_entry() {
    let source = read_unpack_gap_fixture("webpack-5-chunk-id-j.js");
    let alloc = oxc_allocator::Allocator::default();
    let parsed = untwist_core::parse::parse_program(&alloc, &source);
    let bundle = untwist_unpack::unpack_program(&alloc, &parsed.program).expect("a webpack bundle");
    assert_eq!(bundle.entry_id, None);
    assert_eq!(bundle.modules.len(), 2);
}

#[test]
fn a_runtime_helper_property_is_never_an_entry_marker() {
    for helper in ["j", "e", "d", "m", "r"] {
        let source = format!(
            "(() => {{\n  var __webpack_modules__ = {{ 1: (module) => {{ module.exports = 1; }} }};\n  var __webpack_module_cache__ = {{}};\n  function __webpack_require__(id) {{\n    var cached = __webpack_module_cache__[id];\n    if (cached !== undefined) return cached.exports;\n    var m = __webpack_module_cache__[id] = {{ exports: {{}} }};\n    __webpack_modules__[id](m, m.exports, __webpack_require__);\n    return m.exports;\n  }}\n  __webpack_require__.{helper} = 1;\n}})();"
        );
        let alloc = oxc_allocator::Allocator::default();
        let parsed = untwist_core::parse::parse_program(&alloc, &source);
        let bundle =
            untwist_unpack::unpack_program(&alloc, &parsed.program).expect("a webpack bundle");
        assert_eq!(
            bundle.entry_id, None,
            "__webpack_require__.{helper} was read as an entry marker"
        );
    }
}

#[test]
fn a_bundle_with_several_top_level_requires_picks_the_first_and_says_so() {
    let source = read_unpack_gap_fixture("webpack-4-no-canonical-entry.js");
    let alloc = oxc_allocator::Allocator::default();
    let parsed = untwist_core::parse::parse_program(&alloc, &source);
    let mut bundle =
        untwist_unpack::unpack_program(&alloc, &parsed.program).expect("a webpack bundle");
    assert_eq!(bundle.entry_id.as_deref(), Some("9896"));
    assert_eq!(bundle.entry_candidates, 3);
    let notes = untwist_unpack::finish(&alloc, &mut bundle, &[]);
    assert!(
        notes
            .iter()
            .any(|note| note.message.contains("first of 3 top-level require calls")),
        "{notes:?}"
    );
}

#[test]
fn a_bundle_with_exactly_one_top_level_require_names_it_and_says_nothing_about_candidates() {
    let source = read_unpack_gap_fixture("webpack-4-no-canonical-entry.js").replacen(
        "  __webpack_require__(3021);\n  __webpack_require__(775);\n",
        "",
        1,
    );
    assert_eq!(source.matches("__webpack_require__(9896)").count(), 1);
    assert!(!source.contains("__webpack_require__(3021)"), "{source}");
    let alloc = oxc_allocator::Allocator::default();
    let parsed = untwist_core::parse::parse_program(&alloc, &source);
    let mut bundle =
        untwist_unpack::unpack_program(&alloc, &parsed.program).expect("a webpack bundle");
    assert_eq!(bundle.entry_id.as_deref(), Some("9896"));
    assert_eq!(bundle.entry_candidates, 1);
    let notes = untwist_unpack::finish(&alloc, &mut bundle, &[]);
    assert!(
        !notes
            .iter()
            .any(|note| note.message.contains("top-level require calls")),
        "{notes:?}"
    );
}

#[test]
fn a_bundle_with_no_top_level_require_says_nothing_about_candidates() {
    let source = read_unpack_gap_fixture("webpack-5-chunk-id-j.js");
    let alloc = oxc_allocator::Allocator::default();
    let parsed = untwist_core::parse::parse_program(&alloc, &source);
    let mut bundle =
        untwist_unpack::unpack_program(&alloc, &parsed.program).expect("a webpack bundle");
    assert_eq!(bundle.entry_candidates, 0);
    let notes = untwist_unpack::finish(&alloc, &mut bundle, &[]);
    assert!(
        !notes
            .iter()
            .any(|note| note.message.contains("top-level require calls")),
        "{notes:?}"
    );
}

#[test]
fn a_sparse_module_table_yields_only_its_real_modules() {
    let source = read_unpack_gap_fixture("webpack-4-no-canonical-entry.js");
    let alloc = oxc_allocator::Allocator::default();
    let parsed = untwist_core::parse::parse_program(&alloc, &source);
    let bundle = untwist_unpack::unpack_program(&alloc, &parsed.program).expect("a webpack bundle");
    assert_eq!(bundle.modules.len(), 1);
    assert!(
        bundle.modules.contains_key("2"),
        "{:?}",
        bundle.modules.keys().collect::<Vec<_>>()
    );
}

#[test]
fn the_first_dispatcher_after_a_container_is_its_require_function() {
    let alloc = Allocator::default();
    let source = "function early(i) { c[i](); }\nvar c = [function () {}];\nfunction first(i) { c[i].call(0); c[i](); }\nfunction second(i) { c[i](); d[i](); }\n";
    let program = parse(&alloc, source);
    let dispatchers = Dispatchers::of(&program.body);
    assert!(!dispatchers.is_empty());
    assert_eq!(dispatchers.first_after("c", 1), Some("first"));
    assert_eq!(dispatchers.first_after("c", 2), Some("second"));
    assert_eq!(dispatchers.first_after("c", 3), None);
    assert_eq!(dispatchers.first_after("d", 0), Some("second"));
    assert_eq!(dispatchers.first_after("e", 0), None);
}

#[test]
fn a_statement_list_without_a_dispatcher_has_no_require_function() {
    let alloc = Allocator::default();
    let program = parse(
        &alloc,
        "var c = [function () {}];\nfunction n(t) { var q = c[t]; }\n",
    );
    assert!(Dispatchers::of(&program.body).is_empty());
}
