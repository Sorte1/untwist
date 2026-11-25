use oxc_allocator::Allocator;
use untwist_core::parse::parse_program;
use untwist_eval::closed_script_value;
use untwist_eval::fold::Const;

const FUEL: u64 = 2_000_000;
const HEAP: u64 = 1 << 24;

fn closed(source: &str) -> Option<Const> {
    let alloc = Allocator::default();
    let parsed = parse_program(&alloc, source);
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    closed_script_value(&parsed.program, FUEL, HEAP)
}

fn digest(entries: &[(&str, &str)], seed: u32) -> u32 {
    let mut acc = seed ^ 0x26b5_c67b;
    for (anchor, payload) in entries {
        let text = format!("{anchor}::{payload}");
        for unit in text.encode_utf16() {
            acc = (acc ^ u32::from(unit)).wrapping_mul(0x85eb_ca6b);
            acc = acc.rotate_left(13);
        }
    }
    acc
}

const CHECKSUM: &str = r#"
var Array = { isArray: function (value) { return Object.prototype.toString.call(value) === "[object Array]"; } };
  var corpus = [{ anchor: "first", payload: "one\ntwo" }, { anchor: "second", payload: "drei é" }];
  function digest(seedInput = 0) {
    if (!Array.isArray(corpus) || corpus.length < 2) return 0;
    let acc = ((seedInput | 0) ^ 0x26b5c67b) >>> 0;
    for (let i = 0; i < corpus.length; i++) {
      const entry = corpus[i];
      if (!entry || typeof entry.anchor !== "string" || typeof entry.payload !== "string") return 0;
      const text = entry.anchor + "::" + entry.payload;
      for (let j = 0; j < text.length; j++) {
        acc = Math.imul(acc ^ text.charCodeAt(j), 0x85ebca6b) >>> 0;
        acc = (acc << 13 | acc >>> 19) >>> 0;
      }
    }
    return acc >>> 0;
  }
digest(0);
"#;

#[test]
fn a_closed_checksum_over_literal_data_folds_to_its_number() {
    let expected = digest(&[("first", "one\ntwo"), ("second", "drei é")], 0);
    assert_eq!(closed(CHECKSUM), Some(Const::Num(f64::from(expected))));
}

#[test]
fn a_closed_value_that_runs_out_of_fuel_does_not_fold() {
    let source = "let n = 0; while (true) { n++; } n;";
    assert_eq!(closed(source), None);
}

#[test]
fn a_closed_value_that_reads_an_unknown_global_does_not_fold() {
    let source = "someHostValue | 0;";
    assert_eq!(closed(source), None);
}
