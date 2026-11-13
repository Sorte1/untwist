mod common;

use common::bin;
use common::{stdin_output, A_DECODER_THAT_SPENDS_OVER_50M_STEPS, SPLIT_STRINGS_BEHIND_A_DECODER};

#[test]
fn the_jobs_flag_is_accepted_and_the_outputs_are_in_input_order() {
    let dir = tempfile::tempdir().unwrap();
    let a = dir.path().join("a.js");
    let b = dir.path().join("b.js");
    std::fs::write(&a, "var a=!0;").unwrap();
    std::fs::write(&b, "var b=!1;").unwrap();
    let out = bin()
        .args(["--jobs", "2", "--no-unpack"])
        .arg(&a)
        .arg(&b)
        .output()
        .unwrap();
    assert!(out.status.success());
    assert_eq!(
        String::from_utf8_lossy(&out.stdout).trim_end(),
        "var a = true;\n\nvar b = false;"
    );
    let zero = bin().args(["-j", "0"]).arg(&a).output().unwrap();
    assert!(!zero.status.success());
}

#[test]
fn the_fuel_flag_reaches_the_interpreter_and_refuses_zero() {
    let starved = stdin_output(
        SPLIT_STRINGS_BEHIND_A_DECODER,
        &["--fuel", "1", "--report", "json"],
    );
    assert!(starved.status.success());
    let v: serde_json::Value = serde_json::from_slice(&starved.stdout).unwrap();
    let notes = v["notes"].as_array().unwrap();
    assert!(
        notes
            .iter()
            .any(|note| note.as_str().unwrap().contains("step budget exhausted")),
        "{v}"
    );
    let zero = stdin_output(SPLIT_STRINGS_BEHIND_A_DECODER, &["--fuel", "0"]);
    assert!(!zero.status.success());
}

#[test]
fn an_unlimited_fuel_decodes_what_the_default_budget_gives_up_on() {
    let heavy = A_DECODER_THAT_SPENDS_OVER_50M_STEPS;
    let default = stdin_output(heavy, &[]);
    assert!(default.status.success());
    assert!(String::from_utf8_lossy(&default.stdout).contains("b(0) + b(1)"));
    let unlimited = stdin_output(heavy, &["--fuel", "unlimited"]);
    assert!(unlimited.status.success());
    assert!(
        String::from_utf8_lossy(&unlimited.stdout).contains("console.log(\"Hello\");"),
        "{}",
        String::from_utf8_lossy(&unlimited.stdout)
    );
    let words = stdin_output(heavy, &["--fuel", "lots"]);
    assert!(!words.status.success());
}

const NESTING_BETWEEN_THE_TWO_STACKS: usize = 5_200;

#[test]
fn a_nesting_depth_that_parses_under_two_jobs_also_parses_alone_with_the_unpack_reparse_on() {
    let dir = tempfile::tempdir().unwrap();
    let deep = dir.path().join("deep.js");
    let other = dir.path().join("other.js");
    let depth = NESTING_BETWEEN_THE_TWO_STACKS;
    let source = format!("var x = {}1{};", "[".repeat(depth), "]".repeat(depth));
    std::fs::write(&deep, source).unwrap();
    std::fs::write(&other, "var z = 1;").unwrap();
    let nested = format!("var x = {}1", "[".repeat(depth));
    for jobs in ["2", "1"] {
        let out = bin()
            .args(["-j", jobs])
            .arg(&deep)
            .arg(&other)
            .output()
            .unwrap();
        assert!(out.status.success(), "-j {jobs}: {:?}", out.status);
    }
    let alone = bin().arg(&deep).output().unwrap();
    assert!(alone.status.success(), "alone: {:?}", alone.status);
    assert!(String::from_utf8_lossy(&alone.stdout).contains(&nested));
}

#[test]
fn list_passes_prints_every_pass_with_its_stage_and_exits_zero() {
    let out = bin().arg("--list-passes").output().unwrap();
    assert!(out.status.success());
    let text = String::from_utf8_lossy(&out.stdout);
    let rows: Vec<&str> = text.lines().collect();
    assert_eq!(rows.len(), untwist::PASSES.len() + 1, "{text}");
    for pass in untwist::PASSES {
        assert!(
            rows.iter().any(|row| {
                let mut cells = row.split_whitespace();
                cells.next() == Some(pass.name) && cells.next() == Some(pass.stage)
            }),
            "{} is missing from:\n{text}",
            pass.name
        );
    }
}

#[test]
fn disable_turns_a_named_pass_off_and_refuses_an_unknown_name() {
    let on = stdin_output("var a = !0;", &["--no-unpack"]);
    assert_eq!(
        String::from_utf8_lossy(&on.stdout).trim_end(),
        "var a = true;"
    );
    let off = stdin_output(
        "var a = !0;",
        &["--no-unpack", "--disable", "unminify-booleans"],
    );
    assert!(off.status.success());
    assert_eq!(
        String::from_utf8_lossy(&off.stdout).trim_end(),
        "var a = !0;"
    );
    let listed = stdin_output(
        "var a = !0, b = void 0;",
        &[
            "--no-unpack",
            "--disable",
            "unminify-booleans,void-to-undefined",
        ],
    );
    let listed_text = String::from_utf8_lossy(&listed.stdout);
    assert!(
        listed_text.contains("!0") && listed_text.contains("void 0"),
        "{listed_text}"
    );
    let unknown = stdin_output("var a = !0;", &["--disable", "no-such-pass"]);
    assert_eq!(unknown.status.code(), Some(2));
    let message = String::from_utf8_lossy(&unknown.stderr);
    assert!(
        message.contains("no-such-pass") && message.contains("--list-passes"),
        "{message}"
    );
}

#[test]
fn split_assume_modules_is_refused_without_split_hoisted() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("in.js");
    std::fs::write(&input, "var a = 1;").unwrap();
    let alone = bin()
        .arg("--split-assume-modules")
        .arg(&input)
        .output()
        .unwrap();
    assert_eq!(alone.status.code(), Some(2));
    let stderr = String::from_utf8_lossy(&alone.stderr);
    assert!(stderr.contains("--split-hoisted"), "{stderr}");
    let paired = bin()
        .args(["--split-hoisted", "--split-assume-modules"])
        .arg(&input)
        .output()
        .unwrap();
    assert!(paired.status.success());
}
