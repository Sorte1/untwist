mod common;

use common::{stdin_output, SPLIT_STRINGS_BEHIND_A_DECODER};

fn wide_array(elements: usize) -> String {
    let zeros = vec!["0"; elements].join(",");
    format!("var a = [{zeros}];\nconsole.log(a.length);\n")
}

#[test]
fn an_input_whose_parse_goes_past_the_memory_cap_is_passed_through_with_the_reason() {
    let source = wide_array(1 << 20);
    let out = stdin_output(&source, &["--max-memory", "48MiB"]);
    let stderr = String::from_utf8_lossy(&out.stderr).into_owned();
    assert_eq!(out.status.code(), Some(3), "{stderr}");
    assert!(stderr.contains("budget-memory"), "{stderr}");
    assert_eq!(String::from_utf8_lossy(&out.stdout).trim(), source.trim());
}

#[test]
fn the_reports_carry_the_budget_and_the_memory_peak() {
    let out = stdin_output(
        SPLIT_STRINGS_BEHIND_A_DECODER,
        &[
            "--report",
            "json",
            "--max-memory",
            "1GiB",
            "--timeout",
            "600",
        ],
    );
    assert!(out.status.success());
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(v["budget"]["max_memory"], 1u64 << 30);
    assert_eq!(v["budget"]["timeout"], 600.0);
    assert!(v["budget"]["memory_peak"].is_u64());
    assert_eq!(v["budget"]["hits"], serde_json::json!([]));
    let text = stdin_output(SPLIT_STRINGS_BEHIND_A_DECODER, &["--report", "text"]);
    let report = String::from_utf8_lossy(&text.stderr).into_owned();
    assert!(
        report.contains("budget: max-memory 4.0 GiB, timeout none, memory peak"),
        "{report}"
    );
}

#[test]
fn a_bad_budget_value_is_a_usage_error() {
    for args in [["--max-memory", "lots"], ["--timeout", "0"]] {
        let out = stdin_output("var a;", &args);
        assert_eq!(out.status.code(), Some(2));
    }
}
