use super::*;
use oxc_allocator::Allocator;
use oxc_span::SourceType;
use untwist_core::parse::parse_with;
use untwist_core::semantic::build_scoping;

fn reached(text: &str, trusted: Option<bool>) -> Option<&'static str> {
    let alloc = Allocator::default();
    let parsed = parse_with(&alloc, text, SourceType::cjs());
    let scoping = build_scoping(&parsed.program).scoping;
    let intact = crate::host_keys::key_builtins_intact(&parsed.program, &scoping);
    let _keys = crate::host_keys::trust_key_builtins(trusted.unwrap_or(intact));
    program_reach(&parsed.program, &scoping)
}

fn refused(text: &str) -> Option<&'static str> {
    reached(text, None)
}

#[test]
fn keys_that_cannot_name_a_runner_pass() {
    for text in [
        "for (var i = 0; i < a.length; i++) a[i].x = b[i];",
        "var i = 0; i += 2; f(a[i]);",
        "f(a[n - 1]); f(a[n * 2]); f(a[-n]); f(a[n + 1]);",
        "f(a['on' + type]); f(a[type + 'Handler']); f(a[`x${k}`]);",
        "f(o[typeof k]); f(o[`\\x61${k}`]); f(o[`${k}\\x61`]);",
        "Object.defineProperty(t, k, Object.getOwnPropertyDescriptor(s, k)); Reflect.defineProperty(t, k, Object.getOwnPropertyDescriptor(s, k));",
        "var t = {}; t[k] = s[k];",
        "var t = {}; for (var k in s) if (s[k]) t[k] = s[k];",
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
        "f(Reflect.ownKeys(o)); Reflect.construct(A, [], B); typeof Reflect;",
        "f(Reflect.get(o, 'x'));",
        "var { a, b: c } = o; ({ a } = o);",
        "f(typeof x.eval); x.constructor = A;",
        "var t = {}; var ks = Object.keys(s); for (var i = 0; i < ks.length; i++) t[ks[i]] = s[ks[i]];",
        "var __defProp = Object.defineProperty; var __getOwnPropSymbols = Object.getOwnPropertySymbols; var __hasOwnProp = Object.prototype.hasOwnProperty; var __propIsEnum = Object.prototype.propertyIsEnumerable; var __defNormalProp = (obj, key, value) => key in obj ? __defProp(obj, key, { enumerable: true, configurable: true, writable: true, value }) : obj[key] = value; var __spreadValues = (a, b) => { for (var prop in b ||= {}) if (__hasOwnProp.call(b, prop)) __defNormalProp(a, prop, b[prop]); if (__getOwnPropSymbols) for (var prop of __getOwnPropSymbols(b)) { if (__propIsEnum.call(b, prop)) __defNormalProp(a, prop, b[prop]); } return a; };",
        "var dst = {}; for (var k in src) if (src.hasOwnProperty(k)) dst[k] = src[k];",
        "var test2 = {}; for (var i = 0; i < 10; i++) { test2['_' + String.fromCharCode(i)] = i; } var order2 = Object.getOwnPropertyNames(test2).map(function (n) { return test2[n]; });",
        "var m = {}; m[k] = 'x'; m[j] += 1; m[n]++; m[q] = typeof v; f(m[z]);",
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
        "var ks = Object.keys(s); t[ks[i]] = s[ks[j]];",
        "var ks = Object.keys(s); t[ks[i]] = r[ks[i]]; f(s[ks[i]]);",
        "var ks = foo.keys(s); t[ks[i]] = s[ks[i]];",
        "var ks = s.list; t[ks[i]] = s[ks[i]];",
        "var lk = foo.keys; var ks = lk(s); t[ks[i]] = s[ks[i]];",
        "var ks = Object.keys(s); t[ks[i + 1]] = s[ks[i + 1]];",
        "var m = {}; m[k] = {}; f(m[j]);",
        "var m = {}; m[k] ||= v; f(m[j]);",
        "var m = {}; m[k] = null; f(m[j]);",
    ] {
        assert!(refused(text).is_some(), "{text}");
    }
}

#[test]
fn built_in_keys_pass_only_while_the_built_ins_are_trusted() {
    for text in [
        "f(o[Symbol.iterator]);",
        "f(o[Symbol.for(k)]);",
        "f(o[Math.max(a, b)]);",
        "f(o[Date.now()]);",
        "f(o[/x/]);",
        "f(o[parseInt(k)]);",
    ] {
        assert_eq!(reached(text, Some(true)), None, "{text}");
        assert!(reached(text, Some(false)).is_some(), "{text}");
    }
}

#[test]
fn the_sixth_review_shapes_refuse() {
    for text in [
        "(function(){})[`\\x63onstructor${z}`]('x')();",
        "var g = globalThis; g[`\\x65val${z}`]('x');",
        "(function(){})[`\\x63onstructor` + z]('x')();",
        "(function(){})[z + `\\x6fr`]('x')();",
        "Reflect.get(...[P, K])('x')();",
        "Object.getOwnPropertyDescriptor(...[P, K]).value('x')();",
        "Reflect.get(...[P, K], 'x')('x')();",
        "var h = {defineProperty: function(a, b, d) { return d.value; }}; h.defineProperty(0, K, Object.getOwnPropertyDescriptor(P, K))('x')();",
        "var h = {create: function(a, d) { return d; }}; var ds = h.create(0, Object.getOwnPropertyDescriptors(P)); ds[K].value('x')();",
        "var h = {defineProperty: function(a, b, d) { return d; }}; var dp = h.defineProperty; dp(0, K, Object.getOwnPropertyDescriptor(P, K)).value('x')();",
        "var ds = Object.create(...[Object.getOwnPropertyDescriptors(P)]); ds[K].value('x')();",
        "var t = {}; t[K] = P[K]; Object.values(t)[0]('x')();",
        "var t = {}; var v = P[K]; t[K] = v; f(t);",
        "a.b[K] = P[K];",
        "for (var k in s) t[k] = s[k];",
        "var n = {}; var a = Object.defineProperty; for (c in e) { if ((r = a && Object.getOwnPropertyDescriptor(e, c)) && (r.get || r.set)) { a(n, c, r); } else { n[c] = e[c]; } } var r;",
        "Symbol.foo = K; (function(){})[Symbol.foo]('x')();",
        "Math.foo = function() { return K; }; (function(){})[Math.foo()]('x')();",
        "RegExp.prototype.toString = function() { return K; }; (function(){})[/x/]('x')();",
        "Object.prototype[Symbol.toPrimitive] = function() { return K; }; (function(){})[/x/]('x')();",
        "parseInt = function() { return K; }; (function(){})[parseInt(z)]('x')();",
        "(function(){})[Symbol.keyFor(Symbol.for(K))]('x')();",
        "var gs = Object.getOwnPropertySymbols; var pe = Object.prototype.propertyIsEnumerable; function assign(t) { var from, symbols; for (var s = 1; s < arguments.length; s++) { from = Object(arguments[s]); for (var key in from) { if (hasOwnProperty.call(from, key)) { t[key] = from[key]; } } if (gs) { symbols = gs(from); for (var i = 0; i < symbols.length; i++) { if (pe.call(from, symbols[i])) { t[symbols[i]] = from[symbols[i]]; } } } } return t; }",
        "var __assign = Object.assign || function (t) { for (var s, i = 1, n = arguments.length; i < n; i++) { s = arguments[i]; for (var p in s) if (Object.prototype.hasOwnProperty.call(s, p)) t[p] = s[p]; } return t; };",
        "function _extends() { _extends = Object.assign ? Object.assign.bind() : function (target) { for (var i = 1; i < arguments.length; i++) { var source = arguments[i]; for (var key in source) { if (Object.prototype.hasOwnProperty.call(source, key)) { target[key] = source[key]; } } } return target; }; return _extends.apply(this, arguments); }",
    ] {
        assert!(refused(text).is_some(), "{text}");
    }
}

#[test]
fn a_with_statement_refuses() {
    assert!(refused("with (o) { f(); }").is_some());
}
