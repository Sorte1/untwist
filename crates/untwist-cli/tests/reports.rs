mod common;

use common::{stdin_output, SPLIT_STRINGS_BEHIND_A_DECODER};

#[test]
fn the_json_report_times_every_pass_it_counts() {
    let out = stdin_output(
        SPLIT_STRINGS_BEHIND_A_DECODER,
        &["--unsafe", "--report", "json"],
    );
    assert!(out.status.success());
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    for stage in v["stages"].as_array().unwrap() {
        let counted: Vec<&str> = stage["per_pass"]
            .as_array()
            .unwrap()
            .iter()
            .map(|row| row[0].as_str().unwrap())
            .collect();
        let timed: Vec<&str> = stage["per_pass_ms"]
            .as_array()
            .unwrap()
            .iter()
            .map(|row| row[0].as_str().unwrap())
            .collect();
        assert_eq!(counted, timed, "{stage}");
        for row in stage["per_pass_ms"].as_array().unwrap() {
            assert!(row[1].as_f64().unwrap() >= 0.0, "{row}");
        }
    }
    let text = stdin_output(
        SPLIT_STRINGS_BEHIND_A_DECODER,
        &["--unsafe", "--report", "text"],
    );
    let report = String::from_utf8_lossy(&text.stderr).into_owned();
    let counted_line = report
        .lines()
        .find(|line| line.trim_start().starts_with("find-string-array"))
        .unwrap_or_else(|| panic!("no find-string-array line in {report}"));
    assert!(counted_line.trim_end().ends_with("ms"), "{counted_line}");
}

#[test]
fn the_json_report_times_every_iteration_in_three_phases() {
    let out = stdin_output(
        SPLIT_STRINGS_BEHIND_A_DECODER,
        &["--unsafe", "--report", "json"],
    );
    assert!(out.status.success());
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    let stages = v["stages"].as_array().unwrap();
    let unminify = stages
        .iter()
        .find(|stage| stage["stage"] == "unminify")
        .unwrap_or_else(|| panic!("no unminify stage in {v}"));
    let rows = unminify["iterations_ms"].as_array().unwrap();
    assert_eq!(rows.len() as u64, unminify["iterations"].as_u64().unwrap());
    for row in rows {
        let phases = row.as_array().unwrap();
        assert_eq!(phases.len(), 3, "{row}");
        assert!(phases[0].as_f64().unwrap() >= 0.0, "{row}");
    }
    let deobfuscate = stages
        .iter()
        .find(|stage| stage["stage"] == "deobfuscate")
        .unwrap_or_else(|| panic!("no deobfuscate stage in {v}"));
    let counted: Vec<&str> = deobfuscate["fixpoint_per_pass"]
        .as_array()
        .unwrap()
        .iter()
        .map(|row| row[0].as_str().unwrap())
        .collect();
    let timed: Vec<&str> = deobfuscate["fixpoint_per_pass_ms"]
        .as_array()
        .unwrap()
        .iter()
        .map(|row| row[0].as_str().unwrap())
        .collect();
    assert_eq!(counted, timed);
    let text = stdin_output(
        SPLIT_STRINGS_BEHIND_A_DECODER,
        &["--unsafe", "--report", "text"],
    );
    assert!(!String::from_utf8_lossy(&text.stderr).contains("iterations_ms"));
}

fn deobfuscate_rebuilds(source: &str) -> Vec<bool> {
    let out = stdin_output(source, &["--report", "json"]);
    assert!(out.status.success());
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    let deobfuscate = v["stages"]
        .as_array()
        .unwrap()
        .iter()
        .find(|stage| stage["stage"] == "deobfuscate")
        .unwrap_or_else(|| panic!("no deobfuscate stage in {v}"))
        .clone();
    deobfuscate["iterations_ms"]
        .as_array()
        .unwrap()
        .iter()
        .map(|row| !row[1].is_null())
        .collect()
}

#[test]
fn the_deobfuscate_stage_times_prepare_then_every_round_in_order() {
    assert_eq!(deobfuscate_rebuilds("x(1);"), [false, false, false]);
    assert_eq!(
        deobfuscate_rebuilds("x(\"a\" + \"b\");"),
        [false, false, true, false, false, false, false]
    );
}

#[test]
fn a_default_lane_alias_refusal_prints_the_unsafe_hint_on_stderr() {
    let high = include_str!("../../../fixtures/reference/deobfuscate/obfuscator.io-high.js");
    let hint = untwist::UNSAFE_LANE_DECODES;
    let out = stdin_output(high, &[]);
    assert!(out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr).into_owned();
    let line = stderr
        .lines()
        .find(|line| line.ends_with(hint))
        .unwrap_or_else(|| panic!("no hint in {stderr}"));
    assert!(line.starts_with("warning: "), "{line}");
    let unsafe_out = stdin_output(high, &["--unsafe"]);
    let unsafe_stderr = String::from_utf8_lossy(&unsafe_out.stderr).into_owned();
    assert!(!unsafe_stderr.contains(hint), "{unsafe_stderr}");
    let json = stdin_output(high, &["--report", "json"]);
    let report: serde_json::Value = serde_json::from_slice(&json.stdout).unwrap();
    let noted = report["notes"]
        .as_array()
        .unwrap()
        .iter()
        .any(|note| note.as_str().is_some_and(|text| text.ends_with(hint)));
    assert!(noted, "{}", report["notes"]);
}

#[test]
fn the_text_report_counts_app_and_library_modules_of_a_split_bundle() {
    let fixture = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../fixtures/libraries/core-js/rollup-cjs.terser.js"
    );
    let dir = tempfile::tempdir().unwrap();
    let out = common::bin()
        .args([
            "--split-hoisted",
            "--split-assume-modules",
            "--report",
            "text",
            "-o",
        ])
        .arg(dir.path().join("out"))
        .arg(fixture)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let report = String::from_utf8_lossy(&out.stderr).into_owned();
    let line = report
        .lines()
        .find(|line| line.starts_with("modules: "))
        .unwrap_or_else(|| panic!("no modules line in {report}"));
    let (counts, libraries) = line
        .strip_prefix("modules: ")
        .and_then(|rest| rest.split_once(" ("))
        .unwrap_or_else(|| panic!("{line}"));
    let (app, library) = counts.split_once(", ").unwrap();
    let app: usize = app.split(' ').next().unwrap().parse().unwrap();
    let library: usize = library.split(' ').next().unwrap().parse().unwrap();
    assert!(counts.ends_with("library modules"), "{line}");
    assert!(library > 0, "{line}");
    let core_js = libraries
        .strip_suffix(')')
        .unwrap()
        .split(", ")
        .find(|part| part.starts_with("core-js 3."))
        .unwrap_or_else(|| panic!("{line}"));
    let core_js_count: usize = core_js.rsplit(": ").next().unwrap().parse().unwrap();
    let json: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(dir.path().join("out/bundle.json")).unwrap())
            .unwrap();
    let paths: Vec<&str> = json["modules"]
        .as_array()
        .unwrap()
        .iter()
        .map(|module| module["path"].as_str().unwrap())
        .collect();
    let under_core_js = paths
        .iter()
        .filter(|path| path.starts_with("node_modules/core-js/"))
        .count();
    let under_node_modules = paths
        .iter()
        .filter(|path| path.starts_with("node_modules/"))
        .count();
    assert_eq!(core_js_count, under_core_js, "{line}");
    assert_eq!(library, under_node_modules, "{line}");
    assert_eq!(app + library, paths.len(), "{line}");
    assert!(dir.path().join("out/node_modules/core-js").is_dir());
}

#[test]
fn the_text_report_has_no_modules_line_without_library_modules() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("in.js");
    std::fs::write(&input, common::WEBPACK_4).unwrap();
    let out = common::bin()
        .args(["--report", "text", "-o"])
        .arg(dir.path().join("out"))
        .arg(&input)
        .output()
        .unwrap();
    assert!(out.status.success());
    let report = String::from_utf8_lossy(&out.stderr).into_owned();
    assert!(report.contains("bundle: "), "{report}");
    assert!(!report.contains("\nmodules: "), "{report}");
}
