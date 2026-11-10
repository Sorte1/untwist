mod common;

use common::{conservative_run, default_run};
use oxc_allocator::Allocator;
use untwist_core::parse::parse_program;
use untwist_core::pass::PassOptions;
use untwist_core::print::{print, PrintStyle};
use untwist_core::semantic::build_scoping;
use untwist_deobfuscate::control_flow_object::boxed;
use untwist_deobfuscate::notes;

const DEAD_STRING_TEST: &str = "function hi() {\n  var obj = { rLxJs: \"6|0|4\" };\n  if (\"aa\" === \"bb\") {\n    obj = { rLxJs: \"x\" };\n  }\n  console.log(obj.rLxJs);\n}";

const A_BARE_DEAD_BRANCH_WITH_A_SIDE_EFFECT: &str = "function hi(m, n) {\n  var obj = {\n    aokfw: function (a, b) {\n      return a + b;\n    },\n    bqwer: function (a, b) {\n      return a * b;\n    }\n  };\n  if (\"xx\" === \"yy\") {\n    obj.zzzzz = sideEffect();\n  }\n  console.log(obj.aokfw(m, n), obj.bqwer(m, n));\n}\nhi(2, 3);";

const DEAD_DISPATCH_CALL: &str = "function hi() {\n  var obj = {\n    rLxJs: \"6|0|4\",\n    aokfw: function(a, b) {\n      return a === b;\n    }\n  };\n  if (obj.aokfw(\"aa\", \"bb\")) {\n    obj = { rLxJs: \"x\" };\n  }\n  console.log(obj.rLxJs);\n}";

const A_LIVE_BRANCH: &str = "function hi() {\n  var obj = { rLxJs: \"6|0|4\" };\n  if (q) {\n    obj = { rLxJs: \"x\" };\n  }\n  console.log(obj.rLxJs);\n}";

const THE_TAKEN_BRANCH: &str = "function hi() {\n  var obj = { rLxJs: \"6|0|4\" };\n  if (\"aa\" === \"aa\") {\n    obj = { rLxJs: \"x\" };\n  }\n  console.log(obj.rLxJs);\n}";

const A_DEAD_CONDITIONAL_SIDE: &str = "function hi() {\n  var obj = {\n    rLxJs: \"6|0|4\",\n    aokfw: function(a, b) {\n      return a === b;\n    }\n  };\n  obj.aokfw(\"aa\", \"bb\") ? obj = { rLxJs: \"x\" } : 0;\n  console.log(obj.rLxJs);\n}";

const A_MUTATION_OUTSIDE_THE_DEAD_BRANCH: &str = "function hi() {\n  var obj = {\n    rLxJs: \"6|0|4\",\n    aokfw: function(a, b) {\n      return a === b;\n    }\n  };\n  if (obj.aokfw(\"aa\", \"bb\")) {\n    obj = { rLxJs: \"x\" };\n  }\n  sink(obj);\n  console.log(obj.rLxJs);\n}";

const A_SAME_NAMED_RECEIVER_IN_ANOTHER_SCOPE: &str = "function makeGate() {\n  return { aokfw: function (a, b) {\n    return a !== b;\n  } };\n}\nvar obj = makeGate();\nif (obj.aokfw(\"aa\", \"bb\")) {\n  (function () {\n    var obj = {\n      rLxJs: \"6|0|4\",\n      aokfw: function (a, b) {\n        return a === b;\n      }\n    };\n    obj = { rLxJs: \"MUTATED\" };\n    console.log(obj.rLxJs);\n  })();\n}";

const A_SHADOWED_OUTER_RECEIVER: &str = "function hi() {\n  var obj = other;\n  if (obj.aokfw(\"aa\", \"bb\")) {\n    let obj = {\n      rLxJs: \"6|0|4\",\n      aokfw: function (a, b) {\n        return a === b;\n      }\n    };\n    obj = { rLxJs: \"MUTATED\" };\n    console.log(obj.rLxJs);\n  }\n}";

const A_SHADOWING_INNER_BINDING: &str = "function hi() {\n  var obj = {\n    rLxJs: \"6|0|4\",\n    aokfw: function(a, b) {\n      return a === b;\n    }\n  };\n  {\n    let obj = other;\n    if (obj.aokfw(\"aa\", \"aa\")) {\n      obj = { rLxJs: \"x\" };\n    }\n  }\n  console.log(obj.rLxJs);\n}";

const A_DEAD_STATEMENT_THAT_ONLY_READS: &str = "function hi() {\n  var obj = {\n    rLxJs: \"6|0|4\",\n    aokfw: function(a, b) {\n      return a === b;\n    }\n  };\n  if (obj.aokfw(\"aa\", \"bb\")) {\n    q(obj, 1);\n  }\n  console.log(obj.rLxJs);\n}";

const DEAD_BRANCH_NOTES: &[&str] = &[
    notes::CONTROL_FLOW_LIVE_MUTATION,
    notes::CONTROL_FLOW_DEAD_WRITE_KEPT,
];

fn dead_branch_notes_of(source: &str) -> Vec<String> {
    let (_code, all) = common::run_all_stages_extended(source);
    all.into_iter()
        .filter(|note| DEAD_BRANCH_NOTES.iter().any(|text| note.contains(text)))
        .collect()
}

fn left_alone(source: &str) {
    let alloc = Allocator::default();
    assert_eq!(default_run(&alloc, source, boxed).trim(), source.trim());
}

fn deobfuscate_stage(source: &str, unsafe_transforms: bool, extended: bool) -> String {
    let alloc = Allocator::default();
    let mut parsed = parse_program(&alloc, source);
    assert!(
        parsed.diagnostics.is_empty(),
        "test input must parse cleanly: {:?}",
        parsed.diagnostics
    );
    let sem = build_scoping(&parsed.program);
    let opts = PassOptions {
        unsafe_transforms,
        extended,
        max_iterations: Some(10),
        fuel: None,
        ..PassOptions::default()
    };
    let (_sem, report) =
        untwist_deobfuscate::run_deobfuscate(&alloc, &mut parsed.program, sem, &opts);
    assert!(report.converged(), "did not converge: {:?}", report.stop);
    print(&parsed.program, PrintStyle::Pretty)
}

#[test]
fn a_bare_literal_comparison_decides_nothing_in_either_mode() {
    left_alone(DEAD_STRING_TEST);
    let alloc = Allocator::default();
    assert_eq!(
        conservative_run(&alloc, DEAD_STRING_TEST, boxed).trim(),
        DEAD_STRING_TEST.trim()
    );
}

#[test]
fn the_default_lane_drops_the_never_taken_branch_then_folds() {
    for extended in [false, true] {
        assert_eq!(
            deobfuscate_stage(A_BARE_DEAD_BRANCH_WITH_A_SIDE_EFFECT, false, extended).trim(),
            "function hi(m, n) {\n  console.log(m + n, m * n);\n}\nhi(2, 3);",
            "extended={extended}"
        );
    }
}

#[test]
fn unsafe_removes_the_branch_and_the_object_then_folds() {
    assert_eq!(
        deobfuscate_stage(A_BARE_DEAD_BRANCH_WITH_A_SIDE_EFFECT, true, true).trim(),
        "function hi(m, n) {\n  console.log(m + n, m * n);\n}\nhi(2, 3);"
    );
    assert_eq!(
        deobfuscate_stage(DEAD_STRING_TEST, true, true).trim(),
        "function hi() {\n  console.log(\"6|0|4\");\n}"
    );
}

#[test]
fn issue_45_a_mutation_in_a_dead_dispatch_call_branch_is_ignored() {
    let alloc = Allocator::default();
    let code = default_run(&alloc, DEAD_DISPATCH_CALL, boxed);
    assert!(code.contains(r#"console.log("6|0|4")"#), "{code}");
    assert!(!code.contains("obj"), "{code}");
}

#[test]
fn a_mutation_in_a_live_branch_still_refuses() {
    left_alone(A_LIVE_BRANCH);
}

#[test]
fn a_mutation_in_the_taken_branch_still_refuses() {
    left_alone(THE_TAKEN_BRANCH);
}

#[test]
fn a_dead_write_inside_a_live_statement_is_left_in_place() {
    let alloc = Allocator::default();
    let code = default_run(&alloc, A_DEAD_CONDITIONAL_SIDE, boxed);
    assert!(code.contains(r#"obj = { rLxJs: "x" }"#), "{code}");
    assert!(code.contains(r#"console.log("6|0|4")"#), "{code}");
}

#[test]
fn a_dead_write_left_in_place_is_noted() {
    assert_eq!(
        dead_branch_notes_of(A_DEAD_CONDITIONAL_SIDE),
        vec![notes::CONTROL_FLOW_DEAD_WRITE_KEPT.to_string()]
    );
}

#[test]
fn a_mutation_outside_every_dead_branch_is_refused_and_noted() {
    let alloc = Allocator::default();
    let code = default_run(&alloc, A_MUTATION_OUTSIDE_THE_DEAD_BRANCH, boxed);
    assert!(code.contains("obj.rLxJs"), "{code}");
    assert_eq!(
        dead_branch_notes_of(A_MUTATION_OUTSIDE_THE_DEAD_BRANCH),
        vec![notes::CONTROL_FLOW_LIVE_MUTATION.to_string()]
    );
}

#[test]
fn a_shape_with_no_decidable_branch_is_refused_without_a_note() {
    assert_eq!(dead_branch_notes_of(A_LIVE_BRANCH), Vec::<String>::new());
    assert_eq!(dead_branch_notes_of(THE_TAKEN_BRANCH), Vec::<String>::new());
}

#[test]
fn a_same_named_receiver_in_another_scope_decides_nothing() {
    left_alone(A_SAME_NAMED_RECEIVER_IN_ANOTHER_SCOPE);
}

#[test]
fn a_shadowed_outer_receiver_decides_nothing() {
    left_alone(A_SHADOWED_OUTER_RECEIVER);
}

#[test]
fn a_shadowing_inner_binding_leaves_the_outer_object_substitutable() {
    let alloc = Allocator::default();
    let code = default_run(&alloc, A_SHADOWING_INNER_BINDING, boxed);
    assert!(code.contains(r#"console.log("6|0|4")"#), "{code}");
    assert!(code.contains("let obj = other;"), "{code}");
    assert!(code.contains(r#"obj = { rLxJs: "x" };"#), "{code}");
    let alloc = Allocator::default();
    assert_eq!(
        conservative_run(&alloc, A_SHADOWING_INNER_BINDING, boxed).trim(),
        "function hi() {\n  {\n    let obj = other;\n    if (obj.aokfw(\"aa\", \"aa\")) {\n      obj = { rLxJs: \"x\" };\n    }\n  }\n  console.log(\"6|0|4\");\n}"
    );
}

#[test]
fn a_dead_statement_that_only_reads_the_object_is_removed() {
    let alloc = Allocator::default();
    let code = default_run(&alloc, A_DEAD_STATEMENT_THAT_ONLY_READS, boxed);
    assert!(code.contains(r#"console.log("6|0|4")"#), "{code}");
    assert!(!code.contains("obj"), "{code}");
}

const A_TWICE_DECLARED_OBJECT_WITH_A_DEAD_WRITE: &str = "function hi() {\n  var obj = {\n    aokfw: function (a, b) {\n      return a === b;\n    },\n    rLxJs: \"x\"\n  };\n  var obj = {\n    aokfw: function (a, b) {\n      return a !== b;\n    },\n    rLxJs: \"y\"\n  };\n  if (obj.aokfw(\"a\", \"b\")) {\n    obj.zzzzz = 1;\n  } else {\n    console.log(obj.rLxJs);\n  }\n}\nhi();";

#[test]
fn an_object_declared_twice_is_left_alone_on_the_dead_branch_route() {
    left_alone(A_TWICE_DECLARED_OBJECT_WITH_A_DEAD_WRITE);
    let alloc = Allocator::default();
    assert_eq!(
        conservative_run(&alloc, A_TWICE_DECLARED_OBJECT_WITH_A_DEAD_WRITE, boxed).trim(),
        A_TWICE_DECLARED_OBJECT_WITH_A_DEAD_WRITE.trim()
    );
}

#[test]
fn an_object_declared_twice_never_answers_from_its_first_declaration() {
    for (unsafe_transforms, extended) in [(false, true), (true, true), (false, false)] {
        let code = deobfuscate_stage(
            A_TWICE_DECLARED_OBJECT_WITH_A_DEAD_WRITE,
            unsafe_transforms,
            extended,
        );
        assert!(
            !code.contains("console.log(\"x\")"),
            "unsafe={unsafe_transforms} extended={extended}\n{code}"
        );
        assert!(
            code.contains("obj.zzzzz = 1"),
            "unsafe={unsafe_transforms} extended={extended}\n{code}"
        );
    }
}
