mod common;

use oxc_allocator::Allocator;
use untwist_core::parse::parse_program;
use untwist_core::print::{print, PrintStyle};
use untwist_deobfuscate::notes::{
    SWITCH_BINDING_BLOCK_FUNCTION, SWITCH_BINDING_DIRECT_EVAL, SWITCH_BINDING_GLOBAL_REACH,
    SWITCH_CASE_BINDING_SHARED, SWITCH_CASE_EXIT, SWITCH_CASE_SHADOW, SWITCH_FALLTHROUGH,
    SWITCH_SEQUENCE,
};
use untwist_testing::assert_transform;

fn run_one<'a>(alloc: &'a Allocator, source: &'a str) -> (String, Vec<String>) {
    common::run_ctors(
        alloc,
        source,
        &[untwist_deobfuscate::control_flow_switch::boxed],
    )
}

fn printed(source: &str) -> String {
    let alloc = Allocator::default();
    let parsed = parse_program(&alloc, source);
    print(&parsed.program, PrintStyle::Pretty)
}

const A_FALL_THROUGH_INTO_THE_NEXT_CASE: &str = "function hi() {\n  var d = '0|1'.split('|');\n  var e = 0;\n  while (!![]) {\n    switch (d[e++]) {\n      case '0':\n        a();\n      case '1':\n        b();\n        continue;\n    }\n    break;\n  }\n}";

const A_CHAIN_OF_FALL_THROUGHS: &str = "function hi() {\n  var d = '0|1|2'.split('|');\n  var e = 0;\n  while (!![]) {\n    switch (d[e++]) {\n      case '0':\n        a();\n      case '1':\n        b();\n      case '2':\n        c();\n        continue;\n    }\n    break;\n  }\n}";

const AN_EMPTY_CASE_THAT_FALLS_THROUGH: &str = "function hi() {\n  var d = '0|1'.split('|');\n  var e = 0;\n  while (!![]) {\n    switch (d[e++]) {\n      case '0':\n      case '1':\n        b();\n        continue;\n    }\n    break;\n  }\n}";

const A_SEQUENCE_THAT_IS_NOT_A_PERMUTATION: &str = "function hi() {\n  var d = '0|9'.split('|');\n  var e = 0;\n  while (!![]) {\n    switch (d[e++]) {\n      case '0':\n        a();\n        continue;\n    }\n    break;\n  }\n}";

const A_CASE_THAT_BREAKS_OUT_OF_THE_LOOP: &str = "function hi() {\n  var d = '0|1'.split('|');\n  var e = 0;\n  while (!![]) {\n    switch (d[e++]) {\n      case '0':\n        a();\n        break;\n      case '1':\n        b();\n        continue;\n    }\n    break;\n  }\n}";

const A_CASE_THAT_BREAKS_TO_A_LABEL: &str = "function hi() {\n  outer: {\n    var d = '0|1'.split('|');\n    var e = 0;\n    while (!![]) {\n      switch (d[e++]) {\n        case '0':\n          a();\n          break outer;\n        case '1':\n          b();\n          continue;\n      }\n      break;\n    }\n  }\n}";

const A_CASE_THAT_CONTINUES_A_LABELLED_LOOP: &str = "function hi() {\n  outer: while (c()) {\n    var d = '0|1'.split('|');\n    var e = 0;\n    while (!![]) {\n      switch (d[e++]) {\n        case '0':\n          a();\n          continue outer;\n        case '1':\n          b();\n          continue;\n      }\n      break;\n    }\n  }\n}";

#[test]
fn the_dispatch_sequence_is_flattened() {
    assert_transform!(
        "function hi() {\n  var d = '2|0|1'.split('|');\n  var e = 0;\n  while (!![]) {\n    switch (d[e++]) {\n      case '0':\n        a();\n        continue;\n      case '1':\n        b();\n        continue;\n      case '2':\n        c();\n        continue;\n    }\n    break;\n  }\n}",
        [untwist_deobfuscate::control_flow_switch::boxed],
        @r#"
    function hi() {
      c();
      a();
      b();
    }
    "#
    );
}

#[test]
fn a_case_that_returns_instead_of_continuing_is_kept_whole() {
    assert_transform!(
        "function f() {\n  var d = '0'.split('|');\n  var e = 0;\n  while (true) {\n    switch (d[e++]) {\n      case '0':\n        if (true) {\n          return 123;\n        } else {\n          return 456;\n        }\n    }\n    break;\n  }\n}",
        [untwist_deobfuscate::control_flow_switch::boxed],
        @r#"
    function f() {
      if (true) {
        return 123;
      } else {
        return 456;
      }
    }
    "#
    );
}

#[test]
fn statements_after_the_loop_are_preserved() {
    assert_transform!(
        "function hi() {\n  var d = '0'.split('|');\n  var e = 0;\n  while (!![]) {\n    switch (d[e++]) {\n      case '0':\n        a();\n        continue;\n    }\n    break;\n  }\n  z();\n}",
        [untwist_deobfuscate::control_flow_switch::boxed],
        @r#"
    function hi() {
      a();
      z();
    }
    "#
    );
}

#[test]
fn a_fall_through_into_the_next_case_is_refused() {
    let alloc = Allocator::default();
    let (code, notes) = run_one(&alloc, A_FALL_THROUGH_INTO_THE_NEXT_CASE);
    assert_eq!(code, printed(A_FALL_THROUGH_INTO_THE_NEXT_CASE));
    assert!(
        notes.iter().any(|note| note == SWITCH_FALLTHROUGH),
        "{notes:?}"
    );
}

#[test]
fn a_chain_of_fall_throughs_is_refused() {
    let alloc = Allocator::default();
    let (code, notes) = run_one(&alloc, A_CHAIN_OF_FALL_THROUGHS);
    assert_eq!(code, printed(A_CHAIN_OF_FALL_THROUGHS));
    assert!(
        notes.iter().any(|note| note == SWITCH_FALLTHROUGH),
        "{notes:?}"
    );
}

#[test]
fn an_empty_case_consequent_that_falls_through_is_refused() {
    let alloc = Allocator::default();
    let (code, notes) = run_one(&alloc, AN_EMPTY_CASE_THAT_FALLS_THROUGH);
    assert_eq!(code, printed(AN_EMPTY_CASE_THAT_FALLS_THROUGH));
    assert!(
        notes.iter().any(|note| note == SWITCH_FALLTHROUGH),
        "{notes:?}"
    );
}

#[test]
fn a_fall_through_the_sequence_would_reorder_is_refused() {
    assert_transform!(
        "function hi() {\n  var d = '1|0'.split('|');\n  var e = 0;\n  while (!![]) {\n    switch (d[e++]) {\n      case '0':\n        a();\n      case '1':\n        b();\n        continue;\n    }\n    break;\n  }\n}",
        [untwist_deobfuscate::control_flow_switch::boxed],
        @r#"
    function hi() {
      var d = "1|0".split("|");
      var e = 0;
      while (!![]) {
        switch (d[e++]) {
          case "0": a();
          case "1":
            b();
            continue;
        }
        break;
      }
    }
    "#
    );
}

#[test]
fn a_fall_through_the_sequence_would_reorder_is_noted() {
    let alloc = Allocator::default();
    let (_code, notes) = run_one(
        &alloc,
        "function hi() {\n  var d = '1|0'.split('|');\n  var e = 0;\n  while (!![]) {\n    switch (d[e++]) {\n      case '0':\n        a();\n      case '1':\n        b();\n        continue;\n    }\n    break;\n  }\n}",
    );
    assert!(
        notes.iter().any(|note| note == SWITCH_FALLTHROUGH),
        "{notes:?}"
    );
}

#[test]
fn a_sequence_that_is_not_a_permutation_is_refused() {
    assert_transform!(
        "function hi() {\n  var d = '0|9'.split('|');\n  var e = 0;\n  while (!![]) {\n    switch (d[e++]) {\n      case '0':\n        a();\n        continue;\n    }\n    break;\n  }\n}",
        [untwist_deobfuscate::control_flow_switch::boxed],
        @r#"
    function hi() {
      var d = "0|9".split("|");
      var e = 0;
      while (!![]) {
        switch (d[e++]) {
          case "0":
            a();
            continue;
        }
        break;
      }
    }
    "#
    );
}

#[test]
fn a_sequence_that_is_not_a_permutation_names_the_label_it_cannot_place() {
    let alloc = Allocator::default();
    let (_code, notes) = run_one(&alloc, A_SEQUENCE_THAT_IS_NOT_A_PERMUTATION);
    assert!(
        notes
            .iter()
            .any(|note| note == &format!("{SWITCH_SEQUENCE}: 9")),
        "{notes:?}"
    );
}

#[test]
fn a_sequence_that_leaves_a_case_unplaced_is_refused() {
    let alloc = Allocator::default();
    let (code, notes) = run_one(
        &alloc,
        "function hi() {\n  var d = '0'.split('|');\n  var e = 0;\n  while (!![]) {\n    switch (d[e++]) {\n      case '0':\n        a();\n        continue;\n      case '1':\n        b();\n        continue;\n    }\n    break;\n  }\n}",
    );
    assert!(code.contains("switch"), "{code}");
    assert!(
        notes
            .iter()
            .any(|note| note == &format!("{SWITCH_SEQUENCE}: 1")),
        "{notes:?}"
    );
}

#[test]
fn a_case_that_breaks_out_of_the_dispatch_loop_is_refused() {
    let alloc = Allocator::default();
    let (code, notes) = run_one(&alloc, A_CASE_THAT_BREAKS_OUT_OF_THE_LOOP);
    assert_eq!(code, printed(A_CASE_THAT_BREAKS_OUT_OF_THE_LOOP));
    assert!(
        notes.iter().any(|note| note == SWITCH_CASE_EXIT),
        "{notes:?}"
    );
}

#[test]
fn a_case_that_breaks_to_a_label_is_refused() {
    let alloc = Allocator::default();
    let (code, notes) = run_one(&alloc, A_CASE_THAT_BREAKS_TO_A_LABEL);
    assert_eq!(code, printed(A_CASE_THAT_BREAKS_TO_A_LABEL));
    assert!(
        notes.iter().any(|note| note == SWITCH_CASE_EXIT),
        "{notes:?}"
    );
}

#[test]
fn a_case_that_continues_a_labelled_outer_loop_is_refused() {
    let alloc = Allocator::default();
    let (code, notes) = run_one(&alloc, A_CASE_THAT_CONTINUES_A_LABELLED_LOOP);
    assert_eq!(code, printed(A_CASE_THAT_CONTINUES_A_LABELLED_LOOP));
    assert!(
        notes.iter().any(|note| note == SWITCH_CASE_EXIT),
        "{notes:?}"
    );
}

#[test]
fn a_refused_rewrite_leaves_the_statement_list_byte_identical() {
    for source in [
        A_SEQUENCE_THAT_IS_NOT_A_PERMUTATION,
        A_CASE_THAT_BREAKS_OUT_OF_THE_LOOP,
    ] {
        let alloc = Allocator::default();
        let (code, _notes) = run_one(&alloc, source);
        assert_eq!(code, printed(source), "{source}");
    }
}

#[test]
fn a_list_whose_first_three_statements_are_not_the_prelude_says_nothing() {
    let alloc = Allocator::default();
    let source = "function hi() {\n  var d = compute();\n  var e = 0;\n  while (!![]) {\n    switch (d[e++]) {\n      case '0':\n        a();\n        continue;\n    }\n    break;\n  }\n}";
    let (code, notes) = run_one(&alloc, source);
    assert_eq!(code, printed(source));
    assert!(notes.is_empty(), "{notes:?}");
}

#[test]
fn a_dispatch_loop_is_flattened_in_a_program_with_no_string_array() {
    let (code, notes) = common::run_passes(
        "function hi() {\n  var d = '1|0'.split('|');\n  var e = 0;\n  while (!![]) {\n    switch (d[e++]) {\n      case '0':\n        console.log('first');\n        continue;\n      case '1':\n        console.log('second');\n        continue;\n    }\n    break;\n  }\n}\nhi();",
    );
    assert_eq!(
        code.trim(),
        "function hi() {\n  console.log(\"second\");\n  console.log(\"first\");\n}\nhi();",
        "{code}"
    );
    assert!(
        notes
            .iter()
            .any(|note| note == untwist_deobfuscate::NO_STRING_ARRAY_NOTE),
        "{notes:?}"
    );
}

const A_LAST_CASE_THAT_FALLS_OUT_OF_THE_SWITCH_TOO_EARLY: &str = "function hi() {\n  var d = '0|1'.split('|');\n  var e = 0;\n  while (!![]) {\n    switch (d[e++]) {\n      case '1':\n        b();\n        continue;\n      case '0':\n        a();\n    }\n    break;\n  }\n  z();\n}";

#[test]
fn a_last_case_that_falls_out_of_the_switch_before_the_sequence_ends_is_refused() {
    let alloc = Allocator::default();
    let (code, notes) = run_one(&alloc, A_LAST_CASE_THAT_FALLS_OUT_OF_THE_SWITCH_TOO_EARLY);
    assert_eq!(
        code,
        printed(A_LAST_CASE_THAT_FALLS_OUT_OF_THE_SWITCH_TOO_EARLY)
    );
    assert!(
        notes.iter().any(|note| note == SWITCH_FALLTHROUGH),
        "{notes:?}"
    );
}

#[test]
fn a_last_case_that_falls_out_of_the_switch_last_in_the_sequence_is_flattened() {
    assert_transform!(
        "function hi() {\n  var d = '1|0'.split('|');\n  var e = 0;\n  while (!![]) {\n    switch (d[e++]) {\n      case '1':\n        b();\n        continue;\n      case '0':\n        a();\n    }\n    break;\n  }\n  z();\n}",
        [untwist_deobfuscate::control_flow_switch::boxed],
        @r#"
    function hi() {
      b();
      a();
      z();
    }
    "#
    );
}

const A_CASE_WITH_A_NESTED_BREAK: &str = "function hi() {\n  var d = '0|1'.split('|');\n  var e = 0;\n  while (!![]) {\n    switch (d[e++]) {\n      case '0':\n        if (x) {\n          break;\n        }\n        a();\n        continue;\n      case '1':\n        b();\n        continue;\n    }\n    break;\n  }\n}";

const A_CASE_WITH_A_NESTED_CONTINUE: &str = "function hi() {\n  var d = '0|1'.split('|');\n  var e = 0;\n  while (!![]) {\n    switch (d[e++]) {\n      case '0':\n        if (x) continue;\n        a();\n        continue;\n      case '1':\n        b();\n        continue;\n    }\n    break;\n  }\n}";

const A_CASE_THAT_BREAKS_TO_AN_OUTER_LABEL_FROM_AN_INNER_LOOP: &str = "function hi() {\n  outer: {\n    var d = '0|1'.split('|');\n    var e = 0;\n    while (!![]) {\n      switch (d[e++]) {\n        case '0':\n          for (var i = 0; i < 3; i++) {\n            if (i) break outer;\n          }\n          continue;\n        case '1':\n          b();\n          continue;\n      }\n      break;\n    }\n  }\n}";

#[test]
fn a_case_whose_nested_break_targets_the_dispatch_switch_is_refused() {
    let alloc = Allocator::default();
    let (code, notes) = run_one(&alloc, A_CASE_WITH_A_NESTED_BREAK);
    assert_eq!(code, printed(A_CASE_WITH_A_NESTED_BREAK));
    assert!(
        notes.iter().any(|note| note == SWITCH_CASE_EXIT),
        "{notes:?}"
    );
}

#[test]
fn a_case_whose_nested_continue_targets_the_dispatch_loop_is_refused() {
    let alloc = Allocator::default();
    let (code, notes) = run_one(&alloc, A_CASE_WITH_A_NESTED_CONTINUE);
    assert_eq!(code, printed(A_CASE_WITH_A_NESTED_CONTINUE));
    assert!(
        notes.iter().any(|note| note == SWITCH_CASE_EXIT),
        "{notes:?}"
    );
}

#[test]
fn a_case_that_breaks_to_an_outer_label_from_an_inner_loop_is_refused() {
    let alloc = Allocator::default();
    let (code, notes) = run_one(
        &alloc,
        A_CASE_THAT_BREAKS_TO_AN_OUTER_LABEL_FROM_AN_INNER_LOOP,
    );
    assert_eq!(
        code,
        printed(A_CASE_THAT_BREAKS_TO_AN_OUTER_LABEL_FROM_AN_INNER_LOOP)
    );
    assert!(
        notes.iter().any(|note| note == SWITCH_CASE_EXIT),
        "{notes:?}"
    );
}

#[test]
fn a_break_that_belongs_to_a_loop_inside_the_case_still_flattens() {
    assert_transform!(
        "function hi() {\n  var d = '0|1'.split('|');\n  var e = 0;\n  while (!![]) {\n    switch (d[e++]) {\n      case '0':\n        for (var i = 0; i < 3; i++) {\n          if (i) break;\n        }\n        continue;\n      case '1':\n        b();\n        continue;\n    }\n    break;\n  }\n}",
        [untwist_deobfuscate::control_flow_switch::boxed],
        @r#"
    function hi() {
      for (var i = 0; i < 3; i++) {
        if (i) break;
      }
      b();
    }
    "#
    );
}

#[test]
fn a_labelled_continue_to_a_label_inside_the_case_still_flattens() {
    assert_transform!(
        "function hi() {\n  var d = '0|1'.split('|');\n  var e = 0;\n  while (!![]) {\n    switch (d[e++]) {\n      case '0':\n        inner: for (var i = 0; i < 3; i++) {\n          if (i) continue inner;\n          a();\n        }\n        continue;\n      case '1':\n        b();\n        continue;\n    }\n    break;\n  }\n}",
        [untwist_deobfuscate::control_flow_switch::boxed],
        @r#"
    function hi() {
      inner: for (var i = 0; i < 3; i++) {
        if (i) continue inner;
        a();
      }
      b();
    }
    "#
    );
}

const AN_ITERATOR_READ_AFTER_THE_LOOP: &str = "function hi() {\n  var d = '0'.split('|');\n  var e = 0;\n  while (!![]) {\n    switch (d[e++]) {\n      case '0':\n        a();\n        continue;\n    }\n    break;\n  }\n  console.log('e =', e);\n}";

const A_CASE_THAT_READS_THE_SEQUENCE_ARRAY: &str = "function hi() {\n  var d = '0|1'.split('|');\n  var e = 0;\n  while (!![]) {\n    switch (d[e++]) {\n      case '0':\n        a(d);\n        continue;\n      case '1':\n        b();\n        continue;\n    }\n    break;\n  }\n}";

const A_FOR_HEAD_WITH_AN_INIT_AND_AN_UPDATE: &str = "function hi() {\n  var d = '0'.split('|');\n  var e = 0;\n  for (g(); ; h()) {\n    switch (d[e++]) {\n      case '0':\n        a();\n        continue;\n    }\n    break;\n  }\n}";

const A_CASE_DECLARATION_THAT_COLLIDES_ONCE_HOISTED: &str = "function hi() {\n  var d = '0'.split('|');\n  var e = 0;\n  while (!![]) {\n    switch (d[e++]) {\n      case '0':\n        let x = 2;\n        a(x);\n        continue;\n    }\n    break;\n  }\n  let x = 1;\n  b(x);\n}";

#[test]
fn a_prelude_binding_read_after_the_loop_is_left_alone() {
    let alloc = Allocator::default();
    let (code, notes) = run_one(&alloc, AN_ITERATOR_READ_AFTER_THE_LOOP);
    assert_eq!(code, printed(AN_ITERATOR_READ_AFTER_THE_LOOP));
    assert!(notes.is_empty(), "{notes:?}");
}

#[test]
fn a_case_that_reads_the_sequence_array_is_left_alone() {
    let alloc = Allocator::default();
    let (code, notes) = run_one(&alloc, A_CASE_THAT_READS_THE_SEQUENCE_ARRAY);
    assert_eq!(code, printed(A_CASE_THAT_READS_THE_SEQUENCE_ARRAY));
    assert!(notes.is_empty(), "{notes:?}");
}

#[test]
fn a_for_head_that_carries_an_init_or_an_update_is_left_alone() {
    let alloc = Allocator::default();
    let (code, notes) = run_one(&alloc, A_FOR_HEAD_WITH_AN_INIT_AND_AN_UPDATE);
    assert_eq!(code, printed(A_FOR_HEAD_WITH_AN_INIT_AND_AN_UPDATE));
    assert!(notes.is_empty(), "{notes:?}");
}

#[test]
fn a_bare_for_head_still_flattens() {
    assert_transform!(
        "function hi() {\n  var d = '0'.split('|');\n  var e = 0;\n  for (;;) {\n    switch (d[e++]) {\n      case '0':\n        a();\n        continue;\n    }\n    break;\n  }\n}",
        [untwist_deobfuscate::control_flow_switch::boxed],
        @r#"
    function hi() {
      a();
    }
    "#
    );
}

#[test]
fn a_case_declaration_that_would_collide_once_hoisted_is_refused() {
    let alloc = Allocator::default();
    let (code, notes) = run_one(&alloc, A_CASE_DECLARATION_THAT_COLLIDES_ONCE_HOISTED);
    assert_eq!(code, printed(A_CASE_DECLARATION_THAT_COLLIDES_ONCE_HOISTED));
    assert!(
        notes.iter().any(|note| note == SWITCH_CASE_SHADOW),
        "{notes:?}"
    );
}

#[test]
fn a_case_declaration_that_collides_with_nothing_still_flattens() {
    assert_transform!(
        "function hi() {\n  var d = '0|1'.split('|');\n  var e = 0;\n  while (!![]) {\n    switch (d[e++]) {\n      case '0':\n        const y = 2;\n        a(y);\n        continue;\n      case '1':\n        b();\n        continue;\n    }\n    break;\n  }\n}",
        [untwist_deobfuscate::control_flow_switch::boxed],
        @r#"
    function hi() {
      const y = 2;
      a(y);
      b();
    }
    "#
    );
}

fn dispatch(prelude: &str, cases: &str, after: &str) -> String {
    format!(
        "function hi(p) {{\n  var d = '0|1|2'.split('|');\n  {prelude}\n  while (!![]) {{\n    switch (d[e++]) {{\n{cases}\n    }}\n    break;\n  }}\n{after}}}"
    )
}

const PLAIN_CASES: &str =
    "      case '0':\n        a();\n        continue;\n      case '1':\n        b();\n        continue;\n      case '2':\n        c();\n        continue;";

fn assert_left_alone(source: &str, wanted: Option<&str>) {
    let alloc = Allocator::default();
    let (code, notes) = run_one(&alloc, source);
    assert_eq!(code, printed(source));
    match wanted {
        Some(note) => assert!(notes.iter().any(|found| found == note), "{notes:?}"),
        None => assert!(notes.is_empty(), "{notes:?}"),
    }
}

fn cases_with(first: &str, second: &str) -> String {
    format!(
        "      case '0':\n{first}\n        continue;\n      case '1':\n{second}\n        continue;\n      case '2':\n        c();\n        continue;"
    )
}

#[test]
fn an_iterator_that_starts_past_zero_is_left_alone() {
    assert_left_alone(&dispatch("var e = 1;", PLAIN_CASES, ""), None);
}

#[test]
fn an_iterator_initialised_by_a_call_is_left_alone() {
    assert_left_alone(&dispatch("var e = g();", PLAIN_CASES, ""), None);
}

#[test]
fn an_iterator_without_an_initialiser_is_left_alone() {
    assert_left_alone(&dispatch("var e;", PLAIN_CASES, ""), None);
}

#[test]
fn an_iterator_initialised_by_a_string_zero_is_left_alone() {
    assert_left_alone(&dispatch("var e = '0';", PLAIN_CASES, ""), None);
}

#[test]
fn an_iterator_initialised_by_arithmetic_that_folds_to_zero_still_flattens() {
    let source = dispatch("var e = 3 - 3;", PLAIN_CASES, "");
    let alloc = Allocator::default();
    let (code, notes) = run_one(&alloc, &source);
    assert_eq!(code.trim(), "function hi(p) {\n  a();\n  b();\n  c();\n}");
    assert!(notes.is_empty(), "{notes:?}");
}

#[test]
fn a_case_that_redeclares_the_iterator_is_left_alone() {
    let cases = cases_with("        a();\n        var e = 5;", "        b();");
    assert_left_alone(&dispatch("var e = 0;", &cases, ""), None);
}

#[test]
fn a_case_that_redeclares_the_sequence_array_is_left_alone() {
    let cases = cases_with("        a();\n        var d = [];", "        b();");
    assert_left_alone(&dispatch("var e = 0;", &cases, ""), None);
}

#[test]
fn an_iterator_that_shares_its_name_with_a_parameter_is_left_alone() {
    let cases = cases_with("        arguments[0] = 5;", "        b();");
    assert_left_alone(
        &dispatch("var p = 0;", &cases, "").replace("d[e++]", "d[p++]"),
        None,
    );
}

#[test]
fn a_case_declaration_that_would_capture_a_global_name_is_refused() {
    let cases = cases_with(
        "        let name = 'inner';\n        a(name);",
        "        b();",
    );
    assert_left_alone(
        &dispatch("var e = 0;", &cases, "  z(typeof name, name);\n"),
        Some(SWITCH_CASE_SHADOW),
    );
}

#[test]
fn a_case_declaration_read_by_another_case_is_refused() {
    let cases = cases_with("        let x = 1;\n        a(x);", "        b(x);");
    assert_left_alone(
        &dispatch("var e = 0;", &cases, ""),
        Some(SWITCH_CASE_BINDING_SHARED),
    );
}

#[test]
fn a_case_declaration_captured_by_a_closure_in_another_case_is_refused() {
    let cases = cases_with(
        "        const x = 1;\n        a(x);",
        "        b(function () {\n          return x;\n        });",
    );
    assert_left_alone(
        &dispatch("var e = 0;", &cases, ""),
        Some(SWITCH_CASE_BINDING_SHARED),
    );
}

#[test]
fn a_case_class_used_by_another_case_is_refused() {
    let cases = cases_with("        class K {}\n        a(K);", "        b(new K());");
    assert_left_alone(
        &dispatch("var e = 0;", &cases, ""),
        Some(SWITCH_CASE_BINDING_SHARED),
    );
}

#[test]
fn a_case_function_called_by_another_case_is_refused() {
    let cases = cases_with("        function k() {}\n        a(k);", "        b(k);");
    let source = dispatch("var e = 0;", &cases, "");
    let alloc = Allocator::default();
    let (code, _) = run_one(&alloc, &source);
    assert_eq!(code, printed(&source));
}

#[test]
fn a_case_function_called_after_the_loop_is_refused() {
    let cases = cases_with("        function k() {}\n        a(k);", "        b();");
    let source = dispatch("var e = 0;", &cases, "  k();\n");
    let alloc = Allocator::default();
    let (code, _) = run_one(&alloc, &source);
    assert_eq!(code, printed(&source));
}

#[test]
fn a_case_declaration_at_the_top_level_of_a_script_is_refused() {
    let source = "var d = '0|1'.split('|');\nvar e = 0;\nwhile (!![]) {\n  switch (d[e++]) {\n    case '0':\n      let x = 1;\n      a(x);\n      continue;\n    case '1':\n      b();\n      continue;\n  }\n  break;\n}";
    assert_left_alone(source, Some(SWITCH_CASE_SHADOW));
}

#[test]
fn a_case_declaration_used_only_inside_its_case_still_flattens() {
    let cases = cases_with(
        "        let x = 1;\n        class K {}\n        a(x, K);",
        "        b();",
    );
    let source = dispatch("var e = 0;", &cases, "");
    let alloc = Allocator::default();
    let (code, notes) = run_one(&alloc, &source);
    assert_eq!(
        code.trim(),
        "function hi(p) {\n  let x = 1;\n  class K {}\n  a(x, K);\n  b();\n  c();\n}"
    );
    assert!(notes.is_empty(), "{notes:?}");
}

#[test]
fn a_strict_case_function_called_by_another_case_is_refused() {
    let cases = cases_with("        function k() {}\n        a(k);", "        b(k);");
    let source = format!("'use strict';\n{}", dispatch("var e = 0;", &cases, ""));
    assert_left_alone(&source, Some(SWITCH_CASE_BINDING_SHARED));
}

#[test]
fn a_case_declaration_named_like_a_global_read_in_another_function_is_refused() {
    let cases = cases_with(
        "        let name = 'inner';\n        a(name);",
        "        b();",
    );
    let source = format!(
        "{}\nfunction other() {{\n  return name;\n}}",
        dispatch("var e = 0;", &cases, "")
    );
    assert_left_alone(&source, Some(SWITCH_CASE_SHADOW));
}

#[test]
fn a_block_function_named_like_the_iterator_is_refused() {
    let cases = cases_with(
        "        {\n          function e() {}\n        }\n        a();",
        "        b();",
    );
    assert_left_alone(
        &dispatch("var e = 0;", &cases, ""),
        Some(SWITCH_BINDING_BLOCK_FUNCTION),
    );
}

#[test]
fn a_block_function_named_like_the_order_array_is_refused() {
    let cases = cases_with("        if (a()) function d() {}", "        b();");
    assert_left_alone(
        &dispatch("var e = 0;", &cases, ""),
        Some(SWITCH_BINDING_BLOCK_FUNCTION),
    );
}

#[test]
fn a_strict_block_function_named_like_the_iterator_still_flattens() {
    let cases = cases_with(
        "        {\n          function e() {}\n        }\n        a();",
        "        b();",
    );
    let source = format!("'use strict';\n{}", dispatch("var e = 0;", &cases, ""));
    let alloc = Allocator::default();
    let (code, notes) = run_one(&alloc, &source);
    assert_eq!(
        code.trim(),
        "\"use strict\";\nfunction hi(p) {\n  {\n    function e() {}\n  }\n  a();\n  b();\n  c();\n}"
    );
    assert!(notes.is_empty(), "{notes:?}");
}

#[test]
fn a_block_function_in_a_nested_function_still_flattens() {
    let cases = cases_with(
        "        a(function () {\n          {\n            function e() {}\n          }\n        });",
        "        b();",
    );
    let source = dispatch("var e = 0;", &cases, "");
    let alloc = Allocator::default();
    let (code, notes) = run_one(&alloc, &source);
    assert_eq!(
        code.trim(),
        "function hi(p) {\n  a(function () {\n    {\n      function e() {}\n    }\n  });\n  b();\n  c();\n}"
    );
    assert!(notes.is_empty(), "{notes:?}");
}

#[test]
fn a_direct_eval_in_a_case_is_refused() {
    let cases = cases_with("        eval(a);", "        b();");
    assert_left_alone(
        &dispatch("var e = 0;", &cases, ""),
        Some(SWITCH_BINDING_DIRECT_EVAL),
    );
}

#[test]
fn a_direct_eval_after_the_loop_is_refused() {
    let cases = cases_with("        let q = 1;\n        a(q);", "        b();");
    assert_left_alone(
        &dispatch("var e = 0;", &cases, "  z(eval('typeof q'));\n"),
        Some(SWITCH_BINDING_DIRECT_EVAL),
    );
}

#[test]
fn a_direct_eval_in_a_closure_of_a_case_is_refused() {
    let cases = cases_with("        a(() => eval(b));", "        b();");
    assert_left_alone(
        &dispatch("var e = 0;", &cases, ""),
        Some(SWITCH_BINDING_DIRECT_EVAL),
    );
}

#[test]
fn a_direct_eval_in_another_function_still_flattens() {
    let source = format!(
        "{}\nfunction other() {{\n  eval(a);\n}}",
        dispatch("var e = 0;", PLAIN_CASES, "")
    );
    let alloc = Allocator::default();
    let (code, notes) = run_one(&alloc, &source);
    assert_eq!(
        code.trim(),
        "function hi(p) {\n  a();\n  b();\n  c();\n}\nfunction other() {\n  eval(a);\n}"
    );
    assert!(notes.is_empty(), "{notes:?}");
}

const A_SCRIPT_DISPATCH_PREFIX: &str =
    "var d = '0|1'.split('|');\nvar e = 0;\nwhile (!![]) {\n  switch (d[e++]) {\n    case '0':\n";

const A_SCRIPT_DISPATCH_SUFFIX: &str =
    "      continue;\n    case '1':\n      b();\n      continue;\n  }\n  break;\n}";

#[test]
fn a_script_global_counter_written_through_the_global_object_is_refused() {
    let source =
        format!("{A_SCRIPT_DISPATCH_PREFIX}      globalThis.e = 5;\n{A_SCRIPT_DISPATCH_SUFFIX}");
    assert_left_alone(&source, Some(SWITCH_BINDING_GLOBAL_REACH));
}

#[test]
fn a_script_global_counter_no_code_can_reach_still_flattens() {
    let source = format!("{A_SCRIPT_DISPATCH_PREFIX}      a();\n{A_SCRIPT_DISPATCH_SUFFIX}");
    let alloc = Allocator::default();
    let (code, notes) = run_one(&alloc, &source);
    assert_eq!(code.trim(), "a();\nb();");
    assert!(notes.is_empty(), "{notes:?}");
}
