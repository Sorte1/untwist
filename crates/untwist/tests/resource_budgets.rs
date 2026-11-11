use std::alloc::System;
use std::time::Duration;

use untwist::{
    BudgetHit, CountingAlloc, Options, Output, Pipeline, BUDGET_MEMORY_REACHED,
    BUDGET_TIME_REACHED, TOO_LARGE_FOR_THE_BUDGET,
};

#[global_allocator]
static ALLOCATOR: CountingAlloc<System> = CountingAlloc(System);

const SMALL_BUDGET: u64 = 24 << 20;

fn decoder_program(body: &str, calls: usize) -> String {
    let sites: String = (0..calls)
        .map(|index| format!("console.log(_0x44bc({}));\n", index % 2))
        .collect();
    format!(
        "function _0x44bc(_0x3fc3a6, _0x44bc45) {{\n  var _0x2004c0 = _0x3fc3();\n  _0x44bc = function (_0x4338c0, _0x393a93) {{\n    _0x4338c0 = _0x4338c0 - 0x0;\n    {body}\n  }};\n  return _0x44bc(_0x3fc3a6, _0x44bc45);\n}}\nfunction _0x3fc3() {{\n  var _0x1e397 = ['Hello\\x20World!', 'split', 'log'];\n  _0x3fc3 = function () {{\n    return _0x1e397;\n  }};\n  return _0x3fc3();\n}}\n{sites}"
    )
}

fn run(source: &str, options: Options) -> Output {
    Pipeline::new(Options {
        unpack: false,
        ..options
    })
    .run(source)
}

fn under_memory(source: &str, bytes: u64) -> Output {
    run(
        source,
        Options {
            max_memory: Some(bytes),
            ..Options::default()
        },
    )
}

fn notes_of(output: &Output) -> Vec<&str> {
    output
        .notes
        .iter()
        .map(|note| note.message.as_str())
        .collect()
}

fn assert_refused_for(output: &Output, hit: BudgetHit, note: &str) {
    assert_eq!(output.budget_hits, [(0, hit)], "{:?}", notes_of(output));
    assert!(notes_of(output).contains(&note), "{:?}", notes_of(output));
    assert!(output.panicked.is_empty());
}

#[test]
fn a_decoder_that_doubles_a_string_stops_at_the_memory_budget() {
    let source = decoder_program(
        "var s = _0x2004c0[_0x4338c0]; for (var k = 0x0; k < 0x40; k++) { s = s + s; } return s;",
        2,
    );
    let output = under_memory(&source, SMALL_BUDGET);
    assert_refused_for(&output, BudgetHit::Memory, BUDGET_MEMORY_REACHED);
    assert!(output.code().contains("_0x44bc(0)"), "{}", output.code());
}

#[test]
fn a_decoder_that_grows_an_array_stops_at_the_memory_budget() {
    let source = decoder_program(
        "var p = _0x2004c0[_0x4338c0]; for (var j = 0x0; j < 0x6; j++) { p = p + p; } var a = []; for (var k = 0x0; k < 0xc350; k++) { a.push(p + k); } return a.length;",
        2,
    );
    let output = under_memory(&source, SMALL_BUDGET);
    assert_refused_for(&output, BudgetHit::Memory, BUDGET_MEMORY_REACHED);
}

#[test]
fn a_long_decoded_string_inlined_at_many_sites_stops_at_the_memory_budget() {
    let long = "x".repeat(256 << 10);
    let source = decoder_program("return _0x2004c0[_0x4338c0];", 400)
        .replace("'Hello\\x20World!'", &format!("'{long}'"));
    let output = under_memory(&source, SMALL_BUDGET);
    assert_refused_for(&output, BudgetHit::Memory, BUDGET_MEMORY_REACHED);
}

#[test]
fn a_self_expanding_nested_value_stops_at_the_memory_budget() {
    let source = decoder_program(
        "var o = _0x2004c0[_0x4338c0]; for (var k = 0x0; k < 0x1e; k++) { o = [o, o]; } return String(o);",
        2,
    );
    let output = under_memory(&source, SMALL_BUDGET);
    assert_refused_for(&output, BudgetHit::Memory, BUDGET_MEMORY_REACHED);
}

#[test]
fn a_decoder_that_loops_for_long_stops_at_the_time_budget() {
    let source = decoder_program(
        "var t = 0x0; for (var a = 0x0; a < 0x15f90; a++) { for (var b = 0x0; b < 0x15f90; b++) { t = t + 0x1; } } return _0x2004c0[_0x4338c0] + t;",
        2,
    );
    let started = std::time::Instant::now();
    let output = run(
        &source,
        Options {
            unsafe_transforms: true,
            timeout: Some(Duration::from_millis(200)),
            ..Options::default()
        },
    );
    assert_refused_for(&output, BudgetHit::Time, BUDGET_TIME_REACHED);
    assert!(started.elapsed() < Duration::from_secs(5));
}

#[test]
fn an_input_too_large_to_parse_within_the_budget_is_passed_through() {
    let source = "var a = 1;\n".repeat(20_000);
    let output = under_memory(&source, 64 << 10);
    assert_eq!(output.depth_refused, [0]);
    assert_eq!(output.code(), source);
    assert!(notes_of(&output).contains(&TOO_LARGE_FOR_THE_BUDGET));
}

#[test]
fn a_generous_budget_changes_nothing() {
    let source = decoder_program("return _0x2004c0[_0x4338c0];", 4);
    let plain = run(&source, Options::default());
    let budgeted = run(
        &source,
        Options {
            max_memory: Some(3 << 30),
            timeout: Some(Duration::from_secs(600)),
            ..Options::default()
        },
    );
    assert_eq!(plain.code(), budgeted.code());
    assert!(budgeted.budget_hits.is_empty());
    assert!(plain.code().contains("Hello World!"), "{}", plain.code());
}

#[test]
fn a_memory_budget_hit_is_deterministic() {
    let source = decoder_program(
        "var s = _0x2004c0[_0x4338c0]; for (var k = 0x0; k < 0x40; k++) { s = s + s; } return s;",
        2,
    );
    let first = under_memory(&source, SMALL_BUDGET);
    let second = under_memory(&source, SMALL_BUDGET);
    assert_eq!(first.code(), second.code());
    assert_eq!(notes_of(&first), notes_of(&second));
}

const DOLLAR_TEMPLATE_PROBE: &str = r#"function _0xarr() { var h = ['hello','world','foo','bar']; _0xarr = function () { return h; }; return _0xarr(); }
function _0xdec(c, d) {
  var e = _0xarr();
  _0xdec = function (f, g) { f = f - 0; var h = e[f]; var s = "x"; for (var i = 0; i < 23; i++) s += s; var r = "$'"; for (var i = 0; i < 10; i++) r += r; r = r.slice(0, 2048); try { h = h + s.replace("x", r).length; } catch (z) { h = h + "caught"; } return h; };
  return _0xdec(c, d);
}
console.log(_0xdec(0));
"#;

#[test]
fn a_replace_template_that_repeats_the_subject_is_refused_inside_the_memory_budget() {
    let output = under_memory(DOLLAR_TEMPLATE_PROBE, 256 << 20);
    assert!(output.budget_hits.is_empty(), "{:?}", notes_of(&output));
    assert!(output.panicked.is_empty());
    assert!(output.code().contains("_0xdec(0)"), "{}", output.code());
}

#[test]
fn each_replace_template_form_is_refused_inside_the_memory_budget() {
    for (template, pattern) in [
        ("$&", "/x+/"),
        ("$`", "/x$/"),
        ("$'", "/x/"),
        ("$1$'", "/(x)/"),
    ] {
        let source = DOLLAR_TEMPLATE_PROBE
            .replace("var r = \"$'\"", &format!("var r = \"{template}\""))
            .replace("s.replace(\"x\", r)", &format!("s.replace({pattern}, r)"));
        let output = under_memory(&source, 256 << 20);
        assert!(
            output.budget_hits.is_empty(),
            "{template}: {:?}",
            notes_of(&output)
        );
        assert!(
            output.code().contains("_0xdec(0)"),
            "{template}: {}",
            output.code()
        );
    }
}

fn small_decoder(body: &str) -> String {
    DOLLAR_TEMPLATE_PROBE.replace(
        "var s = \"x\"; for (var i = 0; i < 23; i++) s += s; var r = \"$'\"; for (var i = 0; i < 10; i++) r += r; r = r.slice(0, 2048); try { h = h + s.replace(\"x\", r).length; } catch (z) { h = h + \"caught\"; }",
        body,
    )
}

fn assert_refused_inside_the_budget(body: &str) {
    let source = small_decoder(body);
    assert_ne!(source, DOLLAR_TEMPLATE_PROBE);
    let output = under_memory(&source, 256 << 20);
    assert!(
        output.budget_hits.is_empty(),
        "{body}: {:?}",
        notes_of(&output)
    );
    assert!(output.panicked.is_empty());
    assert!(
        output.code().contains("_0xdec(0)"),
        "{body}: {}",
        output.code()
    );
}

#[test]
fn regexp_source_strings_kept_in_an_array_are_refused_inside_the_memory_budget() {
    assert_refused_inside_the_budget(
        "var p = \"\"; for (var i = 0; i < 4096; i++) p += \"/\"; var re = new RegExp(p); var q = []; for (var i = 0; i < 20000; i++) q.push(re.toString()); h = h + q.length;",
    );
}

#[test]
fn the_keys_of_a_long_string_are_refused_inside_the_memory_budget() {
    for read in ["Object.keys(s)", "Object.values(s)", "Object.assign({}, s)"] {
        assert_refused_inside_the_budget(&format!(
            "var s = \"x\"; for (var i = 0; i < 24; i++) s += s; h = h + typeof {read};"
        ));
    }
}

#[test]
fn a_regexp_over_a_long_non_ascii_string_is_refused_inside_the_memory_budget() {
    assert_refused_inside_the_budget(
        "var s = \"\\u00e9\"; for (var i = 0; i < 23; i++) s += s; h = h + /y/.test(s);",
    );
}
