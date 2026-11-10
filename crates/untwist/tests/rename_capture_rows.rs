use untwist::{Options, Pipeline, RenameMode};
use untwist_testing::node_rows::{mismatches, node_or_skip, Mode};

const MODES: [RenameMode; 3] = [RenameMode::Mangle, RenameMode::Simple, RenameMode::Unique];

const ANNEX_B_VAR: &str =
    "function g() { var f = 1; { function f() { return 2; } } return typeof f; }
console.log(g());";

const ANNEX_B_PARAM: &str = "function g(f) { { function f() { return 2; } } return typeof f; }
console.log(g(1));";

const MACHINERY_PARAM: &str = "function f(__DECODE_0__, s) {
  return [{ foo: __DECODE_0__ }, { __DECODE_0__: s }];
}
console.log(JSON.stringify(f(1, 2)));";

const CATCH_VAR_AT_TOP: &str = "try { throw 1; } catch (x) { var a = 5; console.log(x); }
console.log(a, typeof this.a);";

const CATCH_VAR_KEPT_NAME: &str =
    "function H(o) { try { throw o; } catch (_0x3) { var e1 = 5; console.log(_0x3); } return e1; }
console.log(H(1));";

const CATCH_VAR_TAKES_KEPT_NAME: &str = "function f() { try { throw 1; } catch (num1) { var _0x1 = 5; console.log(num1); } return _0x1; }
console.log(f());";

const CATCH_VAR_SHORTHAND: &str =
    "function f(o) { try { throw o; } catch (e) { var k = 5; var r = { c: e }; } return [r, k]; }
console.log(JSON.stringify(f(1)));";

const BLOCK_VAR_BESIDE_LET: &str =
    "var aa = 1; { let x = 2; var a = 7; console.log(x); } console.log(this.a, this.aa);";

const BLOCK_VAR_KEPT_NAME: &str =
    "function I(o) { { let _0x4 = [o]; var arr1 = 5; console.log(_0x4[0]); } return arr1; }
console.log(I(1));";

const FUNCTION_NAMES: &str = "function foo() {} var bar = function () {}; class Baz {} var qux = class {}; var arrow = () => 1;
var later; later = function () {};
console.log(foo.name, bar.name, Baz.name, qux.name, arrow.name, later.name, new Baz().constructor.name);";

const NAME_READ_BY_KEY: &str = "var _0x1 = function () {}; function _0x2() {}
console.log(_0x1['name'], _0x2['name']);";

const NAME_DESTRUCTURED: &str = "var _0x1 = function () {};
var { name } = _0x1;
console.log(name);";

const CODE_READS_LEXICALS: &str = "var _0x1 = 5;
function _0x2() { return 6; }
let _0x3 = 7;
var e = eval, F = Function;
console.log((0, eval)('typeof _0x1'), e('_0x2()'), new F('return typeof _0x1')(), (0, eval)('typeof _0x3'));";

const HOST_PROPERTY_KEYS: &str = "var _0x1 = '/next';
var _0x2 = { first: 'bob' };
function _0x3() { return 1; }
var _0x4 = 10, _0x5 = 0;
console.log(JSON.stringify({ location: _0x1, name: _0x2, top: _0x3, status: _0x4, length: _0x5 }));";

fn options(rename: RenameMode, rename_all: bool) -> Options {
    Options {
        rename,
        rename_all,
        unpack: false,
        jsx: false,
        ..Options::default()
    }
}

fn renamed(source: &str, rename: RenameMode, rename_all: bool) -> String {
    Pipeline::new(options(rename, rename_all))
        .run(source)
        .code()
        .to_string()
}

fn rows(programs: &[(&str, &str)], rename_all: bool) -> Vec<String> {
    let Some(node) = node_or_skip() else {
        return Vec::new();
    };
    let programs: Vec<(String, String)> = programs
        .iter()
        .map(|(label, source)| (label.to_string(), source.to_string()))
        .collect();
    let mangle = |source: &str| renamed(source, RenameMode::Mangle, rename_all);
    let simple = |source: &str| renamed(source, RenameMode::Simple, rename_all);
    let unique = |source: &str| renamed(source, RenameMode::Unique, rename_all);
    let modes: [Mode<'_>; 3] = [
        ("mangle", &mangle),
        ("simple", &simple),
        ("unique", &unique),
    ];
    mismatches(&node, &programs, &modes)
}

#[test]
fn a_var_and_a_sloppy_block_function_of_one_name_stay_one_binding() {
    let wrong = rows(
        &[
            ("annex b var", ANNEX_B_VAR),
            ("annex b param", ANNEX_B_PARAM),
        ],
        true,
    );
    assert!(wrong.is_empty(), "{wrong:#?}");
}

#[test]
fn a_binding_with_a_machinery_name_keeps_it_and_no_other_binding_takes_it() {
    let wrong = rows(&[("machinery param", MACHINERY_PARAM)], true);
    assert!(wrong.is_empty(), "{wrong:#?}");
    for mode in MODES {
        let code = renamed(MACHINERY_PARAM, mode, true);
        let exact = code
            .split(|ch: char| !(ch.is_ascii_alphanumeric() || ch == '_' || ch == '$'))
            .filter(|word| *word == "__DECODE_0__")
            .count();
        assert_eq!(exact, 3, "{mode:?}: {code}");
    }
}

#[test]
fn a_var_written_in_a_block_or_catch_body_is_seen_where_it_hoists() {
    let wrong = rows(
        &[
            ("catch var at top", CATCH_VAR_AT_TOP),
            ("catch var shorthand", CATCH_VAR_SHORTHAND),
            ("block var beside let", BLOCK_VAR_BESIDE_LET),
        ],
        true,
    );
    assert!(wrong.is_empty(), "{wrong:#?}");
}

#[test]
fn a_var_in_a_catch_or_block_body_never_shares_a_kept_name() {
    let wrong = rows(
        &[
            ("catch var kept name", CATCH_VAR_KEPT_NAME),
            ("catch var takes kept name", CATCH_VAR_TAKES_KEPT_NAME),
            ("block var kept name", BLOCK_VAR_KEPT_NAME),
        ],
        false,
    );
    assert!(wrong.is_empty(), "{wrong:#?}");
}

#[test]
fn function_and_class_names_survive_when_the_program_reads_name() {
    let wrong = rows(
        &[
            ("function names", FUNCTION_NAMES),
            ("name read by key", NAME_READ_BY_KEY),
            ("name destructured", NAME_DESTRUCTURED),
        ],
        true,
    );
    assert!(wrong.is_empty(), "{wrong:#?}");
}

#[test]
fn functions_are_renamed_when_nothing_reads_name() {
    let source = "function _0x2() { return 1; } console.log(_0x2());";
    for mode in MODES {
        let code = renamed(source, mode, true);
        assert!(!code.contains("_0x2"), "{mode:?}: {code}");
    }
}

#[test]
fn top_level_lexicals_keep_their_names_when_code_reaches_script_globals() {
    let wrong = rows(&[("code reads lexicals", CODE_READS_LEXICALS)], true);
    assert!(wrong.is_empty(), "{wrong:#?}");
    for mode in MODES {
        let code = renamed(CODE_READS_LEXICALS, mode, true);
        assert!(code.contains("let _0x3 = 7"), "{mode:?}: {code}");
    }
}

#[test]
fn no_top_level_script_binding_is_renamed_onto_a_host_global_property() {
    for mode in MODES {
        let code = renamed(HOST_PROPERTY_KEYS, mode, true);
        for host in ["location", "name", "top", "status", "length"] {
            assert!(
                !code.contains(&format!("var {host} "))
                    && !code.contains(&format!("var {host},"))
                    && !code.contains(&format!("function {host}(")),
                "{mode:?}: {host}: {code}"
            );
        }
    }
}

#[test]
fn a_module_top_level_binding_may_still_take_a_shorthand_key() {
    let source =
        "export const out = 1; const _0x4 = 10; console.log(JSON.stringify({ status: _0x4 }));";
    let code = renamed(source, RenameMode::Simple, true);
    assert!(code.contains("{ status }"), "{code}");
}

#[test]
fn top_level_lexicals_keep_their_names_when_a_script_element_runs_text() {
    let source = "let _0x3 = 7;
var s = document.createElement('script');
s.text = 'console.log(_0x3)';
document.head.appendChild(s);";
    for mode in MODES {
        let code = renamed(source, mode, true);
        assert!(code.contains("let _0x3 = 7"), "{mode:?}: {code}");
    }
}
