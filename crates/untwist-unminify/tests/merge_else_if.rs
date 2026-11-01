use untwist_core::pass::PassOptions;
use untwist_testing::{assert_transform, transform_with, Allocator};

fn run(source: &str, extended: bool) -> String {
    let alloc = Allocator::default();
    transform_with(
        &alloc,
        source,
        &[untwist_unminify::merge_else_if::boxed],
        PassOptions {
            unsafe_transforms: false,
            extended,
            max_iterations: None,
            fuel: None,
            ..PassOptions::default()
        },
    )
}

#[test]
fn merge() {
    assert_transform!(
        "if (x) {\n} else {\n  if (y) {}\n}",
        [untwist_unminify::merge_else_if::boxed],
        @"if (x) {} else if (y) {}"
    );
}

#[test]
fn ignore_when_it_contains_other_statements() {
    assert_transform!(
        "if (x) {\n} else {\n  if (y) {}\n  z();\n}",
        [untwist_unminify::merge_else_if::boxed],
        @r"
    if (x) {} else {
      if (y) {}
      z();
    }
    "
    );
}

#[test]
fn a_three_level_chain_flattens_in_both_modes() {
    let source = "if (a) {\n  p();\n} else {\n  if (b) {\n    q();\n  } else {\n    if (c) {\n      r();\n    }\n  }\n}";
    let expected = "if (a) {\n  p();\n} else if (b) {\n  q();\n} else if (c) {\n  r();\n}";
    assert_eq!(run(source, false), expected);
    assert_eq!(run(source, true), expected);
}

#[test]
fn the_default_inverts_a_negated_test_so_the_chain_can_merge() {
    let source = "if (!c) {\n  if (d) {\n    p();\n  } else {\n    q();\n  }\n} else {\n  r();\n}";
    assert_eq!(
        run(source, true),
        "if (c) {\n  r();\n} else if (d) {\n  p();\n} else {\n  q();\n}"
    );
}

#[test]
fn conservative_leaves_the_negated_form_alone() {
    let source = "if (!c) {\n  if (d) {\n    p();\n  } else {\n    q();\n  }\n} else {\n  r();\n}";
    assert_eq!(run(source, false), source);
}

#[test]
fn a_negated_test_with_no_else_is_left_alone_in_both_modes() {
    let source = "if (!c) {\n  if (d) {\n    p();\n  }\n}";
    assert_eq!(run(source, true), source);
    assert_eq!(run(source, false), source);
}

#[test]
fn a_negated_test_whose_consequent_is_not_a_lone_if_is_left_alone() {
    let source = "if (!c) {\n  p();\n} else {\n  q();\n}";
    assert_eq!(run(source, true), source);
}

#[test]
fn an_inequality_test_is_not_a_negation() {
    let source = "if (c != d) {\n  if (e) {\n    p();\n  }\n} else {\n  q();\n}";
    assert_eq!(run(source, true), source);
}

#[test]
fn an_else_if_alternate_is_still_inverted_correctly() {
    let source = "if (!c) {\n  if (d) {\n    p();\n  }\n} else if (e) {\n  r();\n}";
    let once = run(source, true);
    assert_eq!(
        once,
        "if (c) {\n  if (e) {\n    r();\n  }\n} else if (d) {\n  p();\n}"
    );
    assert_eq!(run(&once, true), once);
}
