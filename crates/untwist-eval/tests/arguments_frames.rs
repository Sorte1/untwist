use std::path::PathBuf;

use oxc_allocator::Allocator;
use oxc_ast::ast::Program;
use untwist_core::parse::parse_program;
use untwist_eval::budget::Budget;
use untwist_eval::interp::{eval_script_decoding, eval_script_in, Interp};
use untwist_eval::test_hooks::{
    compile_census, frozen, record_units_charged, CompileCensus, UnitRecord,
};
use untwist_testing::node;

struct Row {
    name: &'static str,
    definitions: &'static str,
    call: &'static str,
    node: &'static str,
}

const ROWS: &[Row] = &[
    Row {
        name: "a1_a_parameter_write_reads_back_through_arguments_in_a_literal_and_a_template",
        definitions: "function f(a, b) { var o = {first: arguments[0]}; a = 7; o.after = arguments[0]; arguments[1] = 9; return `${o.first},${o.after},${b},${arguments.length}`; }",
        call: "f(1, 2)",
        node: "1,7,9,2",
    },
    Row {
        name: "a2_a_compiled_delete_unmaps_the_slot",
        definitions: "function f(a) { var o = {d: delete arguments[0]}; a = 5; return [o.d, arguments[0], a].join(); }",
        call: "f(1)",
        node: "true,,5",
    },
    Row {
        name: "a3_logical_assignment_through_arguments_and_the_parameter",
        definitions: "function f(a, b) { arguments[0] ||= 4; arguments[1] ??= 6; b &&= 8; return [a, b, arguments[1], {n: arguments.length}.n].join(); }",
        call: "f(0, null)",
        node: "4,8,8,2",
    },
    Row {
        name: "a4_an_escaping_arguments_object_keeps_its_mapping_only_while_a_closure_holds_the_frame",
        definitions: "function f(a) { var g = () => a; return {args: arguments, g: g}; } function h(a) { return {args: arguments}; } function run() { var x = f(1); x.args[0] = 5; var y = h(1); y.args[0] = 6; return [x.g(), x.args[0], y.args[0], typeof y.args].join(); }",
        call: "run()",
        node: "5,5,6,object",
    },
];

fn parsed<R>(source: &str, body: impl FnOnce(&Program<'_>) -> R) -> R {
    let alloc = Allocator::default();
    let parsed = parse_program(&alloc, source);
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    body(&parsed.program)
}

fn records(program: &Program<'_>, decoding: bool) -> Vec<UnitRecord> {
    record_units_charged(|| match decoding {
        true => eval_script_decoding(program, Budget::default(), |_| ()),
        false => eval_script_in(program, Budget::default(), |_, _| ()),
    })
}

fn compared(row: &Row, decoding: bool) -> (Vec<UnitRecord>, CompileCensus) {
    let source = format!("{}\nString({});\n", row.definitions, row.call);
    parsed(&source, |program| {
        let mut compiled = Vec::new();
        let census = compile_census(|| compiled = records(program, decoding));
        frozen(
            "arguments_frames",
            &format!("{}\n{decoding}\n{source}", row.name),
            &compiled,
        );
        (compiled, census)
    })
}

fn printed_by_node(row: &Row) -> Option<String> {
    let Some(binary) = node::node() else {
        let required = std::env::var("UNTWIST_REQUIRE_NODE").ok();
        assert!(
            node::skip_allowed(required.as_deref()),
            "required: {}",
            node::SKIP_MESSAGE
        );
        return None;
    };
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("arguments_frames");
    std::fs::create_dir_all(&dir).expect("temp dir");
    let path = dir.join(format!("{}.js", row.name));
    let program = format!("{}\nconsole.log(String({}));\n", row.definitions, row.call);
    std::fs::write(&path, program).expect("write the node program");
    let observed = node::run(&binary, &path)
        .expect("run node")
        .expect("node finished in time");
    assert_eq!(observed.exit, Some(0), "{}: {}", row.name, observed.stderr);
    Some(observed.stdout.trim_end().to_string())
}

#[test]
fn to_a4_mapped_arguments_alias_compiled_frame_slots_as_node_does() {
    for row in ROWS {
        if let Some(printed) = printed_by_node(row) {
            assert_eq!(
                printed, row.node,
                "{}: node prints something else",
                row.name
            );
        }
        let (units, census) = compared(row, false);
        assert!(census.calls > 0, "{}: no call entered compiled", row.name);
        assert_eq!(
            units[0].values,
            [format!("string:{}", row.node)],
            "{}",
            row.name
        );
        compared(row, true);
    }
}

const TEARDOWN: &str = "function f(a) { a = a + 1; return {v: arguments[0]}.v; }\nvar s = 0;\nfor (var i = 0; i < 10000; i++) { s = s + f(i); }\ns;";

fn live_counts(program: &Program<'_>) -> Vec<(usize, usize)> {
    let mut interp = Interp::new(program.source_text, Budget::default());
    let env = interp.global_env();
    interp.hoist(&program.body, &env).expect("hoists");
    let mut counts = vec![interp.realm.live_tracked()];
    for statement in &program.body {
        interp.run_root_statement(statement).expect("runs");
        counts.push(interp.realm.live_tracked());
    }
    counts
}

#[test]
fn ten_thousand_mapped_arguments_calls_return_to_the_baseline_live_count() {
    parsed(TEARDOWN, |program| {
        let mut compiled = Vec::new();
        let census = compile_census(|| compiled = live_counts(program));
        assert_eq!(census.calls, 10_000, "{census:?}");
        frozen("arguments_frames live counts", TEARDOWN, &compiled);
        let before_the_loop = compiled[2];
        let after_the_loop = compiled[3];
        assert_eq!(
            after_the_loop, before_the_loop,
            "the loop left frames or objects alive: {compiled:?}"
        );
    });
}
