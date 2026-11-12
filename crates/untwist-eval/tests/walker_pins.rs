mod common;

use oxc_allocator::Allocator;
use untwist_core::parse::parse_program;
use untwist_eval::budget::{heap_budget, Budget};
use untwist_eval::decode::{Evaluator, InterpEvaluator};
use untwist_eval::heap::HeapLedger;
use untwist_eval::interp::eval_script_in;
use untwist_eval::test_hooks::{record_units, UnitKind, UnitRecord};

fn only(mut records: Vec<UnitRecord>) -> UnitRecord {
    assert_eq!(records.len(), 1, "{records:?}");
    records.remove(0)
}

fn script(source: &str) -> UnitRecord {
    let alloc = Allocator::default();
    let parsed = parse_program(&alloc, source);
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    let record = only(record_units(|| {
        eval_script_in(&parsed.program, Budget::default(), |_, _| ());
    }));
    assert_eq!(record.kind, UnitKind::Script);
    record
}

fn decode_unit(source: &str, heap: bool) -> UnitRecord {
    let alloc = Allocator::default();
    let parsed = parse_program(&alloc, source);
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    let program = common::decoder_program(&parsed.program, "arr", &["dec"], None);
    let calls = common::decoder_calls_anywhere(&parsed.program, &["dec"]);
    let mut evaluator = InterpEvaluator::new(Budget::default());
    if heap {
        evaluator.heap = Some(HeapLedger::new(heap_budget(source.len())));
    }
    let record = only(record_units(|| {
        let _ = evaluator.decode(&program, &calls);
    }));
    assert_eq!(record.kind, UnitKind::Decode);
    record
}

struct Pin {
    values: &'static [&'static str],
    bail: Option<&'static str>,
    steps: u64,
    heap: (u64, u64),
}

fn assert_pinned(label: &str, record: &UnitRecord, pin: &Pin) {
    let values: Vec<&str> = record.values.iter().map(String::as_str).collect();
    assert_eq!(values, pin.values, "{label}: {record:?}");
    assert_eq!(record.bail.as_deref(), pin.bail, "{label}: {record:?}");
    assert_eq!(record.steps_used, pin.steps, "{label}: {record:?}");
    assert_eq!(
        (record.heap_peak, record.heap_final),
        pin.heap,
        "{label}: {record:?}"
    );
    assert!(record.notes.is_empty(), "{label}: {record:?}");
}

const NO_HEAP: (u64, u64) = (0, 0);
const NESTING: Option<&str> = Some("nesting depth cap exceeded");
const CHAIN: Option<&str> = Some("scope chain too deep");

fn frames(count: usize, parens: usize, leaf: usize) -> String {
    format!(
        "function f(n) {{ if (n === 0) return {}0; return {}f(n - 1); }}\nf({});\n",
        "~".repeat(leaf),
        "~".repeat(parens),
        count - 1
    )
}

fn blocks(depth: usize) -> String {
    let mut source = String::from("var r = 0;\n");
    for index in 0..depth {
        source.push_str(&format!("{{ let a{index} = {index};\n"));
    }
    source.push_str("r = Math.PI;\n");
    source.push_str(&"}\n".repeat(depth));
    source.push_str("r;\n");
    source
}

fn capture_unit(body: &str) -> String {
    format!(
        "function arr() {{ return ['a', 'b']; }}\nfunction dec(i) {{ var keep; {body} return arr()[i] + typeof keep; }}\ndec(0);\n"
    )
}

fn heap_unit(body: &str) -> String {
    format!(
        "function arr() {{ var h = ['x']; arr = function () {{ return h; }}; return arr(); }}\n\
         function dec(c, d) {{\nvar e = arr();\nreturn (dec = function (f, g) {{\nf = f - 0;\nvar i = e[f];\n{body}\n}}), dec(c, d);\n}}\n\
         console.log(dec(0));\n"
    )
}

fn deep_vars(vars: usize, depth: usize) -> String {
    let names: Vec<String> = (0..vars).map(|n| format!("v{n}")).collect();
    heap_unit(&format!(
        "function r(n) {{ var {}; if (n > 0) return r(n - 1); return 0; }} r({depth}); return 'done' + i;",
        names.join(",")
    ))
}

fn wide_function(vars: usize) -> String {
    let names: Vec<String> = (0..vars).map(|n| format!("v{n}")).collect();
    format!(
        "function f() {{ var {}; v{} = 'last'; return v{}; }}\nf();\n",
        names.join(", "),
        vars - 1,
        vars - 1
    )
}

const DEFAULT_READS_LATER_PARAMETER: &str =
    "var b = 'outer';\nfunction f(a = b, b) { return a; }\nf();\n";
const DEFAULT_NAMES_BODY_VAR: &str =
    "var x = 'outer';\nfunction f(a = x) { var x = 1; return a; }\nf();\n";
const DEFAULT_READS_ARGUMENTS: &str = "function f(a = arguments) { return typeof a; }\nf();\n";
const EARLY_LET_OUTSIDE_DECODING: &str =
    "var t = 'outer';\nfunction f() { var seen = t; let t = 'inner'; return seen; }\nf();\n";
const VAR_INITIALISER_INSIDE_CATCH: &str = "function f() { var inner; try { throw 'thrown'; } catch (e) { var e = 'assigned'; inner = e; } return inner + ',' + e; }\nf();\n";

const PER_FRAME_DEPTH: usize = 98;
const LEAF_UNDER_THE_CAP: usize = 196;

#[test]
fn the_walker_bails_nesting_depth_in_the_last_of_999_frames() {
    let record = script(&frames(999, PER_FRAME_DEPTH, LEAF_UNDER_THE_CAP + 1));
    let pin = Pin {
        values: &[],
        bail: NESTING,
        steps: 110_986,
        heap: NO_HEAP,
    };
    assert_pinned("999 frames", &record, &pin);
    let one_frame_less = script(&frames(998, PER_FRAME_DEPTH, LEAF_UNDER_THE_CAP + 1));
    let pin = Pin {
        values: &["number:bff0000000000000"],
        bail: None,
        steps: 208_779,
        heap: NO_HEAP,
    };
    assert_pinned("at 998 frames, node -1", &one_frame_less, &pin);
}

#[test]
fn the_same_shape_one_level_shallower_does_not_bail() {
    let record = script(&frames(999, PER_FRAME_DEPTH, LEAF_UNDER_THE_CAP));
    let pin = Pin {
        values: &["number:0000000000000000"],
        bail: None,
        steps: 208_986,
        heap: NO_HEAP,
    };
    assert_pinned("one level shallower, node 0", &record, &pin);
}

#[test]
fn a_global_read_257_scopes_deep_bails_at_the_read() {
    let under = script(&blocks(255));
    let pin = Pin {
        values: &["number:400921fb54442d18"],
        bail: None,
        steps: 773,
        heap: NO_HEAP,
    };
    assert_pinned("at 256 scopes, node 3.141592653589793", &under, &pin);
    let over = script(&blocks(256));
    let pin = Pin {
        values: &[],
        bail: CHAIN,
        steps: 774,
        heap: NO_HEAP,
    };
    assert_pinned("at 257 scopes, node 3.141592653589793", &over, &pin);
}

fn captured(label: &str, body: &str, steps: u64, heap: (u64, u64)) {
    let record = decode_unit(&capture_unit(body), true);
    let pin = Pin {
        values: &["string:afunction"],
        bail: None,
        steps,
        heap,
    };
    assert_pinned(label, &record, &pin);
}

#[test]
fn to_h5_the_ledger_after_a_closure_made_inside_each_scope_kind() {
    captured(
        "for (var ...), node afunction",
        "for (var k = 0; k < 1; k++) { keep = function () { return k; }; }",
        40,
        (4_256, 114),
    );
    captured(
        "switch, node afunction",
        "switch (i) { case 0: keep = function () { return i; }; }",
        31,
        (4_208, 114),
    );
    captured(
        "catch, node afunction",
        "try { throw 0; } catch (x) { keep = function () { return x; }; }",
        31,
        (4_256, 114),
    );
    captured(
        "a named function expression, node afunction",
        "keep = (function named() { return function () { return named; }; })();",
        32,
        (4_832, 114),
    );
    let control = decode_unit(&capture_unit(""), true);
    let pin = Pin {
        values: &["string:aundefined"],
        bail: None,
        steps: 25,
        heap: (3_344, 116),
    };
    assert_pinned("control, no closure", &control, &pin);
}

#[test]
#[cfg_attr(
    debug_assertions,
    ignore = "the high preset's rotation search is release-only"
)]
fn the_high_presets_session_ledger_peak_and_final() {
    let alloc = Allocator::default();
    let source = common::read_fixture("obfuscator.io-high.js");
    let parsed = parse_program(&alloc, &source);
    let program = common::decoder_program(&parsed.program, "_0x4d6b", &["_0x4c3e"], Some(0));
    let mut evaluator = InterpEvaluator::new(Budget::default());
    evaluator.heap = Some(HeapLedger::new(heap_budget(source.len())));
    let record = only(record_units(|| {
        let _ = evaluator.decode(&program, &[]);
    }));
    let pin = Pin {
        values: &[],
        bail: None,
        steps: 21_383_392,
        heap: (198_036, 0),
    };
    assert_pinned("high preset session", &record, &pin);
    assert_eq!(record.memo_inserts, 2_024, "{record:?}");
}

fn walker_and_node(label: &str, source: &str, walker: Pin, node: &str, divergence: Option<&str>) {
    let record = script(source);
    assert_pinned(label, &record, &walker);
    let agrees = walker
        .values
        .first()
        .is_some_and(|value| value.strip_prefix("string:") == Some(node.trim_matches('"')));
    assert_eq!(agrees, divergence.is_none(), "{label}: {divergence:?}");
}

#[test]
fn a_default_that_reads_a_later_parameter_is_refused() {
    let walker = Pin {
        values: &[],
        bail: Some("unsupported construct: a parameter default that reads its own parameter, a later one or a binding of the function body"),
        steps: 5,
        heap: NO_HEAP,
    };
    walker_and_node(
        "default reads a later parameter",
        DEFAULT_READS_LATER_PARAMETER,
        walker,
        "ReferenceError: Cannot access 'b' before initialization",
        Some("parameters have no dead zone, so a default that reads a later parameter is refused where node throws"),
    );
}

#[test]
fn a_default_that_names_a_body_var_reads_the_outer_binding() {
    let walker = Pin {
        values: &["string:outer"],
        bail: None,
        steps: 11,
        heap: NO_HEAP,
    };
    walker_and_node(
        "default names a body var",
        DEFAULT_NAMES_BODY_VAR,
        walker,
        "\"outer\"",
        None,
    );
}

#[test]
fn a_default_that_reads_arguments_reads_the_arguments_object() {
    let walker = Pin {
        values: &["string:object"],
        bail: None,
        steps: 9,
        heap: NO_HEAP,
    };
    walker_and_node(
        "default reads arguments",
        DEFAULT_READS_ARGUMENTS,
        walker,
        "\"object\"",
        None,
    );
}

#[test]
fn an_early_let_read_outside_decoding_reads_the_outer_binding() {
    let walker = Pin {
        values: &["string:outer"],
        bail: None,
        steps: 12,
        heap: NO_HEAP,
    };
    walker_and_node(
        "early let outside decoding",
        EARLY_LET_OUTSIDE_DECODING,
        walker,
        "ReferenceError: Cannot access 't' before initialization",
        Some("outside decoding the walker has no TDZ: the let is absent until it runs, so the read resolves outward"),
    );
}

#[test]
fn a_var_initialiser_inside_catch_writes_the_catch_parameter() {
    let walker = Pin {
        values: &["string:assigned,undefined"],
        bail: None,
        steps: 23,
        heap: NO_HEAP,
    };
    walker_and_node(
        "var initialiser inside catch",
        VAR_INITIALISER_INSIDE_CATCH,
        walker,
        "\"assigned,undefined\"",
        None,
    );
}

#[test]
fn a_function_with_70000_vars_reads_its_last() {
    let record = script(&wide_function(70_000));
    let pin = Pin {
        values: &["string:last"],
        bail: None,
        steps: 10,
        heap: NO_HEAP,
    };
    assert_pinned("70000 vars, node last", &record, &pin);
}

#[test]
fn the_var_100000_heap_row_stops_where_the_cli_heap_row_4c_h_recursive_var_100000_notes_its_budget()
{
    let record = decode_unit(&deep_vars(100_000, 900), true);
    let pin = Pin {
        values: &["unevaluated"],
        bail: None,
        steps: 242,
        heap: (67_208_832, 0),
    };
    assert_pinned("var-100000 recursive", &record, &pin);
    let small = decode_unit(&deep_vars(100_000, 0), true);
    let pin = Pin {
        values: &["string:donex"],
        bail: None,
        steps: 61,
        heap: (4_805_088, 106),
    };
    assert_pinned("var-100000 alone", &small, &pin);
}

const ENTRY_TOKENS: [&str; 19] = [
    "eval_expression(",
    "eval_statement(",
    "hoist(",
    "declare(",
    "declare_if_absent(",
    "declare_immutable(",
    "lookup(",
    "Env<'",
    "make_closure(",
    "global_env(",
    "new_env(",
    "track_scope(",
    "Scope<'",
    "interp.call(",
    "FrameRef<'",
    "Frame<'",
    "synthetic_frame(",
    "compile_in(",
    "eval_leaf(",
];

const ATTEMPTS: &str = "session entry: the checksum attempts over the frame the entries open";
const COMPILED_ENTRIES_ROLE: &str =
    "session entry, compiled: the synthetic checksum frame, the rotator and arguments at the root, the direct literal path";
const DIRECT_ENTRIES: &str =
    "session entry, direct: the checksum on the frame model's declarations and hoist with node-free terms";
const ROOT_RUNNERS: &str =
    "the compiled evaluator: a statement or an expression run at the program root";
const INTERPRETER: &str = "the interpreter: the root frame, its hoist and the closures made there";
const CALL_PATH: &str = "the call path: a compiled frame's arguments object";
const NAME_READS: &str = "session entry: name reads into closure scopes";
const HOLDER: &str = "holder of an environment";
const REALM: &str = "the realm: scope registry, install, new_env";
const SHARED: &str = "shared value-level semantics";
const UNRELATED: &str = "unrelated: a function of the same name";
const UNRELATED_SCOPE: &str = "unrelated: the recipe compiler's own private Scope";
const UNRELATED_FRAME: &str = "unrelated: the state collector's own private Frame";
const COMPILED: &str =
    "the compiled evaluator: fixed frames, resolved reads and writes, closures, calls";
const COMPILED_ROW: &str = "the compiler's own unit row: a frame for a key it evaluates";
const FRAME_API: &str = "the frame model's own by-name declarations";
const ROOT_DECLARATIONS: &str =
    "the compiled evaluator: a top-level let and a var bound by name on the root, as the walker binds them";
const SESSION: &str =
    "session entry: the root env, the machinery closures, the string-array and decoder calls";
const NATIVE_LOOPS: &str =
    "native decoder loops: a compiled frame's slots, read before the loop and written back after it";

const INVENTORY: [(&str, &str, usize, &str); 76] = [
    ("builtins/charged.rs", "FrameRef<'", 3, HOLDER),
    ("builtins/mod.rs", "FrameRef<'", 7, REALM),
    ("builtins/mod.rs", "Frame<'", 1, REALM),
    ("builtins/mod.rs", "declare(", 1, REALM),
    ("builtins/mod.rs", "lookup(", 1, UNRELATED),
    ("builtins/mod.rs", "new_env(", 1, REALM),
    ("builtins/mod.rs", "track_scope(", 4, REALM),
    ("builtins/shape.rs", "lookup(", 1, UNRELATED),
    ("compile/assign.rs", "FrameRef<'", 3, COMPILED),
    ("compile/block.rs", "FrameRef<'", 4, COMPILED),
    ("compile/block.rs", "make_closure(", 1, COMPILED),
    ("compile/call.rs", "FrameRef<'", 7, COMPILED),
    ("compile/call.rs", "make_closure(", 1, COMPILED),
    ("compile/declare.rs", "FrameRef<'", 3, COMPILED),
    ("compile/depth.rs", "FrameRef<'", 2, COMPILED),
    ("compile/declare.rs", "declare(", 4, ROOT_DECLARATIONS),
    ("compile/entry.rs", "FrameRef<'", 14, COMPILED_ENTRIES_ROLE),
    ("compile/entry.rs", "compile_in(", 1, COMPILED_ENTRIES_ROLE),
    ("compile/entry.rs", "declare(", 2, DIRECT_ENTRIES),
    ("compile/entry.rs", "global_env(", 1, COMPILED_ROW),
    ("compile/entry.rs", "interp.call(", 1, DIRECT_ENTRIES),
    (
        "compile/entry.rs",
        "make_closure(",
        1,
        COMPILED_ENTRIES_ROLE,
    ),
    ("compile/expr.rs", "FrameRef<'", 11, COMPILED),
    ("compile/expr.rs", "global_env(", 1, COMPILED_ROW),
    ("compile/expr.rs", "interp.call(", 1, COMPILED),
    ("compile/expr.rs", "lookup(", 1, COMPILED),
    ("compile/expr.rs", "make_closure(", 2, COMPILED),
    ("compile/literal.rs", "FrameRef<'", 1, COMPILED),
    ("compile/loops.rs", "FrameRef<'", 20, COMPILED),
    ("compile/mod.rs", "FrameRef<'", 19, COMPILED),
    ("compile/program.rs", "FrameRef<'", 1, COMPILED),
    ("compile/program.rs", "global_env(", 2, ROOT_RUNNERS),
    ("compile/scope.rs", "Frame<'", 2, COMPILED),
    ("compile/scope.rs", "FrameRef<'", 11, COMPILED),
    ("compile/scope.rs", "lookup(", 2, COMPILED),
    ("compile/stmt.rs", "FrameRef<'", 3, COMPILED),
    ("decode/dead_terms.rs", "FrameRef<'", 13, NAME_READS),
    ("decode/dead_terms.rs", "global_env(", 1, NAME_READS),
    ("decode/entries.rs", "FrameRef<'", 4, COMPILED_ENTRIES_ROLE),
    ("decode/entries.rs", "compile_in(", 2, COMPILED_ENTRIES_ROLE),
    ("decode/entries.rs", "declare(", 2, DIRECT_ENTRIES),
    ("decode/entries.rs", "eval_leaf(", 1, COMPILED_ENTRIES_ROLE),
    ("decode/entries.rs", "hoist(", 1, DIRECT_ENTRIES),
    ("decode/entries.rs", "new_env(", 1, DIRECT_ENTRIES),
    (
        "decode/entries.rs",
        "synthetic_frame(",
        1,
        COMPILED_ENTRIES_ROLE,
    ),
    ("decode/mod.rs", "FrameRef<'", 5, ATTEMPTS),
    ("decode/mod.rs", "declare(", 2, SESSION),
    ("decode/mod.rs", "global_env(", 1, SESSION),
    ("decode/mod.rs", "interp.call(", 2, SESSION),
    ("decode/mod.rs", "make_closure(", 2, SESSION),
    ("frame.rs", "declare(", 1, FRAME_API),
    ("frame.rs", "declare_if_absent(", 1, FRAME_API),
    ("frame.rs", "declare_immutable(", 1, FRAME_API),
    ("interp/call.rs", "FrameRef<'", 3, CALL_PATH),
    ("interp/call.rs", "declare(", 2, CALL_PATH),
    ("interp/call.rs", "declare_if_absent(", 1, CALL_PATH),
    ("interp/mod.rs", "FrameRef<'", 9, INTERPRETER),
    ("interp/mod.rs", "declare(", 1, INTERPRETER),
    ("interp/mod.rs", "declare_if_absent(", 1, INTERPRETER),
    ("interp/mod.rs", "global_env(", 2, INTERPRETER),
    ("interp/mod.rs", "hoist(", 2, INTERPRETER),
    ("interp/mod.rs", "make_closure(", 2, INTERPRETER),
    ("interp/mod.rs", "new_env(", 1, INTERPRETER),
    ("native_body/base64.rs", "FrameRef<'", 6, NATIVE_LOOPS),
    ("native_body/exec.rs", "FrameRef<'", 7, NATIVE_LOOPS),
    ("native_body/mod.rs", "FrameRef<'", 1, NATIVE_LOOPS),
    ("native_body/rc4.rs", "FrameRef<'", 7, NATIVE_LOOPS),
    ("release.rs", "FrameRef<'", 2, HOLDER),
    ("release.rs", "Frame<'", 3, HOLDER),
    ("semantics/assign.rs", "FrameRef<'", 1, SHARED),
    ("semantics/assign.rs", "declare(", 1, SHARED),
    ("state/collect/judge.rs", "Frame<'", 1, UNRELATED_FRAME),
    ("state/collect/scope.rs", "Frame<'", 1, UNRELATED_FRAME),
    ("state/recipe.rs", "Scope<'", 2, UNRELATED_SCOPE),
    ("state/registry.rs", "lookup(", 4, UNRELATED),
    ("value.rs", "FrameRef<'", 2, HOLDER),
];

const FRAME_TYPES: [&str; 2] = ["FrameRef<'", "Frame<'"];

fn sources(dir: &std::path::Path, out: &mut Vec<std::path::PathBuf>) {
    let mut entries: Vec<_> = std::fs::read_dir(dir)
        .expect("read the source tree")
        .map(|entry| entry.expect("a directory entry").path())
        .collect();
    entries.sort();
    for path in entries {
        if path.is_dir() {
            sources(&path, out);
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            out.push(path);
        }
    }
}

fn occurrences(text: &str, token: &str) -> usize {
    text.match_indices(token)
        .filter(|(at, _)| {
            text[..*at]
                .chars()
                .next_back()
                .is_none_or(|before| !(before.is_alphanumeric() || before == '_'))
        })
        .count()
}

fn found_entry_points() -> Vec<(String, &'static str, usize)> {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut files = Vec::new();
    sources(&root, &mut files);
    let mut found = Vec::new();
    for file in files {
        let relative = file
            .strip_prefix(&root)
            .expect("under src")
            .to_string_lossy()
            .replace('\\', "/");
        if relative == "env.rs" {
            continue;
        }
        let text = std::fs::read_to_string(&file).expect("read a source file");
        for token in ENTRY_TOKENS {
            if relative == "frame.rs" && FRAME_TYPES.contains(&token) {
                continue;
            }
            let count = occurrences(&text, token);
            if count > 0 {
                found.push((relative.clone(), token, count));
            }
        }
    }
    found.sort();
    found
}

#[test]
fn every_evaluator_entry_point_and_environment_holder_is_inventoried() {
    let mut expected: Vec<(String, &str, usize)> = INVENTORY
        .iter()
        .map(|(file, token, count, _)| ((*file).to_string(), *token, *count))
        .collect();
    expected.sort();
    assert_eq!(found_entry_points(), expected);
    assert!(INVENTORY.iter().all(|(_, _, _, role)| !role.is_empty()));
}

const WALKER_ENTRIES: [&str; 5] = [
    "eval_expression(",
    "eval_statement(",
    "hoist(",
    "new_env(",
    "lookup(",
];
const COMPILED_ENTRIES: [&str; 3] = ["synthetic_frame(", "compile_in(", "eval_leaf("];

fn decode_source(file: &str) -> String {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("src/decode")
        .join(file);
    std::fs::read_to_string(path).unwrap_or_default()
}

#[test]
fn the_session_entry_points_run_compiled_with_no_walker_entry_left() {
    for file in ["mod.rs", "dead_terms.rs"] {
        let text = decode_source(file);
        let walker: Vec<&str> = WALKER_ENTRIES
            .into_iter()
            .filter(|token| occurrences(&text, token) > 0)
            .collect();
        assert!(
            walker.is_empty(),
            "decode/{file} still enters the walker: {walker:?}"
        );
    }
    let entries = decode_source("entries.rs");
    for token in COMPILED_ENTRIES {
        assert!(
            occurrences(&entries, token) > 0,
            "decode/entries.rs: {token}"
        );
    }
}
