use oxc_allocator::Allocator;
use oxc_ast::ast::{Expression, Function, Statement};
use oxc_span::GetSpan;
use untwist_core::parse::parse_program;
use untwist_eval::{function_text, run_sink, SinkCallee, SinkOutcome, SinkScript};

const FUEL: u64 = 2_000_000;
const HEAP: u64 = 1 << 24;

fn pooled(texts: &[&str]) -> Vec<String> {
    texts.iter().map(|text| (*text).to_string()).collect()
}

fn outcome(prefix: &str, region: &str, pool: &[Vec<String>]) -> SinkOutcome {
    let mut script = format!("{prefix}\n;{region};\nif (0) {{\n");
    for texts in pool {
        script.push_str(&format!("({});\n", function_text(texts)));
    }
    script.push_str("}\n");
    let alloc = Allocator::default();
    let parsed = parse_program(&alloc, &script);
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    let program = &parsed.program;
    let region_start = u32::try_from(prefix.len() + 2).unwrap();
    let sink = program
        .body
        .iter()
        .position(|statement| statement.span().start == region_start)
        .expect("the region statement");
    let Some(Statement::IfStatement(last)) = program.body.last() else {
        panic!("a pool");
    };
    let Statement::BlockStatement(block) = &last.consequent else {
        panic!("a pool block");
    };
    let functions: Vec<&Function<'_>> = block
        .body
        .iter()
        .map(|statement| match statement {
            Statement::ExpressionStatement(only) => match &only.expression {
                Expression::FunctionExpression(function) => &**function,
                _ => panic!("a pooled function"),
            },
            _ => panic!("a pooled statement"),
        })
        .collect();
    let pool = pool
        .iter()
        .map(|texts| function_text(texts))
        .zip(functions)
        .collect();
    let script = SinkScript {
        program,
        sink,
        region: region_start,
        pool,
    };
    run_sink(script, FUEL, HEAP).outcome
}

#[test]
fn a_function_text_outside_the_pool_is_asked_for() {
    let found = outcome(
        "var F = [].constructor.constructor;",
        "[F, F('return \"x\"')()]",
        &[],
    );
    assert_eq!(found, SinkOutcome::Unknown(pooled(&["return \"x\""])));
}

#[test]
fn a_pooled_function_runs_and_the_sink_is_reached() {
    let pool = [pooled(&["return \"con\" + \"sole\""])];
    let found = outcome(
        "var F = [].constructor.constructor;",
        "[F, F('return \"con\" + \"sole\"')()]",
        &pool,
    );
    assert_eq!(
        found,
        SinkOutcome::Reached {
            callee: SinkCallee::Function,
            texts: pooled(&["console"]),
        }
    );
}

#[test]
fn a_pooled_function_prints_as_the_function_constructor_does() {
    let pool = [pooled(&["a", "return a"])];
    let found = outcome("", "[Function, Function('a', 'return a') + '']", &pool);
    let text = "function anonymous(a\n) {\nreturn a\n}";
    assert_eq!(
        found,
        SinkOutcome::Reached {
            callee: SinkCallee::Function,
            texts: pooled(&[text]),
        }
    );
}

#[test]
fn an_indirect_eval_is_reached_as_eval() {
    let pool = [pooled(&["return eval"])];
    let found = outcome("", "[Function('return eval')(), 'x' + 1]", &pool);
    assert_eq!(
        found,
        SinkOutcome::Reached {
            callee: SinkCallee::Eval,
            texts: pooled(&["x1"]),
        }
    );
}

#[test]
fn a_callback_from_the_region_may_run() {
    let found = outcome(
        "var k = 1;",
        "[eval, [98, 99].map(function (c) { return String.fromCharCode(c - k); }).join('')]",
        &[],
    );
    assert_eq!(
        found,
        SinkOutcome::Reached {
            callee: SinkCallee::Eval,
            texts: pooled(&["ab"]),
        }
    );
}

#[test]
fn a_function_from_before_the_region_may_not_run_in_it() {
    let found = outcome("var f = function () { return 'x'; };", "[eval, f()]", &[]);
    assert_eq!(found, SinkOutcome::Refused);
}

#[test]
fn a_writing_native_may_not_run_in_the_region() {
    let found = outcome("var a = ['x'];", "[eval, a.reverse().join('')]", &[]);
    assert_eq!(found, SinkOutcome::Refused);
}

#[test]
fn a_fresh_array_may_be_reversed_in_the_region() {
    let found = outcome("", "[eval, 'ba'.split('').reverse().join('')]", &[]);
    assert_eq!(
        found,
        SinkOutcome::Reached {
            callee: SinkCallee::Eval,
            texts: pooled(&["ab"]),
        }
    );
}

#[test]
fn a_callee_that_is_no_code_runner_is_refused() {
    let found = outcome("", "[String, 'x']", &[]);
    assert_eq!(found, SinkOutcome::Refused);
}

#[test]
fn a_region_that_reads_the_host_is_refused() {
    let rows = [
        "[eval, typeof location]",
        "[eval, Math.SQRT2 + '']",
        "[eval, String.name]",
    ];
    for region in rows {
        assert_eq!(outcome("", region, &[]), SinkOutcome::Refused, "{region}");
    }
}

#[test]
fn an_array_key_and_a_built_in_text_are_what_node_computes() {
    let found = outcome("", "[eval, [][[]] + ([].filter + '').slice(9, 16)]", &[]);
    assert_eq!(
        found,
        SinkOutcome::Reached {
            callee: SinkCallee::Eval,
            texts: pooled(&["undefinedfilter("]),
        }
    );
}
