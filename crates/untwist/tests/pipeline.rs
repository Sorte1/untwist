use untwist::{Options, Pipeline, StopReason};

#[test]
fn default_pipeline_unminifies() {
    let out = Pipeline::new(Options::default()).run("var a=!0,b=!1;x(),y();console[\"log\"](a,b)");
    assert_eq!(
        out.code(),
        "var a = true;\nvar b = false;\nx();\ny();\nconsole.log(a, b);"
    );
    assert!(out.stages.iter().any(|s| s.stage == "prepare"));
    assert!(out.stages.iter().any(|s| s.stage == "unminify"));
}

#[test]
fn unminify_can_be_disabled() {
    let out = Pipeline::new(Options {
        unminify: false,
        ..Options::default()
    })
    .run("var a=!0;");
    assert_eq!(out.code(), "var a = !0;");
    assert!(!out.stages.iter().any(|s| s.stage == "unminify"));
}

#[test]
fn unsafe_passes_only_run_when_enabled() {
    let src = "var s = \"a\".concat(b, \"c\");";
    let safe = Pipeline::new(Options::default()).run(src);
    assert_eq!(safe.code(), "var s = \"a\".concat(b, \"c\");");
    let unsafe_out = Pipeline::new(Options {
        unsafe_transforms: true,
        ..Options::default()
    })
    .run(src);
    assert_eq!(unsafe_out.code(), "var s = `a${b}c`;");
}

#[test]
fn bookmarklet_prefix_is_handled() {
    let out = Pipeline::new(Options::default()).run("javascript:alert(%22hi%22)");
    assert_eq!(out.code(), "alert(\"hi\");");
}

#[test]
fn iteration_cap_is_reported_when_hit() {
    let out = Pipeline::new(Options {
        max_iterations: Some(1),
        ..Options::default()
    })
    .run("var a=!0,b=!1;x(),y();console[\"log\"](a,b)");
    assert!(out
        .stages
        .iter()
        .any(|s| s.stop == StopReason::IterationCap));
}

#[test]
fn a_default_pipeline_reports_convergence_on_every_stage() {
    assert_eq!(Options::default().max_iterations, None);
    let out = Pipeline::new(Options::default()).run("var a=!0,b=!1;x(),y();console[\"log\"](a,b)");
    assert!(out.stages.iter().all(|s| s.stop == StopReason::Converged));
}

#[test]
fn the_deobfuscate_stage_runs_and_is_reported() {
    let source = "\
function a() { var h = ['log', 'Hello World!']; a = function () { return h; }; return a(); }
function b(c, d) { var e = a(); b = function (f, g) { f = f - 0x0; return e[f]; }; return b(c, d); }
console[b(0x0)](b(0x1));";
    let out = untwist::Pipeline::new(untwist::Options::default()).run(source);
    assert_eq!(out.code().trim(), "console.log(\"Hello World!\");");
    let stage = out
        .stages
        .iter()
        .find(|stage| stage.stage == "deobfuscate")
        .expect("deobfuscate stage reported");
    assert!(stage.changes > 0);
    assert!(!out
        .notes
        .iter()
        .any(|note| note.message.contains("deobfuscate: not implemented")));
}

#[test]
fn no_deobfuscate_leaves_the_machinery_alone() {
    let source = "\
function a() { var h = ['log']; a = function () { return h; }; return a(); }
function b(c, d) { var e = a(); b = function (f, g) { f = f - 0x0; return e[f]; }; return b(c, d); }
console.log(b(0x0));";
    let options = untwist::Options {
        deobfuscate: false,
        ..untwist::Options::default()
    };
    let out = untwist::Pipeline::new(options).run(source);
    assert!(out.code().contains("function a()"), "{}", out.code());
    assert!(out.code().contains("b(0)"), "{}", out.code());
    assert!(out.stages.iter().all(|stage| stage.stage != "deobfuscate"));

    let on = untwist::Pipeline::new(untwist::Options::default()).run(source);
    assert!(!on.code().contains("function a()"), "{}", on.code());
    assert!(!on.code().contains("b(0)"), "{}", on.code());
    assert!(on.code().contains("console.log(\"log\")"), "{}", on.code());
    assert!(on.stages.iter().any(|stage| stage.stage == "deobfuscate"));
}

const SELF_DEFENDING: &str = "function hi() {\n  var e = (function () {\n    var f = !![];\n    return function (g, h) {\n      var i = f ? function () {\n        if (h) {\n          var j = h['apply'](g, arguments);\n          h = null;\n          return j;\n        }\n      } : function () {};\n      f = ![];\n      return i;\n    };\n  })();\n  (function () {\n    e(this, function () {\n      return 1;\n    })();\n  })();\n  console.log('x');\n}\nhi();";

fn stage_index(out: &untwist::Output, name: &str, round: u32) -> usize {
    out.stages
        .iter()
        .position(|stage| stage.stage == name && stage.round == round)
        .unwrap_or_else(|| panic!("{name} stage, round {round}, reported"))
}

#[test]
fn the_protections_stage_runs_after_rename_and_removes_the_self_defending_block() {
    let out = Pipeline::new(Options::default()).run(SELF_DEFENDING);
    assert_eq!(
        out.code().trim(),
        "function hi() {\n  console.log(\"x\");\n}\nhi();"
    );
    let protections = &out.stages[stage_index(&out, "protections", 1)];
    assert_eq!(protections.changes, 2);
    assert_eq!(
        protections.per_pass,
        vec![("self-defending", 2), ("debug-protection", 0)]
    );
    assert!(stage_index(&out, "protections", 1) > stage_index(&out, "deobfuscate", 1));
    assert!(stage_index(&out, "protections", 1) > stage_index(&out, "unminify", 1));
}

#[test]
fn a_renamed_program_still_matches_the_protection_template() {
    let out = Pipeline::new(Options {
        rename: untwist::RenameMode::Simple,
        rename_all: false,
        extended: false,
        ..Options::default()
    })
    .run(SELF_DEFENDING);
    assert!(!out.code().contains("h.apply"), "{}", out.code());
    assert!(out.code().contains("console.log(\"x\")"), "{}", out.code());
    assert!(stage_index(&out, "protections", 1) > stage_index(&out, "rename", 1));
    assert!(stage_index(&out, "protections", 1) > stage_index(&out, "rename-shorthand", 1));
}

#[test]
fn no_deobfuscate_leaves_the_protections_alone() {
    let out = Pipeline::new(Options {
        deobfuscate: false,
        ..Options::default()
    })
    .run(SELF_DEFENDING);
    assert!(out.code().contains("h.apply"), "{}", out.code());
    assert!(out.stages.iter().all(|stage| stage.stage != "protections"));
}

const A_MERGEABLE_OBJECT_AND_AN_ATOB_CALL: &str = "const obj = {};\nobj.a = 1;\nconsole.log(obj);\nconsole.log(obj);\nconsole.log(atob(\"aGVsbG8=\"));";

#[test]
fn the_cleanup_stage_runs_after_protections_and_rewrites_both_passes_shapes() {
    let out = Pipeline::new(Options::default()).run(A_MERGEABLE_OBJECT_AND_AN_ATOB_CALL);
    assert!(
        out.code().contains("const obj = { a: 1 };"),
        "{}",
        out.code()
    );
    assert!(out.code().contains("\"hello\""), "{}", out.code());
    assert!(!out.code().contains("atob("), "{}", out.code());
    assert!(stage_index(&out, "cleanup", 1) > stage_index(&out, "protections", 1));
}

#[test]
fn no_deobfuscate_skips_the_cleanup_stage() {
    let out = Pipeline::new(Options {
        deobfuscate: false,
        ..Options::default()
    })
    .run(A_MERGEABLE_OBJECT_AND_AN_ATOB_CALL);
    assert!(out.code().contains("obj.a = 1;"), "{}", out.code());
    assert!(out.code().contains("atob("), "{}", out.code());
    assert!(out.stages.iter().all(|stage| stage.stage != "cleanup"));
}

const A_LITERAL_MERGE_OBJECT_ASSIGNMENTS_INLINES: &str =
    "var o = { foo: 1 };\nconsole.log(o[atob(\"Zm9v\")]);";

#[test]
fn the_default_pipeline_reads_through_a_literal_merge_object_assignments_builds() {
    let out = Pipeline::new(Options::default()).run(A_LITERAL_MERGE_OBJECT_ASSIGNMENTS_INLINES);
    assert!(out.code().contains("console.log(1)"), "{}", out.code());
}

const A_CLEANUP_RESULT_UNMINIFY_CAN_SIMPLIFY: &str =
    "var o = { foo: f() };\nconsole.log(o[atob(\"Zm9v\")]);";

#[test]
fn a_cleanup_result_unminify_can_simplify_is_simplified_in_the_same_run() {
    let out = Pipeline::new(Options::default()).run(A_CLEANUP_RESULT_UNMINIFY_CAN_SIMPLIFY);
    assert!(out.code().contains(".foo"), "{}", out.code());
    assert!(!out.code().contains("[\"foo\"]"), "{}", out.code());
    assert_eq!(
        out.stages
            .iter()
            .filter(|stage| stage.stage == "unminify")
            .count(),
        2,
        "{:?}",
        out.stages.iter().map(|s| s.stage).collect::<Vec<_>>()
    );
}

#[test]
fn a_settle_round_is_numbered_so_no_consumer_keys_on_the_first() {
    let out = Pipeline::new(Options::default()).run(A_CLEANUP_RESULT_UNMINIFY_CAN_SIMPLIFY);
    let rounds: Vec<u32> = out
        .stages
        .iter()
        .filter(|stage| stage.stage == "unminify")
        .map(|stage| stage.round)
        .collect();
    assert_eq!(rounds, vec![1, 2], "{rounds:?}");
    assert!(stage_index(&out, "unminify", 2) > stage_index(&out, "unminify", 1));
    assert!(out
        .stages
        .iter()
        .all(|stage| stage.round >= 1 && stage.fixpoint_per_pass.len() <= 4));
}

#[test]
fn the_pipeline_settles_when_cleanup_feeds_unminify() {
    let pipeline = Pipeline::new(Options::default());
    let first = pipeline.run(A_CLEANUP_RESULT_UNMINIFY_CAN_SIMPLIFY);
    let second = pipeline.run(first.code());
    for stage in &second.stages {
        assert_eq!(
            (stage.iterations, stage.changes),
            (1, 0),
            "{} is not idempotent",
            stage.stage
        );
    }
    assert_eq!(second.code(), first.code());
}

const A_COMPUTED_MEMBER_THE_CLEANUP_STAGE_DECODES: &str = "console.log(x[atob(\"Zm9v\")]);";

#[test]
fn conservative_leaves_the_decoded_computed_member_where_reference_leaves_it() {
    let out = Pipeline::new(Options {
        extended: false,
        ..Options::default()
    })
    .run(A_COMPUTED_MEMBER_THE_CLEANUP_STAGE_DECODES);
    assert_eq!(out.code().trim(), "console.log(x[\"foo\"]);");
    assert_eq!(
        out.stages
            .iter()
            .filter(|stage| stage.stage == "unminify")
            .count(),
        1
    );
}

#[test]
fn the_default_pipeline_simplifies_the_decoded_computed_member() {
    let pipeline = Pipeline::new(Options::default());
    let first = pipeline.run(A_COMPUTED_MEMBER_THE_CLEANUP_STAGE_DECODES);
    assert_eq!(first.code().trim(), "console.log(x.foo);");
    let second = pipeline.run(first.code());
    for stage in &second.stages {
        assert_eq!(
            (stage.iterations, stage.changes),
            (1, 0),
            "{} is not idempotent",
            stage.stage
        );
    }
    assert_eq!(second.code(), first.code());
}

#[test]
fn a_refusal_the_settle_round_repeats_is_reported_once() {
    let out = Pipeline::new(Options::default())
        .run("var o = { foo: 1 };\nconsole.log(o[atob(\"Zm9v\")]);\ng(atob(\"-\"));");
    let refusals = out
        .notes
        .iter()
        .filter(|note| note.message.contains("was not evaluated"))
        .count();
    assert_eq!(refusals, 1, "{:?}", out.notes);
}

#[test]
fn a_renamed_pipeline_settles_too() {
    let pipeline = Pipeline::new(Options {
        rename: untwist::RenameMode::Mangle,
        ..Options::default()
    });
    let first = pipeline.run(A_CLEANUP_RESULT_UNMINIFY_CAN_SIMPLIFY);
    let second = pipeline.run(first.code());
    for stage in &second.stages {
        assert_eq!(
            (stage.iterations, stage.changes),
            (1, 0),
            "{} is not idempotent",
            stage.stage
        );
    }
    assert_eq!(second.code(), first.code());
}

#[test]
fn every_pipeline_note_constant_is_catalogued_exactly_once() {
    untwist_testing::notes::every_constant_is_catalogued_exactly_once(
        include_str!("../src/pipeline/notes.rs"),
        &[untwist::PIPELINE_NOTES],
    );
}

#[test]
fn no_pipeline_note_is_a_substring_of_another() {
    untwist_testing::notes::no_note_is_a_substring_of_another(&[untwist::PIPELINE_NOTES]);
}

#[test]
fn all_pass_ctors_holds_every_group() {
    let expected = untwist_unminify::prepare_ctors().len()
        + untwist_unminify::transpile_ctors().len()
        + untwist_unminify::unminify_ctors().len()
        + untwist_unminify::lexical_ctors().len()
        + untwist_deobfuscate::deobfuscate_ctors().len()
        + untwist_deobfuscate::cleanup_fixpoint_ctors().len()
        + untwist_deobfuscate::protections_ctors().len()
        + untwist_deobfuscate::cleanup_ctors().len()
        + untwist_jsx::jsx_ctors().len();
    assert_eq!(untwist::all_pass_ctors().len(), expected);
}

#[test]
fn a_classic_scripts_top_level_let_survives_the_default_pipeline() {
    let script = "let x = 1;\n(0, eval)(\"x = 2\");\nconsole.log(x);";
    let out = Pipeline::new(Options {
        unpack: false,
        ..Options::default()
    })
    .run(script);
    assert_eq!(out.code(), script);

    let module = format!("{script}\nexport {{}};");
    let out = Pipeline::new(Options {
        unpack: false,
        ..Options::default()
    })
    .run(&module);
    assert_eq!(
        out.code(),
        "const x = 1;\n(0, eval)(\"x = 2\");\nconsole.log(x);\nexport {};"
    );
}

const A_SURVIVING_CONTROLLER: &str = "var ctrl = function () {
  var f = !![];
  return function (c, fn) {
    var r = f ? function () {
      if (fn) {
        var x = fn.apply(c, arguments);
        fn = null;
        return x;
      }
    } : function () {};
    f = ![];
    return r;
  };
}();";

const THE_CONTROLLER_ON_ONE_LINE: &str =
    "function () {if (fn) {var x = fn.apply(c, arguments);fn = null;return x;}}";

fn without_deobfuscation() -> Options {
    Options {
        deobfuscate: false,
        unminify: false,
        ..Options::default()
    }
}

fn guard_messages(out: &untwist::Output) -> Vec<String> {
    out.notes
        .iter()
        .filter(|note| {
            note.message.starts_with(untwist::GUARD_ON_ONE_LINE)
                || note.message.starts_with(untwist::GUARD_KEPT_ACROSS_LINES)
        })
        .map(|note| note.message.clone())
        .collect()
}

#[test]
fn both_guard_notes_are_reachable() {
    let compacted = Pipeline::new(without_deobfuscation()).run(A_SURVIVING_CONTROLLER);
    assert!(
        compacted.code().contains(THE_CONTROLLER_ON_ONE_LINE),
        "{}",
        compacted.code()
    );
    assert_eq!(
        guard_messages(&compacted),
        vec![format!("{} 1", untwist::GUARD_ON_ONE_LINE)]
    );
    let kept = Pipeline::new(without_deobfuscation()).run(
        &A_SURVIVING_CONTROLLER.replace("fn.apply(c, arguments)", "fn[`ap\nply`](c, arguments)"),
    );
    assert_eq!(
        guard_messages(&kept),
        vec![format!("{} 1", untwist::GUARD_KEPT_ACROSS_LINES)]
    );
}

#[test]
fn a_let_a_const_and_a_plain_assignment_guard_are_each_noted_once() {
    let one = vec![format!("{} 1", untwist::GUARD_ON_ONE_LINE)];
    for source in [
        A_SURVIVING_CONTROLLER.replacen("var ctrl", "let ctrl", 1),
        A_SURVIVING_CONTROLLER.replacen("var ctrl", "const ctrl", 1),
        A_SURVIVING_CONTROLLER.replacen("var ctrl = ", "var ctrl;\nctrl = ", 1),
    ] {
        let out = Pipeline::new(without_deobfuscation()).run(&source);
        assert!(
            out.code().contains(THE_CONTROLLER_ON_ONE_LINE),
            "{}",
            out.code()
        );
        assert_eq!(guard_messages(&out), one, "{source}");
    }
}

#[test]
fn a_compound_or_destructuring_assignment_of_a_guard_is_not_noted() {
    for source in [
        A_SURVIVING_CONTROLLER.replacen("var ctrl = ", "var ctrl;\nctrl += ", 1),
        A_SURVIVING_CONTROLLER.replacen("var ctrl = ", "var ctrl;\n[ctrl] = ", 1),
    ] {
        let out = Pipeline::new(without_deobfuscation()).run(&source);
        assert!(
            !out.code().contains(THE_CONTROLLER_ON_ONE_LINE),
            "{}",
            out.code()
        );
        assert!(guard_messages(&out).is_empty(), "{source}");
    }
}
