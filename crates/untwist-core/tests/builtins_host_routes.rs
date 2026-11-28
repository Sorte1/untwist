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
            "(function () { function cb() {} setTimeout(cb, 0); })();",
            "(function () { var cb = function () {}; setTimeout(cb, 0); })();",
            "const cb = () => 1; setInterval(cb, 5);",
            "let cb = function () {}; setTimeout(cb, 0);",
            "export function cb() {} setTimeout(cb, 0);",
            "setTimeout(function () {}, 0);",
            "setTimeout(() => 1, 0);",
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
            "(function () { function cb() {} o.setTimeout(cb, 0); })();",
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
            "(function () { 'use strict'; function F() {} F.prototype.m = function () { this[k] = 1; }; new F().m(); })();",
            "(function () { 'use strict'; var w = document.defaultView; w.m = function () { var k = 'atob'; this[k] = f; }; w.m(); })();",
            "addEventListener('load', function (e) { 'use strict'; e.currentTarget.m = function () { var k = 'atob'; this[k] = f; }; e.currentTarget.m(); });",
            "(function () { 'use strict'; var f = function () { var k = 'atob'; this[k] = g; }; (f?.call)(document.defaultView); })();",
            "(function () { 'use strict'; (function n() { var k = 'atob'; this[k] = g; }?.call)(document.defaultView); })();",
            "(function () { 'use strict'; function B() { return document.defaultView; } class A extends B { constructor() { super(); var k = 'atob'; this[k] = f; } } new A(); })();",
        ],
        &ATOB,
    );
    assert_intact(
        &[
            "(function () { 'use strict'; function g() { this[k] = 1; } g(); })();",
            "'use strict'; (function () { this[k] = 1; })();",
            "(function () { 'use strict'; var o = { m: function () { this[k] = 1; } }; o.m(); })();",
            "(function () { 'use strict'; function F() { this[k] = 1; } new F(); })();",
            "(function () { 'use strict'; var o = {}; o.m = function () { this[k] = 1; }; o.m(); })();",
            "'use strict'; class A { constructor() { this[k] = 1; } } new A();",
            "(function () { 'use strict'; setTimeout(function () { this.x = 1; }, 0); })();",
        ],
        &ATOB,
    );
}

#[test]
fn a_code_runner_module_is_judged_by_the_module_not_the_member() {
    assert_not_intact(
        &[
            "import { runInThisContext as r } from 'vm'; r('atob = f');",
            "import { 'runInThisContext' as r } from 'vm'; r('atob = f');",
            "import { runInThisContext } from 'node:vm'; runInThisContext('atob = f');",
            "import vm, { compileFunction as c } from 'vm'; c('atob = f')();",
            "import * as ns from 'vm'; Object.values(ns)[0]('atob = f');",
            "import 'vm';",
            "import { Worker } from 'worker_threads';",
            "import { createRequire } from 'node:module';",
            "export * from 'vm';",
            "export { runInThisContext as r } from 'vm';",
            "const { default: vm } = await import('vm'); Object.values(vm)[0]('atob = f');",
            "import('node:repl');",
            "var fns = Object.values(require('vm')); fns[0]('atob = f');",
            "for (var fn of Object.values(require('vm'))) fn('atob = f');",
            "new Map(Object.entries(require('vm'))).get('runInThisContext')('atob = f');",
            "var o = Object.assign({}, require('node:vm'));",
            "var { ...rest } = require('vm');",
            "var vm = require('vm');",
            "require('module');",
            "require('node:worker_threads');",
            "require('child_process');",
            "require('repl');",
            "var p = make(); require(p);",
            "require('a' + b);",
            "var r = require; r('vm');",
            "require.call(null, 'vm');",
            "module.require('vm');",
            "var q = module.require; q('fs');",
            "var { require: q } = module; q('vm');",
            "process.getBuiltinModule('vm');",
            "process.getBuiltinModule('node:vm');",
            "process.getBuiltinModule(name);",
            "process.getBuiltinModule?.('vm');",
            "var m = process.getBuiltinModule('vm'); m.runInThisContext('atob = f');",
            "var g = typeof process.getBuiltinModule === 'function' && process.getBuiltinModule; g('vm');",
            "var g = process.getBuiltinModule; g('fs');",
            "var { getBuiltinModule } = process;",
            "var cr = m.createRequire;",
            "createRequire(import.meta.url)('vm');",
            "M.prototype._compile.call(mm, 'atob = f', 'x.js');",
        ],
        &ATOB,
    );
    assert_intact(
        &[
            "var fs = require('fs'); fs.readFileSync('x');",
            "var path = require('node:path');",
            "import fs from 'fs';",
            "import { join } from 'node:path';",
            "export { join } from 'node:path';",
            "import('./x.js');",
            "if (typeof require === 'function') {}",
            "if (require.main === module) {}",
            "process.getBuiltinModule('fs');",
            "process.getBuiltinModule('node:inspector');",
            "function h() { var m = process.getBuiltinModule('fs'); return m.readFileSync('x'); }",
            "var i = typeof process.getBuiltinModule === 'function' ? process.getBuiltinModule('node:inspector') : null;",
            "if (typeof process.getBuiltinModule === 'function') {}",
        ],
        &ATOB,
    );
}

#[test]
fn a_write_to_a_window_alias_under_a_key_the_scan_cannot_resolve_is_judged() {
    assert_not_intact(
        &[
            "var k = 'atob'; document.defaultView[k] = f;",
            "document.defaultView.Object.defineProperty(document.defaultView, 'at' + 'ob', { value: f });",
            "document.defaultView.Object.defineProperty(document.defaultView, k, { value: f });",
            "addEventListener('load', function (e) { var k = 'atob'; e.currentTarget[k] = f; });",
            "Object.assign(document.defaultView, { atob: f });",
            "Object.defineProperty(e.currentTarget, k, { value: f });",
            "Reflect.set(frame.contentWindow, k, f);",
            "var w = document.defaultView; w[k] = f;",
            "var w; w = e.currentTarget; var v = w; v[k] = f;",
            "var { defaultView: w } = document; w[k] = f;",
            "var w; ({ contentWindow: w } = frame); w[k] = f;",
            "opener[k] = f;",
            "e.view.__defineGetter__(k, f);",
        ],
        &ATOB,
    );
    assert_intact(
        &[
            "document.defaultView.x = f;",
            "(function () { var w = document.defaultView; w.foo = 1; })();",
            "var o = {}; o[k] = f;",
            "e.currentTarget.className = 'x';",
            "e.target.value = '';",
        ],
        &ATOB,
    );
}

#[test]
fn a_strict_function_the_escape_walk_does_not_follow_may_see_the_global_object_as_this() {
    assert_not_intact(
        &[
            "'use strict'; (function (cb = function () { var k = 'atob'; this[k] = f; }) { addEventListener('load', cb); })();",
            "'use strict'; (function () { var { cb = function () { this[k] = f; } } = {}; addEventListener('load', cb); })();",
            "'use strict'; (function () { var cb; [cb = function () { this[k] = f; }] = []; addEventListener('load', cb); })();",
            "'use strict'; (async function () { var cb = await function () { this[k] = f; }; addEventListener('load', cb); })();",
            "class C { m() { var k = 'atob'; this[k] = f; } } addEventListener('load', new C().m);",
            "'use strict'; (function () { class D { static f = function () { this[k] = f; }; } addEventListener('load', D.f); })();",
            "'use strict'; (function () { function cb() { this[k] = f; } var { x = cb } = {}; addEventListener('load', x); })();",
            "'use strict'; (function () { function cb() { this[k] = f; } (function (x = cb) { addEventListener('load', x); })(); })();",
            "'use strict'; (async function () { function cb() { this[k] = f; } var x = await cb; addEventListener('load', x); })();",
            "'use strict'; (function () { function cb() { this[k] = f; } class D { static f = cb; } addEventListener('load', D.f); })();",
            "class C { m() { this.atob = f; } }",
        ],
        &ATOB,
    );
    assert_intact(
        &[
            "'use strict'; class A { constructor() { this.x = 1; } } new A();",
            "class A { m() { this.x = 1; } } new A().m();",
            "'use strict'; (function () { this[k] = 1; })();",
            "'use strict'; new (function () { this[k] = 1; })();",
        ],
        &ATOB,
    );
}

#[test]
fn a_timer_callback_bound_or_rebindable_through_the_global_object_is_not_a_function() {
    assert_not_intact(
        &[
            "function cb() {} cb.bind = function () { return 'atob = f'; }; setTimeout(cb.bind(), 0);",
            "function cb() {} window.cb = 'atob = f'; setTimeout(cb, 0);",
            "function cb() {} this.cb = 'atob = f'; setTimeout(cb, 0);",
            "function cb() {} setTimeout(cb, 0);",
            "var cb = function () {}; setTimeout(cb, 0);",
            "setTimeout(function () { return 1; }.bind(null), 0);",
            "setTimeout((() => 1).bind(null), 0);",
            "(function () { function tick() {} setTimeout(tick.bind(null, 1), 0); })();",
            "setTimeout(make(), 0);",
            "o.setTimeout(cb.bind(o), 0);",
        ],
        &ATOB,
    );
}

#[test]
fn a_callback_declared_in_a_loop_head_is_not_a_proven_function() {
    assert_not_intact(
        &[
            "(function () { for (var cb = function () {} in { 'atob = f': 1 }) ; setTimeout(cb, 0); })();",
            "(function () { for (var cb in { 'atob = f': 1 }) ; setTimeout(cb, 0); })();",
        ],
        &ATOB,
    );
    assert_intact(
        &["(function () { var cb = function () {}; setTimeout(cb, 0); })();"],
        &ATOB,
    );
}

#[test]
fn a_loader_imported_by_name_or_process_taken_as_a_value_is_judged() {
    assert_not_intact(
        &[
            "import { getBuiltinModule as g } from 'node:process'; g('vm');",
            "import { 'getBuiltinModule' as g } from 'process'; g('vm');",
            "import { default as p } from 'process'; p.getBuiltinModule('vm');",
            "import p from 'process'; Object.values(p);",
            "import * as ns from 'process'; Object.values(ns);",
            "export * from 'process';",
            "export { getBuiltinModule } from 'process';",
            "import { createRequire } from './x.js';",
            "await import('process');",
            "var g = Object.values(process).filter(Boolean)[0];",
            "Object.values(require('process'));",
            "var { ...r } = process;",
            "f(process);",
            "for (var k in process) ;",
            "var o = { ...process };",
            "var p = globalThis.process; p.mainModule;",
            "process.binding('contextify');",
            "x = process[k];",
        ],
        &ATOB,
    );
    assert_intact(
        &[
            "if (typeof process !== 'undefined' && process.env.NODE_ENV) {}",
            "console.log(process.argv.length, process.env.HOME);",
            "process.exitCode = 1;",
            "var env = require('process').env;",
            "import { argv } from 'node:process'; console.log(argv);",
            "process.on('exit', function () {});",
        ],
        &ATOB,
    );
}

#[test]
fn a_specifier_the_url_parser_normalises_is_judged_after_normalising() {
    assert_not_intact(
        &[
            "import 'da\\tta:text/javascript,atob = f';",
            "import '\\x01data:text/javascript,atob = f';",
            "import ' data:text/javascript,atob = f';",
            "await import('d\\nata:text/javascript,atob = f');",
            "import('DATA:text/javascript,atob = f');",
            "import('weird-scheme:atob');",
            "require('no\\tde:vm');",
            "require('node:vm?x');",
            "require('vm/');",
        ],
        &ATOB,
    );
    assert_intact(
        &[
            "import './x.js';",
            "import 'node:fs';",
            "import 'https://example.com/x.js';",
            "require('fs');",
        ],
        &ATOB,
    );
}

#[test]
fn a_window_alias_used_as_a_value_makes_the_result_unknown() {
    assert_not_intact(
        &[
            "(function (w) { var k = 'atob'; w[k] = f; })(document.defaultView);",
            "var a = [document.defaultView]; var k = 'atob'; a[0][k] = f;",
            "var k = 'atob'; var o = { w: document.defaultView }; o.w[k] = f;",
            "function id() { return document.defaultView; } var k = 'atob'; id()[k] = f;",
            "var id = () => document.defaultView; var k = 'atob'; id()[k] = f;",
            "addEventListener('load', function (e) { var k = 'atob'; e.composedPath()[0][k] = f; });",
            "var k = 'atob'; open('', '_self')[k] = f;",
            "const r = 'defaultView'; var w = document[r]; var k = 'atob'; w[k] = f;",
            "var k = 'atob'; [document.defaultView][0][k] = f;",
            "for (var w of [document.defaultView]) { var k = 'atob'; w[k] = f; }",
            "var w; [w] = [document.defaultView]; var k = 'atob'; w[k] = f;",
            "var w = document.defaultView; g(w); o[k] = 1;",
            "var w = document.defaultView; var k = 'atob'; window.w[k] = f;",
            "x = document.defaultView; var k = 'atob'; globalThis.x[k] = f;",
        ],
        &ATOB,
    );
    assert_intact(
        &[
            "(function () { var w = document.defaultView; console.log(w.innerWidth); w.focus(); })();",
            "(function () { var w = document.defaultView; if (w && typeof w === 'object') {} })();",
            "el.addEventListener('click', function (e) { console.log(e.target.id); });",
            "var m = { exports: {} }; console.log(m.exports.self, m.parent); f(e.target);",
        ],
        &ATOB,
    );
}

#[test]
fn deleting_a_global_slot_is_a_write() {
    assert_eq!(
        verdict_for("delete globalThis.atob;", &ATOB),
        Builtins::Written
    );
    assert_eq!(
        verdict_for("delete window['atob'];", &ATOB),
        Builtins::Written
    );
    assert_eq!(verdict_for("delete atob;", &ATOB), Builtins::Written);
}
