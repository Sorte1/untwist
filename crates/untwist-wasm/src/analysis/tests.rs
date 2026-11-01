use super::{analyze, Analysis, Outcome};

fn at(text: &str, needle: &str, nth: usize) -> u32 {
    let byte = text
        .match_indices(needle)
        .nth(nth)
        .map(|(byte, _)| byte)
        .unwrap_or_else(|| panic!("no {nth}th {needle}"));
    u32::try_from(text[..byte].encode_utf16().count()).unwrap()
}

fn starts(analysis: &Analysis, pos: u32) -> Vec<u32> {
    analysis
        .references(pos)
        .chunks(2)
        .map(|pair| pair[0])
        .collect()
}

fn applied(text: &str, outcome: Outcome) -> String {
    let Outcome::Edits(edits) = outcome else {
        panic!("refused: {outcome:?}");
    };
    let mut units: Vec<u16> = text.encode_utf16().collect();
    for (from, to, new) in edits.iter().rev() {
        units.splice(*from as usize..*to as usize, new.encode_utf16());
    }
    String::from_utf16(&units).unwrap()
}

fn refusal(outcome: Outcome) -> String {
    match outcome {
        Outcome::Refused(reason) => reason,
        Outcome::Edits(edits) => panic!("renamed: {edits:?}"),
    }
}

const SHADOWS: &str = "var q = 1;
function f(q) {
  try { g(q); } catch (q) { g(q); }
  class C { m() { let q = 2; return q; } }
  return q;
}
g(q);
";

#[test]
fn definition_and_references_follow_shadowing_in_functions_catch_and_classes() {
    let analysis = analyze(SHADOWS);
    let outer = at(SHADOWS, "q", 0);
    let param = at(SHADOWS, "q", 1);
    let caught = at(SHADOWS, "q", 3);
    let local = at(SHADOWS, "q", 5);
    assert_eq!(
        starts(&analysis, at(SHADOWS, "g(q)", 2) + 2),
        vec![outer, at(SHADOWS, "q", 8)]
    );
    assert_eq!(
        starts(&analysis, param),
        vec![param, at(SHADOWS, "q", 2), at(SHADOWS, "q", 7)]
    );
    assert_eq!(
        starts(&analysis, at(SHADOWS, "q", 4)),
        vec![caught, at(SHADOWS, "q", 4)]
    );
    assert_eq!(
        starts(&analysis, at(SHADOWS, "q", 6)),
        vec![local, at(SHADOWS, "q", 6)]
    );
    let definition = analysis.definition(at(SHADOWS, "q", 7)).unwrap();
    assert_eq!(
        definition,
        format!("{{\"span\":[{param},{}],\"kind\":\"var\"}}", param + 1)
    );
    assert_eq!(analysis.definition(at(SHADOWS, "g", 0)), None);
}

#[test]
fn offsets_are_utf16_units_after_an_astral_plane_string() {
    let text = "const s = \"😀😀\"; let b = s; b;";
    let analysis = analyze(text);
    let b = at(text, "b", 0);
    assert_eq!(b, 22);
    assert_eq!(analysis.references(b), vec![22, 23, 29, 30]);
    let renamed = applied(text, analysis.rename_outcome(b, "c"));
    assert_eq!(renamed, "const s = \"😀😀\"; let c = s; c;");
}

#[test]
fn a_rename_changes_exactly_the_references_and_expands_shorthands() {
    let text = "import { x } from \"./m\";\nconst a = { x };\nconst { y } = a;\nlet z; ({ z } = a);\nx(y, z);\n";
    let analysis = analyze(text);
    let x = applied(text, analysis.rename_outcome(at(text, "x", 0), "x2"));
    assert_eq!(
        x,
        text.replace("{ x } from", "{ x as x2 } from")
            .replace("{ x }", "{ x: x2 }")
            .replace("x(y", "x2(y")
    );
    let y = applied(text, analysis.rename_outcome(at(text, "y", 0), "y2"));
    assert_eq!(y, text.replace("{ y }", "{ y: y2 }").replace("(y,", "(y2,"));
    let z = applied(text, analysis.rename_outcome(at(text, "z", 0), "z2"));
    assert_eq!(
        z,
        text.replace("let z", "let z2")
            .replace("{ z }", "{ z: z2 }")
            .replace(" z)", " z2)")
    );
}

#[test]
fn a_rename_that_an_inner_binding_would_capture_is_refused() {
    let text = "function f(a) { function g() { let b = 1; return a + b; } return g; }";
    let analysis = analyze(text);
    let reason = refusal(analysis.rename_outcome(at(text, "a", 0), "b"));
    assert!(reason.contains("already declared"), "{reason}");
}

#[test]
fn a_rename_that_would_capture_an_outer_or_free_name_is_refused() {
    let text = "let b = 1; function f() { let a = 2; return a + b + c; }";
    let analysis = analyze(text);
    let outer = refusal(analysis.rename_outcome(at(text, "a", 0), "b"));
    assert!(outer.contains("outer or global `b`"), "{outer}");
    let free = refusal(analysis.rename_outcome(at(text, "a", 0), "c"));
    assert!(free.contains("outer or global `c`"), "{free}");
    let fine = applied(text, analysis.rename_outcome(at(text, "a", 0), "d"));
    assert_eq!(
        fine,
        "let b = 1; function f() { let d = 2; return d + b + c; }"
    );
}

#[test]
fn an_exported_symbol_and_a_reserved_word_are_refused() {
    let text = "export const a = 1; const b = 2; export { b as c }; let d = a + b;";
    let analysis = analyze(text);
    assert!(refusal(analysis.rename_outcome(at(text, "a", 0), "e")).contains("exported"));
    assert!(refusal(analysis.rename_outcome(at(text, "b", 0), "e")).contains("exported"));
    let reserved = refusal(analysis.rename_outcome(at(text, "d", 0), "yield"));
    assert!(reserved.contains("reserved word"), "{reserved}");
    assert!(refusal(analysis.rename_outcome(at(text, "d", 0), "1x")).contains("not an identifier"));
}

#[test]
fn an_unresolved_name_and_a_direct_eval_scope_are_refused() {
    let text = "function f() { let a = 1; eval(\"a\"); } g;";
    let analysis = analyze(text);
    assert!(refusal(analysis.rename_outcome(at(text, "a", 0), "b")).contains("eval"));
    assert!(refusal(analysis.rename_outcome(at(text, "g;", 0), "h")).contains("not declared"));
}

#[test]
fn a_var_in_a_block_cannot_take_the_name_of_a_let_in_that_block() {
    let text = "function f() { { let b; var a = 1; } return a; }";
    let analysis = analyze(text);
    assert!(refusal(analysis.rename_outcome(at(text, "a =", 0), "b")).contains("already declared"));
}

#[test]
fn a_function_name_can_take_a_name_its_own_body_does_not_use() {
    let text = "function f(b) { return b; } f(1);";
    let analysis = analyze(text);
    let renamed = applied(text, analysis.rename_outcome(at(text, "f(", 0), "b"));
    assert_eq!(renamed, "function b(b) { return b; } b(1);");
}

#[test]
fn completions_list_the_innermost_bindings_first_then_globals_then_keywords() {
    let text = "let outer = 1; function f(param) { let inner = 2;  } console;";
    let analysis = analyze(text);
    let list: serde_json::Value =
        serde_json::from_str(&analysis.completions(at(text, "  }", 0) + 1)).unwrap();
    let labels: Vec<&str> = list
        .as_array()
        .unwrap()
        .iter()
        .map(|item| item["label"].as_str().unwrap())
        .collect();
    let position = |label: &str| labels.iter().position(|l| *l == label).unwrap();
    assert!(position("inner") < position("outer"));
    assert!(position("param") < position("f"));
    assert!(position("outer") < position("console"));
    assert!(position("console") < position("return"));
    assert_eq!(list[position("console")]["kind"], "global");
    assert_eq!(list[position("inner")]["kind"], "let");
}

#[test]
fn the_word_being_typed_is_not_offered_as_a_global() {
    let text = "let sum = 1; s";
    let list = analyze(text).completions(14);
    assert!(!list.contains("\"label\":\"s\""), "{list}");
    assert!(list.contains("\"label\":\"sum\""), "{list}");
}

#[test]
fn a_typo_shows_one_problem_with_its_span() {
    let text = "let a = 1;\nlet b = a +;\n";
    let analysis = analyze(text);
    let problems: serde_json::Value = serde_json::from_str(&analysis.problems_json()).unwrap();
    assert_eq!(problems.as_array().unwrap().len(), 1, "{problems}");
    assert_eq!(problems[0]["severity"], "error");
    assert_eq!(problems[0]["from"], at(text, ";\n", 1));
}

#[test]
fn a_redeclaration_is_a_semantic_problem() {
    let analysis = analyze("let a = 1; let a = 2;");
    let problems: serde_json::Value = serde_json::from_str(&analysis.problems_json()).unwrap();
    assert_eq!(problems.as_array().unwrap().len(), 1, "{problems}");
}

#[test]
fn a_required_module_links_its_binding_and_its_members_to_the_export() {
    let user = "const m = require(\"./802.js\");\n(0, m.c)(\"x\");\n";
    let analysis = analyze(user);
    let on_member = analysis.definition(at(user, "c)", 0)).unwrap();
    assert_eq!(
        on_member,
        "{\"span\":[6,7],\"kind\":\"member\",\"module\":\"./802.js\",\"name\":\"c\"}"
    );
    let on_binding = analysis.definition(at(user, "m.", 0)).unwrap();
    assert_eq!(
        on_binding,
        "{\"span\":[6,7],\"kind\":\"const\",\"module\":\"./802.js\",\"name\":\"*\"}"
    );
    let module =
        "\"use strict\";\nfunction say(line) {\n  console.log(line);\n}\nexport { say as c };";
    let target = analyze(module);
    let say = at(module, "say", 0);
    assert_eq!(target.export_span("c"), vec![say, say + 3]);
    assert!(target.export_span("say").is_empty());
}

#[test]
fn commonjs_exports_and_default_exports_are_found_by_name() {
    let cjs = analyze("exports.join = function () {}; module.exports.tag = 1;");
    assert_eq!(cjs.export_span("join"), vec![8, 12]);
    assert_eq!(cjs.export_span("tag"), vec![46, 49]);
    let esm = analyze("export default function main() {}");
    assert_eq!(esm.export_span("default"), vec![24, 28]);
}

#[test]
fn a_text_above_the_cap_has_no_features() {
    let text = format!(
        "let a = 1;{}",
        " ".repeat(super::ANALYSIS_CAP_BYTES as usize)
    );
    let analysis = analyze(&text);
    assert!(!analysis.available());
    assert!(analysis.references(4).is_empty());
    assert_eq!(analysis.problems_json(), "[]");
}

fn slice(text: &str, span: &[u32]) -> String {
    let units: Vec<u16> = text.encode_utf16().collect();
    String::from_utf16(&units[span[0] as usize..span[1] as usize]).unwrap()
}

#[test]
fn a_member_of_a_required_webpack_module_resolves_to_that_modules_export() {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../fixtures/unpack-bundlers/webpack/webpack5-prod.js"
    );
    let source = std::fs::read_to_string(path).unwrap();
    let report = crate::report::run_report(&source, "", false);
    let codes: Vec<String> = (0..report.module_count())
        .filter_map(|index| report.module_code(index))
        .collect();
    let user = codes
        .iter()
        .find(|code| code.contains("require(\"./802.js\")"))
        .unwrap();
    let member = user.find("_0__.c)").map(|byte| &user[..byte + 5]).unwrap();
    let pos = u32::try_from(member.encode_utf16().count()).unwrap();
    let definition: serde_json::Value =
        serde_json::from_str(&analyze(user).definition(pos).unwrap()).unwrap();
    assert_eq!(definition["module"], "./802.js");
    assert_eq!(definition["name"], "c");
    let target = codes
        .iter()
        .find(|code| code.contains("export { say as c }"))
        .unwrap();
    assert_eq!(slice(target, &analyze(target).export_span("c")), "say");
}

#[test]
fn engine_info_names_the_analysis_cap() {
    let info: serde_json::Value = serde_json::from_str(&crate::report::engine_info()).unwrap();
    assert_eq!(info["analysisCapBytes"], super::ANALYSIS_CAP_BYTES);
}

#[test]
fn a_jsx_component_cannot_take_a_lowercase_name() {
    let text = "const Foo = () => null; const x = <Foo></Foo>;";
    let analysis = analyze(text);
    assert!(refusal(analysis.rename_outcome(at(text, "Foo", 0), "foo")).contains("JSX"));
    let renamed = applied(text, analysis.rename_outcome(at(text, "Foo", 0), "Bar"));
    assert_eq!(renamed, "const Bar = () => null; const x = <Bar></Bar>;");
}

#[test]
fn a_sloppy_block_function_and_a_var_of_its_name_are_not_renamed() {
    let text = "var f = 1; { function f() {} } f();";
    let analysis = analyze(text);
    let inner = at(text, "f()", 0);
    assert!(refusal(analysis.rename_outcome(inner, "g")).contains("Annex B"));
    assert!(refusal(analysis.rename_outcome(at(text, "f", 0), "h")).contains("Annex B"));
    let strict = "\"use strict\"; { function f() {} f(); }";
    let renamed = applied(
        strict,
        analyze(strict).rename_outcome(at(strict, "f()", 0), "g"),
    );
    assert_eq!(renamed, "\"use strict\"; { function g() {} g(); }");
}

#[test]
fn a_text_with_syntax_errors_and_a_global_object_name_are_refused() {
    let broken = "let a = 1; let b = ;";
    assert!(refusal(analyze(broken).rename_outcome(4, "c")).contains("syntax errors"));
    let text = "function f() { let a = 1; return a; }";
    let reason = refusal(analyze(text).rename_outcome(at(text, "a", 0), "undefined"));
    assert!(reason.contains("reserved word"), "{reason}");
}

#[test]
fn a_default_parameter_sees_the_outer_binding_not_the_body_var() {
    let text = "var x = 1; function f(a = x) { var x = 2; return a; }";
    let analysis = analyze(text);
    let body = at(text, "x = 2", 0);
    let outcome = analysis.rename_outcome(body, "z");
    if let Outcome::Edits(_) = &outcome {
        let renamed = applied(text, outcome);
        assert_eq!(
            renamed,
            "var x = 1; function f(a = x) { var z = 2; return a; }"
        );
    }
}

#[test]
fn shorthands_with_escapes_and_self_named_imports_rename_cleanly() {
    let text = "let a = 1; const o = ({ \\u0061 });";
    let renamed = applied(text, analyze(text).rename_outcome(at(text, "a", 0), "b"));
    assert_eq!(renamed, "let b = 1; const o = ({ a: b });");
    let import = "import { a as a } from \"m\"; a;";
    let renamed = applied(
        import,
        analyze(import).rename_outcome(at(import, "a;", 0), "b"),
    );
    assert_eq!(renamed, "import { a as b } from \"m\"; b;");
    let class = "const C = class D { m() { return D; } };";
    let renamed = applied(
        class,
        analyze(class).rename_outcome(at(class, "D;", 0), "E"),
    );
    assert_eq!(renamed, "const C = class E { m() { return E; } };");
}
