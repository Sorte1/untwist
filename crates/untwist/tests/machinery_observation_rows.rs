use std::path::Path;
use untwist::{Options, Pipeline};

use untwist_testing::node_rows::{
    node_or_skip, patched_after_rotator, printed, with_machinery_restored, Mode,
};

const ALWAYS: &str = "1 === 1";

const ALIASED_NATIVES: [(&str, &str); 7] = [
    ("aliased console", "var o = console; o.log(this);"),
    (
        "console from an array",
        "var o = [console][0]; o.dir(this, { depth: 0 });",
    ),
    ("aliased dir", "var c = console; c.dir(this, { depth: 0 });"),
    (
        "callee on an arguments object",
        "function f(q) { if (q === 1) { arguments.callee(this); } else { console.log(q); } } f(1);",
    ),
    ("host global beside this", "console.log(this, URL);"),
    ("host object before this", "console.log(TextEncoder, this);"),
    (
        "call on a native read into a binding",
        "var c = console.log; c.call(console, this);",
    ),
];

const HELD_GLOBALS: [(&str, &str); 17] = [
    ("object literal", "console.log({ g: this });"),
    ("array literal", "console.log([this]);"),
    ("bound object literal", "var o = { g: this }; console.log(o);"),
    ("written field", "var o = {}; o.g = this; console.log(o);"),
    (
        "constructed object",
        "function F() { this.g = globalThis; } console.log(new F());",
    ),
    (
        "returned through a forwarder",
        "var m = { show: function (x) { return x; } }; console.log(m.show([this]));",
    ),
    (
        "returned object",
        "function mk() { return { g: globalThis }; } console.log(mk());",
    ),
    (
        "field through an alias",
        "function fn() {} var g = fn; g.x = this; console.log(fn);",
    ),
    (
        "field written by a helper",
        "function fn() {} function put(t) { t.x = globalThis; } put(fn); console.log(fn);",
    ),
    (
        "custom inspector",
        "var o = { k: 1 }; var s = Symbol.for('nodejs.util.inspect.custom'); o[s] = function () { return globalThis; }; console.log(o);",
    ),
    (
        "custom inspector through an alias",
        "function fn() {} var t = fn; var s = Symbol.for('nodejs.util.inspect.custom'); t[s] = function () { return globalThis; }; console.log(fn);",
    ),
    ("regexp field", "var r = /x/; r.g = this; console.log(r);"),
    (
        "overwritten array iterator",
        "Array.prototype[Symbol.iterator] = function* () { yield globalThis; }; console.log([...[1]]);",
    ),
    (
        "function prototype inspector",
        "Function.prototype[Symbol.for('nodejs.util.inspect.custom')] = function () { return globalThis; }; function mk() { return function () {}; } console.log(mk());",
    ),
    (
        "static getter",
        "console.dir(class { static get g() { return globalThis; } }, { showHidden: true, getters: true });",
    ),
    ("static field", "console.log(class { static g = globalThis; });"),
    (
        "reassigned parameter",
        "function f(x) { x = globalThis; return x; } console.log(f(1));",
    ),
];

const FUNCTION_SCOPE: [(&str, &str); 6] = [
    (
        "arguments beside globalThis",
        "function f() { console.log(arguments, globalThis); } f(1);",
    ),
    (
        "arguments length beside globalThis",
        "function f() { console.log(arguments.length, globalThis); } f(1);",
    ),
    (
        "arguments beside this",
        "function f() { console.log(arguments, this); } f(1);",
    ),
    (
        "new.target beside globalThis",
        "function f() { console.log(new.target, globalThis); } f();",
    ),
    (
        "this member beside globalThis",
        "function f() { console.log(this.length, globalThis); } f();",
    ),
    (
        "super member beside globalThis",
        "var o = { m() { console.log(super.toString === Object.prototype.toString, globalThis); } }; o.m();",
    ),
];

const CREATED_NAMES: [(&str, &str); 4] = [
    ("global property", "this.zz = console; zz.log(this);"),
    (
        "global element",
        "globalThis['zz'] = console.log; zz(this);",
    ),
    (
        "function text",
        "new Function('zz = console')(); zz.log(this);",
    ),
    (
        "script text",
        "process.mainModule.require('vm').runInThisContext('var zz = console'); zz.log(this);",
    ),
];

const CONTROLLERS: [(&str, &str); 4] = [
    (
        "plain controller",
        "var ctl = function () { var flag = true; return function (ctx, fn) { var r = flag ? function () { if (fn) { var res = fn.apply(ctx, arguments); fn = null; return res; } } : function () {}; flag = false; return r; }; }(); var g = ctl(this, function () { return 1; }); g();",
    ),
    (
        "controller behind an opaque if",
        "var ctl = function () { if ('ab' !== 'ab') { return function (ctx) { console.log(Object.keys(ctx).length > 0); }; } else { var flag = true; return function (ctx, fn) { var r = flag ? function () { if (fn) { var res = fn.apply(ctx, arguments); fn = null; return res; } } : function () {}; flag = false; return r; }; } }(); var g = ctl(this, function () { return 1; }); g();",
    ),
    (
        "live branch returns the global",
        "var ctl = function () { if ('a' === 'a') { return function () { return this; }; } else { return function () { return function () {}; }; } }(); console.log(Object.keys(ctl()).indexOf('a') >= 0);",
    ),
    (
        "printed controller result",
        "var ctl = function () { if ('a' !== 'a') { return function () { return function () {}; }; } return function () { return globalThis; }; }(); console.log(ctl());",
    ),
];

const FORWARDERS: [(&str, &str); 9] = [
    (
        "forwarded user function",
        "var p = { K: function (f, x) { return f(x); } }; p.K(function () {}, this);",
    ),
    (
        "forwarded printer",
        "var p = { K: function (f, x) { return f(x); } }; p.K(console.log, this);",
    ),
    (
        "forwarder read as a value",
        "var p = { K: function (f, x) { return f(x); } }; var g = p.K; g(console.log, this);",
    ),
    (
        "forwarder under a literal key",
        "var p = { K: function (f, x) { return f(x); } }; p['K'](console.log, this);",
    ),
    (
        "forwarder under a computed key",
        "var p = { K: function (f, x) { return f(x); } }; var k = 'K'; p[k](console.log, this);",
    ),
    (
        "forwarder through call",
        "var p = { K: function (f, x) { return f(x); } }; p.K.call(null, console.log, this);",
    ),
    (
        "forwarder reached through the global object",
        "var p = { K: function (f, x) { return f(x); } }; this.p.K(console.log, this);",
    ),
    (
        "forwarder-shaped printer",
        "var p = { K: function (f, x) { return console.log(x); } }; p.K(function () {}, this);",
    ),
    (
        "forwarder in an escaping literal",
        "var p = { K: function (f, x) { return f(x); } }; var q = p; q.K(console.log, this);",
    ),
];

const CLEARED: [&str; 3] = [
    "plain controller",
    "controller behind an opaque if",
    "forwarded user function",
];

fn transformed(options: &Options, source: &str) -> String {
    Pipeline::new(options.clone())
        .run(source)
        .code()
        .to_string()
}

fn unsafe_options() -> Options {
    Options {
        unsafe_transforms: true,
        ..Options::default()
    }
}

fn programs(rows: &[(&str, &str)]) -> Vec<(String, String)> {
    rows.iter()
        .map(|(label, patch)| (label.to_string(), patched_after_rotator(ALWAYS, patch)))
        .collect()
}

fn agrees_with_node(rows: &[(&str, &str)]) {
    let Some(node) = node_or_skip() else {
        return;
    };
    let safe = Options::default();
    let unsafe_transforms = unsafe_options();
    let run_safe = |source: &str| transformed(&safe, source);
    let run_unsafe =
        |source: &str| with_machinery_restored(source, &transformed(&unsafe_transforms, source));
    let modes: [Mode<'_>; 2] = [("default", &run_safe), ("unsafe", &run_unsafe)];
    let wrong = stable_mismatches(&node, &programs(rows), &modes);
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

const VOLATILE_KEYS: [&str; 10] = [
    "pid",
    "ppid",
    "timeOrigin",
    "duration",
    "nodeStart",
    "v8Start",
    "bootstrapComplete",
    "environment",
    "loopStart",
    "idleTime",
];

const GETTER_PREFIXES: [&str; 2] = ["[Getter: ", "[Getter/Setter: "];

fn volatile_value(value: &str) -> Option<(usize, usize)> {
    let from = GETTER_PREFIXES
        .iter()
        .find_map(|prefix| value.strip_prefix(prefix))
        .map_or(0, |inner| value.len() - inner.len());
    let number = &value[from..];
    let digits = number
        .find(|c: char| !(c.is_ascii_digit() || c == '.' || c == '-' || c == 'e'))
        .unwrap_or(number.len());
    (digits > 0).then_some((from, from + digits))
}

fn volatile_at(text: &str, at: usize) -> Option<(usize, usize)> {
    let before = text[..at].chars().next_back();
    if before.is_some_and(|c| c.is_alphanumeric() || c == '_' || c == '$') {
        return None;
    }
    let rest = &text[at..];
    VOLATILE_KEYS.iter().find_map(|key| {
        let value = rest.strip_prefix(key)?.strip_prefix(": ")?;
        let offset = rest.len() - value.len();
        let (from, to) = volatile_value(value)?;
        Some((offset + from, offset + to))
    })
}

fn stable(printed: &str) -> String {
    let mut out = String::with_capacity(printed.len());
    let mut at = 0;
    while let Some(offset) = printed[at..].find(|c: char| c.is_ascii_alphabetic()) {
        let start = at + offset;
        match volatile_at(printed, start) {
            Some((from, to)) => {
                out.push_str(&printed[at..start + from]);
                out.push('#');
                at = start + to;
            }
            None => {
                let next = start + printed[start..].chars().next().map_or(1, char::len_utf8);
                out.push_str(&printed[at..next]);
                at = next;
            }
        }
    }
    out.push_str(&printed[at..]);
    out
}

fn stable_printed(node: &Path, dir: &Path, text: &str) -> Result<String, String> {
    printed(node, dir, text).map(|text| stable(&text))
}

fn stable_mismatches(
    node: &Path,
    programs: &[(String, String)],
    modes: &[Mode<'_>],
) -> Vec<String> {
    let dir = tempfile::tempdir().unwrap_or_else(|error| panic!("a scratch dir: {error}"));
    let mut wrong = Vec::new();
    for (label, source) in programs {
        let before = stable_printed(node, dir.path(), source);
        for (mode, transform) in modes {
            let after = stable_printed(node, dir.path(), &transform(source));
            if before.is_err() || after.is_err() || after != before {
                wrong.push(format!(
                    "{label} ({mode}): node printed {before:?}, untwist's output printed {after:?}"
                ));
            }
        }
    }
    wrong
}

#[test]
fn the_comparison_masks_only_numbers_that_change_between_node_runs() {
    let printed = "process { pid: 41, ppid: 7, title: 'node', apid: 3 }\n  timeOrigin: 1761234567.891,\n  pid: [Getter]\n";
    let masked =
        "process { pid: #, ppid: #, title: 'node', apid: 3 }\n  timeOrigin: #,\n  pid: [Getter]\n";
    assert_eq!(stable(printed), masked);
    assert_eq!(
        stable("{ g: {} }\nlog!\nexit 0\n"),
        "{ g: {} }\nlog!\nexit 0\n"
    );
}

#[test]
fn the_comparison_masks_timings_node_prints_through_a_getter() {
    let printed = "    timeOrigin: [Getter: 1793207166667.996],\n      nodeStart: [Getter: 0.6048080027103424],\n      loopStart: [Getter: -1],\n    hardwareConcurrency: [Getter: 32],\n    pid: [Getter/Setter: 2482335]\n";
    let masked = "    timeOrigin: [Getter: #],\n      nodeStart: [Getter: #],\n      loopStart: [Getter: #],\n    hardwareConcurrency: [Getter: 32],\n    pid: [Getter/Setter: #]\n";
    assert_eq!(stable(printed), masked);
}

fn kept_in_default(rows: &[(&str, &str)]) -> Vec<(String, bool)> {
    programs(rows)
        .into_iter()
        .map(|(label, source)| {
            let code = transformed(&Options::default(), &source);
            (label, code.contains("function a()"))
        })
        .collect()
}

fn assert_kept(rows: &[(&str, &str)]) {
    let removed: Vec<String> = kept_in_default(rows)
        .into_iter()
        .filter(|(_, kept)| !kept)
        .map(|(label, _)| label)
        .collect();
    assert!(
        removed.is_empty(),
        "removed under observation: {}",
        removed.join(", ")
    );
}

#[test]
fn a_native_reached_through_an_alias_runs_like_node() {
    agrees_with_node(&ALIASED_NATIVES);
}

#[test]
fn a_native_reached_through_an_alias_keeps_the_machinery() {
    assert_kept(&ALIASED_NATIVES);
}

#[test]
fn a_value_holding_the_global_object_runs_like_node() {
    agrees_with_node(&HELD_GLOBALS);
}

#[test]
fn a_value_holding_the_global_object_keeps_the_machinery() {
    assert_kept(&HELD_GLOBALS);
}

#[test]
fn names_every_function_scope_defines_run_like_node() {
    agrees_with_node(&FUNCTION_SCOPE);
}

#[test]
fn names_every_function_scope_defines_keep_the_machinery() {
    assert_kept(&FUNCTION_SCOPE);
}

#[test]
fn a_name_created_at_run_time_runs_like_node() {
    agrees_with_node(&CREATED_NAMES);
}

#[test]
fn a_name_created_at_run_time_keeps_the_machinery() {
    assert_kept(&CREATED_NAMES);
}

#[test]
fn call_controllers_run_like_node() {
    agrees_with_node(&CONTROLLERS);
}

#[test]
fn object_literal_forwarders_run_like_node() {
    agrees_with_node(&FORWARDERS);
}

fn kept_mismatches(rows: &[(&str, &str)]) -> Vec<String> {
    kept_in_default(rows)
        .into_iter()
        .filter(|(label, kept)| *kept == CLEARED.contains(&label.as_str()))
        .map(|(label, kept)| format!("{label}: kept {kept}"))
        .collect()
}

#[test]
fn a_forwarder_is_judged_at_its_call_sites() {
    let wrong = kept_mismatches(&FORWARDERS);
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

#[test]
fn a_call_controller_behind_an_opaque_if_does_not_keep_the_machinery() {
    let kept: Vec<String> = kept_in_default(&CONTROLLERS)
        .into_iter()
        .filter(|(label, kept)| *kept == CLEARED.contains(&label.as_str()))
        .map(|(label, kept)| format!("{label}: kept {kept}"))
        .collect();
    assert!(kept.is_empty(), "{}", kept.join("\n"));
}
