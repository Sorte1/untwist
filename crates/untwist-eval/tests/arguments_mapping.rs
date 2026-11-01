use std::path::PathBuf;

use oxc_allocator::Allocator;
use oxc_ast::ast::{Function, Program, Statement};
use untwist_core::parse::parse_program;
use untwist_core::semantic::program_is_strict;
use untwist_eval::bail::Bail;
use untwist_eval::budget::Budget;
use untwist_eval::decode::{Call, DecoderProgram, FnRef, InterpEvaluator};
use untwist_eval::interp::eval_script_in;
use untwist_eval::value::{AbruptKind, Value};
use untwist_testing::node;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Outcome {
    Node,
    Refused,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Prefix {
    Script,
    StrictScript,
    Module,
}

impl Prefix {
    fn text(self) -> &'static str {
        match self {
            Prefix::Script => "",
            Prefix::StrictScript => "\"use strict\";\n",
            Prefix::Module => "export {};\n",
        }
    }

    fn extension(self) -> &'static str {
        match self {
            Prefix::Module => "mjs",
            _ => "js",
        }
    }
}

struct Row {
    name: &'static str,
    prefix: Prefix,
    definitions: &'static str,
    call: &'static str,
    node: &'static str,
    decoding: Outcome,
    script: Outcome,
}

const fn row(
    name: &'static str,
    definitions: &'static str,
    call: &'static str,
    node: &'static str,
) -> Row {
    Row {
        name,
        prefix: Prefix::Script,
        definitions,
        call,
        node,
        decoding: Outcome::Node,
        script: Outcome::Node,
    }
}

const fn refused_while_decoding(base: Row) -> Row {
    Row {
        decoding: Outcome::Refused,
        ..base
    }
}

const fn refused_everywhere(base: Row) -> Row {
    Row {
        decoding: Outcome::Refused,
        script: Outcome::Refused,
        ..base
    }
}

const fn under(prefix: Prefix, base: Row) -> Row {
    Row { prefix, ..base }
}

const ROWS: &[Row] = &[
    row(
        "mapped_write_param_reads_arguments",
        "function f(a, b) { a = 7; return arguments[0]; }",
        "f(1, 2)",
        "7",
    ),
    row(
        "mapped_write_arguments_reads_param",
        "function f(a, b) { arguments[1] = 9; return b; }",
        "f(1, 2)",
        "9",
    ),
    row(
        "template_subtract_then_capture",
        "function f(a, b) { a = a - 1; var c = arguments; return c[0]; }",
        "f(1, 2)",
        "0",
    ),
    row(
        "strict_function_unmapped",
        "function f(a, b) { \"use strict\"; a = 7; return arguments[0]; }",
        "f(1, 2)",
        "1",
    ),
    under(
        Prefix::StrictScript,
        row(
            "strict_program_unmapped",
            "function f(a, b) { a = 7; return arguments[0]; }",
            "f(1, 2)",
            "1",
        ),
    ),
    row(
        "default_param_unmapped",
        "function f(a, b = 2) { a = 7; return arguments[0]; }",
        "f(1, 2)",
        "1",
    ),
    refused_everywhere(row(
        "rest_param_unmapped",
        "function f(a, ...r) { a = 7; return arguments[0]; }",
        "f(1, 2)",
        "1",
    )),
    refused_everywhere(row(
        "destructured_param_unmapped",
        "function f({x}, b) { b = 7; return arguments[1]; }",
        "f({x: 1}, 2)",
        "2",
    )),
    row(
        "escaping_arguments_still_aliases",
        "function g(o) { o[0] = 5; }\nfunction f(a, b) { g(arguments); return a; }",
        "f(1, 2)",
        "5",
    ),
    row(
        "missing_argument_unmapped",
        "function f(a, b) { b = 7; return arguments[1]; }",
        "f(1)",
        "undefined",
    ),
    refused_everywhere(row(
        "length_change_adds_no_mapping",
        "function f(a, b) { arguments.length = 2; b = 7; return arguments[1]; }",
        "f(1)",
        "undefined",
    )),
    refused_while_decoding(row(
        "delete_breaks_the_mapping",
        "function f(a, b) { delete arguments[0]; a = 7; return arguments[0]; }",
        "f(1, 2)",
        "undefined",
    )),
    refused_while_decoding(row(
        "delete_then_readd_stays_unmapped",
        "function f(a, b) { delete arguments[0]; arguments[0] = 3; a = 7; return arguments[0]; }",
        "f(1, 2)",
        "3",
    )),
    under(
        Prefix::Module,
        Row {
            script: Outcome::Refused,
            ..row(
                "module_code_unmapped",
                "function f(a, b) { a = 7; return arguments[0]; }",
                "f(1, 2)",
                "1",
            )
        },
    ),
    refused_everywhere(row(
        "class_method_unmapped",
        "class K { m(a) { a = 7; return arguments[0]; } }",
        "new K().m(1)",
        "1",
    )),
    refused_everywhere(row(
        "class_expression_static_unmapped",
        "var K = class { static m(a) { a = 7; return arguments[0]; } };",
        "K.m(1)",
        "1",
    )),
    row(
        "nested_function_inherits_strict",
        "function outer() { \"use strict\"; function f(a) { a = 7; return arguments[0]; } return f(1); }",
        "outer()",
        "1",
    ),
    row(
        "duplicate_parameter_maps_the_last",
        "function f(a, a) { a = 7; return arguments[0] + ',' + arguments[1]; }",
        "f(1, 2)",
        "1,7",
    ),
    row(
        "a_duplicate_past_the_passed_arguments_unmaps_the_earlier_one",
        "function f(a, b, a) { a = 7; return arguments[0] + ',' + a; }",
        "f(1)",
        "1,7",
    ),
    row(
        "object_method_is_mapped",
        "var o = { m: function (a) { a = 7; return arguments[0]; }, n(a) { a = 8; return arguments[0]; } };",
        "o.m(1) + ',' + o.n(1)",
        "7,8",
    ),
    row(
        "arrow_reads_the_enclosing_arguments",
        "function f(a) { var g = () => arguments[0]; a = 7; return g(); }",
        "f(1)",
        "7",
    ),
    row(
        "arrow_writes_through_the_enclosing_arguments",
        "function f(a) { var g = () => { arguments[0] = 6; }; g(); return a; }",
        "f(1)",
        "6",
    ),
    refused_while_decoding(row(
        "for_of_reads_live_values",
        "function f(a, b) { var out = ''; for (var x of arguments) { out += x; b = 9; } return out; }",
        "f(1, 2)",
        "19",
    )),
    row(
        "apply_spreads_current_values",
        "function g(x, y) { return x + ',' + y; }\nfunction f(a, b) { a = 5; return g.apply(null, arguments); }",
        "f(1, 2)",
        "5,2",
    ),
    row(
        "escaped_after_return_keeps_the_final_value",
        "function f(a) { a = a - 1; return arguments; }",
        "f(3)[0]",
        "2",
    ),
    row(
        "escaped_after_return_writes_do_not_reach_a_dead_scope",
        "function f(a) { var c = arguments; a = 4; return c; }\nvar s = f(3); s[0] = 8;",
        "s[0]",
        "8",
    ),
    row(
        "captured_scope_stays_aliased_after_return",
        "function f(a) { return { args: arguments, get: function () { return a; }, set: function (v) { a = v; } }; }\nvar r = f(1); r.set(5); var first = r.args[0]; r.args[0] = 9;",
        "first + ',' + r.get()",
        "5,9",
    ),
    row(
        "a_parameter_named_arguments_is_the_parameter",
        "function f(arguments) { return arguments; }",
        "f(4)",
        "4",
    ),
    row(
        "a_function_never_naming_arguments_still_computes",
        "function f(a, b) { a = a + b; return a; }",
        "f(1, 2)",
        "3",
    ),
    row(
        "string_keys_on_mapped_arguments_are_plain",
        "function f(a) { var c = arguments; c['k'] = 3; a = 4; return c.k + ',' + c[0] + ',' + c.length; }",
        "f(1)",
        "3,4,1",
    ),
];

fn node_source(row: &Row) -> String {
    format!(
        "{}{}\nconsole.log(String({}));\n",
        row.prefix.text(),
        row.definitions,
        row.call
    )
}

fn decoder_source(row: &Row) -> String {
    format!(
        "{}function A() {{ return []; }}\nfunction D() {{\n{}\nreturn String({});\n}}\nD();\n",
        row.prefix.text(),
        row.definitions,
        row.call
    )
}

fn script_source(row: &Row) -> String {
    format!(
        "{}{}\nString({});\n",
        row.prefix.text(),
        row.definitions,
        row.call
    )
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
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("arguments_mapping");
    std::fs::create_dir_all(&dir).expect("temp dir");
    let path = dir.join(format!("{}.{}", row.name, row.prefix.extension()));
    std::fs::write(&path, node_source(row)).expect("write the node program");
    let observed = node::run(&binary, &path)
        .expect("run node")
        .expect("node finished in time");
    assert_eq!(observed.exit, Some(0), "{}: {}", row.name, observed.stderr);
    Some(observed.stdout.trim_end().to_string())
}

fn named_function<'a>(program: &'a Program<'a>, name: &str) -> &'a Function<'a> {
    program
        .body
        .iter()
        .find_map(|statement| match statement {
            Statement::FunctionDeclaration(function)
                if function.id.as_ref().is_some_and(|id| id.name == name) =>
            {
                Some(function.as_ref())
            }
            _ => None,
        })
        .expect("function declaration")
}

fn decoded(row: &Row) -> Result<String, String> {
    let source = decoder_source(row);
    let alloc = Allocator::default();
    let parsed = parse_program(&alloc, &source);
    assert!(
        parsed.diagnostics.is_empty(),
        "{}: {:?}",
        row.name,
        parsed.diagnostics
    );
    let program = &parsed.program;
    let strict = program_is_strict(program);
    let decoder_program = DecoderProgram {
        source_text: program.source_text,
        string_array: FnRef {
            name: "A",
            function: named_function(program, "A"),
            strict,
        },
        decoders: vec![FnRef {
            name: "D",
            function: named_function(program, "D"),
            strict,
        }],
        rotator: None,
        strict,
    };
    let mut evaluator = InterpEvaluator::new(Budget::default());
    let rendered = evaluator
        .instantiate(&decoder_program)
        .and_then(|mut session| {
            untwist_eval::object::set_tracing(true);
            let value = session.eval_call(&Call {
                decoder: 0,
                span: oxc_span::SPAN,
                arguments: Vec::new(),
            });
            untwist_eval::object::set_tracing(false);
            value.map(|value| render(&value))
        });
    rendered.map_err(|bail: Bail| bail.to_string())
}

fn scripted(row: &Row) -> Result<String, String> {
    let source = script_source(row);
    let alloc = Allocator::default();
    let parsed = parse_program(&alloc, &source);
    assert!(
        parsed.diagnostics.is_empty(),
        "{}: {:?}",
        row.name,
        parsed.diagnostics
    );
    eval_script_in(
        &parsed.program,
        Budget::default(),
        |result, _| match result {
            Ok(value) => Ok(render(&value)),
            Err(abrupt) => match abrupt.into_kind() {
                AbruptKind::Bail(bail) => Err(bail.to_string()),
                AbruptKind::Throw(_) => Err("threw".to_string()),
            },
        },
    )
}

fn render(value: &Value<'_>) -> String {
    match value {
        Value::Str(text) => text.to_lossy_string(),
        other => format!("not a string: {other:?}"),
    }
}

fn check(
    row: &Row,
    context: &str,
    outcome: Outcome,
    got: Result<String, String>,
) -> Option<String> {
    match (outcome, got) {
        (Outcome::Node, Ok(text)) if text == row.node => None,
        (Outcome::Refused, Err(_)) => None,
        (expected, got) => Some(format!(
            "{} ({context}): expected {expected:?} printing {:?}, got {got:?}",
            row.name, row.node
        )),
    }
}

#[test]
fn every_row_prints_what_node_prints() {
    let mut failures = Vec::new();
    for row in ROWS {
        if let Some(printed) = printed_by_node(row) {
            assert_eq!(
                printed, row.node,
                "{}: node prints something else",
                row.name
            );
        }
        failures.extend(check(row, "decoding", row.decoding, decoded(row)));
        failures.extend(check(row, "script", row.script, scripted(row)));
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn the_plan_rows_are_all_present() {
    let names: Vec<&str> = ROWS.iter().map(|row| row.name).collect();
    for expected in [
        "mapped_write_param_reads_arguments",
        "mapped_write_arguments_reads_param",
        "template_subtract_then_capture",
        "strict_function_unmapped",
        "strict_program_unmapped",
        "default_param_unmapped",
        "rest_param_unmapped",
        "destructured_param_unmapped",
        "escaping_arguments_still_aliases",
        "missing_argument_unmapped",
        "length_change_adds_no_mapping",
        "delete_breaks_the_mapping",
        "delete_then_readd_stays_unmapped",
        "module_code_unmapped",
        "class_method_unmapped",
        "class_expression_static_unmapped",
        "nested_function_inherits_strict",
    ] {
        assert!(names.contains(&expected), "missing row {expected}");
    }
}
