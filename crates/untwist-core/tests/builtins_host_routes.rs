use oxc_allocator::Allocator;
use untwist_core::parse::parse_program;
use untwist_core::semantic::builtins::{scan, Builtins, Reliance};
use untwist_core::semantic::{build_scoping, has_module_syntax};

const ARRAY_CONCAT: Reliance = Reliance {
    roots: &["Array"],
    members: &[
        ("Array", false, &["prototype"]),
        ("Array", true, &["concat"]),
    ],
};

const ATOB: Reliance = Reliance {
    roots: &["atob"],
    members: &[],
};

fn verdict_for(source: &str, reliance: &Reliance) -> Builtins {
    let alloc = Allocator::default();
    let parsed = parse_program(&alloc, source);
    let sem = build_scoping(&parsed.program);
    let module = has_module_syntax(&parsed.program);
    scan(&parsed.program, &sem.scoping, module, false, reliance).verdict
}

fn assert_not_intact(sources: &[&str], reliance: &Reliance) {
    let intact: Vec<&str> = sources
        .iter()
        .copied()
        .filter(|source| verdict_for(source, reliance) == Builtins::Intact)
        .collect();
    assert!(intact.is_empty(), "{intact:#?}");
}

fn assert_intact(sources: &[&str], reliance: &Reliance) {
    let refused: Vec<&str> = sources
        .iter()
        .copied()
        .filter(|source| verdict_for(source, reliance) != Builtins::Intact)
        .collect();
    assert!(refused.is_empty(), "{refused:#?}");
}

#[test]
fn a_tagged_template_is_judged_like_a_call() {
    assert_not_intact(
        &[
            "var p = [].__proto__.valueOf``; p.concat = f;",
            "[].__proto__.valueOf``.concat = f;",
            "[].__proto__.valueOf`${0}`.concat = f;",
            "Array.prototype.valueOf``.concat = f;",
            "[].__proto__.reverse``.concat = f;",
            "[].__proto__.fill``.concat = f;",
            "[].constructor.valueOf``.prototype.concat = f;",
            "Object.assign([].__proto__.valueOf``, { concat: f });",
            "(function () { return this.valueOf``; })().Reflect.getPrototypeOf([]).concat = f;",
        ],
        &ARRAY_CONCAT,
    );
    assert_not_intact(
        &[
            "var g = globalThis.valueOf``; var k = 'atob'; g[k] = f;",
            "globalThis.valueOf``.eval('atob = f');",
            "(function () { this.valueOf``.eval('atob = f'); })();",
            "(function () {}).constructor.valueOf``('atob = f')();",
            "var g = globalThis.valueOf``; g.Object.defineProperty(g, 'atob', { value: f });",
            "require('vm').runInThisContext`atob = f`;",
            "setTimeout`atob = f`;",
            "globalThis.setInterval`atob = f`;",
        ],
        &ATOB,
    );
    assert_intact(
        &[
            "var s = String.raw`a${1}b`;",
            "var t = tag`x`;",
            "f`a`.length;",
        ],
        &ATOB,
    );
}

#[test]
fn a_legacy_definer_under_any_key_form_on_a_reached_receiver_is_judged() {
    assert_not_intact(
        &[
            "[].__proto__['__defineGetter__']('concat', g);",
            "[].__proto__[`__defineGetter__`]('concat', g);",
            "[].__proto__?.['__defineGetter__']('concat', g);",
            "[].__proto__?.__defineGetter__('concat', g);",
            "[].constructor.prototype['__defineGetter__']('concat', g);",
            "[].__proto__['__defineSetter__']('concat', g);",
            "[].__proto__['__define' + 'Getter__']('concat', g);",
            "const k = '__defineGetter__'; [].__proto__[k]('concat', g);",
            "({}).__proto__['__defineGetter__']('concat', g);",
            "var l = [].__proto__['__lookupGetter__']('concat');",
            "var l = [].__proto__[`__lookupSetter__`]('concat');",
        ],
        &ARRAY_CONCAT,
    );
}

#[test]
fn a_context_runner_taken_as_a_value_is_a_code_runner() {
    assert_not_intact(
        &[
            "var r = require('vm').runInThisContext; r('atob = f');",
            "var { runInThisContext } = require('vm'); runInThisContext('atob = f');",
            "var { runInThisContext: r } = require('vm'); r('atob = f');",
            "var { ['runInThisContext']: r } = require('vm'); r('atob = f');",
            "require('vm').runInThisContext.call(null, 'atob = f');",
            "require('vm')['runInThisContext'].apply(null, ['atob = f']);",
            "var vm = require('vm'); var s = new vm.Script('atob = f'); var run = s.runInThisContext.bind(s); run();",
            "var c = require('vm').compileFunction('atob = f'); c();",
            "var n = require('vm').runInNewContext;",
            "module._compile('atob = f', 'x.js');",
            "module['_compile']('atob = f', 'x.js');",
            "var c = module._compile; c.call(module, 'atob = f', 'x.js');",
            "var { _compile } = module;",
        ],
        &ATOB,
    );
}

#[test]
fn a_timer_callback_is_a_function_only_when_it_is_never_rewritten() {
    assert_not_intact(
        &[
            "function cb() {} cb = 'atob = f'; setTimeout(cb, 0);",
            "setTimeout(cb2, 0); function cb2() {} cb2 = 'atob = f';",
            "function cb() {} var cb = 'atob = f'; setTimeout(cb, 0);",
            "var cb = function () {}; cb = 'atob = f'; setTimeout(cb, 0);",
            "let cb = () => 1; cb = 'atob = f'; setTimeout(cb, 0);",
            "setTimeout({ bind: function () { return 'atob = f'; } }.bind(), 0);",
            "setTimeout(o.bind(null), 0);",
            "setTimeout(o.m.bind(o), 0);",
            "function run(cb) { setTimeout(cb, 0); }",
            "var cb = make(); setTimeout(cb, 0);",
        ],
        &ATOB,
    );
    assert_intact(
        &[
            "function cb() {} setTimeout(cb, 0);",
            "var cb = function () {}; setTimeout(cb, 0);",
            "const cb = () => 1; setInterval(cb, 5);",
            "setTimeout(function () { return 1; }.bind(null), 0);",
            "setTimeout((() => 1).bind(null), 0);",
            "function tick() {} setTimeout(tick.bind(null, 1), 0);",
            "var cb = function () {}; setTimeout(cb.bind(null).bind(null), 0);",
        ],
        &ATOB,
    );
}

#[test]
fn a_runner_name_read_off_any_receiver_is_a_code_runner() {
    assert_not_intact(
        &[
            "document.defaultView.eval('atob = f');",
            "var w = document.defaultView; w.Function('atob = f')();",
            "addEventListener('load', function (e) { e.currentTarget.eval('atob = f'); });",
            "setTimeout(function () { 'use strict'; this.eval('atob = f'); }, 0);",
            "onload = function () { 'use strict'; this.eval('atob = f'); };",
            "requestAnimationFrame(function () { 'use strict'; this.eval('atob = f'); });",
            "'use strict'; setTimeout(function () { this.Function('atob = f')(); }, 0);",
            "var e = opener.eval;",
            "var F = o['Function'];",
            "o.execScript('atob = f');",
            "var c = e.currentTarget.constructor;",
        ],
        &ATOB,
    );
    assert_intact(
        &[
            "var t = typeof o.evaluate;",
            "o.evalScript = 1;",
            "o.functions();",
        ],
        &ATOB,
    );
}
