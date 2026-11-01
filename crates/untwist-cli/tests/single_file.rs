mod common;

use common::{bin, stdin_output, WEBPACK_4};

#[test]
fn single_file_flag_writes_one_module_file() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("in.js");
    std::fs::write(&input, WEBPACK_4).unwrap();
    let outdir = dir.path().join("out");
    let out = bin()
        .arg("--single-file")
        .arg("-o")
        .arg(&outdir)
        .arg(&input)
        .output()
        .unwrap();
    assert!(out.status.success());
    let mut names: Vec<String> = std::fs::read_dir(&outdir)
        .unwrap()
        .filter_map(Result::ok)
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    assert_eq!(
        names,
        vec!["bundle.json".to_string(), "deobfuscated.js".to_string()],
        "a module was written as its own file"
    );
}

#[test]
fn single_file_flag_prints_the_concatenation_to_stdout() {
    let out = stdin_output(WEBPACK_4, &["--single-file"]);
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout).into_owned();
    assert!(
        stdout.starts_with("// untwist: every module of this bundle, concatenated for reading"),
        "{stdout}"
    );
    assert!(stdout.contains("// ==== "), "{stdout}");
}

#[test]
fn single_file_with_several_inputs_and_no_bundle_banners_each_input() {
    let dir = tempfile::tempdir().unwrap();
    let first = dir.path().join("a.js");
    let second = dir.path().join("b.js");
    std::fs::write(&first, "var a=!0;").unwrap();
    std::fs::write(&second, "var b=!1;").unwrap();
    let out = bin()
        .arg("--single-file")
        .arg(&first)
        .arg(&second)
        .output()
        .unwrap();
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout).into_owned();
    assert_eq!(
        stdout.trim_end(),
        "// ==== input 1 ====\nvar a = true;\n\n// ==== input 2 ====\nvar b = false;"
    );
}

#[test]
fn single_file_with_one_input_and_no_bundle_stays_bare() {
    let out = stdin_output("var a=!0;", &["--single-file"]);
    assert!(out.status.success());
    assert_eq!(
        String::from_utf8_lossy(&out.stdout).trim_end(),
        "var a = true;"
    );
}

#[test]
fn single_file_json_report_puts_the_concatenation_in_code() {
    let out = stdin_output(WEBPACK_4, &["--single-file", "--report", "json"]);
    assert!(out.status.success());
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    let code = v["code"].as_str().expect("code is a string");
    assert!(
        code.starts_with("// untwist: every module of this bundle, concatenated for reading"),
        "{code}"
    );
    assert!(code.contains("// ==== "), "{code}");
}
