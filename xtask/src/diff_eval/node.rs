use std::path::Path;
use std::process::Command;

use anyhow::{bail, Context, Result};
use untwist_eval::builtins::all_root_names;
use untwist_eval::builtins::shape::SHAPES;
use untwist_eval::inspect::shape_label;

use super::render::{canonical, NODE_RENDER};
use super::Case;

const NODE_RUNNER: &str = r#"
var OUT = [];
process.on('unhandledRejection', function () {});
var WATCHED = [], BASE = [], COUNTS = [], SLOTS = [];
var HOST = ['process', 'console', 'globalThis', 'navigator', 'performance', 'crypto'];
function keysOf(o) { return o === globalThis ? Object.getOwnPropertyNames(o) : Reflect.ownKeys(o); }
function snapshot(o) {
  if (HOST.indexOf(PRO.get(o)) >= 0) return [];
  var keys = Reflect.ownKeys(o), slots = [];
  for (var j = 0; j < keys.length; j++) { var d = Reflect.getOwnPropertyDescriptor(o, keys[j]); slots.push([keys[j], d, 'value' in d ? d.value : d.get]); }
  return slots;
}
function watch() {
  WATCHED = [globalThis].concat(Array.from(NAT.keys()), Array.from(PRO.keys()));
  BASE = WATCHED.map(function (o) { return new Set(keysOf(o)); });
  COUNTS = BASE.map(function (b) { return b.size; });
  SLOTS = WATCHED.map(function (o, i) { return i === 0 ? [] : snapshot(o); });
}
function changed(o, slots) {
  var dirty = false;
  for (var j = 0; j < slots.length; j++) {
    var d = Reflect.getOwnPropertyDescriptor(o, slots[j][0]);
    var now = d === undefined ? undefined : 'value' in d ? d.value : d.get;
    if (d !== undefined && Object.is(now, slots[j][2])) continue;
    dirty = true;
    try { Reflect.defineProperty(o, slots[j][0], slots[j][1]); } catch (e) {}
  }
  return dirty;
}
function polluted() {
  var dirty = false;
  for (var i = 0; i < WATCHED.length; i++) {
    dirty = changed(WATCHED[i], SLOTS[i]) || dirty;
    var keys = keysOf(WATCHED[i]);
    if (keys.length === COUNTS[i]) continue;
    dirty = true;
    for (var j = 0; j < keys.length; j++) if (!BASE[i].has(keys[j])) { try { delete WATCHED[i][keys[j]]; } catch (e) {} }
    var left = keysOf(WATCHED[i]);
    BASE[i] = new Set(left);
    COUNTS[i] = left.length;
    SLOTS[i] = i === 0 ? [] : snapshot(WATCHED[i]);
  }
  return dirty;
}
function compile(__source) { return eval('(function () { return ' + __source + '; })'); }
function run(__source) {
  var out, fn;
  try { fn = compile(__source); } catch (e) { OUT.push('S'); return; }
  var value, threw = false;
  try { value = fn(); } catch (e) { value = e; threw = true; }
  try { out = (threw ? 'T' : 'V') + R(value, 0); } catch (e) { out = 'Rerr'; }
  if (polluted()) out = 'P' + out;
  OUT.push(out);
}
"#;

fn native_labels() -> String {
    let pairs: Vec<serde_json::Value> = SHAPES
        .iter()
        .filter(|entry| entry.owner != "hostile")
        .map(|entry| {
            let label = shape_label(entry.owner, entry.name);
            serde_json::json!([label, canonical(&label)])
        })
        .collect();
    serde_json::Value::Array(pairs).to_string()
}

fn json_list(items: &[String]) -> String {
    serde_json::Value::Array(
        items
            .iter()
            .map(|item| serde_json::Value::String(item.clone()))
            .collect(),
    )
    .to_string()
}

fn header() -> String {
    let roots: Vec<String> = all_root_names()
        .iter()
        .map(|name| name.to_string())
        .collect();
    let mut script = String::from(NODE_RENDER);
    script.push_str(NODE_RUNNER);
    script.push_str(&format!(
        "nameIntrinsics({}, {});\nPRO.set(globalThis, 'globalThis');\nwatch();\n",
        native_labels(),
        json_list(&roots)
    ));
    script
}

fn case_line(source: &str) -> String {
    format!("run({});\n", serde_json::Value::String(source.to_string()))
}

fn script(cases: &[Case]) -> String {
    let mut script = header();
    for case in cases {
        script.push_str(&case_line(&case.decode));
        if let Some(fold) = &case.fold {
            script.push_str(&case_line(fold));
        }
    }
    script.push_str("process.stdout.write(OUT.join('\\n') + '\\n');\n");
    script
}

pub fn run_node(node: &Path, cases: &[Case], dir: &Path) -> Result<Vec<String>> {
    let path = dir.join("diff-eval.js");
    std::fs::write(&path, script(cases)).context("write the node script")?;
    let out = Command::new(node)
        .arg("--stack-size=4000")
        .arg(&path)
        .current_dir(dir)
        .output()
        .context("spawn node")?;
    if !out.status.success() {
        bail!("node failed: {}", String::from_utf8_lossy(&out.stderr));
    }
    Ok(String::from_utf8_lossy(&out.stdout)
        .lines()
        .map(str::to_string)
        .collect())
}
