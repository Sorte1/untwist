use std::process::Command;

use untwist_testing::node::{node, skip_allowed, SKIP_MESSAGE};
use untwist_testing::{transform, Allocator, PassCtorAlias};

struct Char {
    string: &'static str,
    raws: &'static [&'static str],
}

const fn ch(string: &'static str, raws: &'static [&'static str]) -> Char {
    Char { string, raws }
}

const CHARS: &[Char] = &[
    ch("\\0", &["\\0", "\\x00"]),
    ch("\\\\", &["\\\\"]),
    ch("`", &["\\`"]),
    ch("$", &["$", "\\$"]),
    ch("{", &["{", "\\{"]),
    ch("}", &["}"]),
    ch("\\r", &["\\r", "\r", "\\\r"]),
    ch("\\n", &["\\n", "\n", "\\\n"]),
    ch("\\u2028", &["\\u2028", "\u{2028}", "\\\u{2028}"]),
    ch("\\u2029", &["\\u2029", "\u{2029}"]),
    ch("\\b", &["\\b"]),
    ch("\\f", &["\\f"]),
    ch("\\t", &["\\t", "\t"]),
    ch("\\v", &["\\v"]),
    ch("0", &["0"]),
    ch("1", &["1"]),
    ch("7", &["7"]),
    ch("8", &["8"]),
    ch("9", &["9"]),
    ch("x", &["x"]),
    ch("u", &["u"]),
    ch("a", &["a"]),
    ch("F", &["F"]),
    ch("\\x1b", &["\\x1b", "\u{1b}"]),
    ch("\\x07", &["\\x07"]),
    ch("\\x7f", &["\\x7f"]),
    ch("\\u009b", &["\\u009b"]),
    ch("\\u202e", &["\\u202e"]),
    ch("'", &["'"]),
    ch("\\\"", &["\""]),
    ch("\\u{1F600}", &["\\u{1F600}", "\u{1F600}"]),
];

const DRIVER: &str = r#"
const cases = JSON.parse(require("fs").readFileSync(process.argv[2], "utf8"));
const run = (code) => { try { return { value: (0, eval)(code) }; } catch (e) { return { error: String(e) }; } };
const bad = [];
for (const [input, output] of cases) {
  const want = run(input);
  const got = run(output);
  if (want.error !== undefined || got.error !== undefined || want.value !== got.value) {
    bad.push({ input, output, want, got });
  }
}
console.log(JSON.stringify({ checked: cases.length, bad: bad.slice(0, 20), failures: bad.length }));
"#;

fn pair_cases(a: &Char, b: &Char) -> Vec<String> {
    let (sa, sb) = (a.string, b.string);
    let mut cases = vec![
        format!("`${{\"{sa}{sb}\"}}`;"),
        format!("`${{\"{sa}\"}}${{\"{sb}\"}}`;"),
        format!("`x${{\"{sa}\"}}${{1}}${{\"{sb}\"}}x`;"),
    ];
    for ra in a.raws {
        cases.push(format!("`{ra}${{\"{sb}\"}}`;"));
        cases.push(format!("`{ra}` + \"{sb}\";"));
        for rb in b.raws {
            cases.push(format!("`{ra}` + `{rb}`;"));
        }
    }
    for rb in b.raws {
        cases.push(format!("`${{\"{sa}\"}}{rb}`;"));
        cases.push(format!("\"{sa}\" + `{rb}`;"));
    }
    cases
}

fn transformed(source: &str) -> String {
    let alloc = Allocator::default();
    let ctors: &[PassCtorAlias<'_>] = &[
        untwist_unminify::string_literal_in_template::boxed,
        untwist_unminify::template_literals::boxed,
    ];
    transform(&alloc, source, ctors)
}

#[test]
fn merged_template_text_means_what_node_reads_for_every_character_pair() {
    let Some(node) = node() else {
        let required = std::env::var("UNTWIST_REQUIRE_NODE").ok();
        assert!(
            skip_allowed(required.as_deref()),
            "required: {SKIP_MESSAGE}"
        );
        eprintln!("{SKIP_MESSAGE}");
        return;
    };
    let cases: Vec<(String, String)> = CHARS
        .iter()
        .flat_map(|a| CHARS.iter().map(move |b| (a, b)))
        .flat_map(|(a, b)| pair_cases(a, b))
        .map(|input| {
            let output = transformed(&input);
            (input, output)
        })
        .collect();
    let dir = tempfile::tempdir().expect("temp dir");
    let driver = dir.path().join("driver.js");
    let data = dir.path().join("cases.json");
    std::fs::write(&driver, DRIVER).expect("write driver");
    std::fs::write(&data, serde_json::to_string(&cases).expect("cases json")).expect("write cases");
    let out = Command::new(node)
        .arg(&driver)
        .arg(&data)
        .output()
        .expect("run node");
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let report: serde_json::Value = serde_json::from_slice(&out.stdout).expect("driver json");
    assert_eq!(report["checked"].as_u64(), Some(cases.len() as u64));
    assert_eq!(
        report["failures"].as_u64(),
        Some(0),
        "{}",
        serde_json::to_string_pretty(&report["bad"]).unwrap_or_default()
    );
}
