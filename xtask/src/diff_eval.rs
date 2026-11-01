mod aliasing;
mod flow;
mod node;
mod pool;
mod regexps;
mod render;
mod routes;
mod statements;

use std::collections::BTreeMap;
use std::path::Path;

use anyhow::{Context, Result};
use oxc_allocator::Allocator;
use oxc_ast::ast::Statement;
use untwist_core::parse::parse_program;
use untwist_eval::budget::Budget;
use untwist_eval::builtins::shape::{
    Kinds, NativeShape, Shape, ARGUMENTS, ARRAY, CALLABLE, MISSING, NUMBER, PLAIN, REGEXP, SHAPES,
    STRING,
};
use untwist_eval::fold::{fold_to_const, Const, FoldOptions};
use untwist_eval::interp::eval_script_decoding;
use untwist_eval::value::{AbruptKind, JsString, Value};

use node::run_node;
use pool::{pick, Rng, Sample, POOL};
use render::{Rendered, Renderer};
use routes::{callee, function_call, method_call, typeof_text};

const IN_SHAPE_PERCENT: u64 = 60;
const MAPPED_ARGUMENTS_PERCENT: u64 = 50;
const TYPEOF_PERCENT: u64 = 4;
const STATEMENT_PERCENT: u64 = 40;
const REGEXP_PERCENT: u64 = 15;
const SHOWN_MISMATCHES: usize = 1000;

pub struct Case {
    label: String,
    in_shape: bool,
    decode: String,
    fold: Option<String>,
}

#[derive(Default)]
struct NativeStats {
    cases: usize,
    in_shape: usize,
    answered: usize,
    folded: usize,
}

pub struct Outcome {
    pub cases: usize,
    pub answered: usize,
    pub bailed: usize,
    pub folded: usize,
    pub mismatches: Vec<String>,
}

fn natural_receiver(owner: &str) -> Kinds {
    match owner {
        "Array.prototype" => ARRAY,
        "String.prototype" => STRING,
        "Number.prototype" => NUMBER,
        "RegExp.prototype" => REGEXP,
        "Function.prototype" => CALLABLE,
        "Object.prototype" => PLAIN,
        _ => 0,
    }
}

fn accepted_at(shape: &Shape, index: usize) -> Kinds {
    shape.args.get(index).copied().unwrap_or(shape.rest)
}

struct Drawn {
    receiver: Option<&'static Sample>,
    args: Vec<&'static Sample>,
    in_shape: bool,
}

const MUTATORS: &[&str] = &[
    "Object.assign",
    "Object.defineProperty",
    "Object.freeze",
    "Object.setPrototypeOf",
    "Array.prototype.copyWithin",
    "Array.prototype.fill",
    "Array.prototype.pop",
    "Array.prototype.push",
    "Array.prototype.reverse",
    "Array.prototype.shift",
    "Array.prototype.sort",
    "Array.prototype.splice",
    "Array.prototype.unshift",
];

fn draw(rng: &mut Rng, entry: &NativeShape) -> Drawn {
    let shape = &entry.shape;
    let fresh_only = MUTATORS.contains(&callee(entry).as_str());
    let is_method = entry.owner.ends_with(".prototype");
    let mut in_shape = true;
    let receiver = if is_method {
        let inside = rng.chance(IN_SHAPE_PERCENT);
        let found = pick(rng, shape.receiver, inside, fresh_only)
            .or_else(|| pick(rng, shape.receiver, !inside, fresh_only));
        in_shape &= found.is_some_and(|sample| shape.receiver & sample.kind != 0);
        found
    } else {
        None
    };
    let declared = shape.args.len();
    let count = (declared + rng.below(4)).saturating_sub(1);
    let mut args = Vec::with_capacity(count);
    for index in 0..count {
        let accepted = accepted_at(shape, index);
        let inside = rng.chance(IN_SHAPE_PERCENT);
        let Some(sample) = pick(rng, accepted, inside, fresh_only)
            .or_else(|| pick(rng, accepted, !inside, fresh_only))
        else {
            continue;
        };
        in_shape &= accepted & sample.kind != 0;
        args.push(sample);
    }
    for index in args.len()..declared {
        in_shape &= accepted_at(shape, index) & MISSING != 0;
    }
    if shape.receiver == 0 {
        in_shape = false;
    }
    Drawn {
        receiver,
        args,
        in_shape,
    }
}

fn call_text(rng: &mut Rng, entry: &NativeShape, drawn: &Drawn) -> String {
    let count = drawn.args.len();
    let Some(receiver) = drawn.receiver else {
        return function_call(rng, entry, count);
    };
    let natural = natural_receiver(entry.owner) & receiver.kind != 0;
    method_call(rng, entry, count, natural)
}

fn decode_text(rng: &mut Rng, entry: &NativeShape, drawn: &Drawn) -> String {
    let call = call_text(rng, entry, drawn);
    let mut body = String::new();
    let mut tail = vec!["v".to_string()];
    let mapped = drawn
        .receiver
        .is_some_and(|sample| sample.kind == ARGUMENTS)
        && rng.chance(MAPPED_ARGUMENTS_PERCENT);
    if let Some(receiver) = drawn.receiver {
        if !mapped {
            body.push_str(&format!("var r = {}; ", receiver.source));
        }
        tail.push("r".to_string());
    }
    for (index, sample) in drawn.args.iter().enumerate() {
        body.push_str(&format!("var a{index} = {}; ", sample.source));
        tail.push(format!("a{index}"));
    }
    if !mapped {
        return format!(
            "(function () {{ {body}var v = {call}; return [{}]; }})()",
            tail.join(", ")
        );
    }
    let first = POOL[rng.below(POOL.len())].source;
    let second = POOL[rng.below(POOL.len())].source;
    format!(
        "(function (p0, p1) {{ var r = arguments; {body}var v = {call}; return [{}, p0, p1]; }})({first}, {second})",
        tail.join(", ")
    )
}

fn fold_text(entry: &NativeShape, drawn: &Drawn) -> Option<String> {
    if !drawn.args.iter().all(|sample| sample.foldable) {
        return None;
    }
    let list: Vec<&str> = drawn.args.iter().map(|sample| sample.source).collect();
    let list = list.join(", ");
    match drawn.receiver {
        Some(receiver)
            if receiver.foldable && natural_receiver(entry.owner) & receiver.kind != 0 =>
        {
            Some(format!("({}).{}({list})", receiver.source, entry.name))
        }
        Some(_) => None,
        None => Some(format!("{}({list})", callee(entry))),
    }
}

fn native_case(rng: &mut Rng, natives: &[usize]) -> Case {
    let entry = &SHAPES[natives[rng.below(natives.len())]];
    let drawn = draw(rng, entry);
    Case {
        label: callee(entry),
        in_shape: drawn.in_shape,
        decode: decode_text(rng, entry, &drawn),
        fold: fold_text(entry, &drawn),
    }
}

fn statement_case(rng: &mut Rng) -> Case {
    let (label, decode) = statements::statement_case(rng);
    Case {
        label,
        in_shape: true,
        decode,
        fold: None,
    }
}

fn regexp_case(rng: &mut Rng) -> Case {
    let (label, decode) = regexps::regexp_case(rng);
    Case {
        label,
        in_shape: true,
        decode,
        fold: None,
    }
}

fn typeof_case(rng: &mut Rng) -> Case {
    let (label, decode) = typeof_text(rng);
    Case {
        fold: Some(label.clone()),
        label,
        in_shape: true,
        decode,
    }
}

pub fn generate(seed: u64, count: usize) -> Vec<Case> {
    let mut rng = Rng::new(seed);
    let natives: Vec<usize> = (0..SHAPES.len())
        .filter(|index| SHAPES[*index].owner != "hostile")
        .collect();
    (0..count)
        .map(|_| {
            if rng.chance(STATEMENT_PERCENT) {
                statement_case(&mut rng)
            } else if rng.chance(REGEXP_PERCENT) {
                regexp_case(&mut rng)
            } else if rng.chance(TYPEOF_PERCENT) {
                typeof_case(&mut rng)
            } else {
                native_case(&mut rng, &natives)
            }
        })
        .collect()
}

fn answer(result: Rendered, prefix: char) -> Option<String> {
    Some(match result {
        Ok(text) => format!("{prefix}{text}"),
        Err(why) => why,
    })
}

fn decode_answer(source: &str) -> Option<String> {
    let alloc = Allocator::default();
    let parsed = parse_program(&alloc, source);
    if !parsed.diagnostics.is_empty() {
        return Some("S".to_string());
    }
    let renderer = Renderer { source };
    eval_script_decoding(&parsed.program, Budget::default(), |result| match result {
        Ok(value) => answer(renderer.render(&value, 0), 'V'),
        Err(abrupt) => match abrupt.into_kind() {
            AbruptKind::Throw(thrown) => answer(renderer.render(&thrown, 0), 'T'),
            AbruptKind::Bail(_) => None,
        },
    })
}

fn fold_answer(source: &str) -> Option<String> {
    let alloc = Allocator::default();
    let parsed = parse_program(&alloc, source);
    let Some(Statement::ExpressionStatement(statement)) = parsed.program.body.first() else {
        return None;
    };
    let options = FoldOptions {
        allow_pure_calls: true,
        allow_local_mutation: true,
        bindings: Default::default(),
    };
    let value = match fold_to_const(&statement.expression, &options)? {
        Const::Num(n) => Value::Number(n),
        Const::Str(text) => Value::Str(JsString::from_str(&text)),
        Const::Bool(flag) => Value::Bool(flag),
        Const::Null => Value::Null,
        Const::Undefined => Value::Undefined,
    };
    let renderer = Renderer { source };
    answer(renderer.render(&value, 0), 'V')
}

fn describe(case: &Case, source: &str, context: &str, node: &str, mine: &str) -> String {
    format!(
        "{context} {} ({}): {source}\n  node:    {node}\n  untwist: {mine}",
        case.label,
        if case.in_shape {
            "in shape"
        } else {
            "out of shape"
        }
    )
}

fn check_decode(case: &Case, expected: &str, outcome: &mut Outcome, stats: &mut NativeStats) {
    let Some(mine) = decode_answer(&case.decode) else {
        outcome.bailed += 1;
        return;
    };
    outcome.answered += 1;
    stats.answered += 1;
    if mine != expected {
        let text = describe(case, &case.decode, "decode", expected, &mine);
        outcome.mismatches.push(text);
    }
}

fn check_fold(
    case: &Case,
    fold: &str,
    expected: &str,
    outcome: &mut Outcome,
    stats: &mut NativeStats,
) {
    let Some(mine) = fold_answer(fold) else {
        return;
    };
    outcome.folded += 1;
    stats.folded += 1;
    if mine != expected {
        outcome
            .mismatches
            .push(describe(case, fold, "fold", expected, &mine));
    }
}

fn print_stats(per_native: &BTreeMap<String, NativeStats>) {
    for (name, stats) in per_native {
        println!(
            "{name:<36} cases {:>5}  in-shape {:>5}  answered {:>5}  folded {:>5}",
            stats.cases, stats.in_shape, stats.answered, stats.folded
        );
    }
}

pub fn compare(node: &Path, cases: &[Case], dir: &Path) -> Result<Outcome> {
    let mut lines = run_node(node, cases, dir)?.into_iter();
    let mut outcome = Outcome {
        cases: cases.len(),
        answered: 0,
        bailed: 0,
        folded: 0,
        mismatches: Vec::new(),
    };
    let mut per_native: BTreeMap<String, NativeStats> = BTreeMap::new();
    for case in cases {
        let stats = per_native.entry(case.label.clone()).or_default();
        stats.cases += 1;
        stats.in_shape += usize::from(case.in_shape);
        let expected = lines.next().context("node printed too few lines")?;
        check_decode(case, &expected, &mut outcome, stats);
        if let Some(fold) = &case.fold {
            let expected = lines.next().context("node printed too few lines")?;
            check_fold(case, fold, &expected, &mut outcome, stats);
        }
    }
    print_stats(&per_native);
    Ok(outcome)
}

pub fn run(node: &Path, seed: u64, count: usize) -> Result<Outcome> {
    let cases = generate(seed, count);
    let dir = tempfile::tempdir().context("temporary directory")?;
    let outcome = compare(node, &cases, dir.path())?;
    println!(
        "diff-eval seed {seed}: {} cases, {} answered in decode context, {} bailed, {} folded, {} differ from node",
        outcome.cases,
        outcome.answered,
        outcome.bailed,
        outcome.folded,
        outcome.mismatches.len()
    );
    for mismatch in outcome.mismatches.iter().take(SHOWN_MISMATCHES) {
        println!("{mismatch}");
    }
    Ok(outcome)
}
