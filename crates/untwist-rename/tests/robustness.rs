mod common;

use common::{assert_scope_safe, rename_compact, render};
use untwist_core::pass::Note;
use untwist_core::print::PrintStyle;
use untwist_rename::{RenameConfig, RenameMode};

const MODES: [RenameMode; 3] = [RenameMode::Mangle, RenameMode::Simple, RenameMode::Unique];

fn run(source: &str, mode: RenameMode, rename_all: bool) -> String {
    let output = rename_compact(
        source,
        RenameConfig {
            mode,
            rename_all,
            extended: true,
        },
    );
    assert_scope_safe(source, &output);
    output
}

#[test]
fn non_ascii_string_contents_survive_every_mode() {
    let source = r#"var _0xa = "日本語"; var _0xb = "café ☕"; var _0xc = "foo-bar-🗿-ä"; _0xa; _0xb; _0xc;"#;
    for mode in MODES {
        let output = run(source, mode, true);
        assert!(output.contains("`日本語`"), "{mode:?}: {output}");
        assert!(output.contains("`café ☕`"), "{mode:?}: {output}");
        assert!(output.contains("`foo-bar-🗿-ä`"), "{mode:?}: {output}");
    }
}

#[test]
fn lone_surrogates_survive_every_mode() {
    let source = "var _0xa = \"\\ud800\"; _0xa;";
    for mode in MODES {
        let output = run(source, mode, true);
        assert!(output.contains("ud800"), "{mode:?}: {output}");
    }
}

#[test]
fn non_ascii_identifiers_are_left_alone_unless_rename_all() {
    let source = "var café = 1; var 日本 = café; 日本;";
    assert_eq!(
        run(source, RenameMode::Simple, false),
        "var café=1;var 日本=café;日本;"
    );
    assert_eq!(
        run(source, RenameMode::Unique, false),
        "var café=1;var 日本=café;日本;"
    );
}

#[test]
fn non_ascii_identifiers_are_renamed_under_rename_all() {
    let source = "var café = 1; var 日本 = café; 日本;";
    assert_eq!(run(source, RenameMode::Simple, true), "var a=1;var b=a;b;");
    assert_eq!(
        run(source, RenameMode::Unique, true),
        "var num1=1;var num2=num1;num2;"
    );
}

#[test]
fn a_non_ascii_confusable_name_is_generated_not_written() {
    let source = "var IÏïïïìi = 1; IÏïïïìi;";
    assert_eq!(run(source, RenameMode::Simple, false), "var a=1;a;");
    assert_eq!(run(source, RenameMode::Unique, false), "var num1=1;num1;");
}

#[test]
fn mangle_reduces_a_non_ascii_string_initialiser_to_its_ascii_stem() {
    assert_eq!(
        run(r#"let x = "日本";"#, RenameMode::Mangle, false),
        r#"let vLS=`日本`;"#
    );
}

#[test]
fn a_with_statement_is_reported_once_and_still_renames() {
    let config = RenameConfig {
        mode: RenameMode::Simple,
        rename_all: false,
        extended: true,
    };
    let (output, notes) = render(
        "var _0xa = 1; with (o) { _0xa; }",
        config,
        PrintStyle::Compact,
        false,
    );
    assert!(output.contains("with("), "{output}");

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
        hazards[0].message.starts_with("with statement"),
        "{:?}",
        hazards[0]
    );
}

#[test]
fn the_shorthand_stage_does_not_repeat_the_hazard_note() {
    let config = RenameConfig {
        mode: RenameMode::Simple,
        rename_all: false,
        extended: true,
    };
    let (_, notes) = render(
        "var _0xa = 1; let { sin: _0xk } = Math; _0xk(eval('_0xa'));",
        config,
        PrintStyle::Compact,
        true,
    );
    let hazards: Vec<&Note> = notes
        .iter()
        .filter(|note| note.message.contains("may defeat scope analysis"))
        .collect();
    assert_eq!(
        hazards.len(),
        1,
        "the mode stage reports the hazard; the shorthand stage must not repeat it: {notes:?}"
    );
    assert_eq!(
        hazards[0].pass, "rename-simple",
        "the surviving note belongs to the mode stage: {:?}",
        hazards[0]
    );
}

fn wide_program(count: usize) -> String {
    let mut source = String::new();
    for index in 0..count {
        source.push_str(&format!("var _0x{index} = {index};\n"));
    }
    for index in 0..count {
        source.push_str(&format!("_0x{index};\n"));
    }
    source
}

fn deep_program(depth: usize) -> String {
    let mut source = String::new();
    for level in 0..depth {
        source.push_str(&format!("function _0xf{level}(_0xp{level}) {{\n"));
    }
    source.push_str("return 1;\n");
    for _ in 0..depth {
        source.push_str("}\n");
    }
    source
}

#[test]
fn wide_and_deep_programs_do_not_panic_and_stay_alpha_equivalent() {
    for mode in MODES {
        let wide = wide_program(2000);
        assert_scope_safe(&wide, &run(&wide, mode, true));
        let deep = deep_program(200);
        assert_scope_safe(&deep, &run(&deep, mode, true));
    }
}

#[test]
fn degenerate_programs_do_not_panic() {
    let sources = [
        "",
        ";;;",
        "console.log(window, globalThis);",
        "var _0xa; var _0xa; var _0xa = 1; _0xa;",
        "with (o) { _0xa; }",
        "function _0xa() { return eval('_0xa'); } _0xa();",
        "label: for (;;) { break label; }",
        "class _0xa { static { var _0xb = 1; _0xb; } }",
        "try {} catch {} finally {}",
        "export default class {}",
    ];
    for source in sources {
        for mode in MODES {
            for rename_all in [false, true] {
                run(source, mode, rename_all);
            }
        }
    }
}

#[test]
fn the_default_config_renames_nothing_but_is_extended_and_renames_all() {
    let config = RenameConfig::default();
    assert_eq!(config.mode, RenameMode::None);
    assert!(
        config.extended,
        "the library default must not be the conservative mode's naming"
    );
    assert!(
        config.rename_all,
        "the library default must match Options::default()"
    );
    let mangle = RenameConfig {
        mode: RenameMode::Mangle,
        ..RenameConfig::default()
    };
    let once = rename_compact("var f = 1; function g() { return f; } g();", mangle);
    assert_eq!(once, "var vLN1=1;function f(){return vLN1}f();");
    assert_eq!(rename_compact(&once, mangle), once);
}
