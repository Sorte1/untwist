use untwist::{Options, Pipeline, RenameMode};

fn with(mode: RenameMode, rename_all: bool) -> Options {
    Options {
        rename: mode,
        rename_all,
        ..Options::default()
    }
}

#[test]
fn rename_is_off_by_default() {
    let out = Pipeline::new(Options::default()).run("var _0xa = 1;\n_0xa;");
    assert!(
        out.code().contains("_0xa"),
        "default options must not rename: {}",
        out.code()
    );
}

#[test]
fn mangle_mode_runs_through_the_pipeline() {
    let out = Pipeline::new(with(RenameMode::Mangle, false)).run("var _0xa = 1;\n_0xa;");
    assert_eq!(out.code(), "var vLN1 = 1;\nvLN1;");
}

#[test]
fn mangle_is_idempotent_on_a_var_holding_a_named_function_expression() {
    let source = "var named = function inner() {\n  return 2;\n};";
    let once = Pipeline::new(with(RenameMode::Mangle, false)).run(source);
    assert_eq!(once.code(), "var vF = function f() {\n  return 2;\n};");
    let twice = Pipeline::new(with(RenameMode::Mangle, false)).run(once.code());
    assert_eq!(twice.code(), once.code());
}

#[test]
fn under_conservative_mangle_matches_the_references_first_run() {
    let out = Pipeline::new(Options {
        rename: RenameMode::Mangle,
        extended: false,
        ..Options::default()
    })
    .run("var named = function inner() {\n  return 2;\n};");
    assert_eq!(out.code(), "var vInner = function f() {\n  return 2;\n};");
}

#[test]
fn simple_mode_runs_through_the_pipeline() {
    let out = Pipeline::new(with(RenameMode::Simple, false)).run("var _0xa = 1;\n_0xa;");
    assert_eq!(out.code(), "var a = 1;\na;");
}

#[test]
fn unique_mode_shares_its_counters_across_inputs() {
    let out = Pipeline::new(with(RenameMode::Unique, false))
        .run_many(&["function _0xa() {}", "function _0xb() {}"]);
    assert_eq!(out.codes[0], "function fn1() {}");
    assert_eq!(out.codes[1], "function fn2() {}");
}

#[test]
fn unique_mode_numbers_the_same_way_however_many_inputs_precede_an_empty_one() {
    let out = Pipeline::new(with(RenameMode::Unique, false)).run_many(&[
        "",
        "function _0xa() {}",
        "",
        "function _0xb() {}",
    ]);
    assert_eq!(out.codes[1], "function fn1() {}");
    assert_eq!(out.codes[3], "function fn2() {}");
}

#[test]
fn human_written_names_are_renamed_by_default() {
    let out = Pipeline::new(Options {
        rename: RenameMode::Simple,
        ..Options::default()
    })
    .run("function handleClick(event) {\n  return event;\n}");
    assert_eq!(out.code(), "function a(a) {\n  return a;\n}");
}

#[test]
fn keeping_human_written_names_is_opt_in() {
    let out = Pipeline::new(with(RenameMode::Simple, false))
        .run("function handleClick(event) {\n  return event;\n}");
    assert!(
        out.code().contains("handleClick"),
        "--keep-human-names must keep a human-looking name: {}",
        out.code()
    );
}

#[test]
fn the_shorthand_stage_runs_after_the_mode_stage() {
    let out =
        Pipeline::new(with(RenameMode::Simple, false)).run("let { sin: _0xk } = Math;\n_0xk(1);");
    assert_eq!(out.code(), "let { sin } = Math;\nsin(1);");
}

#[test]
fn the_stage_order_is_prepare_unminify_rename() {
    let out = Pipeline::new(with(RenameMode::Simple, false)).run("var _0xa = 1;\n_0xa;");
    let stages: Vec<&str> = out.stages.iter().map(|stage| stage.stage).collect();
    let index = |name: &str| stages.iter().position(|stage| *stage == name);
    assert!(index("unminify") < index("rename"), "stages: {stages:?}");
    assert!(
        index("rename") < index("rename-shorthand"),
        "stages: {stages:?}"
    );
    assert!(
        index("rename-shorthand") < index("unpack"),
        "stages: {stages:?}"
    );
}

#[test]
fn the_shorthand_stage_is_filtered_out_under_conservative() {
    let source = "let { sin: _0xk } = Math;\n_0xk(1);";
    let beyond = Pipeline::new(with(RenameMode::Mangle, false)).run(source);
    assert!(
        beyond.code().contains("{ sin }"),
        "the untwist-only stage runs by default: {}",
        beyond.code()
    );
    let conservative = Pipeline::new(Options {
        rename: RenameMode::Mangle,
        extended: false,
        ..Options::default()
    })
    .run(source);
    assert!(
        !conservative.code().contains("{ sin }"),
        "the shorthand stage has no counterpart in the reference deobfuscator, so the conservative mode must not run it: {}",
        conservative.code()
    );
}

#[test]
fn a_sloppy_block_level_function_keeps_its_call_site_through_the_pipeline() {
    for mode in [RenameMode::Mangle, RenameMode::Simple, RenameMode::Unique] {
        let out = Pipeline::new(with(mode, true)).run(
            "function f() {\n  {\n    function g() {\n      return 7;\n    }\n  }\n  return g();\n}",
        );
        assert!(
            out.code().contains("function g(") && out.code().contains("return g()"),
            "{mode:?} stranded the Annex B.3.3 call site: {}",
            out.code()
        );
    }
}

#[test]
fn an_if_clause_function_the_unminifier_wrapped_in_a_block_keeps_its_call_site() {
    for mode in [RenameMode::Mangle, RenameMode::Simple, RenameMode::Unique] {
        let out = Pipeline::new(with(mode, true)).run(
            "function f() {\n  if (true) function g() {\n    return 7;\n  }\n  return g();\n}",
        );
        assert!(
            out.code().contains("function g(") && out.code().contains("return g()"),
            "{mode:?} stranded the call after the unminifier turned the if clause into a block: {}",
            out.code()
        );
    }
}

#[test]
fn no_rename_stage_is_reported_when_the_mode_is_none() {
    let out = Pipeline::new(Options::default()).run("var _0xa = 1;");
    let stages: Vec<&str> = out.stages.iter().map(|stage| stage.stage).collect();
    assert!(!stages.contains(&"rename"), "stages: {stages:?}");
    assert!(!stages.contains(&"rename-shorthand"), "stages: {stages:?}");
}

#[test]
fn unique_mode_with_no_inputs_returns_cleanly() {
    let out = Pipeline::new(with(RenameMode::Unique, false)).run_many(&[]);
    assert!(out.codes.is_empty());
    assert_eq!(out.code(), "");
    assert!(out.notes.is_empty(), "{:?}", out.notes);
}

#[test]
fn class_hints_never_bind_a_global_the_program_reads() {
    let source = "var _0xa = (function () {\n  function _0xb() {\n    this.type = \"Map\";\n    this.items = new Map();\n  }\n  return _0xb;\n})();\n\
         class _0xc {\n  constructor() {\n    this.type = \"Array\";\n    this.list = Array.of(1);\n  }\n}\n\
         var _0xd = new _0xa();\nvar _0xe = new _0xc();\nconsole.log(_0xd.items instanceof Map, Array.isArray(_0xe.list));";
    for rename_all in [false, true] {
        let code = Pipeline::new(with(RenameMode::Unique, rename_all))
            .run(source)
            .code()
            .to_string();
        for global in ["Map", "Array"] {
            for binder in ["function ", "class ", "var ", "let ", "const "] {
                assert!(
                    !code.contains(&format!("{binder}{global} "))
                        && !code.contains(&format!("{binder}{global}(")),
                    "{global} is bound ({rename_all}):\n{code}"
                );
            }
        }
        assert!(code.contains("new Map()"), "{code}");
        assert!(code.contains("Array.isArray("), "{code}");
    }
}
