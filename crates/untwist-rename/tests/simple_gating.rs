mod common;

use common::{assert_scope_safe, rename_compact, render};
use untwist_core::pass::Note;
use untwist_core::print::PrintStyle;
use untwist_rename::{RenameConfig, RenameMode};

fn simple(source: &str, rename_all: bool) -> String {
    let config = RenameConfig {
        mode: RenameMode::Simple,
        rename_all,
        extended: true,
    };
    let output = rename_compact(source, config);
    assert_scope_safe(source, &output);
    output
}

#[test]
fn human_written_names_survive_by_default() {
    assert_eq!(
        simple(
            "function handleClick(event) { var _0xa = event; return _0xa; }",
            false
        ),
        "function handleClick(event){var a=event;return a}"
    );
}

#[test]
fn rename_all_takes_the_human_written_names_too() {
    assert_eq!(
        simple(
            "function handleClick(event) { var _0xa = event; return _0xa; }",
            true
        ),
        "function a(a){var b=a;return b}"
    );
}

#[test]
fn confusable_and_hex_names_are_renamed_without_rename_all() {
    assert_eq!(simple("var IÏïïïìi = 1; IÏïïïìi;", false), "var a=1;a;");
    assert_eq!(simple("var _0x4c3e = 1; _0x4c3e;", false), "var a=1;a;");
    assert_eq!(simple("var Wq = 1; Wq;", false), "var a=1;a;");
}

#[test]
fn exported_bindings_are_pinned_in_both_settings() {
    for rename_all in [false, true] {
        assert_eq!(
            simple("export const _0xa = 1;", rename_all),
            "export const _0xa=1;"
        );
        let source = "const _0xb = 1; export { _0xb };";
        assert_eq!(simple(source, rename_all), "const _0xb=1;export{_0xb};");
    }
}

#[test]
fn an_aliased_export_is_still_renamed() {
    assert_eq!(
        simple("const _0xc = 1; export { _0xc as publicName };", false),
        "const a=1;export{a as publicName};"
    );
}

fn shadowing_stress(depth: usize) -> String {
    let mut source = String::from("var _0xroot = 0;\n");
    for level in 0..depth {
        source.push_str(&format!(
            "function _0xf{level}(_0xp{level}) {{ var _0xv{level} = _0xroot + _0xp{level};\n"
        ));
    }
    source.push_str("var _0xleaf = 1; _0xleaf;\n");
    for _ in 0..depth {
        source.push_str("return _0xroot; }\n");
    }
    source
}

#[test]
fn deeply_nested_shadowing_stays_alpha_equivalent_and_idempotent() {
    for rename_all in [false, true] {
        let source = shadowing_stress(12);
        let once = simple(&source, rename_all);
        let twice = simple(&once, rename_all);
        assert_eq!(
            once, twice,
            "Simple must be idempotent (rename_all={rename_all})"
        );
    }
}

#[test]
fn a_direct_eval_program_still_renames_and_reports_the_hazard_once() {
    let config = RenameConfig {
        mode: RenameMode::Simple,
        rename_all: false,
        extended: true,
    };
    let (output, notes) = render(
        "var _0xa = 1; function _0xb() { return eval('_0xa'); }",
        config,
        PrintStyle::Compact,
        false,
    );
    assert!(output.contains("eval("), "{output}");

    let hazards: Vec<&Note> = notes
        .iter()
        .filter(|note| note.message.contains("may defeat scope analysis"))
        .collect();
    assert_eq!(
        hazards.len(),
        1,
        "exactly one hazard note per stage: {notes:?}"
    );
    assert!(
        hazards[0].message.starts_with("direct eval"),
        "{:?}",
        hazards[0]
    );
}

#[test]
fn the_wrapper_names_are_exempt_from_rename_all_in_simple_too() {
    let source = "(function (require, module, exports, define) { var sum = require(\"./sum\"); module.exports = sum; define; })();";
    let once = simple(source, true);
    assert_eq!(
        once,
        "(function(require,module,exports,define){var a=require(\"./sum\");module.exports=a;define})();"
    );
    assert_eq!(simple(&once, true), once);
}
