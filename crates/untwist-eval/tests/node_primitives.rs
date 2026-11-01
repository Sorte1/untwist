mod common;

use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

use oxc_syntax::operator::BinaryOperator;
use untwist_eval::coerce::{number_to_string, parse_int, to_int32, to_number};
use untwist_eval::ops::{binary_op, loose_equals};
use untwist_eval::value::{JsString, Value};

const SKIP: &str = "skipping node differential: `node` not found on PATH and UNTWIST_NODE is unset";

static NEXT_RUN: AtomicU64 = AtomicU64::new(0);

fn node_binary() -> Option<PathBuf> {
    let candidate = std::env::var("UNTWIST_NODE").unwrap_or_else(|_| "node".to_string());
    Command::new(&candidate)
        .arg("--version")
        .output()
        .ok()
        .filter(|out| out.status.success())
        .map(|_| PathBuf::from(candidate))
}

fn script_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/primitives.js")
}

struct TempRunDir {
    path: PathBuf,
}

impl Drop for TempRunDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.path);
    }
}

fn run_node(node: &PathBuf, payload: &str) -> Option<Vec<String>> {
    let run = NEXT_RUN.fetch_add(1, Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!("untwist-node-{}-{run}", std::process::id()));
    std::fs::create_dir_all(&dir).ok()?;
    let _guard = TempRunDir { path: dir.clone() };
    let input = dir.join("cases.json");
    std::fs::write(&input, payload).ok()?;
    let out = Command::new(node)
        .arg(script_path())
        .arg(&input)
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    serde_json::from_slice(&out.stdout).ok()
}

const NUMBER_STRINGS: [&str; 17] = [
    "",
    "  12  ",
    "0x1cc",
    "0X1CC",
    "0b101",
    "0o17",
    "1e3",
    "12.",
    ".5",
    "12px",
    "Infinity",
    "-Infinity",
    "+Infinity",
    "0x",
    "590708XPLBWj",
    "  -12abc",
    "08",
];

const NUMBERS: [f64; 12] = [
    0.0,
    -0.0,
    1.0,
    255.0,
    1e21,
    5e-7,
    0.30000000000000004,
    -1.5,
    1e10,
    2147483648.0,
    -2147483649.0,
    9007199254740993.0,
];

#[test]
fn to_number_to_string_parse_int_and_to_int32_match_node() {
    let Some(node) = node_binary() else {
        println!("{SKIP}");
        return;
    };
    let mut cases = Vec::new();
    let mut mine = Vec::new();
    for text in NUMBER_STRINGS {
        let units: Vec<u16> = text.encode_utf16().collect();
        cases.push(serde_json::json!({ "op": "to_number", "a": text }));
        mine.push(common::render_number(to_number(text)));
        cases.push(serde_json::json!({ "op": "parse_int", "a": text, "radix": null }));
        mine.push(common::render_number(
            parse_int(&units, None).unwrap_or(f64::NAN),
        ));
        cases.push(serde_json::json!({ "op": "parse_int", "a": text, "radix": 16 }));
        mine.push(common::render_number(
            parse_int(&units, Some(16.0)).unwrap_or(f64::NAN),
        ));
    }
    for value in NUMBERS {
        cases.push(serde_json::json!({ "op": "to_string", "a": value }));
        mine.push(format!("string:{}", number_to_string(value)));
        cases.push(serde_json::json!({ "op": "to_int32", "a": value }));
        mine.push(common::render_number(f64::from(to_int32(value))));
    }
    let payload = serde_json::to_string(&cases).expect("serialise cases");
    let theirs = run_node(&node, &payload).expect("node produced parsable output");
    assert_eq!(theirs.len(), mine.len());
    for (index, (from_node, ours)) in theirs.iter().zip(mine.iter()).enumerate() {
        assert_eq!(ours, from_node, "case {index}: {}", cases[index]);
    }
}

const OPERANDS: [&str; 9] = [
    "1", "'2'", "true", "false", "null", "'0x1cc'", "''", "'a'", "0",
];

fn operand_value(token: &str) -> Value<'static> {
    match token {
        "true" => Value::Bool(true),
        "false" => Value::Bool(false),
        "null" => Value::Null,
        text if text.starts_with('\'') => Value::Str(JsString::from_str(text.trim_matches('\''))),
        text => Value::Number(text.parse::<f64>().unwrap_or(f64::NAN)),
    }
}

fn json_operand(token: &str) -> serde_json::Value {
    match token {
        "true" => serde_json::json!(true),
        "false" => serde_json::json!(false),
        "null" => serde_json::Value::Null,
        text if text.starts_with('\'') => serde_json::json!(text.trim_matches('\'')),
        text => serde_json::json!(text.parse::<f64>().unwrap_or(0.0)),
    }
}

#[test]
fn addition_and_abstract_equality_match_node() {
    let Some(node) = node_binary() else {
        println!("{SKIP}");
        return;
    };
    let mut host = common::PureTestHost::default();
    let mut cases = Vec::new();
    let mut mine = Vec::new();
    for left in OPERANDS {
        for right in OPERANDS {
            let a = operand_value(left);
            let b = operand_value(right);
            let sum = binary_op(BinaryOperator::Addition, &a, &b, &mut host).expect("no throw");
            mine.push(match &sum {
                Value::Str(text) => format!("string:{}", text.to_lossy_string()),
                Value::Number(n) => common::render_number(*n),
                other => format!("{other:?}"),
            });
            cases.push(serde_json::json!({
                "op": "add", "a": json_operand(left), "b": json_operand(right)
            }));
            let equal = loose_equals(&a, &b, &mut host).expect("no throw");
            mine.push(format!("boolean:{equal}"));
            cases.push(serde_json::json!({
                "op": "loose_eq", "a": json_operand(left), "b": json_operand(right)
            }));
        }
    }
    let payload = serde_json::to_string(&cases).expect("serialise cases");
    let theirs = run_node(&node, &payload).expect("node produced parsable output");
    for (index, (from_node, ours)) in theirs.iter().zip(mine.iter()).enumerate() {
        assert_eq!(ours, from_node, "case {index}: {}", cases[index]);
    }
}

#[test]
fn decode_uri_component_matches_node_including_its_throws() {
    let Some(node) = node_binary() else {
        println!("{SKIP}");
        return;
    };
    let inputs = ["%48%69", "%E2%9C%93", "%FF", "%", "%zz", "plain", "%00"];
    let cases: Vec<serde_json::Value> = inputs
        .iter()
        .map(|input| serde_json::json!({ "op": "decode_uri_component", "a": input }))
        .collect();
    let payload = serde_json::to_string(&cases).expect("serialise cases");
    let theirs = run_node(&node, &payload).expect("node produced parsable output");
    for (input, from_node) in inputs.iter().zip(theirs.iter()) {
        let ours = match common::eval(&format!("decodeURIComponent('{input}')")) {
            Ok(value) => value,
            Err(message) if message.starts_with("throw error:URIError") => {
                "throw:URIError".to_string()
            }
            Err(message) => panic!("decodeURIComponent({input:?}) failed unexpectedly: {message}"),
        };
        assert_eq!(&ours, from_node, "decodeURIComponent({input:?})");
    }
}
