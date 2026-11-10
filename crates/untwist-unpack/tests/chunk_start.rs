use oxc_allocator::Allocator;
use untwist_core::parse::parse_program;
use untwist_unpack::{unpack_inputs, UnpackOptions};

const STARTS_WITH_CODE: &str = r#"(self["webpackChunkapp"] = self["webpackChunkapp"] || []).push([[1], {
  5: function (module, exports, __webpack_require__) { module.exports = "five"; },
  6: function (module, exports, __webpack_require__) { console.log(__webpack_require__(5)); }
}, function (__webpack_require__) { console.log("chunk runtime callback"); __webpack_require__(6); }]);
"#;

const DEFERRED: &str = r#"(window.webpackJsonp = window.webpackJsonp || []).push([[1], {
  5: function (module, exports, __webpack_require__) { module.exports = "five"; }
}, [[5, 0]]]);
"#;

fn outcome(source: &str) -> (Option<usize>, Vec<String>) {
    let alloc = Allocator::default();
    let parsed = parse_program(&alloc, source);
    let mut programs = vec![parsed.program];
    let (bundle, notes) = unpack_inputs(&alloc, &mut programs, &[], UnpackOptions::default());
    (
        bundle.map(|bundle| bundle.modules.len()),
        notes.into_iter().map(|note| note.message).collect(),
    )
}

#[test]
fn a_chunk_that_runs_its_own_code_on_arrival_is_refused() {
    let (modules, notes) = outcome(STARTS_WITH_CODE);
    assert_eq!(modules, None);
    assert!(
        notes
            .iter()
            .any(|note| note.contains("chunk 1 runs code on arrival that is not webpack's deferred-entry list")),
        "{notes:?}"
    );
}

#[test]
fn a_chunk_with_deferred_entries_is_written_and_says_they_do_not_start() {
    let (modules, notes) = outcome(DEFERRED);
    assert_eq!(modules, Some(1));
    assert!(
        notes
            .iter()
            .any(|note| note.contains("starts these modules only once webpack's runtime loads: 5")),
        "{notes:?}"
    );
}
