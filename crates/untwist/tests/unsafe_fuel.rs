use untwist::{Options, Pipeline};
use untwist_deobfuscate::notes;
use untwist_testing::node_rows::{node_or_skip, printed};

const WORDS: usize = 2000;
const SHIFTS: usize = 1900;
const ROUNDS: usize = 1000;
const TERMS: [usize; 4] = [3, 11, 19, 27];
const PRINTED: [usize; 3] = [0, 1, WORDS - 1];

fn words() -> Vec<String> {
    (0..WORDS)
        .map(|index| match TERMS.iter().position(|term| *term == index) {
            Some(position) => format!("{}k", (position + 1) * 1000),
            None => format!("w{index}x"),
        })
        .collect()
}

fn quoted(word: &str) -> String {
    format!("\"{word}\"")
}

fn printing(words: impl Iterator<Item = String>) -> String {
    format!("console.log({});\n", words.collect::<Vec<_>>().join(", "))
}

fn rotated_program() -> String {
    let words = words();
    let stored: Vec<String> = (0..WORDS)
        .map(|index| quoted(&words[(index + WORDS - SHIFTS) % WORDS]))
        .collect();
    let checksum: Vec<String> = TERMS
        .iter()
        .enumerate()
        .map(|(position, index)| format!("parseInt(d({index})) / {}", position + 1))
        .collect();
    let expected = TERMS.len() * 1000;
    format!(
        "function a() {{\n  var s = [{array}];\n  a = function () {{\n    return s;\n  }};\n  return a();\n}}\n\
         function d(i) {{\n  var s = a();\n  var t = s[i];\n  var o = \"\";\n  for (var r = 0; r < {ROUNDS}; r++) {{\n    o = t.split(\"\").reverse().join(\"\");\n  }}\n  return o.split(\"\").reverse().join(\"\") === t ? t : o;\n}}\n\
         (function (q, e) {{\n  var s = q();\n  while (true) {{\n    try {{\n      var c = {checksum};\n      if (c === e) {{\n        break;\n      }} else {{\n        s.push(s.shift());\n      }}\n    }} catch (x) {{\n      s.push(s.shift());\n    }}\n  }}\n}})(a, {expected});\n{printed}",
        array = stored.join(", "),
        checksum = checksum.join(" + "),
        printed = printing(PRINTED.iter().map(|index| format!("d({index})"))),
    )
}

fn original() -> String {
    let words = words();
    printing(PRINTED.iter().map(|index| quoted(&words[*index])))
}

fn run(unsafe_transforms: bool) -> untwist::Output {
    let options = Options {
        unsafe_transforms,
        ..Options::default()
    };
    Pipeline::new(options).run(&rotated_program())
}

fn spent(out: &untwist::Output) -> bool {
    let exhausted = notes::decode_bailed(untwist_eval::bail::Bail::Fuel);
    out.notes.iter().any(|note| note.message == exhausted)
}

#[test]
#[cfg_attr(
    debug_assertions,
    ignore = "spends about 60 million interpreter steps; run with cargo test --release"
)]
fn a_rotation_search_past_the_default_budget_decodes_under_unsafe() {
    let out = run(true);
    assert!(!spent(&out), "{:?}", out.notes);
    assert_eq!(out.code().trim_end(), original().trim_end());
}

#[test]
#[cfg_attr(
    debug_assertions,
    ignore = "spends about 60 million interpreter steps; run with cargo test --release"
)]
fn the_default_lane_keeps_its_budget_and_leaves_the_calls() {
    let out = run(false);
    assert!(spent(&out), "{:?}", out.notes);
    assert!(out.code().contains("s.push(s.shift())"), "{}", out.code());
}

#[test]
#[cfg_attr(
    debug_assertions,
    ignore = "spends about 60 million interpreter steps; run with cargo test --release"
)]
fn the_unsafe_output_prints_what_the_original_prints() {
    let Some(node) = node_or_skip() else {
        return;
    };
    let dir = tempfile::tempdir().unwrap();
    let expected = printed(&node, dir.path(), &original());
    let actual = printed(&node, dir.path(), run(true).code());
    assert!(
        expected.is_ok() && expected == actual,
        "{expected:?} vs {actual:?}"
    );
}
