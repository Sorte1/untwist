mod common;

use std::rc::{Rc, Weak};

use oxc_allocator::Allocator;
use untwist_core::parse::parse_program;
use untwist_eval::budget::{Budget, BOUND_CHAIN_CAP};
use untwist_eval::decode::InterpEvaluator;
use untwist_eval::env::{child_env, declare, root_env};
use untwist_eval::interp::eval_script_in;
use untwist_eval::names::Names;
use untwist_eval::value::Value;

const LEAK_ARTIFACT: &str =
    include_str!("../../../fuzz/artifacts/evaluator/leak-3968f0c1984eb067dd5f0aa3136c4da24f3d77e8");

#[test]
fn the_sessions_root_env_and_decoder_are_freed_when_the_session_drops() {
    common::with_program(LEAK_ARTIFACT, |program| {
        let decoder_program = common::decoder_program(program, "a", &["b"], None);
        let mut evaluator = InterpEvaluator::new(Budget::default());
        let (env, decoder) = {
            let session = evaluator
                .instantiate(&decoder_program)
                .expect("the artifact instantiates");
            let env = Rc::downgrade(session.root_env());
            let decoder = session
                .decoder(0)
                .and_then(|value| value.as_object().map(Rc::downgrade))
                .expect("decoder 0 is an object");
            (env, decoder)
        };
        assert!(env.upgrade().is_none(), "the root scope is still held");
        assert!(
            decoder.upgrade().is_none(),
            "the decoder closure is still held"
        );
    });
}

fn weak_object<'a>(
    value: &Value<'a>,
) -> Weak<std::cell::RefCell<untwist_eval::value::JsObject<'a>>> {
    value
        .as_object()
        .map(Rc::downgrade)
        .expect("the script returns an object")
}

#[test]
fn a_self_referencing_object_is_dismantled_when_the_interpreter_drops() {
    let alloc = Allocator::default();
    let parsed = parse_program(&alloc, "var o = { name: 'o' }; o.self = o; o;");
    let probe = eval_script_in(&parsed.program, Budget::default(), |returned, _| {
        let returned = returned.expect("evaluates");
        assert!(
            returned
                .as_object()
                .is_some_and(|obj| !obj.borrow().props.is_empty()),
            "inside inspect the realm is alive and the object still has its properties"
        );
        weak_object(&returned)
    });
    assert!(
        probe.upgrade().is_none(),
        "the realm's teardown left the self-referencing object alive"
    );
}

#[test]
fn a_closure_cycle_through_a_child_scope_is_dismantled_too() {
    let alloc = Allocator::default();
    let parsed = parse_program(
        &alloc,
        "function outer() { var inner = function () { return inner; }; return inner; } outer();",
    );
    let probe = eval_script_in(&parsed.program, Budget::default(), |returned, _| {
        weak_object(&returned.expect("evaluates"))
    });
    assert!(probe.upgrade().is_none());
}

const SMALL_STACK: usize = 2 << 20;

fn on_a_small_stack<R: Send + 'static>(body: impl FnOnce() -> R + Send + 'static) -> R {
    std::thread::Builder::new()
        .stack_size(SMALL_STACK)
        .spawn(body)
        .expect("the small-stack thread spawns")
        .join()
        .expect("the small-stack thread completed")
}

#[test]
fn a_deep_object_chain_released_before_teardown_drops_without_recursing() {
    let outcome = on_a_small_stack(|| {
        common::eval("var a = {}; for (var i = 0; i < 90000; i++) { a = { next: a }; } a = null; 1")
    });
    assert_eq!(outcome, Ok("number:1".to_string()));
}

const DEEP_SCOPE_CHAIN: usize = 90_000;
const LARGE_SCOPE_EVERY: usize = 1_000;
const LARGE_SCOPE_NAMES: [&str; 9] = ["a", "b", "c", "d", "e", "f", "g", "h", "i"];

fn deep_scope_chain(
    root: &untwist_eval::frame::FrameRef<'static>,
    interner: &mut Names<'static>,
) -> untwist_eval::frame::FrameRef<'static> {
    let mut innermost = child_env(root);
    for depth in 0..DEEP_SCOPE_CHAIN {
        let names: &[&'static str] = if depth % LARGE_SCOPE_EVERY == 0 {
            &LARGE_SCOPE_NAMES
        } else {
            &LARGE_SCOPE_NAMES[..1]
        };
        for name in names {
            declare(
                &innermost,
                interner.intern(name),
                Value::Number(depth as f64),
            );
        }
        innermost = child_env(&innermost);
    }
    innermost
}

#[test]
fn a_deep_scope_chain_released_before_teardown_drops_without_recursing() {
    let released = on_a_small_stack(|| {
        let root = root_env();
        let probe = Rc::downgrade(&root);
        let mut interner = Names::default();
        let innermost = deep_scope_chain(&root, &mut interner);
        drop(root);
        let alive_while_held = probe.strong_count() == 1;
        drop(innermost);
        alive_while_held && probe.strong_count() == 0
    });
    assert!(released, "the scope chain was not held, or not released");
}

#[test]
fn a_hundred_thousand_node_chain_alive_at_teardown_drops_without_recursing() {
    let source = "\
var a = {};
for (var i = 0; i < 50000; i++) { a = { next: a }; }
for (var j = 0; j < 50000; j++) { a = { next: a }; }
function keep() { return a; }
keep()";
    let released = on_a_small_stack(move || {
        let alloc = Allocator::default();
        let parsed = parse_program(&alloc, source);
        let probe = eval_script_in(&parsed.program, Budget::default(), |returned, _| {
            weak_object(&returned.expect("evaluates"))
        });
        probe.strong_count() == 0
    });
    assert!(released, "the realm's teardown left the chain alive");
}

#[test]
fn binding_past_the_chain_cap_is_refused_at_construction() {
    let deep = format!(
        "var f = function () {{ return 1; }}; for (var i = 0; i < {}; i++) f = f.bind(null); f()",
        BOUND_CHAIN_CAP + 1
    );
    let outcome = common::eval(&deep);
    assert!(
        outcome
            .as_ref()
            .is_err_and(|bail| bail.contains("bound function chain")),
        "{outcome:?}"
    );
    let at_cap = format!(
        "var f = function () {{ return 1; }}; for (var i = 0; i < {}; i++) f = f.bind(null); f()",
        BOUND_CHAIN_CAP
    );
    assert_eq!(common::eval(&at_cap), Ok("number:1".to_string()));
}

#[test]
fn a_closure_held_in_a_slot_of_the_compiled_frame_it_captures_is_dismantled_too() {
    let alloc = Allocator::default();
    let parsed = parse_program(&alloc, "var mk = (x) => (x = () => x, x); mk(0);");
    let mut probes = None;
    let census = untwist_eval::test_hooks::compile_census(|| {
        probes = Some(eval_script_in(
            &parsed.program,
            Budget::default(),
            |returned, _| {
                let returned = returned.expect("evaluates");
                let obj = returned.as_object().expect("a closure").clone();
                let frame = match &obj.borrow().kind {
                    untwist_eval::value::ObjKind::Function(closure) => closure.env.clone(),
                    _ => panic!("the script returns a closure"),
                };
                assert!(
                    matches!(
                        frame.borrow().body,
                        untwist_eval::frame::FrameBody::Fixed { .. }
                    ),
                    "the closure captures a compiled frame"
                );
                (Rc::downgrade(&obj), Rc::downgrade(&frame))
            },
        ));
    });
    assert_eq!(census.calls, 1);
    let (closure, frame) = probes.expect("the script ran");
    assert!(closure.upgrade().is_none(), "the closure is still held");
    assert!(
        frame.upgrade().is_none(),
        "its compiled frame is still held"
    );
}

#[test]
fn every_compiled_frame_is_tracked_by_the_realm_as_a_walker_frame_is() {
    let source = "var f = (x) => x; var g = function n() { 0; }; f(1); g();";
    let alloc = Allocator::default();
    let parsed = parse_program(&alloc, source);
    let program = &parsed.program;
    let session = || {
        let mut interp = untwist_eval::interp::Interp::new(program.source_text, Budget::default());
        let env = interp.global_env();
        interp.hoist(&program.body, &env).expect("hoists");
        let mut tracked = Vec::new();
        for statement in &program.body {
            let before = interp.realm.tracked_frames();
            interp.run_root_statement(statement).expect("runs");
            tracked.push(interp.realm.tracked_frames() - before);
        }
        tracked
    };
    let mut compiled = Vec::new();
    let census = untwist_eval::test_hooks::compile_census(|| compiled = session());
    assert_eq!(census.calls, 2);
    assert_eq!(
        compiled,
        [0, 0, 1, 2],
        "one frame for the arrow, two for the named function"
    );
}
