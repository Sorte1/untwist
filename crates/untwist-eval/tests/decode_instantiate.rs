mod common;

use untwist_eval::budget::Budget;
use untwist_eval::decode::InterpEvaluator;
use untwist_eval::value::Value;

const SIMPLE: &str = "\
function a() { var h = ['log', 'Hello World!']; a = function () { return h; }; return a(); }
function b(c, d) {
  var e = a();
  return (b = function (f, g) { f = f - 0x0; var i = e[f]; return i; }), b(c, d);
}";

#[test]
fn instantiation_yields_one_shared_array_and_primed_decoders() {
    common::with_program(SIMPLE, |program| {
        let decoder_program = common::decoder_program(program, "a", &["b"], None);
        let mut evaluator = InterpEvaluator::new(Budget::default());
        let session = evaluator
            .instantiate(&decoder_program)
            .expect("instantiation succeeds");
        assert_eq!(session.array_len(), 2);
        assert!(matches!(
            session.array_first(),
            Some(Value::Str(s)) if s.to_lossy_string() == "log"
        ));
        assert!(session.decoder(0).is_some_and(|d| d.is_callable()));
        assert!(session.decoder(1).is_none());
    });
}

#[test]
fn a_string_array_function_that_reads_an_unbound_global_bails() {
    common::with_program("function a(c) { return unresolvedThing; }", |program| {
        let decoder_program = common::decoder_program(program, "a", &[], None);
        let mut evaluator = InterpEvaluator::new(Budget::default());
        assert_eq!(
            evaluator
                .instantiate(&decoder_program)
                .err()
                .map(|bail| bail.to_string()),
            Some("unresolved global binding: unresolvedThing".to_string())
        );
    });
}

const COUNTING_DECODER: &str = "\
function a() { var h = ['zero', 'one']; a = function () { return h; }; return a(); }
function b(c) {
  b.n = (b.n || 0) + 1;
  return a()[c] + ':' + b.n;
}
b(0);";

#[test]
fn a_decoder_is_not_called_before_the_program_calls_it() {
    common::with_program(COUNTING_DECODER, |program| {
        let decoder_program = common::decoder_program(program, "a", &["b"], None);
        let calls = common::calls_for(program, 0);
        let mut evaluator = InterpEvaluator::new(Budget::default());
        let mut session = evaluator
            .instantiate(&decoder_program)
            .expect("instantiation succeeds");
        assert!(
            session.notes().is_empty(),
            "instantiation must not call any decoder: {:?}",
            session.notes()
        );
        let call = calls.first().expect("the program has one decoder call");
        let decoded = session.eval_call(call).expect("the call decodes");
        assert!(
            matches!(&decoded, Value::Str(text) if text.to_lossy_string() == "zero:1"),
            "the first real call must see the counter at 1, not 2 (a synthetic priming \
             call would have advanced it one step early): got {decoded:?}"
        );
    });
}

#[test]
fn a_string_array_function_that_returns_a_non_array_bails() {
    common::with_program("function a(c) { return 1; }", |program| {
        let decoder_program = common::decoder_program(program, "a", &[], None);
        let mut evaluator = InterpEvaluator::new(Budget::default());
        assert_eq!(
            evaluator
                .instantiate(&decoder_program)
                .err()
                .map(|bail| bail.to_string()),
            Some(
                "unsupported construct: the string-array function did not return an array"
                    .to_string()
            )
        );
    });
}

const MEMOIZING: &str = "\
function a() { var h = ['zero', 'one']; a = function () { return h; }; return a(); }
function b(c) {
  if (b['memo'] === undefined) { b['memo'] = {}; }
  var k = '' + c;
  if (b['memo'][k] === undefined) { b['memo'][k] = a()[c]; }
  return b['memo'][k];
}
b(0);";

#[test]
fn priming_does_not_poison_a_decoder_memo_keyed_by_index() {
    common::with_program(MEMOIZING, |program| {
        let decoder_program = common::decoder_program(program, "a", &["b"], None);
        let calls = common::calls_for(program, 0);
        let mut evaluator = InterpEvaluator::new(Budget::default());
        let mut session = evaluator
            .instantiate(&decoder_program)
            .expect("instantiation succeeds");
        session.rotate(1).expect("rotation succeeds");
        let call = calls.first().expect("the program has one decoder call");
        let decoded = session.eval_call(call).expect("the call decodes");
        assert!(
            matches!(&decoded, Value::Str(text) if text.to_lossy_string() == "one"),
            "expected the post-rotation value, got {decoded:?}"
        );
    });
}

const FLOOR_DECODER: &str = "\
function a() { var h = ['zero', 'one']; a = function () { return h; }; return a(); }
function b(c) { return a()[Math['floor'](0.5 * 0) + c]; }
b(1);";

#[test]
fn a_session_reports_its_fuel_and_the_interpreters_notes() {
    common::with_program(FLOOR_DECODER, |program| {
        let decoder_program = common::decoder_program(program, "a", &["b"], None);
        let calls = common::calls_for(program, 0);
        let mut evaluator = InterpEvaluator::new(Budget::default());
        let mut session = evaluator
            .instantiate(&decoder_program)
            .expect("instantiation succeeds");
        let call = calls.first().expect("the program has one decoder call");
        session.eval_call(call).expect("the call decodes");

        let budget = session.budget();
        assert!(budget.steps_used() > 0);
        assert_eq!(
            budget.steps_used() + budget.steps_left(),
            untwist_eval::budget::STEP_BUDGET
        );
        assert!(session.drain_notes().is_empty());
        assert!(session.notes().is_empty());
    });
}

#[test]
fn a_string_array_function_that_throws_names_the_instantiation_step() {
    common::with_program("function a() { throw 'boom'; }", |program| {
        let decoder_program = common::decoder_program(program, "a", &[], None);
        let mut evaluator = InterpEvaluator::new(Budget::default());
        assert_eq!(
            evaluator
                .instantiate(&decoder_program)
                .err()
                .map(|bail| bail.to_string()),
            Some(
                "unsupported construct: uncaught throw while instantiating the decoder program"
                    .to_string()
            )
        );
    });
}
