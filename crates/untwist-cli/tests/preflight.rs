mod common;

use common::bin;

#[test]
fn the_max_input_cap_warns_but_still_runs() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("in.js");
    std::fs::write(&input, "var a=!0;").unwrap();
    let out = bin()
        .arg("--max-input-mb")
        .arg("0")
        .arg(&input)
        .output()
        .unwrap();
    assert!(out.status.success());
    assert!(String::from_utf8_lossy(&out.stderr).contains("--max-input-mb 0"));
    assert_eq!(
        String::from_utf8_lossy(&out.stdout).trim_end(),
        "var a = true;"
    );
}
