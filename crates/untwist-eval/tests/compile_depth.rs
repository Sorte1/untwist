mod common;

use std::ffi::OsString;

use oxc_allocator::Allocator;
use untwist_core::parse::parse_program;
use untwist_eval::budget::{Budget, NESTING_DEPTH_CAP};
use untwist_eval::decode::{Evaluator, InterpEvaluator};
use untwist_eval::interp::eval_script_in;
use untwist_eval::test_hooks::{
    compile_census, nesting_at_first_bail, record_units, CompileCensus, UnitRecord,
    UNCHECKED_LEVELS,
};
use untwist_testing::capped_child::{self, Cap};

const CHILD: &str = "UNTWIST_COMPILE_DEPTH_CHILD";
const PARSE_STACK: usize = 1 << 30;
const SEGMENTS: [(&str, usize); 3] = [("1g", 1 << 30), ("16m", 16 << 20), ("2m", 2 << 20)];
const CEILING: Cap = Cap {
    memory_max: "4G",
    timeout_s: 300,
};
const NESTING: &str = "nesting depth cap exceeded";
const FRAMES: usize = 999;
const PER_FRAME_DEPTH: usize = 98;
const LEAF_UNDER_THE_CAP: usize = 196;
const PLUS_TERMS: usize = 100_000;
const CONDITIONALS: usize = 20_000;
const ELSE_IFS: usize = 20_000;
const ARRAY: &str = "function a() { var h = ['4', '17', '1', '2', '3', '5a', '9']; a = function () { return h; }; return a(); }";

#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    Script,
    Session,
}

fn pinned_frames(count: usize, leaf: usize) -> String {
    format!(
        "function f(n) {{ if (n === 0) return {}0; return {}f(n - 1); }}\nf({});\n",
        "~".repeat(leaf),
        "~".repeat(PER_FRAME_DEPTH),
        count - 1
    )
}

const BLOCKS_TO_THE_CAP: usize = 80;

fn at_the_limit(over: usize, frames: usize, blocks: usize) -> String {
    let tildes = UNCHECKED_LEVELS as usize - 5 + over;
    format!(
        "function f(n) {{ {}return n === 0 ? 0 : {}f(n - 1);{} }}\nf({});\n",
        "{ ".repeat(blocks),
        "~".repeat(tildes),
        " }".repeat(blocks),
        frames - 1
    )
}

fn frames_to_the_cap() -> usize {
    let per_frame = BLOCKS_TO_THE_CAP + UNCHECKED_LEVELS as usize - 2;
    NESTING_DEPTH_CAP as usize / per_frame + 2
}

fn plus_chain() -> String {
    format!(
        "{ARRAY}\nfunction b(i) {{ i = i - 0x10; return a()[i]{}; }}\nb(0x10);\n",
        " + i".repeat(PLUS_TERMS)
    )
}

fn conditional_body() -> String {
    format!(
        "var c = 1; return {}'deep'{};",
        "c ? ".repeat(CONDITIONALS),
        " : 0".repeat(CONDITIONALS)
    )
}

fn else_if_body() -> String {
    let arms: String = (0..ELSE_IFS)
        .map(|index| format!("if (x === {index}) return {index}; else "))
        .collect();
    format!("var x = {}; {arms}return 'none';", ELSE_IFS - 1)
}

fn plus_body() -> String {
    format!("var i = 1; return i{};", " + i".repeat(PLUS_TERMS))
}

fn top_level(body: &str) -> String {
    format!("(function () {{ {body} }})();\n")
}

fn in_recursion(body: &str) -> String {
    format!(
        "function g() {{ {body} }}\nfunction f(n) {{ if (n > 0) return f(n - 1); return g(); }}\nf({});\n",
        FRAMES - 1
    )
}

fn row(name: &str) -> Option<(Kind, String)> {
    let script = |source: String| Some((Kind::Script, source));
    match name {
        "n1" => script(pinned_frames(FRAMES, LEAF_UNDER_THE_CAP + 1)),
        "n1_998" => script(pinned_frames(FRAMES - 1, LEAF_UNDER_THE_CAP + 1)),
        "n2" => script(pinned_frames(FRAMES, LEAF_UNDER_THE_CAP)),
        "d1_at_the_limit" => script(at_the_limit(0, FRAMES, 0)),
        "d1_one_past" => script(at_the_limit(1, FRAMES, 0)),
        "d1_to_the_cap" => script(at_the_limit(0, frames_to_the_cap(), BLOCKS_TO_THE_CAP)),
        "d2" => Some((Kind::Session, plus_chain())),
        "d3" => script(top_level(&conditional_body())),
        "d5" => script(top_level(&else_if_body())),
        "d6_plus" => script(in_recursion(&plus_body())),
        "d6_conditional" => script(in_recursion(&conditional_body())),
        "d6_else_if" => script(in_recursion(&else_if_body())),
        "d6_top_plus" => script(top_level(&plus_body())),
        _ => None,
    }
}

const ROWS: &[&str] = &[
    "n1",
    "n1_998",
    "n2",
    "d1_at_the_limit",
    "d1_one_past",
    "d1_to_the_cap",
    "d2",
    "d3",
    "d5",
    "d6_plus",
    "d6_conditional",
    "d6_else_if",
    "d6_top_plus",
];

fn evaluate(kind: Kind, program: &oxc_ast::ast::Program<'_>) {
    match kind {
        Kind::Script => eval_script_in(program, Budget::new(50_000_000), |_, _| ()),
        Kind::Session => {
            let decoders = common::decoder_program(program, "a", &["b"], None);
            let calls = common::calls_for(program, 0);
            let mut evaluator = InterpEvaluator::new(Budget::default());
            let _ = evaluator.decode(&decoders, &calls);
        }
    }
}

fn recorded(
    kind: Kind,
    program: &oxc_ast::ast::Program<'_>,
) -> (Vec<UnitRecord>, Option<u32>, CompileCensus) {
    let mut records = Vec::new();
    let mut census = CompileCensus::default();
    let nesting = nesting_at_first_bail(|| {
        census = compile_census(|| records = record_units(|| evaluate(kind, program)));
    });
    (records, nesting, census)
}

#[test]
fn compile_depth_child() {
    let Ok(request) = std::env::var(CHILD) else {
        return;
    };
    let (name, segment) = request.split_once(' ').unwrap_or((request.as_str(), "1g"));
    let (kind, source) = row(name).expect("a known row");
    let bytes = SEGMENTS
        .iter()
        .find(|(label, _)| *label == segment)
        .map_or(1 << 30, |(_, bytes)| *bytes);
    let alloc = Allocator::default();
    let parsed = untwist_eval::stack::grow(PARSE_STACK, || parse_program(&alloc, &source));
    assert!(parsed.diagnostics.is_empty());
    let program = &parsed.program;
    let (records, nesting, census) = untwist_eval::stack::grow(bytes, || recorded(kind, program));
    for record in &records {
        println!(
            "compiled {:?} {:?} steps {} nesting {nesting:?}",
            record.values, record.bail, record.steps_used
        );
    }
    println!(
        "census calls {} checked {} unchecked {} overruns {}",
        census.calls, census.checked_units, census.unchecked_units, census.depth_overruns
    );
}

struct Seen {
    compiled: Vec<String>,
    census: Vec<(String, u64)>,
}

impl Seen {
    fn count(&self, what: &str) -> u64 {
        self.census
            .iter()
            .find(|(name, _)| name == what)
            .map_or(u64::MAX, |(_, count)| *count)
    }
}

fn child(name: &str, segment: &str) -> Result<Seen, String> {
    let exe = std::env::current_exe().map_err(|error| error.to_string())?;
    let args: Vec<OsString> = ["compile_depth_child", "--exact", "--nocapture"]
        .into_iter()
        .map(OsString::from)
        .collect();
    let env = [(
        OsString::from(CHILD),
        OsString::from(format!("{name} {segment}")),
    )];
    let outcome = capped_child::run(exe.as_os_str(), &args, &env, None, CEILING);
    if outcome.exit_code != Some(0) || outcome.signal.is_some() {
        return Err(format!("{name} on {segment}: {outcome:?}"));
    }
    let stdout = String::from_utf8_lossy(&outcome.stdout).into_owned();
    let tier = |prefix: &str| -> Vec<String> {
        stdout
            .lines()
            .filter_map(|line| line.strip_prefix(prefix))
            .map(str::to_string)
            .collect()
    };
    let census = stdout
        .lines()
        .find_map(|line| line.strip_prefix("census "))
        .unwrap_or_default()
        .split(' ')
        .collect::<Vec<_>>()
        .chunks(2)
        .filter_map(|pair| match pair {
            [name, count] => Some(((*name).to_string(), count.parse().ok()?)),
            _ => None,
        })
        .collect();
    Ok(Seen {
        compiled: tier("compiled "),
        census,
    })
}

const PINS: &[(&str, &str)] = &[
    (
        "n1",
        "[] Some(\"nesting depth cap exceeded\") steps 110986 nesting Some(100000)",
    ),
    (
        "n1_998",
        "[\"number:bff0000000000000\"] None steps 208779 nesting None",
    ),
    (
        "n2",
        "[\"number:0000000000000000\"] None steps 208986 nesting None",
    ),
    (
        "d2",
        "[] Some(\"nesting depth cap exceeded\") steps 100027 nesting Some(100000)",
    ),
    ("d3", "[\"string:deep\"] None steps 40007 nesting None"),
    (
        "d5",
        "[\"number:40d387c000000000\"] None steps 100007 nesting None",
    ),
    (
        "d6_plus",
        "[] Some(\"nesting depth cap exceeded\") steps 109994 nesting Some(100000)",
    ),
    (
        "d6_conditional",
        "[\"string:deep\"] None steps 52993 nesting None",
    ),
    (
        "d6_else_if",
        "[\"number:40d387c000000000\"] None steps 112993 nesting None",
    ),
    (
        "d6_top_plus",
        "[] Some(\"nesting depth cap exceeded\") steps 100004 nesting Some(100000)",
    ),
];

fn expected(name: &str, segment: &str, seen: &Seen) -> Result<(), String> {
    untwist_eval::test_hooks::frozen(
        "compile_depth",
        &format!("{name} {segment}"),
        &seen.compiled,
    );
    let first = seen.compiled.first().cloned().unwrap_or_default();
    let unchecked = seen.count("unchecked");
    let checked = seen.count("checked");
    let held = match name {
        "d1_at_the_limit" if segment == "2m" => unchecked > FRAMES as u64 / 2,
        "d1_at_the_limit" => checked == 0 && unchecked == FRAMES as u64 + 2,
        "d1_one_past" => unchecked < 4 && checked >= FRAMES as u64,
        "d1_to_the_cap" => first.contains(NESTING) && unchecked > FRAMES as u64,
        pinned => PINS
            .iter()
            .any(|(row, pin)| *row == pinned && first == *pin),
    };
    let common = !seen.compiled.is_empty() && seen.count("overruns") == 0;
    match held && common {
        true => Ok(()),
        false => Err(format!(
            "{name}: compiled {:?} census {:?}",
            seen.compiled, seen.census
        )),
    }
}

#[test]
#[cfg_attr(
    debug_assertions,
    ignore = "release-only: the depth rows in capped children on the 1 GiB, 16 MiB and 2 MiB rungs"
)]
fn compile_expression_grows_per_node_and_every_depth_row_holds_on_all_rungs() {
    if std::env::var_os(CHILD).is_some() {
        return;
    }
    let mut failures = Vec::new();
    for name in ROWS {
        for (segment, _) in SEGMENTS {
            match child(name, segment).and_then(|seen| expected(name, segment, &seen)) {
                Ok(()) => {}
                Err(failure) => failures.push(failure),
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

fn fresh_reaches(depth: usize) -> u64 {
    let source = format!("(function (a) {{ return {}a; }})(1);", "~".repeat(depth));
    let alloc = Allocator::default();
    let parsed = parse_program(&alloc, &source);
    assert!(parsed.diagnostics.is_empty());
    let program = &parsed.program;
    compile_census(|| {
        record_units(|| {
            eval_script_in(program, Budget::default(), |_, _| ());
        });
    })
    .fresh_reaches
}

#[test]
fn a_unit_that_fills_its_reach_exactly_takes_no_fresh_reach() {
    let seen: Vec<u64> = (UNCHECKED_LEVELS as usize - 4..UNCHECKED_LEVELS as usize + 4)
        .map(fresh_reaches)
        .collect();
    assert_eq!(seen, [0, 0, 0, 1, 1, 1, 1, 1], "fresh reaches by depth");
}
