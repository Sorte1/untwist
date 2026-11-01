mod common;

use common::{conservative_run, default_run, pass_options};
use oxc_allocator::Allocator;
use untwist_core::parse::parse_program;
use untwist_core::pass::{Pass, PassOptions, Safety};
use untwist_core::print::{print, PrintStyle};
use untwist_core::semantic::build_scoping;
use untwist_deobfuscate::fold_chains::boxed;
use untwist_deobfuscate::{run_cleanup, run_deobfuscate};

const CHAIN: &str = "console.log(\"!dlroW olleH\".split(\"\").reverse().join(\"\"));";

#[test]
fn issue_131_the_reversed_string_chain_becomes_a_literal_by_default() {
    let alloc = Allocator::default();
    let code = default_run(&alloc, CHAIN, boxed);
    assert!(code.contains(r#"console.log("Hello World!")"#), "{code}");
}

#[test]
fn the_pass_is_inert_under_conservative() {
    let alloc = Allocator::default();
    assert_eq!(conservative_run(&alloc, CHAIN, boxed).trim(), CHAIN.trim());
}

#[test]
fn a_chain_rooted_in_an_identifier_is_left_alone() {
    let source = "console.log(s.split(\"\").reverse().join(\"\"));";
    let alloc = Allocator::default();
    assert_eq!(default_run(&alloc, source, boxed).trim(), source.trim());
}

#[test]
fn a_chain_rooted_in_a_call_is_left_alone() {
    let source = "console.log(f().split(\"\").reverse().join(\"\"));";
    let alloc = Allocator::default();
    assert_eq!(default_run(&alloc, source, boxed).trim(), source.trim());
}

#[test]
fn the_pass_is_named_and_declares_its_mode_and_safety() {
    let pass: Box<dyn Pass<'_>> = boxed();
    assert_eq!(pass.name(), "fold-literal-chains");
    assert!(pass.requires_extended());
    assert_eq!(pass.safety(), Safety::Safe);
}

fn left_alone(source: &str) {
    let alloc = Allocator::default();
    assert_eq!(default_run(&alloc, source, boxed).trim(), source.trim());
}

#[test]
fn a_chain_rooted_in_a_literal_char_code_call_folds() {
    let source = "console.log(String.fromCharCode(65, 66).split(\"\").reverse().join(\"\"));";
    let alloc = Allocator::default();
    let code = default_run(&alloc, source, boxed);
    assert!(code.contains(r#"console.log("BA")"#), "{code}");
}

#[test]
fn a_chain_rooted_in_another_pure_builtin_call_is_left_alone() {
    left_alone("console.log(String.fromCodePoint(65, 66).split(\"\").reverse().join(\"\"));");
}

#[test]
fn a_chain_rooted_in_a_name_resolved_global_is_left_alone() {
    left_alone("console.log(NaN.toFixed(2));");
}

#[test]
fn an_array_root_holding_a_name_resolved_global_is_left_alone() {
    left_alone("console.log([NaN].reverse().join(\"\"));");
}

#[test]
fn a_chain_that_folds_to_undefined_is_left_alone() {
    left_alone("console.log([].reverse().pop());");
}

#[test]
fn a_chain_that_folds_to_nan_is_left_alone() {
    left_alone("console.log(\"\".charCodeAt(0));");
}

#[test]
fn a_chain_whose_last_step_yields_nan_keeps_that_step() {
    let alloc = Allocator::default();
    let code = default_run(
        &alloc,
        "console.log(\"\".split(\"\").reverse().join(\"\").charCodeAt(0));",
        boxed,
    );
    assert_eq!(code.trim(), "console.log(\"\".charCodeAt(0));");
}

fn folded(source: &str) -> String {
    let alloc = Allocator::default();
    default_run(&alloc, source, boxed)
}

const PATCHES_OF_JOIN: [&str; 9] = [
    "Array.prototype.join = function () { return '?'; };",
    "Array.prototype['join'] = function () { return '?'; };",
    "var AP = Array.prototype; AP.join = function () { return '?'; };",
    "var AP = [].__proto__; AP.join = function () { return '?'; };",
    "Object.getPrototypeOf([]).join = function () { return '?'; };",
    "[].constructor.prototype.join = function () { return '?'; };",
    "Object.defineProperty(Array.prototype, 'join', { value: function () { return '?'; } });",
    "var AP = Array.prototype; var k = 'jo' + 'in'; AP[k] = function () { return '?'; };",
    "function def(o, k, v) { o[k] = v; } def(Array.prototype, 'join', function () { return 1; });",
];

#[test]
fn a_join_that_a_recorded_write_may_patch_is_not_folded() {
    for patch in PATCHES_OF_JOIN {
        let source = format!("console.log([1, 2].join(''));\n{patch}\n");
        let code = folded(&source);
        assert!(code.contains("[1, 2].join(\"\")"), "{patch}\n{code}");
    }
}

#[test]
fn a_write_to_another_key_or_another_prototype_leaves_the_fold() {
    for patch in [
        "Array.prototype.foo = 1;",
        "var AP = Array.prototype; AP.last = function () { return 1; };",
        "Number.prototype.join = function () { return '?'; };",
        "function def(o, k, v) { o[k] = v; } def(Array.prototype, 'last', function () { return 1; });",
    ] {
        let source = format!("console.log([1, 2].join(''));\n{patch}\n");
        let code = folded(&source);
        assert!(code.contains("console.log(\"12\")"), "{patch}\n{code}");
    }
}

#[test]
fn a_string_method_that_a_recorded_write_may_patch_is_not_folded() {
    let patch = "var SP = String.prototype; SP.charCodeAt = function () { return 1; };";
    let source = format!("console.log('-'.charCodeAt(0));\n{patch}\n");
    let code = folded(&source);
    assert!(code.contains("charCodeAt(0)"), "{patch}\n{code}");
}

#[test]
fn a_write_to_the_string_prototype_proto_leaves_an_own_method_folded() {
    let patch = "String.prototype.__proto__ = null;";
    let source = format!("console.log('-'.charCodeAt(0));\n{patch}\n");
    let code = folded(&source);
    assert!(code.contains("console.log(45)"), "{patch}\n{code}");
}

#[test]
fn a_write_to_object_prototype_under_a_key_string_prototype_owns_leaves_the_fold() {
    for patch in [
        "Object.prototype.charCodeAt = function () { return 1; };",
        "var k = 'prototype'; var OP = Object[k]; var s = 'to' + 'String'; delete OP[s];",
    ] {
        let source = format!("console.log('-'.charCodeAt(0));\n{patch}\n");
        let code = folded(&source);
        assert!(code.contains("console.log(45)"), "{patch}\n{code}");
    }
}

fn decoder_reading(test: &str, patch: &str, tail: &str) -> String {
    format!(
        "function a() {{ var h = ['Hello World!', 'console', 'deobfuscate', 'log']; a = function () {{ return h; }}; return a(); }}
function b(c, d) {{
var e = a();
return (b = function (f, g) {{
f = f - 0;
var i = e[f];
return i + ({test} ? '!' : '?');
}}), b(c, d);
}}
(function (c, d) {{
var e = c();
var tries = 0;
while (!![]) {{
try {{
if (b(0) === 'log!') break;
else e['push'](e['shift']());
}} catch (h) {{ e['push'](e['shift']()); }}
if (tries++ > 1000) break;
}}
}})(a, 0);
{patch}
console.log(b(0));
{tail}
"
    )
}

fn deobfuscated_and_cleaned(source: &str) -> String {
    let alloc = Allocator::default();
    let mut parsed = parse_program(&alloc, source);
    let opts = PassOptions {
        extended: true,
        ..pass_options()
    };
    let sem = build_scoping(&parsed.program);
    let (sem, _) = run_deobfuscate(&alloc, &mut parsed.program, sem, &opts);
    run_cleanup(&alloc, &mut parsed.program, sem, &opts);
    print(&parsed.program, PrintStyle::Pretty)
}

const JOIN_TEST: &str = "[1, 2].join('') === '12'";
const CHAR_CODE_TEST: &str = "'-'.charCodeAt(0) === 45";

const TEXT_PATCHES_OF_JOIN: [&str; 4] = [
    "Function('Array.prototype.join = function () { return \"?\"; }')();",
    "Function(`Array.prototype.join = function () { return '?'; }`)();",
    "Function('var AP = Array.prototype; AP.join = function () { return \"?\"; }')();",
    "eval('Array.prototype.join = function () { return \"?\"; }');",
];

#[test]
fn a_join_that_code_text_beside_a_judged_decode_may_patch_is_not_folded() {
    for patch in TEXT_PATCHES_OF_JOIN {
        let source = decoder_reading(JOIN_TEST, patch, "console.log([1, 2].join(''));");
        let code = deobfuscated_and_cleaned(&source);
        assert!(!code.contains("console.log(\"12\")"), "{patch}\n{code}");
    }
}

#[test]
fn a_string_method_that_code_text_beside_a_judged_decode_may_patch_is_not_folded() {
    let patch = "Function('String.prototype.charCodeAt = function () { return 1; }')();";
    let source = decoder_reading(CHAR_CODE_TEST, patch, "console.log('-'.charCodeAt(0));");
    let code = deobfuscated_and_cleaned(&source);
    assert!(!code.contains("console.log(45)"), "{code}");
}

#[test]
fn code_text_the_judgement_reads_as_writing_nothing_keeps_the_fold() {
    for (test, tail, folded) in [
        (
            JOIN_TEST,
            "console.log([1, 2].join(''));",
            "console.log(\"12\")",
        ),
        (
            CHAR_CODE_TEST,
            "console.log('-'.charCodeAt(0));",
            "console.log(45)",
        ),
    ] {
        let source = decoder_reading(test, "Function('return 1')();", tail);
        let code = deobfuscated_and_cleaned(&source);
        assert!(code.contains(folded), "{code}");
    }
}

#[test]
fn a_literal_chain_beside_a_direct_eval_is_left_alone() {
    left_alone("eval(text);\nconsole.log([1, 2].join(\"\"));");
    left_alone("function f() {\n  eval(text);\n}\nconsole.log(\"-\".charCodeAt(0));");
}

const DEFINERS_CALLED_ANOTHER_WAY: [&str; 6] = [
    "Object.defineProperty.call(null, Array.prototype, 'join', { value: function () { return '?'; } });",
    "Object.defineProperty.apply(null, [Array.prototype, 'join', { value: 1 }]);",
    "Reflect.apply(Object.defineProperty, null, [Array.prototype, 'join', { value: 1 }]);",
    "Object.assign.bind(null, Array.prototype)({ join: 1 });",
    "Array.prototype.__defineGetter__('join', function () { return function () { return 1; }; });",
    "Object.prototype.__defineSetter__.call(Array.prototype, 'join', function () {});",
];

#[test]
fn a_join_that_a_definer_called_any_other_way_may_patch_is_not_folded() {
    for patch in DEFINERS_CALLED_ANOTHER_WAY {
        let source = format!("console.log([1, 2].join(''));\n{patch}\n");
        let code = folded(&source);
        assert!(code.contains("[1, 2].join(\"\")"), "{patch}\n{code}");
    }
}

#[test]
fn a_definer_called_another_way_on_a_fresh_object_or_another_prototype_keeps_the_fold() {
    for patch in [
        "Object.defineProperty.call(null, {}, 'join', { value: 1 });",
        "Reflect.apply(Object.defineProperty, null, [Number.prototype, 'join', { value: 1 }]);",
        "Array.prototype.__defineGetter__('last', function () { return 1; });",
        "Object.assign({}, src);",
    ] {
        let source = format!("console.log([1, 2].join(''));\n{patch}\n");
        let code = folded(&source);
        assert!(code.contains("console.log(\"12\")"), "{patch}\n{code}");
    }
}

#[test]
fn a_join_written_through_a_global_object_or_its_alias_is_not_folded() {
    for patch in [
        "globalThis.Array.prototype.join = function () { return '?'; };",
        "window['Array'].prototype.join = 1;",
        "var g = self; g.Array.prototype.join = 1;",
        "function f() { g.Array.prototype.join = 1; } var g = globalThis;",
        "var AP = global.Array.prototype; AP.join = 1;",
    ] {
        let source = format!("console.log([1, 2].join(''));\n{patch}\n");
        let code = folded(&source);
        assert!(code.contains("[1, 2].join(\"\")"), "{patch}\n{code}");
    }
}

#[test]
fn a_global_object_member_that_is_not_a_prototype_or_a_local_alias_keeps_the_fold() {
    let patch = "globalThis.Array.foo = 1;";
    let source = format!("console.log([1, 2].join(''));\n{patch}\n");
    let code = folded(&source);
    assert!(code.contains("console.log(\"12\")"), "{patch}\n{code}");
}

#[test]
fn a_prototype_reached_through_a_global_object_or_a_fresh_root_bails() {
    for patch in [
        "var g = {}; g.Array.prototype.join = 1;",
        "globalThis.Number.prototype.join = 1;",
    ] {
        let source = format!("console.log([1, 2].join(''));\n{patch}\n");
        let code = folded(&source);
        assert!(code.contains("[1, 2].join(\"\")"), "{patch}\n{code}");
    }
}

#[test]
fn a_join_written_through_a_constructor_alias_or_inside_with_is_not_folded() {
    for patch in [
        "var A = Array; A.prototype.join = 1;",
        "var A = Array; A = function () {}; A.prototype.join = 1;",
        "var A = [].constructor; delete A.prototype.join;",
        "function f() { A.prototype['join'] = 1; } var A = Array;",
        "with (Array.prototype) { join = 1; }",
    ] {
        let source = format!("console.log([1, 2].join(''));\n{patch}\n");
        let code = folded(&source);
        assert!(code.contains("[1, 2].join(\"\")"), "{patch}\n{code}");
    }
}

#[test]
fn a_constructor_alias_of_another_root_or_another_key_keeps_the_fold() {
    let patch = "var A = Array; A.prototype.last = 1;";
    let source = format!("console.log([1, 2].join(''));\n{patch}\n");
    let code = folded(&source);
    assert!(code.contains("console.log(\"12\")"), "{patch}\n{code}");
}

#[test]
fn a_constructor_alias_of_another_root_under_the_same_key_bails() {
    let patch = "var N = Number; N.prototype.join = 1;";
    let source = format!("console.log([1, 2].join(''));\n{patch}\n");
    let code = folded(&source);
    assert!(code.contains("[1, 2].join(\"\")"), "{patch}\n{code}");
}

#[test]
fn an_array_creating_chain_beside_a_written_species_is_not_folded_and_a_plain_join_is() {
    for patch in [
        "Object.defineProperty(Array, Symbol.species, { value: X });",
        "Array[Symbol.species] = X;",
    ] {
        let source = format!("console.log([1].concat([2]).join(''), [1, 2].join(''));\n{patch}\n");
        let code = folded(&source);
        assert!(
            code.contains("[1].concat([2]).join(\"\")"),
            "{patch}\n{code}"
        );
        assert!(code.contains("\"12\")"), "{patch}\n{code}");
    }
}

#[test]
fn an_unknown_key_written_on_the_array_constructor_bails_both_chains() {
    let patch = "Array[k] = X;";
    let source = format!("console.log([1].concat([2]).join(''), [1, 2].join(''));\n{patch}\n");
    let code = folded(&source);
    assert!(
        code.contains("[1].concat([2]).join(\"\")"),
        "{patch}\n{code}"
    );
    assert!(code.contains("[1, 2].join(\"\")"), "{patch}\n{code}");
}
