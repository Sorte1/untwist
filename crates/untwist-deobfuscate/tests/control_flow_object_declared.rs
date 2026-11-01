mod common;

use oxc_allocator::Allocator;
use untwist_testing::assert_transform;

fn run_one<'a>(alloc: &'a Allocator, source: &'a str) -> (String, Vec<String>) {
    common::run_ctors(
        alloc,
        source,
        &[untwist_deobfuscate::control_flow_object::boxed],
    )
}

#[test]
fn a_declared_dispatch_object_is_inlined_and_removed() {
    assert_transform!(
        "function hi() {\n  var obj = {\n    rLxJs: '6|0|4',\n    aokfw: function (a, b) { return a + b; },\n    HLAuI: function (a, b, c) { return a(b, c); }\n  };\n  console.log(obj.rLxJs, obj.aokfw(1, 2), obj.HLAuI(f, 3, 4));\n}",
        [untwist_deobfuscate::control_flow_object::boxed],
        @r#"
    function hi() {
      console.log("6|0|4", 1 + 2, f(3, 4));
    }
    "#
    );
}

#[test]
fn the_operands_may_be_swapped() {
    assert_transform!(
        "function hi() {\n  var obj = { aokfw: function (a, b) { return b + a; } };\n  console.log(obj.aokfw(1, 2));\n}",
        [untwist_deobfuscate::control_flow_object::boxed],
        @r#"
    function hi() {
      console.log(2 + 1);
    }
    "#
    );
}

#[test]
fn a_mutated_dispatch_object_is_left_alone() {
    assert_transform!(
        "function hi() {\n  var obj = { rLxJs: '6|0|4' };\n  obj.rLxJs = 'x';\n  console.log(obj.rLxJs);\n}",
        [untwist_deobfuscate::control_flow_object::boxed],
        @r#"
    function hi() {
      var obj = { rLxJs: "6|0|4" };
      obj.rLxJs = "x";
      console.log(obj.rLxJs);
    }
    "#
    );
}

#[test]
fn a_missing_property_is_reported_and_the_object_is_kept() {
    let alloc = Allocator::default();
    let (code, notes) = run_one(
        &alloc,
        "function hi() {\n  var obj = { rLxJs: '6|0|4' };\n  console.log(obj.rLxJs, obj.zzzzz);\n}",
    );
    assert!(code.contains("var obj"), "{code}");
    assert!(
        notes
            .iter()
            .any(|n| n.contains("control_flow_missing_prop")),
        "{notes:?}"
    );
}

#[test]
fn a_hoisted_read_before_the_declaration_keeps_the_object_and_is_reported() {
    let alloc = Allocator::default();
    let (code, notes) = run_one(
        &alloc,
        "function early() { return obj.aokfw(1, 2); }\nearly();\nvar obj = { aokfw: function (a, b) { return a + b; } };",
    );
    assert!(code.contains("obj.aokfw(1, 2)"), "{code}");
    assert!(code.contains("var obj"), "{code}");
    assert!(
        notes
            .iter()
            .any(|n| n.contains("control_flow_use_before_init")),
        "{notes:?}"
    );
}

#[test]
fn an_in_order_forwarder_takes_an_argument_that_runs_code() {
    assert_transform!(
        "function hi() {\n  var obj = { aokfw: function (a, b) { return a + b; } };\n  console.log(obj.aokfw(g(), 2));\n}",
        [untwist_deobfuscate::control_flow_object::boxed],
        @r#"
    function hi() {
      console.log(g() + 2);
    }
    "#
    );
}

#[test]
fn a_swapped_operand_forwarder_keeps_an_argument_that_runs_code() {
    assert_transform!(
        "function hi() {\n  var obj = { aokfw: function (a, b) { return b + a; } };\n  console.log(obj.aokfw(g(), h()));\n}",
        [untwist_deobfuscate::control_flow_object::boxed],
        @r#"
    function hi() {
      var obj = { aokfw: function (a, b) {
        return b + a;
      } };
      console.log(obj.aokfw(g(), h()));
    }
    "#
    );
}

#[test]
fn a_declared_call_site_with_more_arguments_than_parameters_keeps_the_object() {
    assert_transform!(
        "function hi() {\n  var obj = { aokfw: function (a, b) { return a + b; } };\n  console.log(obj.aokfw(1, 2, g()));\n}",
        [untwist_deobfuscate::control_flow_object::boxed],
        @r#"
    function hi() {
      var obj = { aokfw: function (a, b) {
        return a + b;
      } };
      console.log(obj.aokfw(1, 2, g()));
    }
    "#
    );
}

#[test]
fn an_uncalled_function_property_keeps_its_identity_and_the_object() {
    let alloc = Allocator::default();
    let (code, notes) = run_one(
        &alloc,
        "function hi() {\n  var obj = { aokfw: function add(a, b) { return a + b; } };\n  console.log(obj.aokfw === obj.aokfw);\n}",
    );
    assert!(code.contains("var obj"), "{code}");
    assert!(code.contains("obj.aokfw === obj.aokfw"), "{code}");
    assert_eq!(code.matches("function add").count(), 1, "{code}");
    assert!(
        notes
            .iter()
            .any(|n| n.contains("its identity must stay shared")),
        "{notes:?}"
    );
}

#[test]
fn an_uncalled_read_does_not_stop_the_other_reads_of_the_object() {
    assert_transform!(
        "function hi() {\n  var obj = {\n    rLxJs: '6|0|4',\n    aokfw: function (a, b) { return a + b; },\n    HLAuI: function (a, b, c) { return a(b, c); }\n  };\n  console.log(obj.rLxJs, obj.aokfw(1, 2), obj.HLAuI);\n}",
        [untwist_deobfuscate::control_flow_object::boxed],
        @r#"
    function hi() {
      var obj = {
        rLxJs: "6|0|4",
        aokfw: function (a, b) {
          return a + b;
        },
        HLAuI: function (a, b, c) {
          return a(b, c);
        }
      };
      console.log("6|0|4", 1 + 2, obj.HLAuI);
    }
    "#
    );
}

#[test]
fn duplicate_declared_keys_are_ambiguous_so_the_object_is_kept() {
    assert_transform!(
        "function hi() {\n  var obj = { aokfw: 'first', aokfw: 'second' };\n  console.log(obj.aokfw);\n}",
        [untwist_deobfuscate::control_flow_object::boxed],
        @r#"
    function hi() {
      var obj = {
        aokfw: "first",
        aokfw: "second"
      };
      console.log(obj.aokfw);
    }
    "#
    );
}

#[test]
fn a_nested_control_flow_object_resolves_on_the_driver_fixpoint() {
    let (code, _notes) = common::run_passes(
        "function hi() {\n  var b = { qwert: function (x, y) { return x * y; } };\n  var a = { asdfg: function (x, y) { return b.qwert(x, y); } };\n  console.log(a.asdfg(2, 3));\n}",
    );
    assert!(code.contains("2 * 3"), "{code}");
    assert!(!code.contains("var a"), "{code}");
    assert!(!code.contains("var b"), "{code}");
    assert!(!code.contains("a."), "{code}");
    assert!(!code.contains("b."), "{code}");
}

#[test]
fn a_declared_object_no_one_reads_is_removed() {
    assert_transform!(
        "function hi() {\n  var obj = { rLxJs: '6|0|4' };\n  console.log(1);\n}",
        [untwist_deobfuscate::control_flow_object::boxed],
        @r"
    function hi() {
      console.log(1);
    }
    "
    );
}

#[test]
fn a_read_nested_in_another_reads_call_is_substituted_first() {
    assert_transform!(
        "function hi() {\n  var obj = { rLxJs: 'x', aokfw: function (a, b) { return a + b; } };\n  console.log(obj.aokfw(obj.rLxJs, 2));\n}",
        [untwist_deobfuscate::control_flow_object::boxed],
        @r#"
    function hi() {
      console.log("x" + 2);
    }
    "#
    );
}

const A_REDECLARED_DISPATCH_OBJECT: &str =
    "function hi() {\n  var obj = { rLxJs: \"6|0|4\" };\n  log(obj.rLxJs);\n  var obj = { rLxJs: \"zzz\" };\n  log(obj.rLxJs);\n}";

#[test]
fn a_redeclared_dispatch_object_is_left_alone_in_both_modes() {
    let alloc = Allocator::default();
    assert_eq!(
        common::default_run(
            &alloc,
            A_REDECLARED_DISPATCH_OBJECT,
            untwist_deobfuscate::control_flow_object::boxed,
        )
        .trim(),
        A_REDECLARED_DISPATCH_OBJECT.trim()
    );
    let alloc = Allocator::default();
    assert_eq!(
        common::conservative_run(
            &alloc,
            A_REDECLARED_DISPATCH_OBJECT,
            untwist_deobfuscate::control_flow_object::boxed,
        )
        .trim(),
        A_REDECLARED_DISPATCH_OBJECT.trim()
    );
}

const FIVE_UNCALLED_FUNCTION_READS: &str = "function hi() {\n  var obj = {\n    aokfw: function (a, b) {\n      return a + b;\n    },\n    bqwer: 'x'\n  };\n  var g1 = obj.aokfw;\n  var g2 = obj.aokfw;\n  var g3 = obj.aokfw;\n  var g4 = obj.aokfw;\n  var g5 = obj.aokfw;\n  console.log(g1, g2, g3, g4, g5, obj.bqwer);\n}";

#[test]
fn more_than_three_uncalled_reads_collapse_into_one_summary() {
    let (_code, report) = common::run_report_extended(FIVE_UNCALLED_FUNCTION_READS);
    let named: Vec<&untwist_core::pass::Note> = report
        .notes
        .iter()
        .filter(|note| {
            note.message
                .contains(untwist_deobfuscate::notes::CONTROL_FLOW_UNCALLED_FUNCTION_PROPERTY)
        })
        .collect();
    let spanned = named.iter().filter(|note| note.span.is_some()).count();
    assert_eq!(spanned, untwist_core::pass::SPANNED_NOTE_CAP, "{named:?}");
    let summaries: Vec<&String> = named
        .iter()
        .filter(|note| note.span.is_none())
        .map(|note| &note.message)
        .collect();
    assert_eq!(
        summaries,
        vec![&untwist_deobfuscate::notes::control_flow_uncalled_function_properties(5)]
    );
}
