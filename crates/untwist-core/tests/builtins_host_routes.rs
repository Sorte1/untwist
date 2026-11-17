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

#[test]
fn a_timer_named_member_on_any_receiver_is_judged_like_the_free_timer() {
    assert_not_intact(
        &[
            "document.defaultView.setTimeout('atob = f');",
            "var w = document.defaultView; w.setInterval('atob = f', 5);",
            "o.setImmediate('atob = f');",
            "o['setTimeout']('atob = f');",
            "o[`setTimeout`]('atob = f');",
            "o?.setTimeout('atob = f');",
            "const k = 'setTimeout'; o[k]('atob = f');",
            "o.setTimeout`atob = f`;",
            "o.setTimeout.call(o, 'atob = f');",
            "o.setTimeout.apply(o, ['atob = f']);",
            "var t = o.setTimeout; t('atob = f');",
            "var t = o.setTimeout.bind(o); t('atob = f');",
            "var { setTimeout: t } = o; t('atob = f');",
            "e.currentTarget.setTimeout(cb, 0);",
            "addEventListener('load', function (e) { e.currentTarget.setTimeout('atob = f'); });",
            "o.requestAnimationFrame(cb);",
            "o.queueMicrotask(cb);",
            "o.requestIdleCallback(cb);",
            "requestAnimationFrame(cb);",
            "queueMicrotask(cb);",
        ],
        &ATOB,
    );
    assert_intact(
        &[
            "document.defaultView.setTimeout(function () {}, 0);",
            "o.setInterval(() => 1, 5);",
            "function cb() {} o.setTimeout(cb, 0);",
            "var id = o.setTimeout(function () {}, 0); clearTimeout(id);",
            "this.t = o.setTimeout(function () {}, 0);",
            "o.requestAnimationFrame(function () {});",
            "requestAnimationFrame(function () {});",
            "queueMicrotask(() => 1);",
            "var t = typeof o.setTimeout;",
        ],
        &ATOB,
    );
}

#[test]
fn a_strict_function_that_escapes_may_see_the_global_object_as_this() {
    assert_not_intact(
        &[
            "'use strict'; setTimeout(function () { var k = 'atob'; this[k] = f; }, 0);",
            "setTimeout(function () { 'use strict'; var k = 'atob'; this[k] = f; }, 0);",
            "(function () { 'use strict'; function cb() { this[k] = f; } addEventListener('load', cb); })();",
            "(function () { 'use strict'; var cb = function () { this[k] = f; }; requestAnimationFrame(cb); })();",
            "(function () { 'use strict'; var cb = function () { this[k] = f; }; var a = cb; setTimeout(a, 0); })();",
            "(function () { 'use strict'; function cb() { this[k] = f; } cb.call(o); })();",
            "(function () { 'use strict'; function cb() { this[k] = f; } setTimeout(cb.bind(o), 0); })();",
            "(function () { 'use strict'; function cb() { var r = this[k]; } setTimeout(cb, 0); })();",
            "'use strict'; onload = function () { this[k] = f; };",
            "'use strict'; el.onclick = function () { this[k] = f; };",
            "(function () { 'use strict'; var o = {}; o.onload = function () { this[k] = f; }; })();",
            "'use strict'; window.cb = function () { this[k] = f; };",
            "'use strict'; module.exports = function () { this[k] = f; };",
            "(function () { 'use strict'; function make() { return function () { this[k] = f; }; } make(); })();",
            "(function () { 'use strict'; var o = { m: function () { this[k] = f; } }; setTimeout(o.m, 0); })();",
            "(function () { 'use strict'; var o = { m() { this[k] = f; } }; el.addEventListener('x', o.m); })();",
            "(function () { 'use strict'; var o = {}; o.m = function () { this[k] = f; }; setTimeout(o.m, 0); })();",
            "(function () { 'use strict'; [function () { this[k] = f; }].forEach(function (g) { g(); }); })();",
            "'use strict'; setTimeout(function () { this.atob = f; }, 0);",
            "'use strict'; setTimeout(function () { this['atob'] = f; }, 0);",
            "'use strict'; setTimeout(function () { (() => { this[k] = f; })(); }, 0);",
            "'use strict'; function cb() { this[k] = f; }",
            "export default function () { this[k] = f; }",
            "export function cb() { this[k] = f; }",
            "function cb() { this[k] = f; } export { cb };",
        ],
        &ATOB,
    );
    assert_intact(
        &[
            "(function () { 'use strict'; function g() { this[k] = 1; } g(); })();",
            "'use strict'; (function () { this[k] = 1; })();",
            "(function () { 'use strict'; var o = { m: function () { this[k] = 1; } }; o.m(); })();",
            "(function () { 'use strict'; function F() { this[k] = 1; } new F(); })();",
            "(function () { 'use strict'; function F() {} F.prototype.m = function () { this[k] = 1; }; new F().m(); })();",
            "'use strict'; class A { m() { this[k] = 1; } } new A().m();",
            "(function () { 'use strict'; setTimeout(function () { this.x = 1; }, 0); })();",
        ],
        &ATOB,
    );
}
