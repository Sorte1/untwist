use untwist_types::{annotate, Mode, Refusal};

struct Row {
    hazard: &'static str,
    source: &'static str,
    absent: &'static [&'static str],
    present: &'static [&'static str],
}

const ROWS: &[Row] = &[
    Row {
        hazard: "direct eval reaches every binding",
        source: "(function () {\n  function f(n) {\n    return n * 2;\n  }\n  eval(\"\");\n  f(1);\n})();\n",
        absent: &[": "],
        present: &[],
    },
    Row {
        hazard: "with reaches every binding",
        source: "(function (o) {\n  function f(n) {\n    return n * 2;\n  }\n  with (o) {\n    f(1);\n  }\n})({});\n",
        absent: &[": "],
        present: &[],
    },
    Row {
        hazard: "mapped arguments alias the parameters",
        source: "(function () {\n  function f(n) {\n    arguments[0] = \"s\";\n    return n;\n  }\n  f(1);\n})();\n",
        absent: &["n: "],
        present: &[],
    },
    Row {
        hazard: "Reflect.set with a receiver writes an object nobody types",
        source: "(function () {\n  const o = { a: 1 };\n  Reflect.set(o, \"a\", \"s\", o);\n  console.log(o.a);\n})();\n",
        absent: &["o: "],
        present: &[],
    },
    Row {
        hazard: "a prototype setter catches a class field write",
        source: "(function () {\n  class A {\n    constructor() {\n      this.x = 1;\n    }\n  }\n  Object.defineProperty(A.prototype, \"x\", { set(v) {} });\n  console.log(new A().x);\n})();\n",
        absent: &["declare", "x: "],
        present: &[],
    },
    Row {
        hazard: "a Proxy can answer any read",
        source: "(function () {\n  const p = new Proxy({}, { get: () => \"s\" });\n  const n = p.count;\n  console.log(n);\n})();\n",
        absent: &["p: ", "n: "],
        present: &[],
    },
    Row {
        hazard: "defineProperty on a built-in prototype",
        source: "(function () {\n  Object.defineProperty(String.prototype, \"trim\", { value: () => 1 });\n  function f(s) {\n    return s.trim();\n  }\n  console.log(f(\"a\"));\n})();\n",
        absent: &["): string"],
        present: &[],
    },
    Row {
        hazard: "a patched String.prototype method",
        source: "(function () {\n  String.prototype.toUpperCase = function () {\n    return 1;\n  };\n  function f(s) {\n    return s.toUpperCase();\n  }\n  console.log(f(\"a\"));\n})();\n",
        absent: &["): string"],
        present: &[],
    },
    Row {
        hazard: "DOM factories stay untyped (an XML document returns a plain Element)",
        source: "(function () {\n  function make(tag) {\n    return document.createElement(tag);\n  }\n  console.log(make(\"canvas\"));\n})();\n",
        absent: &["HTMLCanvasElement", "): "],
        present: &[],
    },
    Row {
        hazard: "arrays with holes are never typed",
        source: "(function () {\n  function first(a) {\n    return a[0];\n  }\n  console.log(first([1, , 2]));\n})();\n",
        absent: &["[]", "a: "],
        present: &[],
    },
    Row {
        hazard: "negative zero is a number, never the literal 0",
        source: "(function () {\n  function f(z) {\n    return z;\n  }\n  console.log(f(-0) === 0);\n})();\n",
        absent: &["z: 0"],
        present: &[],
    },
    Row {
        hazard: "a bigint operand throws, so it never meets a number",
        source: "(function () {\n  function f(a) {\n    return a * 2;\n  }\n  try {\n    f(1n);\n  } catch (e) {}\n})();\n",
        absent: &["a: bigint"],
        present: &[],
    },
    Row {
        hazard: "JSON.stringify can give undefined",
        source: "(function () {\n  function f(v) {\n    return JSON.stringify(v);\n  }\n  console.log(f(undefined) === undefined);\n})();\n",
        absent: &["): string {"],
        present: &[],
    },
    Row {
        hazard: "a script-global var can be written by a setTimeout string",
        source: "var mode = \"idle\";\nsetTimeout(\"mode = 1\", 0);\nfunction show() {\n  console.log(mode);\n}\nshow();\n",
        absent: &["mode: "],
        present: &[],
    },
    Row {
        hazard: "a local class named Promise shadows the type",
        source: "(function () {\n  class Promise {}\n  async function f() {\n    return 1;\n  }\n  f().then(console.log);\n  console.log(new Promise());\n})();\n",
        absent: &["Promise<"],
        present: &[],
    },
    Row {
        hazard: "a function reached by new returns its this object",
        source: "(function () {\n  function F() {\n    return 1;\n  }\n  console.log(new F());\n})();\n",
        absent: &["F(): "],
        present: &[],
    },
    Row {
        hazard: "a getter is never given a return type",
        source: "(function () {\n  const o = {\n    get v() {\n      return 1;\n    },\n  };\n  console.log(o.v);\n})();\n",
        absent: &["v(): "],
        present: &[],
    },
    Row {
        hazard: "a generator returns an iterator",
        source: "(function () {\n  function* g() {\n    return 1;\n  }\n  console.log(g().next());\n})();\n",
        absent: &["g(): "],
        present: &[],
    },
    Row {
        hazard: "a destructuring write is unknown",
        source: "(function () {\n  let a = 1;\n  [a] = [\"s\"];\n  console.log(a);\n})();\n",
        absent: &["a: "],
        present: &[],
    },
    Row {
        hazard: "a for-of write is unknown",
        source: "(function () {\n  let a = 1;\n  for (a of [\"s\"]) {}\n  console.log(a);\n})();\n",
        absent: &["a: "],
        present: &[],
    },
    Row {
        hazard: "a var read before its declaration holds undefined",
        source: "(function () {\n  console.log(early);\n  var early = 1;\n  console.log(early);\n})();\n",
        absent: &["early: 1 =", "early: number ="],
        present: &["1 | undefined"],
    },
    Row {
        hazard: "a callback's parameter has unknown callers",
        source: "(function () {\n  [1, 2].forEach(function (x) {\n    console.log(x);\n  });\n})();\n",
        absent: &["x: "],
        present: &[],
    },
    Row {
        hazard: "a spread call hides the argument positions",
        source: "(function () {\n  function f(a, b) {\n    return a;\n  }\n  console.log(f(...[1, 2]));\n})();\n",
        absent: &["a: ", "b: "],
        present: &[],
    },
    Row {
        hazard: "a literal compared with a value outside its set would be narrowed by the checker",
        source: "(function () {\n  let state = \"idle\";\n  function go() {\n    state = \"run\";\n  }\n  go();\n  if (state === \"run\") {\n    console.log(1);\n  }\n})();\n",
        absent: &["\"idle\" | \"run\""],
        present: &[],
    },
    Row {
        hazard: "an exported binding can be read by an importer the stage never sees",
        source: "export let count = 1;\nexport function bump() {\n  count += 1;\n}\n",
        absent: &["count: "],
        present: &[],
    },
];

#[test]
fn every_hazard_row_keeps_the_risky_position_bare() {
    let mut failures = Vec::new();
    for row in ROWS {
        for mode in [Mode::Ts, Mode::Jsdoc] {
            let out = annotate(row.source, mode).code;
            let typed: String = out
                .lines()
                .filter(|line| !row.source.lines().any(|plain| plain == *line))
                .collect::<Vec<_>>()
                .join("\n");
            for needle in row.absent {
                let hit = match mode {
                    Mode::Ts => typed.contains(needle),
                    Mode::Jsdoc => typed.contains(needle.trim_end_matches(": ")) && typed.contains("/**"),
                };
                if hit {
                    failures.push(format!("{} ({mode:?}): found {needle:?} in\n{out}", row.hazard));
                }
            }
            for needle in row.present {
                if !out.contains(needle) {
                    failures.push(format!("{} ({mode:?}): missing {needle:?} in\n{out}", row.hazard));
                }
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n\n"));
}

#[test]
fn eval_and_with_refuse_the_whole_file() {
    let eval = annotate("(function () {\n  eval(\"\");\n})();\n", Mode::Ts);
    assert_eq!(eval.refused, Some(Refusal::Hazard("direct eval")));
    let with = annotate("(function (o) {\n  with (o) {\n  }\n})({});\n", Mode::Ts);
    assert_eq!(with.refused, Some(Refusal::Hazard("with statement")));
}
