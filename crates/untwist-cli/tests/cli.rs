mod common;

use std::io::Write;
use std::process::Stdio;

use common::{bin, stdin_output, SPLIT_STRINGS_BEHIND_A_DECODER};

#[test]
fn reads_stdin_and_prints_code() {
    let out = stdin_output("var a=!0;", &[]);
    assert!(out.status.success());
    assert_eq!(
        String::from_utf8_lossy(&out.stdout).trim_end(),
        "var a = true;"
    );
}

#[test]
fn a_clean_run_reports_no_internal_error() {
    let out = stdin_output("var a=!0;", &[]);
    assert!(out.status.success());
    assert!(!String::from_utf8_lossy(&out.stderr).contains("internal error"));
}

#[test]
fn no_unminify_flag_disables_stage() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("in.js");
    std::fs::write(&input, "var a=!0;").unwrap();
    let out = bin().arg("--no-unminify").arg(&input).output().unwrap();
    assert_eq!(
        String::from_utf8_lossy(&out.stdout).trim_end(),
        "var a = !0;"
    );
}

#[test]
fn output_dir_refuses_to_overwrite_without_force() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("in.js");
    std::fs::write(&input, "x()").unwrap();
    let outdir = dir.path().join("out");
    std::fs::create_dir(&outdir).unwrap();
    std::fs::write(outdir.join("deobfuscated.js"), "stale").unwrap();
    let out = bin().arg("-o").arg(&outdir).arg(&input).output().unwrap();
    assert_eq!(out.status.code(), Some(2));
    let out = bin()
        .arg("-o")
        .arg(&outdir)
        .arg("-f")
        .arg(&input)
        .output()
        .unwrap();
    assert!(out.status.success());
    assert_eq!(
        std::fs::read_to_string(outdir.join("deobfuscated.js"))
            .unwrap()
            .trim_end(),
        "x();"
    );
}

#[test]
fn output_dir_allows_empty_pre_existing_directory_without_force() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("in.js");
    std::fs::write(&input, "x()").unwrap();
    let outdir = dir.path().join("out");
    std::fs::create_dir(&outdir).unwrap();
    let out = bin().arg("-o").arg(&outdir).arg(&input).output().unwrap();
    assert!(out.status.success());
    assert_eq!(
        std::fs::read_to_string(outdir.join("deobfuscated.js"))
            .unwrap()
            .trim_end(),
        "x();"
    );
}

#[test]
fn force_keeps_unrelated_files_in_output_dir() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("in.js");
    std::fs::write(&input, "x()").unwrap();
    let outdir = dir.path().join("out");
    std::fs::create_dir(&outdir).unwrap();
    std::fs::write(outdir.join("deobfuscated.js"), "stale").unwrap();
    std::fs::write(outdir.join("notes.txt"), "keep me").unwrap();
    let out = bin()
        .arg("-o")
        .arg(&outdir)
        .arg("-f")
        .arg(&input)
        .output()
        .unwrap();
    assert!(out.status.success());
    assert_eq!(
        std::fs::read_to_string(outdir.join("notes.txt")).unwrap(),
        "keep me"
    );
    assert_eq!(
        std::fs::read_to_string(outdir.join("deobfuscated.js"))
            .unwrap()
            .trim_end(),
        "x();"
    );
}

#[test]
fn force_refuses_to_clean_current_directory() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("in.js");
    std::fs::write(&input, "x()").unwrap();
    let out = bin()
        .current_dir(dir.path())
        .arg("-o")
        .arg(".")
        .arg("-f")
        .arg(&input)
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&out.stderr).contains("error:"));
}

#[test]
fn json_report_contains_stages() {
    let mut child = bin()
        .arg("--report")
        .arg("json")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    child.stdin.take().unwrap().write_all(b"var a=!0;").unwrap();
    let out = child.wait_with_output().unwrap();
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(v["code"], "var a = true;");
    assert!(v["stages"]
        .as_array()
        .unwrap()
        .iter()
        .any(|s| s["stage"] == "unminify"));
}

#[test]
fn json_report_shows_convergence_and_drops_the_old_iteration_cap_field() {
    let mut child = bin()
        .arg("--report")
        .arg("json")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(b"var a=!0,b=!1;x(),y();console[\"log\"](a,b)")
        .unwrap();
    let out = child.wait_with_output().unwrap();
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    let stages = v["stages"].as_array().unwrap();
    assert!(!stages.is_empty());
    for stage in stages {
        assert_eq!(stage["stop"], "converged");
        assert!(stage.get("hit_iteration_cap").is_none());
    }
}

#[test]
fn the_json_report_carries_the_cleanup_groups_counts() {
    let out = stdin_output(
        SPLIT_STRINGS_BEHIND_A_DECODER,
        &["--unsafe", "--report", "json"],
    );
    assert!(out.status.success());
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    let stages = v["stages"].as_array().unwrap();
    for stage in stages {
        assert!(stage.get("fixpoint_per_pass").is_some(), "{stage}");
    }
    let deobfuscate = stages
        .iter()
        .find(|stage| stage["stage"] == "deobfuscate")
        .unwrap();
    assert_eq!(
        deobfuscate["fixpoint_per_pass"],
        serde_json::json!([
            ["merge-strings", 1],
            ["dead-code", 0],
            ["control-flow-object", 0],
            ["control-flow-switch", 0]
        ]),
        "{deobfuscate}"
    );
}

#[test]
fn the_json_and_text_reports_number_the_settle_rounds() {
    let source = "var o = { foo: 1 };\nconsole.log(o[atob(\"Zm9v\")]);";
    let out = stdin_output(source, &["--unsafe", "--report", "json"]);
    assert!(out.status.success());
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    let rounds: Vec<u64> = v["stages"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|stage| stage["stage"] == "unminify")
        .map(|stage| stage["round"].as_u64().unwrap())
        .collect();
    assert_eq!(rounds, vec![1, 2], "{v}");

    let text = stdin_output(source, &["--unsafe", "--report", "text"]);
    let report = String::from_utf8_lossy(&text.stderr).to_string();
    assert!(report.contains("unminify (round 2)"), "{report}");
    assert!(!report.contains("unminify (round 1)"), "{report}");
}

#[test]
fn the_text_report_names_the_cleanup_groups_passes() {
    let out = stdin_output(
        SPLIT_STRINGS_BEHIND_A_DECODER,
        &["--unsafe", "--report", "text"],
    );
    assert!(out.status.success());
    let report = String::from_utf8_lossy(&out.stderr).into_owned();
    assert!(report.contains("merge-strings"), "{report}");
}

#[test]
fn malformed_input_reports_parse_diagnostics_and_exits_nonzero() {
    let mut child = bin()
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(b"var a = ;;; {{{ function(")
        .unwrap();
    let out = child.wait_with_output().unwrap();
    assert_eq!(out.status.code(), Some(1));
    assert!(!String::from_utf8_lossy(&out.stderr).trim().is_empty());
}

#[test]
fn output_dir_is_written_even_with_parse_errors() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("in.js");
    std::fs::write(&input, "var a = ;;; {{{ function(").unwrap();
    let outdir = dir.path().join("out");
    let out = bin().arg("-o").arg(&outdir).arg(&input).output().unwrap();
    assert_eq!(out.status.code(), Some(1));
    assert!(outdir.join("deobfuscated.js").exists());
    assert!(!String::from_utf8_lossy(&out.stderr).trim().is_empty());
}

#[test]
fn output_dir_with_json_report_includes_output_path_not_code() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("in.js");
    std::fs::write(&input, "var a=!0;").unwrap();
    let outdir = dir.path().join("out");
    let out = bin()
        .arg("-o")
        .arg(&outdir)
        .arg("--report")
        .arg("json")
        .arg(&input)
        .output()
        .unwrap();
    assert!(out.status.success());
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert!(v.get("code").is_none());
    let output_path = v["output_path"].as_str().unwrap();
    assert!(output_path.ends_with("deobfuscated.js"));
    assert!(std::fs::read_to_string(outdir.join("deobfuscated.js"))
        .unwrap()
        .trim_end()
        .contains("true"));
}

#[test]
fn iteration_cap_warning_is_printed_to_stderr() {
    let mut child = bin()
        .arg("--max-iterations")
        .arg("1")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(b"var a=!0,b=!1;x(),y();console[\"log\"](a,b)")
        .unwrap();
    let out = child.wait_with_output().unwrap();
    assert!(out.status.success());
    assert!(String::from_utf8_lossy(&out.stderr).contains("iteration cap"));
}

#[test]
fn explicit_report_wins_over_verbose() {
    let mut child = bin()
        .arg("-v")
        .arg("--report")
        .arg("json")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    child.stdin.take().unwrap().write_all(b"var a=!0;").unwrap();
    let out = child.wait_with_output().unwrap();
    assert!(out.status.success());
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(v["code"], "var a = true;");
}

#[test]
fn several_input_files_are_all_deobfuscated() {
    let dir = tempfile::tempdir().unwrap();
    let first = dir.path().join("a.js");
    let second = dir.path().join("b.js");
    std::fs::write(&first, "var a=!0;").unwrap();
    std::fs::write(&second, "var b=!1;").unwrap();
    let out = bin().arg(&first).arg(&second).output().unwrap();
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("var a = true;"), "{stdout}");
    assert!(stdout.contains("var b = false;"), "{stdout}");
}

const WEBPACK_BUNDLE: &str = "!(function (e) {\n  var a = {};\n  function n(t) {\n    if (a[t]) return a[t].exports;\n    var r = (a[t] = { exports: {} });\n    return e[t].call(r.exports, r, r.exports, n), r.exports;\n  }\n  return n((n.s = 0));\n})([\n  function (e, t, i) {\n    const lib = i(1);\n    console.log(lib);\n  },\n  function (e, t, i) {\n    e.exports = 1;\n  },\n]);\n";

#[test]
fn output_dir_gets_bundle_json_and_one_file_per_module() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("in.js");
    std::fs::write(&input, WEBPACK_BUNDLE).unwrap();
    let outdir = dir.path().join("out");
    let out = bin().arg("-o").arg(&outdir).arg(&input).output().unwrap();
    assert!(out.status.success());
    assert!(outdir.join("deobfuscated.js").is_file());
    assert!(outdir.join("index.js").is_file());
    assert!(outdir.join("lib.js").is_file());
    let json: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(outdir.join("bundle.json")).unwrap())
            .unwrap();
    assert_eq!(json["type"], "webpack");
    assert_eq!(json["entryId"], "0");
    assert!(std::fs::read_to_string(outdir.join("index.js"))
        .unwrap()
        .contains("require(\"./lib.js\")"));
}

#[test]
fn force_replaces_stale_module_files_but_keeps_unrelated_ones() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("in.js");
    std::fs::write(&input, WEBPACK_BUNDLE).unwrap();
    let outdir = dir.path().join("out");
    std::fs::create_dir(&outdir).unwrap();
    std::fs::write(
        outdir.join("bundle.json"),
        r#"{"type":"webpack","entryId":"0","modules":[{"id":"9","path":"stale.js"}]}"#,
    )
    .unwrap();
    std::fs::write(outdir.join("stale.js"), "stale").unwrap();
    std::fs::write(outdir.join("notes.txt"), "keep me").unwrap();
    let out = bin()
        .arg("-o")
        .arg(&outdir)
        .arg("-f")
        .arg(&input)
        .output()
        .unwrap();
    assert!(out.status.success());
    assert!(!outdir.join("stale.js").exists());
    assert_eq!(
        std::fs::read_to_string(outdir.join("notes.txt")).unwrap(),
        "keep me"
    );
    assert!(outdir.join("index.js").is_file());
}

#[test]
fn json_report_includes_the_bundle_and_the_written_files() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("in.js");
    std::fs::write(&input, WEBPACK_BUNDLE).unwrap();
    let outdir = dir.path().join("out");
    let out = bin()
        .arg("-o")
        .arg(&outdir)
        .arg("--report")
        .arg("json")
        .arg(&input)
        .output()
        .unwrap();
    assert!(out.status.success());
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(v["bundle"]["type"], "webpack");
    assert_eq!(v["bundle"]["modules"][0]["path"], "index.js");
    let files: Vec<&str> = v["files_written"]
        .as_array()
        .unwrap()
        .iter()
        .map(|value| value.as_str().unwrap())
        .collect();
    assert_eq!(
        files,
        vec!["deobfuscated.js", "bundle.json", "index.js", "lib.js"]
    );
}

#[test]
fn several_inputs_get_one_deobfuscated_file_each() {
    let dir = tempfile::tempdir().unwrap();
    let first = dir.path().join("a.js");
    let second = dir.path().join("b.js");
    std::fs::write(&first, "var a=!0;").unwrap();
    std::fs::write(&second, "var b=!1;").unwrap();
    let outdir = dir.path().join("out");
    let out = bin()
        .arg("-o")
        .arg(&outdir)
        .arg(&first)
        .arg(&second)
        .output()
        .unwrap();
    assert!(out.status.success());
    assert!(outdir.join("deobfuscated.a.js").is_file());
    assert!(outdir.join("deobfuscated.b.js").is_file());
}

const BROWSERIFY_WITH_SUBDIR: &str = r#"(function e(t, n, r) {
  function s(o, u) {
    if (!n[o]) {
      if (!t[o]) {
        var a = typeof require == 'function' && require;
        if (!u && a) return a(o, !0);
        if (i) return i(o, !0);
        var f = new Error("Cannot find module '" + o + "'");
        throw ((f.code = 'MODULE_NOT_FOUND'), f);
      }
      var l = (n[o] = { exports: {} });
      t[o][0].call(
        l.exports,
        function (e) {
          var n = t[o][1][e];
          return s(n ? n : e);
        },
        l,
        l.exports,
        e,
        t,
        n,
        r
      );
    }
    return n[o].exports;
  }
  var i = typeof require == 'function' && require;
  for (var o = 0; o < r.length; o++) s(r[o]);
  return s;
})(
  {
    1: [
      function (require, module, exports) {
        module.exports = 42;
      },
      {},
    ],
    2: [
      function (require, module, exports) {
        var helper = require('./sub/helper');
        console.log(helper);
      },
      { './sub/helper': 1 },
    ],
  },
  {},
  [2]
);
"#;

#[cfg(unix)]
#[test]
fn a_module_path_through_a_symlink_is_refused() {
    let dir = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    let input = dir.path().join("in.js");
    std::fs::write(&input, BROWSERIFY_WITH_SUBDIR).unwrap();
    let outdir = dir.path().join("out");
    std::fs::create_dir(&outdir).unwrap();
    std::os::unix::fs::symlink(outside.path(), outdir.join("sub")).unwrap();
    let out = bin().arg("-o").arg(&outdir).arg(&input).output().unwrap();
    assert!(out.status.success());
    assert!(!outside.path().join("helper.js").exists());
    assert!(String::from_utf8_lossy(&out.stderr)
        .contains("warning: refusing to write unsafe module path"));
    assert!(outdir.join("index.js").is_file());
}

#[cfg(unix)]
#[test]
fn a_dangling_symlink_at_the_target_path_is_refused() {
    let dir = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    let input = dir.path().join("in.js");
    std::fs::write(&input, "x()").unwrap();
    let outdir = dir.path().join("out");
    std::fs::create_dir(&outdir).unwrap();
    let target = outside.path().join("new.js");
    std::os::unix::fs::symlink(&target, outdir.join("deobfuscated.js")).unwrap();
    let out = bin().arg("-o").arg(&outdir).arg(&input).output().unwrap();
    assert!(out.status.success());
    assert!(!target.exists());
    assert!(String::from_utf8_lossy(&out.stderr)
        .contains("warning: refusing to write unsafe module path"));
}

#[cfg(unix)]
#[test]
fn a_hard_linked_target_file_is_not_written_through() {
    let dir = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    let input = dir.path().join("in.js");
    std::fs::write(&input, WEBPACK_BUNDLE).unwrap();
    let outdir = dir.path().join("out");
    std::fs::create_dir(&outdir).unwrap();
    let victim = outside.path().join("victim.js");
    std::fs::write(&victim, "keep me").unwrap();
    std::fs::hard_link(&victim, outdir.join("lib.js")).unwrap();
    let out = bin()
        .arg("-o")
        .arg(&outdir)
        .arg("-f")
        .arg(&input)
        .output()
        .unwrap();
    assert!(out.status.success());
    assert_eq!(std::fs::read_to_string(&victim).unwrap(), "keep me");
    assert_eq!(
        std::fs::read_to_string(outdir.join("lib.js"))
            .unwrap()
            .trim_end(),
        "module.exports = 1;"
    );
    use std::os::unix::fs::PermissionsExt;
    let mode = std::fs::metadata(outdir.join("lib.js"))
        .unwrap()
        .permissions()
        .mode();
    assert_eq!(mode & 0o777, 0o644);
}

#[cfg(unix)]
#[test]
fn force_does_not_remove_a_victim_file_through_a_symlinked_subdirectory() {
    let dir = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    let input = dir.path().join("in.js");
    std::fs::write(&input, "x()").unwrap();
    let outdir = dir.path().join("out");
    std::fs::create_dir(&outdir).unwrap();
    let victim = outside.path().join("victim.js");
    std::fs::write(&victim, "keep me").unwrap();
    std::os::unix::fs::symlink(outside.path(), outdir.join("sub")).unwrap();
    std::fs::write(
        outdir.join("bundle.json"),
        r#"{"type":"webpack","entryId":"0","modules":[{"id":"9","path":"sub/victim.js"}]}"#,
    )
    .unwrap();
    let out = bin()
        .arg("-o")
        .arg(&outdir)
        .arg("-f")
        .arg(&input)
        .output()
        .unwrap();
    assert!(out.status.success());
    assert!(victim.exists());
    assert_eq!(std::fs::read_to_string(&victim).unwrap(), "keep me");
    assert!(String::from_utf8_lossy(&out.stderr).contains("warning:"));
}

fn run_cli(flags: &[&str], source: &str) -> String {
    let mut child = bin()
        .args(flags)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(source.as_bytes())
        .unwrap();
    let out = child.wait_with_output().unwrap();
    assert!(out.status.success(), "{:?}", out.status);
    String::from_utf8_lossy(&out.stdout).into_owned()
}

#[test]
fn no_deobfuscate_keeps_the_string_array() {
    let source = "function a() { var h = ['log']; a = function () { return h; }; return a(); }\nfunction b(c, d) { var e = a(); b = function (f, g) { f = f - 0x0; return e[f]; }; return b(c, d); }\nconsole.log(b(0x0));";
    let with_flag = run_cli(&["--no-deobfuscate"], source);
    let without_flag = run_cli(&[], source);
    assert!(with_flag.contains("function a()"), "{with_flag}");
    assert!(!without_flag.contains("function a()"), "{without_flag}");
    assert!(
        without_flag.contains("console.log(\"log\")"),
        "{without_flag}"
    );
}

#[test]
fn rename_is_off_unless_asked_for() {
    let out = stdin_output("var _0xa = 1;\n_0xa;", &[]);
    assert!(String::from_utf8_lossy(&out.stdout).contains("_0xa"));
}

#[test]
fn rename_simple_renames_generated_names() {
    let out = stdin_output("var _0xa = 1;\n_0xa;", &["--rename", "simple", "--unsafe"]);
    assert_eq!(
        String::from_utf8_lossy(&out.stdout).trim_end(),
        "var a = 1;\na;"
    );
}

#[test]
fn rename_unique_gives_readable_names() {
    let out = stdin_output("function _0xa() {}", &["--rename", "unique", "--unsafe"]);
    assert_eq!(
        String::from_utf8_lossy(&out.stdout).trim_end(),
        "function fn1() {}"
    );
}

#[test]
fn dash_m_is_an_alias_for_rename_mangle() {
    let short = stdin_output("var _0xa = 1;\n_0xa;", &["-m", "--unsafe"]);
    let long = stdin_output("var _0xa = 1;\n_0xa;", &["--rename", "mangle", "--unsafe"]);
    assert_eq!(
        String::from_utf8_lossy(&short.stdout).trim_end(),
        "var vLN1 = 1;\nvLN1;"
    );
    assert_eq!(short.stdout, long.stdout);
}

#[test]
fn human_written_names_are_renamed_by_default_and_kept_on_request() {
    let source = "function handleClick(event) {\n  return event;\n}";
    let without = stdin_output(source, &["--rename", "simple", "--unsafe"]);
    let with_keep = stdin_output(source, &["--rename", "simple", "--keep-human-names"]);
    assert_eq!(
        String::from_utf8_lossy(&without.stdout).trim_end(),
        "function a(a) {\n  return a;\n}"
    );
    assert!(String::from_utf8_lossy(&with_keep.stdout).contains("handleClick"));
}

#[test]
fn rename_and_mangle_conflict() {
    let out = stdin_output("var a = 1;", &["-m", "--rename", "simple"]);
    assert_eq!(out.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&out.stderr).contains("cannot be used with"));
}

#[test]
fn unique_names_stay_distinct_across_several_inputs() {
    let dir = tempfile::tempdir().unwrap();
    let first = dir.path().join("a.js");
    let second = dir.path().join("b.js");
    std::fs::write(&first, "function _0xa() {}").unwrap();
    std::fs::write(&second, "function _0xb() {}").unwrap();
    let outdir = dir.path().join("out");
    let out = bin()
        .arg("--rename")
        .arg("unique")
        .arg("--unsafe")
        .arg("--no-unpack")
        .arg("-o")
        .arg(&outdir)
        .arg(&first)
        .arg(&second)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let a = std::fs::read_to_string(outdir.join("deobfuscated.a.js")).unwrap();
    let b = std::fs::read_to_string(outdir.join("deobfuscated.b.js")).unwrap();
    assert_eq!(a.trim_end(), "function fn1() {}");
    assert_eq!(b.trim_end(), "function fn2() {}");
}

fn names_the_flag(line: &str, flag: &str) -> bool {
    line.match_indices(flag).any(|(at, _)| {
        line[at + flag.len()..]
            .chars()
            .next()
            .is_none_or(|next| !next.is_alphanumeric() && next != '-')
    })
}

fn help_text_for(help: &str, flag: &str) -> String {
    let lines: Vec<&str> = help.lines().collect();
    let Some(at) = lines
        .iter()
        .position(|line| !line.starts_with("Usage:") && names_the_flag(line, flag))
    else {
        return String::new();
    };
    let Some((_, rest)) = lines[at].split_once(flag) else {
        return String::new();
    };
    let trailing = rest.trim_start_matches(['.', '>', ']']);
    if trailing.trim().is_empty() {
        return lines.get(at + 1).unwrap_or(&"").trim().to_string();
    }
    trailing.trim().to_string()
}

#[test]
fn the_help_describes_every_public_flag_and_no_internal_one() {
    let out = bin().arg("--help").output().unwrap();
    assert!(out.status.success());
    let help = String::from_utf8_lossy(&out.stdout).to_string();
    assert!(!help.contains("--conservative"), "{help}");
    assert!(!help.contains("--parity"), "{help}");
    for flag in [
        "[FILES]",
        "--output",
        "--force",
        "--no-jsx",
        "--no-unpack",
        "--no-deobfuscate",
        "--no-unminify",
        "--unsafe",
        "--rename",
        "--mangle",
        "--keep-human-names",
        "--max-iterations",
        "--report",
        "--verbose",
    ] {
        assert!(
            !help_text_for(&help, flag).is_empty(),
            "{flag} has no help line:\n{help}"
        );
    }
}
