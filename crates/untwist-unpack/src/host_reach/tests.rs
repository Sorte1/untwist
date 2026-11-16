use super::*;
use oxc_allocator::Allocator;
use oxc_span::SourceType;
use untwist_core::parse::parse_with;
use untwist_core::semantic::build_scoping;

fn refused(text: &str) -> Option<&'static str> {
    let alloc = Allocator::default();
    let parsed = parse_with(&alloc, text, SourceType::cjs());
    let scoping = build_scoping(&parsed.program).scoping;
    program_reach(&parsed.program, &scoping)
}

#[test]
fn keys_that_cannot_name_a_runner_pass() {
    for text in [
        "for (var i = 0; i < a.length; i++) a[i].x = b[i];",
        "var i = 0; i += 2; f(a[i]);",
        "f(a[n - 1]); f(a[n * 2]); f(a[-n]); f(a[n + 1]);",
        "f(a['on' + type]); f(a[type + 'Handler']); f(a[`x${k}`]);",
        "f(o[Symbol.iterator]); f(o[typeof k]);",
        "var t = {}; t[k] = s[k];",
        "for (var k in s) if (s[k]) t[k] = s[k];",
        "f({a: 1, b: 2}[k]); f([1, 2][k]);",
        "function s(array, start) { var index = -1, length = array.length; var result = Array(length); while (++index < length) { result[index] = array[index + start]; } return result; }",
        "var __gn = Object.getOwnPropertyNames; var __c = (cb, mod) => function r() { return mod || (0, cb[__gn(cb)[0]])((mod = { exports: {} }).exports, mod), mod.exports; }; var a = __c({ 'a.js'(e, m) {} });",
        "var __dp = Object.defineProperty; var __gd = Object.getOwnPropertyDescriptor; var __cp = (to, from, desc) => { for (let key of Object.keys(from)) __dp(to, key, { get: () => from[key], enumerable: !(desc = __gd(from, key)) || desc.enumerable }); return to; };",
        "var __dp = Object.defineProperty; var __cp = (to, from) => { for (var keys = Object.keys(from), i = 0, key; i < keys.length; i++) { key = keys[i]; __dp(to, key, { get: ((k) => from[k]).bind(null, key) }); } return to; };",
        "var __r = ((x) => typeof require !== 'undefined' ? require : typeof Proxy !== 'undefined' ? new Proxy(x, { get: (a, b) => (typeof require !== 'undefined' ? require : a)[b] }) : x)(function (x) { throw Error(x); });",
        "function l(e, t) { var n = e[t]; e[t] = n; }",
        "var __ea = (all) => { let t = Object.setPrototypeOf({}, null); for (var n in all) Object.defineProperty(t, n, { get: all[n] }); return t; }; __ea({ a: () => 1 });",
        "var __c = (cb) => cb[k]('x');",
        "(function (modules) { modules[id].call(this, 1); })([function () {}]);",
        "var __gn = Object.getOwnPropertyNames; var __e = (fn, res) => function i() { return fn && (res = (0, fn[__gn(fn)[0]])(fn = 0)), res; }; var a = __e({ 'a.js'() {} });",
        "function isIt(index, object) { return eq(object[index], 1); } function eq(a, b) { return a === b || (a !== a && b !== b); }",
        "var __dp = Object.defineProperty; var __ex = (target, all) => { for (var name in all) __dp(target, name, { get: all[name], enumerable: true }); }; __ex(e, { a: () => a });",
        "var m = {a: 1}; f(m[k]);",
        "x[k] = 1; delete x[k]; x[k]++;",
        "if (o[k] === undefined) {} var n = typeof o[k];",
        "f(String(o[k])); console.log(o[k]); var s = '' + o[k];",
        "var v = o[k]; if (v) g(v.length);",
        "o[k](); new o[k]();",
        "Object.defineProperty(t, k, Object.getOwnPropertyDescriptor(s, k));",
        "Object.defineProperties(t, Object.getOwnPropertyDescriptors(s));",
        "f(Object.getOwnPropertyDescriptor(s, k).enumerable);",
        "var a = Object.defineProperty; for (c in e) { if ((r = a && Object.getOwnPropertyDescriptor(e, c)) && (r.get || r.set)) { a(n, c, r); } else { n[c] = e[c]; } } var r;",
        "f(Reflect.ownKeys(o)); Reflect.construct(A, [], B); typeof Reflect;",
        "f(Reflect.get(o, 'x'));",
        "var { a, b: c } = o; ({ a } = o);",
        "f(typeof x.eval); x.constructor = A;",
    ] {
        assert_eq!(refused(text), None, "{text}");
    }
}

#[test]
fn keys_that_may_name_a_runner_refuse() {
    for text in [
        "(function(){})[`constructor`]('x')();",
        "var k = ['constructor'][0]; (function(){})[k]('x')();",
        "(function(){})[k]('x');",
        "for (var k in {constructor: 0}) (function(){})[k]('x');",
        "var h = f[k]; h('x');",
        "var h = f[k]; var r = {run: h}; r.run('x');",
        "f(g[k]);",
        "return f[k];",
        "var f = () => o[k];",
        "var {constructor: F} = function(){};",
        "var {['constr' + 'uctor']: F} = function(){};",
        "var F; ({constructor: F} = function(){});",
        "({constructor} = function(){});",
        "var {[k]: F} = o;",
        "Reflect.get(function(){}, 'constructor')('x')();",
        "Reflect.get(function(){}, k)('x');",
        "var R = Reflect;",
        "f(Reflect);",
        "Object.getOwnPropertyDescriptor(F, 'constructor').value('x');",
        "Object.getOwnPropertyDescriptor(F, k).value('x');",
        "var d = Object.getOwnPropertyDescriptor(F, k); if (d.get) d.get('x');",
        "var d = Object.getOwnPropertyDescriptor(F, k); f(d.value);",
        "Object.entries(Object.getOwnPropertyDescriptors(globalThis));",
        "var g = globalThis; g.eval('x');",
        "globalThis[`eval`]('x');",
        "this.Function('x');",
        "constructor('x');",
        "o[k].call(null, 'x');",
        "o[k].bind(null)('x');",
        "o[k]`x`;",
        "x = o[k];",
        "var m = {}; m[k] = v; f(m[j]);",
        "var m = {}; Object.setPrototypeOf(m, F); f(m[j]);",
        "var __c = (cb) => cb[k]('x'); __c(function(){});",
        "var __c = (cb) => cb[k]('x'); __c({}); var g = __c;",
        "(function (modules) { arguments[0] = F; modules[id]('x'); })([]);",
        "var p = new Proxy(f, { get: (a, b) => a[c] });",
        "var t = Object.setPrototypeOf({}, F); f(t[k]);",
        "var __e = (fn) => (fn || g)[k]('x'); __e({});",
        "function g(a) { return a; } g(o[k])('x');",
        "function g(a) { a('x'); } g(o[k]);",
        "(function (m) { var n = m; Object.setPrototypeOf(n, F); m[k]('x'); })({});",
        "var __gd = Object.getOwnPropertyDescriptor; f(__gd);",
        "var __gd = Object.getOwnPropertyDescriptor; __gd(F, k).value('x');",
        "var __dp = Object.defineProperty; __dp(t, j, { get: () => s[k] });",
        "var __dp = Object.defineProperty; __dp(t, k, { set: () => s[k] });",
    ] {
        assert!(refused(text).is_some(), "{text}");
    }
}

#[test]
fn a_with_statement_refuses() {
    assert!(refused("with (o) { f(); }").is_some());
}
