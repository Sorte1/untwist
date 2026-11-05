use std::path::Path;

use untwist::{Options, Pipeline};
use untwist_testing::js_files_in;
use untwist_testing::node::run;
use untwist_testing::node_rows::{
    self, node_or_skip, observed, patched_after_rotator, printed, Mode,
};

const CORPUS_PRINTS: &str = "log\nHello World!\nconsole\ndeobfuscate\n";

fn modes() -> [(&'static str, Options); 2] {
    [
        ("default", Options::default()),
        (
            "conservative",
            Options {
                extended: false,
                unsafe_transforms: true,
                ..Options::default()
            },
        ),
    ]
}

fn is_final_call(statement: &str) -> bool {
    statement.starts_with("b(") || statement.starts_with("(function () { var t = ")
}

fn printing_twin(source: &str) -> String {
    source
        .lines()
        .map(|line| match line.strip_suffix(';') {
            Some(statement) if is_final_call(statement) => format!("console.log({statement});"),
            _ => line.to_string(),
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn unsafe_lane() -> Options {
    Options {
        unsafe_transforms: true,
        ..Options::default()
    }
}

fn default_mismatches(node: &Path, programs: &[(String, String)]) -> Vec<String> {
    let run = |source: &str| {
        Pipeline::new(Options::default())
            .run(source)
            .code()
            .to_string()
    };
    let runs: [Mode<'_>; 1] = [("default", &run)];
    node_rows::mismatches(node, programs, &runs)
}

fn mismatches(node: &Path, programs: &[(String, String)]) -> Vec<String> {
    let [(first, a), (second, b)] = modes();
    let run_a = |source: &str| Pipeline::new(a.clone()).run(source).code().to_string();
    let run_b = |source: &str| Pipeline::new(b.clone()).run(source).code().to_string();
    let runs: [Mode<'_>; 2] = [(first, &run_a), (second, &run_b)];
    node_rows::mismatches(node, programs, &runs)
}

#[test]
fn every_decoder_corpus_case_prints_what_node_prints_after_untwist() {
    let Some(node) = node_or_skip() else {
        return;
    };
    let twins: Vec<(String, String)> = js_files_in("fixtures/eval")
        .into_iter()
        .map(|(label, source)| (label, printing_twin(&source)))
        .collect();
    assert_eq!(twins.len(), 36);
    let dir = tempfile::tempdir().unwrap();
    let expected = format!("{CORPUS_PRINTS}exit 0\n--\n{CORPUS_PRINTS}exit 0\n--\n");
    for (label, twin) in &twins {
        assert_eq!(
            printed(&node, dir.path(), twin),
            Ok(expected.clone()),
            "{label}"
        );
    }
    let wrong = mismatches(&node, &twins);
    assert!(
        wrong.is_empty(),
        "{} of 72 runs differ:\n{}",
        wrong.len(),
        wrong.join("\n")
    );
}

#[test]
fn every_behaviour_fixture_prints_what_node_prints_after_untwist() {
    let Some(node) = node_or_skip() else {
        return;
    };
    let fixtures = js_files_in("fixtures/behaviour");
    let wrong = mismatches(&node, &fixtures);
    assert!(
        wrong.is_empty(),
        "{} of {} runs differ:\n{}",
        wrong.len(),
        2 * fixtures.len(),
        wrong.join("\n")
    );
}

#[test]
fn every_decoder_corpus_case_is_decoded_and_its_machinery_removed() {
    let cases = js_files_in("fixtures/eval");
    assert_eq!(cases.len(), 36);
    for (label, source) in cases {
        for (mode, options) in modes() {
            let code = Pipeline::new(options).run(&source).code().to_string();
            assert!(
                !code.contains("__DECODE_") && !code.contains("__STRING_ARRAY_"),
                "{label} ({mode}):\n{code}"
            );
            assert!(
                !code.contains("function a(") && !code.contains("function b("),
                "{label} ({mode}):\n{code}"
            );
            for plain in [
                "\"log\"",
                "\"Hello World!\"",
                "\"console\"",
                "\"deobfuscate\"",
            ] {
                assert!(code.contains(plain), "{label} ({mode}): {plain}\n{code}");
            }
        }
    }
}

#[test]
fn the_rotator_fixtures_whose_writes_no_decode_reads_are_decoded() {
    let source = behaviour_fixture("55-string-out-of-range.js");
    for (mode, options) in modes() {
        let code = Pipeline::new(options).run(&source).code().to_string();
        assert!(
            !code.contains("b(") && !code.contains("function a("),
            "55-string-out-of-range.js ({mode}):\n{code}"
        );
    }
}

#[test]
fn a_printed_length_of_a_polyfilled_call_keeps_the_machinery_and_inlines_its_calls() {
    let source = behaviour_fixture("51-harmless-flatmap-polyfill.js");
    for (mode, options) in modes() {
        let code = Pipeline::new(options).run(&source).code().to_string();
        assert!(
            !code.contains("console.log(b(") && code.contains("function a("),
            "51-harmless-flatmap-polyfill.js ({mode}):\n{code}"
        );
        assert!(
            code.contains("console.log(\"Hello World!\")"),
            "51-harmless-flatmap-polyfill.js ({mode}):\n{code}"
        );
    }
}

fn printed_as_module(node: &Path, dir: &Path, text: &str) -> String {
    let path = dir.join("program.mjs");
    std::fs::write(&path, text).unwrap();
    let out = run(node, &path).unwrap().expect("finishes");
    observed(&out)
}

#[test]
fn a_module_whose_import_runs_before_the_rotator_prints_what_node_prints_after_untwist() {
    let Some(node) = node_or_skip() else {
        return;
    };
    let path = format!(
        "{}/../../fixtures/behaviour/module-import-hoists-before-rotator.mjs",
        env!("CARGO_MANIFEST_DIR")
    );
    let source = std::fs::read_to_string(path).unwrap();
    let dir = tempfile::tempdir().unwrap();
    let before = printed_as_module(&node, dir.path(), &source);
    assert_eq!(before, "3a 9 1 3 2\nexit 0\n");
    for (mode, options) in modes() {
        let after = printed_as_module(
            &node,
            dir.path(),
            Pipeline::new(options).run(&source).code(),
        );
        assert_eq!(after, before, "{mode}");
    }
}

#[test]
fn under_unsafe_a_module_whose_import_patches_the_rotator_differs_from_node_only_by_that_patch() {
    let Some(node) = node_or_skip() else {
        return;
    };
    let path = format!(
        "{}/../../fixtures/behaviour/module-import-hoists-before-rotator.mjs",
        env!("CARGO_MANIFEST_DIR")
    );
    let source = std::fs::read_to_string(path).unwrap();
    let options = Options {
        unsafe_transforms: true,
        ..Options::default()
    };
    let code = Pipeline::new(options).run(&source).code().to_string();
    assert!(
        code.contains(r#"console.log("3", "2", "3a", "9", "1")"#),
        "{code}"
    );
    let dir = tempfile::tempdir().unwrap();
    let after = printed_as_module(&node, dir.path(), &code);
    assert_eq!(after, "3 2 3a 9 1\nexit 0\n");
    let unpatched = source
        .lines()
        .filter(|line| !line.starts_with("import "))
        .collect::<Vec<_>>()
        .join("\n");
    assert_eq!(printed_as_module(&node, dir.path(), &unpatched), after);
}

const GLOBAL_ENUMERATION: &str = "\
function a() { var h = ['s0', 's1', 's2']; a = function () { return h; }; return a(); }
function b(i) { var e = a(); return e[i]; }
var g = Function('return this')();
for (var k in g) {
  if (typeof g[k] === 'function' && k.length === 1 && g[k].length === 1) {
    g[k] = function (i) { return 'EVIL' + i; };
  }
}
console.log(b(0), b(1), b(2));";

#[test]
fn a_decoder_replaced_through_global_enumeration_is_refused_and_prints_like_node() {
    let Some(node) = node_or_skip() else {
        return;
    };
    let dir = tempfile::tempdir().unwrap();
    let before = printed(&node, dir.path(), GLOBAL_ENUMERATION).unwrap();
    assert_eq!(
        before,
        "s0 s1 s2\nexit 0\n--\nEVIL0 EVIL1 EVIL2\nexit 0\n--\n"
    );
    let code = Pipeline::new(Options::default())
        .run(GLOBAL_ENUMERATION)
        .code()
        .to_string();
    assert!(code.contains("console.log(b(0), b(1), b(2))"), "{code}");
    let after = printed(&node, dir.path(), &code).unwrap();
    assert_eq!(after, before);
    let decoded = Pipeline::new(unsafe_lane())
        .run(GLOBAL_ENUMERATION)
        .code()
        .to_string();
    assert!(!decoded.contains("b(0)"), "{decoded}");
}

const SPLIT_NAME_OTHER_UNIT: &str = "\
function a() { var h = ['xq', 'xd', 'xx']; a = function () { return h; }; return a(); }
function b(i) { var e = a(); return e[i].slice(1); }
function p() { var h = ['u0', 'u1']; p = function () { return h; }; return p(); }
function qd(i) { var e = p(); return e[i]; }
Function(b(0) + b(1) + ' = function (i) { return \"EVIL\" + i; }')();
console.log(qd(0), qd(1), b(2));";

const SPLIT_NAME_OWN_UNIT: &str = "\
function a() { var h = ['q', 'd', 'u0', 'u1']; a = function () { return h; }; return a(); }
function qd(i) { var e = a(); return e[i]; }
var s = qd(0) + qd(1);
console.log(qd(2 - 0));
Function(s + ' = function (i) { return \"EVIL\" + i; }')();
console.log(qd(3 - 0));";

const SPLIT_NAME_OWN_UNIT_READING_A_PROTOTYPE: &str = "\
function a() { var h = ['xq', 'xd', 'u0', 'u1']; a = function () { return h; }; return a(); }
function qd(i) { var e = a(); return e[i].slice(1); }
var s = qd(0) + qd(1);
console.log(qd(2 - 0));
Function(s + ' = function (i) { return \"EVIL\" + i; }')();
console.log(qd(3 - 0));";

fn assert_documented_premise(source: &str, before: &str, after: &str, decoded: &str) {
    let Some(node) = node_or_skip() else {
        return;
    };
    let dir = tempfile::tempdir().unwrap();
    assert_eq!(printed(&node, dir.path(), source).unwrap(), before);
    for (mode, options) in modes() {
        let code = Pipeline::new(options).run(source).code().to_string();
        assert!(code.contains(decoded), "{mode}: {code}");
        assert_eq!(printed(&node, dir.path(), &code).unwrap(), after, "{mode}");
    }
}

#[test]
fn a_decoder_name_built_from_two_decoded_halves_is_the_documented_premise() {
    assert_documented_premise(
        SPLIT_NAME_OTHER_UNIT,
        "u0 u1 x\nexit 0\n--\nEVIL0 EVIL1 x\nexit 0\n--\n",
        "u0 u1 x\nexit 0\n--\nu0 u1 x\nexit 0\n--\n",
        "console.log(\"u0\", \"u1\", b(2))",
    );
}

#[test]
fn a_decoder_name_built_in_its_own_unit_is_refused_and_prints_like_node() {
    if let Some(node) = node_or_skip() {
        let programs = [(
            "split-name-own-unit".to_string(),
            SPLIT_NAME_OWN_UNIT.to_string(),
        )];
        let wrong = default_mismatches(&node, &programs);
        assert!(wrong.is_empty(), "{}", wrong.join("\n"));
    }
    let code = Pipeline::new(Options::default())
        .run(SPLIT_NAME_OWN_UNIT)
        .code()
        .to_string();
    assert!(code.contains("var s = qd(0) + qd(1);"), "{code}");
}

#[test]
fn a_unit_that_reads_a_prototype_beside_code_text_it_cannot_resolve_prints_like_node() {
    assert_prints_like_node(
        "split-name-own-unit-slice",
        SPLIT_NAME_OWN_UNIT_READING_A_PROTOTYPE.to_string(),
    );
}

fn instanceof_decoder(test: &str) -> String {
    format!(
        "function s(){{var a=['hello','world'];s=function(){{return a;}};return s();}} \
         function d(i){{var arr=s();var v=arr[i-0];return v+(({test})?'T':'F');}} \
         console.log(d(0),d(1));"
    )
}

#[test]
fn a_decoder_testing_instanceof_object_is_decoded_and_runs_like_node() {
    let Some(node) = node_or_skip() else {
        return;
    };
    let dir = tempfile::tempdir().unwrap();
    let expected = "helloT worldT\nexit 0\n--\nhelloT worldT\nexit 0\n--\n";
    for test in ["[] instanceof Object", "new Object() instanceof Object"] {
        let source = instanceof_decoder(test);
        assert_eq!(
            printed(&node, dir.path(), &source),
            Ok(expected.to_string())
        );
        for (mode, options) in modes() {
            let out = Pipeline::new(options).run(&source);
            assert!(
                out.code().contains("\"helloT\""),
                "{test} ({mode}): {} {:?}",
                out.code(),
                out.notes
            );
            assert_eq!(
                printed(&node, dir.path(), out.code()),
                Ok(expected.to_string()),
                "{test} ({mode})"
            );
        }
    }
}

fn behaviour_fixture(name: &str) -> String {
    let path = format!(
        "{}/../../fixtures/behaviour/{name}",
        env!("CARGO_MANIFEST_DIR")
    );
    std::fs::read_to_string(path).unwrap()
}

#[test]
fn the_instanceof_object_fixtures_are_decoded() {
    for name in [
        "array-instanceof-object.js",
        "new-object-instanceof-object.js",
        "regexp-instanceof-object.js",
        "function-instanceof-object.js",
    ] {
        let source = behaviour_fixture(name);
        for (mode, options) in modes() {
            let code = Pipeline::new(options).run(&source).code().to_string();
            assert!(
                !code.contains("b(") && !code.contains("function a("),
                "{name} ({mode}):\n{code}"
            );
            assert!(code.contains("\"log!\""), "{name} ({mode}):\n{code}");
        }
    }
}

#[test]
fn a_numeric_key_written_on_an_object_reassigned_only_to_literals_is_decoded() {
    let source = behaviour_fixture("rc4-numeric-key-on-object-assigned-after-null.js");
    for (mode, options) in modes() {
        let code = Pipeline::new(options).run(&source).code().to_string();
        assert!(
            !code.contains("b(461") && !code.contains("b(462"),
            "{mode}:\n{code}"
        );
        assert!(
            code.contains("new Error(\"Hello World!\")")
                && code.contains("console.log(\"log\", \"console\", errors[\"28\"])"),
            "{mode}:\n{code}"
        );
    }
}

#[test]
fn a_numeric_key_written_on_an_object_reassigned_to_a_prototype_stays_refused() {
    let source = behaviour_fixture("rc4-numeric-key-on-object-reassigned-to-prototype.js");
    for (mode, options) in modes() {
        let out = Pipeline::new(options).run(&source);
        assert!(
            out.code().contains("b(460, \"DiKO\")"),
            "{mode}:\n{}",
            out.code()
        );
        assert!(
            out.notes.iter().any(|found| found
                .message
                .contains("the program replaces a standard built-in")),
            "{mode}: {:?}",
            out.notes
        );
    }
}

#[test]
fn a_property_read_off_function_does_not_block_decoding_a_proxy_call() {
    let source = behaviour_fixture("rc4-function-prototype-read-beside-a-proxy-call.js");
    for (mode, options) in modes() {
        let code = Pipeline::new(options).run(&source).code().to_string();
        assert!(!code.contains("b(460, \"DiKO\")"), "{mode}:\n{code}");
        assert!(
            code.contains("helpers.log(x, y)")
                && code.contains("console.log(\"Hello World!\", differs(1, 2), native)"),
            "{mode}:\n{code}"
        );
    }
}

#[test]
fn the_function_constructor_read_through_its_prototype_stays_refused() {
    let source = behaviour_fixture("rc4-function-constructor-read-beside-a-proxy-call.js");
    let decoded = Pipeline::new(unsafe_lane()).run(&source);
    assert!(
        !decoded.code().contains("b(461, \"DiKO\")"),
        "{}",
        decoded.code()
    );
    for (mode, options) in modes().into_iter().take(1) {
        let out = Pipeline::new(options).run(&source);
        assert!(
            out.code().contains("b(461, \"DiKO\")"),
            "{mode}:\n{}",
            out.code()
        );
        assert!(
            out.notes.iter().any(|found| found
                .message
                .contains("may reach the string array's function")),
            "{mode}: {:?}",
            out.notes
        );
    }
}

#[test]
fn a_selector_that_parses_as_code_does_not_block_decoding() {
    let source = behaviour_fixture("rc4-selector-argument-that-parses-as-code.js");
    for (mode, options) in modes() {
        let code = Pipeline::new(options).run(&source).code().to_string();
        assert!(
            code.contains("console.log(\"Hello World!\", found)"),
            "{mode}:\n{code}"
        );
    }
}

#[test]
fn a_decoded_key_read_on_window_does_not_block_decoding() {
    let source = behaviour_fixture("rc4-decoded-key-read-on-window.js");
    for (mode, options) in modes() {
        let code = Pipeline::new(options).run(&source).code().to_string();
        assert!(
            code.contains("window.console.toString()")
                && code.contains("console.log(\"Hello World!\", native)"),
            "{mode}:\n{code}"
        );
    }
}

#[test]
fn a_decoded_key_on_window_that_names_function_stays_refused() {
    let source = behaviour_fixture("rc4-decoded-function-key-read-on-window.js");
    for (mode, options) in modes() {
        let out = Pipeline::new(options).run(&source);
        assert!(
            out.code().contains("window[b(461, \"DiKO\")]"),
            "{mode}:\n{}",
            out.code()
        );
    }
}

#[test]
fn a_decoded_key_on_window_that_names_another_global_object_stays_refused() {
    let source = behaviour_fixture("rc4-decoded-self-key-on-window-patches-the-decoder.js");
    for (mode, options) in modes() {
        let out = Pipeline::new(options).run(&source);
        assert!(
            out.code().contains("b(462, \"DiKO\")"),
            "{mode}:\n{}",
            out.code()
        );
    }
}

#[test]
fn the_global_object_taken_through_the_function_prototype_constructor_stays_refused() {
    let source = behaviour_fixture("rc4-function-prototype-constructor-patches-the-decoder.js");
    for (mode, options) in modes() {
        let out = Pipeline::new(options).run(&source);
        assert!(
            out.code().contains("b(461, \"DiKO\")"),
            "{mode}:\n{}",
            out.code()
        );
    }
}

#[test]
fn a_dead_branch_behind_a_table_assembled_from_decoded_keys_does_not_block_decoding() {
    let source = behaviour_fixture("rc4-dead-branch-behind-an-assembled-table.js");
    for (mode, options) in modes() {
        let code = Pipeline::new(options).run(&source).code().to_string();
        assert!(
            !code.contains("b(46") && code.contains("console.log(\"Hello World!\");"),
            "{mode}:\n{code}"
        );
    }
}

#[test]
fn a_live_branch_behind_an_assembled_table_stays_refused() {
    for name in [
        "rc4-live-branch-behind-an-assembled-table.js",
        "rc4-assembled-table-written-through-its-alias.js",
        "rc4-assembled-table-written-elsewhere.js",
        "rc4-assembled-table-under-a-frozen-object-prototype.js",
        "rc4-assembled-table-rebound-through-arguments.js",
    ] {
        let source = behaviour_fixture(name);
        for (mode, options) in modes() {
            let out = Pipeline::new(options).run(&source);
            assert!(
                out.code().contains("b(461, \"DiKO\")"),
                "{name} {mode}:\n{}",
                out.code()
            );
        }
    }
}

#[test]
fn a_map_get_with_an_unknown_key_does_not_block_decoding() {
    let source = behaviour_fixture("rc4-map-get-with-an-unknown-key.js");
    for (mode, options) in modes() {
        let code = Pipeline::new(options).run(&source).code().to_string();
        assert!(
            code.contains("console.log(\"Hello World!\", lookup("),
            "{mode}:\n{code}"
        );
    }
}

#[test]
fn a_reflective_get_with_an_unknown_key_stays_refused() {
    for name in [
        "rc4-reflect-get-with-an-unknown-key.js",
        "rc4-bound-reflect-get-under-a-get-key.js",
    ] {
        let source = behaviour_fixture(name);
        for (mode, options) in modes() {
            let out = Pipeline::new(options).run(&source);
            assert!(
                out.code().contains("b(461, \"DiKO\")"),
                "{name} {mode}:\n{}",
                out.code()
            );
        }
    }
}

#[test]
fn code_text_passed_to_an_unknown_host_method_stays_refused() {
    let source = behaviour_fixture("rc4-code-argument-to-an-unknown-host-call.js");
    for (mode, options) in modes() {
        let out = Pipeline::new(options).run(&source);
        assert!(
            out.code().contains("b(461, \"DiKO\")"),
            "{mode}:\n{}",
            out.code()
        );
    }
}

#[test]
fn the_fixtures_that_patch_object_stay_refused() {
    for (name, note) in [
        (
            "object-reassigned-before-rotator.js",
            "a statement that may run code precedes the rotator",
        ),
        (
            "object-reassigned-after-rotator.js",
            "the program replaces a standard built-in",
        ),
        (
            "object-has-instance-defined-after-rotator.js",
            "the program replaces a standard built-in",
        ),
    ] {
        let source = behaviour_fixture(name);
        for (mode, options) in modes() {
            let out = Pipeline::new(options).run(&source);
            assert!(
                out.code().contains("console.log(b(0))"),
                "{name} ({mode}):\n{}",
                out.code()
            );
            assert!(
                out.notes.iter().any(|found| found.message.contains(note)),
                "{name} ({mode}): {:?}",
                out.notes
            );
        }
    }
}

#[test]
fn the_fixtures_that_reach_the_builtins_stay_refused() {
    let fixtures: Vec<(String, String)> = js_files_in("fixtures/behaviour")
        .into_iter()
        .filter(|(label, _)| label.contains("/builtin-reach-"))
        .collect();
    assert_eq!(fixtures.len(), 40);
    for (label, source) in fixtures {
        for (mode, options) in modes() {
            let out = Pipeline::new(options).run(&source);
            assert!(
                out.code().contains("console.log(b(0))"),
                "{label} ({mode}):\n{}",
                out.code()
            );
        }
    }
}

#[test]
fn a_second_unit_after_a_proven_first_unit_is_decoded_and_its_near_misses_stay_refused() {
    let fixtures: Vec<(String, String)> = js_files_in("fixtures/behaviour")
        .into_iter()
        .filter(|(label, _)| label.contains("/second-unit-"))
        .collect();
    assert_eq!(fixtures.len(), 22);
    for (label, source) in fixtures {
        let decoded = label.ends_with("/second-unit-two-units-each-printing.js");
        for (mode, options) in modes() {
            let code = Pipeline::new(options).run(&source).code().to_string();
            if decoded {
                assert!(
                    code.contains("\"first\"") && code.contains("\"second\""),
                    "{label} ({mode}):\n{code}"
                );
                assert!(
                    !code.contains("q(") && !code.contains("function p("),
                    "{label} ({mode}):\n{code}"
                );
            } else {
                assert!(code.contains("q("), "{label} ({mode}):\n{code}");
            }
        }
    }
}

fn assert_prints_like_node(label: &str, source: String) {
    let Some(node) = node_or_skip() else {
        return;
    };
    let wrong = mismatches(&node, &[(label.to_string(), source)]);
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

#[test]
fn a_string_replaced_through_a_global_reached_by_function_prints_like_node() {
    assert_prints_like_node(
        "x-fnthis-string",
        patched_after_rotator(
            "String.fromCharCode(33) === '!'",
            "Function('return this')().String = { fromCharCode: function () { return '?'; } };",
        ),
    );
}

const FROM_CHAR_CODE_REPLACED: &str = "{ fromCharCode: function () { return '?'; } }";

fn string_writes() -> Vec<(&'static str, String)> {
    let replaced = FROM_CHAR_CODE_REPLACED;
    let patch = "function () { return '?'; }";
    vec![
        ("s-cond", format!("var g = typeof window !== 'undefined' ? window : global; g.String = {replaced};")),
        ("s-fnthis-member", format!("Function('return this')().String.fromCharCode = {patch};")),
        ("s-ctor-literal", format!("var S = ''['constructor']; S.fromCharCode = {patch};")),
        ("s-ctor-var", format!("var s = 'x'; var S = s['constructor']; S.fromCharCode = {patch};")),
        ("s-alias-from-char-code", format!("var f = Function('return this')(); var S = f.String; S.fromCharCode = {patch};")),
        ("s-define-property", format!("Object.defineProperty(Function('return this')(), 'String', {{ value: {replaced} }});")),
        ("s-assign", format!("Object.assign(Function('return this')(), {{ String: {replaced} }});")),
        ("s-reflect-set", format!("Reflect.set(Function('return this')(), 'String', {replaced});")),
        ("s-define-getter", format!("Function('return this')().__defineGetter__('String', function () {{ return {replaced}; }});")),
        ("s-template-key", format!("Function('return this')()[`String`] = {replaced};")),
        ("s-concatenated-key", format!("Function('return this')()['Str' + 'ing'] = {replaced};")),
        ("s-require-vm", "require('vm').runInThisContext('String.fromCharCode = function () { return \"?\"; }');".to_string()),
        ("s-text-alias", "Function('var g = (function () { return this; })(); var S = g.String; S.fromCharCode = function () { return \"?\"; };')();".to_string()),
    ]
}

#[test]
fn a_string_key_written_through_a_value_the_record_does_not_follow_prints_like_node() {
    for (label, patch) in string_writes() {
        assert_prints_like_node(
            label,
            patched_after_rotator("String.fromCharCode(33) === '!'", &patch),
        );
    }
}

#[test]
fn a_lookup_through_a_cut_function_prototype_prints_like_node() {
    assert_prints_like_node(
        "chain-fn-null",
        patched_after_rotator(
            "typeof b.hasOwnProperty === 'function'",
            "Function.prototype.__proto__ = null;",
        ),
    );
}

const PROTOTYPE_CUTS: [(&str, &str); 11] = [
    ("cut-computed", "Function.prototype['__proto__'] = null;"),
    (
        "cut-alias",
        "var FP = Function.prototype; FP.__proto__ = null;",
    ),
    (
        "cut-literal-chain",
        "(function () {}).__proto__.__proto__ = null;",
    ),
    (
        "cut-get-prototype-of",
        "Object.getPrototypeOf(function () {}).__proto__ = null;",
    ),
    (
        "cut-of-function",
        "var FP = Object.getPrototypeOf(Function); FP.__proto__ = null;",
    ),
    (
        "cut-replaced",
        "Function.prototype.__proto__ = { hasOwnProperty: 1 };",
    ),
    (
        "cut-in-text",
        "Function('Function.prototype.__proto__ = null')();",
    ),
    (
        "cut-chain-in-text",
        "Function('(function () {}).__proto__.__proto__ = null')();",
    ),
    (
        "cut-alias-in-text",
        "Function('var FP = Function.prototype; FP.__proto__ = null')();",
    ),
    (
        "cut-dynamic-key",
        "var k = '__pro' + 'to__'; Function.prototype[k] = null;",
    ),
    (
        "cut-set-prototype-of",
        "Object.setPrototypeOf(Function.prototype, null);",
    ),
];

#[test]
fn a_lookup_through_a_prototype_cut_any_recorded_way_prints_like_node() {
    for (label, patch) in PROTOTYPE_CUTS {
        assert_prints_like_node(
            label,
            patched_after_rotator("typeof b.hasOwnProperty === 'function'", patch),
        );
    }
    assert_prints_like_node(
        "cut-array",
        patched_after_rotator(
            "typeof [].hasOwnProperty === 'function'",
            "Array.prototype.__proto__ = null;",
        ),
    );
}

#[test]
fn a_join_patched_through_an_array_prototype_alias_prints_like_node() {
    assert_prints_like_node(
        "x-ap-alias-join",
        patched_after_rotator(
            "[1, 2].join('') === '12'",
            "var AP = Array.prototype; AP.join = function () { return '?'; };",
        ),
    );
}

#[test]
fn a_literal_method_patched_any_recorded_way_is_not_folded_and_prints_like_node() {
    let join = "[1, 2].join('') === '12'";
    let char_code = "'-'.charCodeAt(0) === 45";
    for (label, test, patch) in [
        (
            "x-ap-alias2-join",
            join,
            "var AP = [].__proto__; AP.join = function () { return '?'; };",
        ),
        (
            "join-direct",
            join,
            "Array.prototype.join = function () { return '?'; };",
        ),
        (
            "join-derived",
            join,
            "Object.getPrototypeOf([]).join = function () { return '?'; };",
        ),
        (
            "join-dynamic",
            join,
            "var AP = Array.prototype; var k = 'jo' + 'in'; AP[k] = function () { return '?'; };",
        ),
        (
            "char-code-alias",
            char_code,
            "var SP = String.prototype; SP.charCodeAt = function () { return 1; };",
        ),
    ] {
        assert_prints_like_node(label, patched_after_rotator(test, patch));
    }
}

#[test]
fn a_literal_method_that_code_text_may_patch_is_not_folded_and_prints_like_node() {
    let join = "[1, 2].join('') === '12'";
    let char_code = "'-'.charCodeAt(0) === 45";
    for (label, test, patch) in [
        (
            "h2-text",
            join,
            "Function('Array.prototype.join = function () { return \"?\"; }')();",
        ),
        (
            "text-template",
            join,
            "Function(`Array.prototype.join = function () { return '?'; }`)();",
        ),
        (
            "text-alias-join",
            join,
            "Function('var AP = Array.prototype; AP.join = function () { return \"?\"; }')();",
        ),
        (
            "text-eval-join",
            join,
            "eval('Array.prototype.join = function () { return \"?\"; }');",
        ),
        (
            "text-char-code",
            char_code,
            "Function('String.prototype.charCodeAt = function () { return 1; }')();",
        ),
        ("text-writes-nothing", join, "Function('return 1')();"),
    ] {
        assert_prints_like_node(label, patched_after_rotator(test, patch));
    }
}
