mod common;

use common::PrintedModule;
use common::{first_expression_statement, fixture, module_summary, parse, prepared};
use oxc_allocator::Allocator;
use untwist_testing::read_unpack_gap_fixture;
use untwist_unpack::webpack::chunk::detect_webpack_chunk;

#[test]
fn unpacks_an_object_container_chunk_with_hashed_ids() {
    let source = fixture("webpack-jsonp-chunk.js");
    let alloc = Allocator::default();
    let program = prepared(&alloc, &source);
    let bundle = detect_webpack_chunk(&alloc, &program, first_expression_statement(&program))
        .expect("chunk bundle");
    assert_eq!(bundle.entry_id, None);
    assert_eq!(bundle.chunk_ids, vec!["15".to_string()]);
    assert_eq!(
        module_summary(&bundle),
        vec![
            ("2wwy".to_string(), "2wwy.js".to_string(), false),
            ("E8gZ".to_string(), "E8gZ.js".to_string(), false),
        ]
    );
    assert_eq!(
        bundle.modules["2wwy"].code().trim_end(),
        "const a = require(\"E8gZ\");\nconst b = require(\"w6GO\");"
    );
}

#[test]
fn unpacks_an_array_container_chunk_with_no_chunk_ids() {
    let source = fixture("webpack-5-json.js");
    let alloc = Allocator::default();
    let program = prepared(&alloc, &source);
    let bundle = detect_webpack_chunk(&alloc, &program, first_expression_statement(&program))
        .expect("chunk bundle");
    assert!(bundle.chunk_ids.is_empty());
    assert_eq!(
        module_summary(&bundle),
        vec![
            ("0".to_string(), "0.js".to_string(), false),
            ("1".to_string(), "1.js".to_string(), false),
        ]
    );
}

#[test]
fn a_trailing_third_element_is_ignored() {
    let source = fixture("webpack-chunk-no-entries.js");
    let alloc = Allocator::default();
    let program = prepared(&alloc, &source);
    let bundle = detect_webpack_chunk(&alloc, &program, first_expression_statement(&program))
        .expect("chunk bundle");
    assert_eq!(bundle.chunk_ids, vec!["1".to_string()]);
    assert_eq!(
        module_summary(&bundle),
        vec![("0".to_string(), "0.js".to_string(), false)]
    );
    assert_eq!(bundle.modules["0"].code().trim_end(), "");
}

#[test]
fn accepts_this_and_bracket_globals() {
    let alloc = Allocator::default();
    let source = "(this['webpackChunk_N_E'] = this['webpackChunk_N_E'] || []).push([[3], [function () { x(); }]]);\n";
    let program = parse(&alloc, source);
    let bundle = detect_webpack_chunk(&alloc, &program, first_expression_statement(&program))
        .expect("chunk bundle");
    assert_eq!(bundle.chunk_ids, vec!["3".to_string()]);
}

#[test]
fn rejects_mismatched_globals_and_non_webpack_names() {
    let alloc = Allocator::default();
    for source in [
        "(window.webpackJsonp = window.other || []).push([[1], [function () {}]]);",
        "(window.other = window.other || []).push([[1], [function () {}]]);",
        "(window.webpackJsonp = window.webpackJsonp || [0]).push([[1], [function () {}]]);",
        "(window.webpackJsonp = window.webpackJsonp || []).push([[1]]);",
        "(window.webpackJsonp = window.webpackJsonp || []).push([[1], 2]);",
    ] {
        let program = parse(&alloc, source);
        let expr = first_expression_statement(&program);
        assert!(
            detect_webpack_chunk(&alloc, &program, expr).is_none(),
            "{source}"
        );
    }
}

#[test]
fn a_promise_chunk_runtime_is_detected_and_its_modules_extracted() {
    let source = read_unpack_gap_fixture("webpack-5-promise-chunk.js");
    let alloc = oxc_allocator::Allocator::default();
    let parsed = untwist_core::parse::parse_program(&alloc, &source);
    let bundle =
        untwist_unpack::unpack_program(&alloc, &parsed.program).expect("a webpack chunk bundle");
    assert_eq!(bundle.kind.as_str(), "webpack");
    assert_eq!(bundle.modules.len(), 2);
    assert!(
        bundle.chunk_ids.iter().any(|id| id == "vendors"),
        "{:?}",
        bundle.chunk_ids
    );
    assert_eq!(bundle.entry_id.as_deref(), Some("4821"));
}

#[test]
fn a_runtime_registration_without_a_module_table_yields_no_bundle() {
    let source = "(__webpack_require__.O(0, [\"vendors\"], () => __webpack_require__(4821)), __webpack_require__.O());";
    let alloc = oxc_allocator::Allocator::default();
    let parsed = untwist_core::parse::parse_program(&alloc, source);
    assert!(untwist_unpack::unpack_program(&alloc, &parsed.program).is_none());
}

#[test]
fn a_registration_whose_inner_callee_is_not_the_require_function_names_no_entry() {
    let source = read_unpack_gap_fixture("webpack-5-promise-chunk.js").replacen(
        "return __webpack_require__(4821);",
        "return other(4821);",
        1,
    );
    assert!(source.contains("return other(4821);"), "{source}");
    let alloc = oxc_allocator::Allocator::default();
    let parsed = untwist_core::parse::parse_program(&alloc, &source);
    let bundle =
        untwist_unpack::unpack_program(&alloc, &parsed.program).expect("a webpack chunk bundle");
    assert_eq!(bundle.modules.len(), 2);
    assert_eq!(bundle.entry_id, None);
}
