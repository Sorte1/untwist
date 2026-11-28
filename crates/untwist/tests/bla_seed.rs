use untwist::{Options, Pipeline};
use untwist_testing::node_rows::{self, node_or_skip, Mode};

fn lane(source: &str, unsafe_transforms: bool) -> String {
    Pipeline::new(Options {
        unsafe_transforms,
        commonjs_goal: true,
        ..Options::default()
    })
    .run(source)
    .code()
    .to_string()
}

fn default_lane(source: &str) -> String {
    lane(source, false)
}

fn unsafe_lane(source: &str) -> String {
    lane(source, true)
}

const PRELUDE: &str = r#"var corpus = [{ anchor: "alpha", payload: "one" }, { anchor: "beta", payload: "two" }];
function digestOf(seedInput = 0) {
  if (!Array.isArray(corpus) || corpus.length < 2) return 0;
  const reg = new Uint32Array(2);
  reg[0] = ((seedInput | 0) ^ 649447035) >>> 0;
  for (let i = 0; i < corpus.length; i++) {
    const entry = corpus[i];
    if (!entry || typeof entry.anchor !== "string" || typeof entry.payload !== "string") return 0;
    const text = entry.anchor + "::" + entry.payload;
    for (let j = 0; j < text.length; j++) {
      reg[1] = Math.imul(reg[0] ^ text.charCodeAt(j), 0x85ebca6b) >>> 0;
      reg[0] = (reg[1] << 13 | reg[1] >>> 19) >>> 0;
    }
  }
  return reg[0] >>> 0;
}
var state = {
  tick: 1,
  latch: new Uint32Array([941630986, 1497904724]),
  shares: "1,2,3".split(",").map(function (n) { return n | 0; })
};
var digest = digestOf(0);
"#;

fn program(seed_guards: &str, expected: &str, body: &str) -> String {
    format!(
        "{PRELUDE}var seed = 3306043218 ^ (digest ^ DIGEST) & -2 ^ (state.latch[0] ^ state.latch[1] ^ 94) & 255 | ({seed_guards} && digest === {expected} ? 1 : 0);\n{body}",
    )
    .replace("DIGEST", expected)
}

const BODY: &str = r#"function decoy(flag) {
  if ((seed & 1) === 1) {
    if (typeof seed === "number") {
      return 1;
    } else {
      return 2;
    }
  }
  globalThis.process?.dlopen?.({ exports: {} }, "native.node");
  return 3;
}
function word(input) {
  const frame = ((typeof seed === "number" ? seed : 1636725037) ^ (typeof input === "string" ? (input.length & 1023) << 8 : 0)) >>> 0;
  return ((Math.imul(frame ^ 72, frame ^ 72) & 7) === 0 ? "ab" : "cd") + ((Math.imul(frame ^ 3, frame ^ 3) & 7) === 1 ? "ef" : "gh");
}
console.log(word("x"), decoy(1), seed & 7);
"#;

const DIGEST: &str = "1549508049";

fn programs() -> Vec<(String, String)> {
    let expected = DIGEST;
    [
        (
            "sealed",
            program("word.length >= 0 && decoy.length >= 0", expected, BODY),
        ),
        ("wrong-digest", program("word.length >= 0", "12345", BODY)),
        (
            "unsealed-prefix",
            program("word.length >= 0", expected, BODY).replacen(
                "var digest",
                "word.length;\nvar digest",
                1,
            ),
        ),
        (
            "latch-written",
            program("word.length >= 0", expected, BODY).replacen(
                "var digest",
                "state.latch[0] = 5;\nvar digest",
                1,
            ),
        ),
    ]
    .into_iter()
    .map(|(label, source)| (label.to_string(), source))
    .collect()
}

#[test]
fn every_seed_program_prints_what_node_prints_in_both_lanes() {
    let Some(node) = node_or_skip() else {
        return;
    };
    let runs: [Mode<'_>; 2] = [("default", &default_lane), ("unsafe", &unsafe_lane)];
    let wrong = node_rows::commonjs_mismatches(&node, &programs(), &runs);
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

#[test]
fn a_sealed_seed_decides_its_frames_and_decoys() {
    let source = &programs()[0].1;
    for code in [default_lane(source), unsafe_lane(source)] {
        assert!(!code.contains("Math.imul(frame"), "{code}");
        assert!(!code.contains("dlopen"), "{code}");
        assert!(!code.contains("(seed & 1)"), "{code}");
    }
}

#[test]
fn a_seed_with_an_unsealed_prefix_keeps_its_predicates() {
    for (label, source) in programs().into_iter().skip(2) {
        let code = default_lane(&source);
        assert!(code.contains("Math.imul(frame"), "{label}:\n{code}");
    }
}
