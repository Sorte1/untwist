use untwist::{Options, Pipeline};
use untwist_testing::node_rows::{self, node_or_skip, patched_after_rotator, Mode};

const JOIN_TEST: &str = "[1, 2].join('') === '12'";
const NODE_PRINTS: &str = "log?\nHello World!?\ndeobfuscate?\nexit 0\n";

const CHAINS: [(&str, &str); 15] = [
    ("optional call", "var o = Object.freeze?.(Array); o.prototype.join = function () { return 'X'; };"),
    ("optional member", "var o = Object?.freeze(Array); o.prototype.join = function () { return 'X'; };"),
    ("optional alias call", "var fr = Object.freeze; var o = fr?.(Array); o.prototype.join = function () { return 'X'; };"),
    ("optional prototype", "var p = Array?.prototype; p.join = function () { return 'X'; };"),
    ("optional getPrototypeOf member", "var p = Object?.getPrototypeOf([]); p.join = function () { return 'X'; };"),
    ("optional getPrototypeOf call as a write path", "(Object.getPrototypeOf?.([])).join = function () { return 'X'; };"),
    ("optional getPrototypeOf call", "var p = Object.getPrototypeOf?.([]); p.join = function () { return 'X'; };"),
    ("optional call of call", "var o = Object.freeze?.call(null, Array); o.prototype.join = function () { return 'X'; };"),
    ("optional global object member", "var A = globalThis?.Array; A.prototype.join = function () { return 'X'; };"),
    ("optional constructor read", "var o = [].constructor?.prototype; o.join = function () { return 'X'; };"),
    ("optional call of an alias's call", "var fr = Object.freeze; var o = fr?.call(null, Array); o.prototype.join = function () { return 'X'; };"),
    ("optional computed member", "var o = Object?.['freeze'](Array); o.prototype.join = function () { return 'X'; };"),
    ("optional assign", "var o = Object.assign?.(Array, {}); o.prototype.join = function () { return 'X'; };"),
    ("optional getPrototypeOf call of call", "var p = Object.getPrototypeOf?.call(null, []); p.join = function () { return 'X'; };"),
    ("optional call after call", "var o = Object.freeze.call?.(null, Array); o.prototype.join = function () { return 'X'; };"),
];

const ARGUMENTS: [(&str, &str); 20] = [
    ("spread", "var o = Object.freeze(...[Array]); o.prototype.join = function () { return 'X'; };"),
    ("spread through an alias", "var fr = Object.freeze; var o = fr(...[Array]); o.prototype.join = function () { return 'X'; };"),
    ("spread after call", "var o = Object.freeze.call(null, ...[Array]); o.prototype.join = function () { return 'X'; };"),
    ("apply of a built list", "var o = Object.freeze.apply(null, Array.of(Array)); o.prototype.join = function () { return 'X'; };"),
    ("bound argument", "var o = Object.freeze.bind(null, Array)(); o.prototype.join = function () { return 'X'; };"),
    ("bound argument held", "var g = Object.freeze.bind(null, Array); var o = g(); o.prototype.join = function () { return 'X'; };"),
    ("call of call", "var fr = Object.freeze; var o = Function.prototype.call.call(fr, null, Array); o.prototype.join = function () { return 'X'; };"),
    ("alias call of call", "var fr = Object.freeze; var o = fr.call.call(fr, null, Array); o.prototype.join = function () { return 'X'; };"),
    ("call of call on the built-in", "var o = Object.freeze.call.call(Object.freeze, null, Array); o.prototype.join = function () { return 'X'; };"),
    ("Reflect.apply", "var o = Reflect.apply(Object.freeze, null, [Array]); o.prototype.join = function () { return 'X'; };"),
    ("Reflect.apply of assign", "var o = Reflect.apply(Object.assign, null, [Array]); o.prototype.join = function () { return 'X'; };"),
    ("spread receiver of call", "var o = Object.freeze.call(...[null, Array]); o.prototype.join = function () { return 'X'; };"),
    ("spread receiver of apply", "var o = Object.freeze.apply(...[null, [Array]]); o.prototype.join = function () { return 'X'; };"),
    ("alias apply of a built list", "var fr = Object.freeze; var o = fr.apply(null, Array.of(Array)); o.prototype.join = function () { return 'X'; };"),
    ("apply through call", "var o = Object.freeze.call.apply(Object.freeze, [null, Array]); o.prototype.join = function () { return 'X'; };"),
    ("call through apply", "var o = Object.freeze.apply.call(Object.freeze, null, [Array]); o.prototype.join = function () { return 'X'; };"),
    ("spread of a binding", "var a = [Array]; var o = Object.freeze(...a); o.prototype.join = function () { return 'X'; };"),
    ("Reflect.apply of a built list", "var o = Reflect.apply(Object.freeze, null, Array.of(Array)); o.prototype.join = function () { return 'X'; };"),
    ("spread inside an applied list", "var o = Object.freeze.apply(null, [...[Array]]); o.prototype.join = function () { return 'X'; };"),
    ("apply of a list built later", "function f() { var o = fr.apply(null, Array.of(Array)); o.prototype.join = function () { return 'X'; }; } var fr = Object.freeze; f();"),
];

const HANDED: [(&str, &str); 3] = [
    ("callback", "var o = [Array].map(Object.freeze)[0]; o.prototype.join = function () { return 'X'; };"),
    ("handed to a function", "var o = (function (f) { return f(Array); })(Object.freeze); o.prototype.join = function () { return 'X'; };"),
    ("call bound to the built-in", "var o = Object.freeze.call.bind(Object.freeze)(null, Array); o.prototype.join = function () { return 'X'; };"),
];

const PROTOTYPE_OF: [(&str, &str); 16] = [
    ("spread", "var p = Object.getPrototypeOf(...[[]]); p.join = function () { return 'X'; };"),
    ("call", "var p = Object.getPrototypeOf.call(null, []); p.join = function () { return 'X'; };"),
    ("alias", "var gp = Object.getPrototypeOf; var p = gp([]); p.join = function () { return 'X'; };"),
    ("apply", "var p = Object.getPrototypeOf.apply(null, [[]]); p.join = function () { return 'X'; };"),
    ("Reflect call", "var p = Reflect.getPrototypeOf.call(null, []); p.join = function () { return 'X'; };"),
    ("destructured", "const { getPrototypeOf } = Object; var p = getPrototypeOf([]); p.join = function () { return 'X'; };"),
    ("bound", "var p = Object.getPrototypeOf.bind(null)([]); p.join = function () { return 'X'; };"),
    ("sequence callee", "var p = (0, Object.getPrototypeOf)([]); p.join = function () { return 'X'; };"),
    ("Reflect alias", "var gp = Reflect.getPrototypeOf; var p = gp([]); p.join = function () { return 'X'; };"),
    ("alias spread", "var gp = Object.getPrototypeOf; var p = gp(...[[]]); p.join = function () { return 'X'; };"),
    ("Reflect.apply", "var p = Reflect.apply(Object.getPrototypeOf, null, [[]]); p.join = function () { return 'X'; };"),
    ("alias assigned later", "var gp; function f() { var p = gp([]); p.join = function () { return 'X'; }; } gp = Object.getPrototypeOf; f();"),
    ("bound alias", "var gp = Object.getPrototypeOf.bind(Object); var p = gp([]); p.join = function () { return 'X'; };"),
    ("destructured from Reflect", "var { getPrototypeOf: gp } = Reflect; var p = gp([]); p.join = function () { return 'X'; };"),
    ("create of a spread", "var o = Object.create(...[Array]); o.prototype.join = function () { return 'X'; };"),
    ("setPrototypeOf of an assigned target", "var o = Reflect.setPrototypeOf(x = {}, Array) && x; var x; o.prototype.join = function () { return 'X'; };"),
];

const REPLACED: [(&str, &str); 22] = [
    ("freeze replaced", "Object.freeze = function () { return Array; }; var o = Object.freeze(String); o.prototype.join = function () { return 'X'; };"),
    ("freeze replaced through an alias", "var O = Object; O.freeze = function () { return Array; }; var o = Object.freeze(String); o.prototype.join = function () { return 'X'; };"),
    ("freeze defined", "Object.defineProperty(Object, 'freeze', { value: function () { return Array; } }); var o = Object.freeze(String); o.prototype.join = function () { return 'X'; };"),
    ("getPrototypeOf replaced", "Object.getPrototypeOf = function () { return Array.prototype; }; var o = Object.getPrototypeOf(''); o.join = function () { return 'X'; };"),
    ("freeze assigned", "Object.assign(Object, { freeze: function () { return Array; } }); var o = Object.freeze(String); o.prototype.join = function () { return 'X'; };"),
    ("freeze defined in a batch", "Object.defineProperties(Object, { freeze: { value: function () { return Array; } } }); var o = Object.freeze(String); o.prototype.join = function () { return 'X'; };"),
    ("freeze set by Reflect", "Reflect.set(Object, 'freeze', function () { return Array; }); var o = Object.freeze(String); o.prototype.join = function () { return 'X'; };"),
    ("Object replaced on the global object", "globalThis.Object = { freeze: function () { return Array; } }; var o = Object.freeze(String); o.prototype.join = function () { return 'X'; };"),
    ("Object replaced", "Object = { freeze: function () { return Array; } }; var o = Object.freeze(String); o.prototype.join = function () { return 'X'; };"),
    ("create replaced", "Object.create = function () { return Array.prototype; }; var o = Object.create(String.prototype); o.join = function () { return 'X'; };"),
    ("Reflect.getPrototypeOf replaced", "Reflect.getPrototypeOf = function () { return Array.prototype; }; var o = Reflect.getPrototypeOf(''); o.join = function () { return 'X'; };"),
    ("freeze replaced under a computed key", "Object['freeze'] = function () { return Array; }; var o = Object.freeze(String); o.prototype.join = function () { return 'X'; };"),
    ("freeze replaced under a held key", "var k = 'freeze'; Object[k] = function () { return Array; }; var o = Object.freeze(String); o.prototype.join = function () { return 'X'; };"),
    ("freeze given a getter", "Object.__defineGetter__('freeze', function () { return function () { return Array; }; }); var o = Object.freeze(String); o.prototype.join = function () { return 'X'; };"),
    ("String replaced on the global object", "globalThis.String = Array; String.prototype.join = function () { return 'X'; };"),
    ("String replaced", "String = Array; String.prototype.join = function () { return 'X'; };"),
    ("freeze replaced, called on a primitive", "Object.freeze = function () { return Array; }; var o = Object.freeze(1); o.prototype.join = function () { return 'X'; };"),
    ("seal replaced", "Object.seal = function () { return Array; }; var o = Object.seal(String); o.prototype.join = function () { return 'X'; };"),
    ("Object defined on the global object", "Object.defineProperty(globalThis, 'Object', { value: { freeze: function () { return Array; } } }); var o = Object.freeze(String); o.prototype.join = function () { return 'X'; };"),
    ("Object replaced through a global alias", "var g = globalThis; g.Object = { freeze: function () { return Array; } }; var o = Object.freeze(String); o.prototype.join = function () { return 'X'; };"),
    ("Reflect.apply replaced", "Reflect.apply = function () { return Array; }; var o = Reflect.apply(Object.freeze, null, [String]); o.prototype.join = function () { return 'X'; };"),
    ("getPrototypeOf replaced through an alias", "var O = Object; O.getPrototypeOf = function () { return Array.prototype; }; var p = Object.getPrototypeOf(''); p.join = function () { return 'X'; };"),
];

const HELD_KEYS: [(&str, &str); 4] = [
    (
        "a held prototype key",
        "var k = 'prototype'; Array[k].join = function () { return 'X'; };",
    ),
    (
        "a held key on an alias",
        "var k = 'prototype'; var A = Array; A[k].join = function () { return 'X'; };",
    ),
    (
        "a held constructor key",
        "var k = 'constructor'; [][k].prototype.join = function () { return 'X'; };",
    ),
    (
        "a held __proto__ key",
        "var k = '__proto__'; [][k].join = function () { return 'X'; };",
    ),
];

const DATA: [(&str, &str); 24] = [
    ("a built-in held in an object literal", "var h = {f: Object.freeze}; var o = h.f(Array); o.prototype.join = function () { return 'X'; };"),
    ("a built-in returned by a getter", "var h = {get f() { return Object.freeze; }}; var o = h.f(Array); o.prototype.join = function () { return 'X'; };"),
    ("a built-in held in a static class field", "class C { static fr = Object.freeze; } var o = C.fr(Array); o.prototype.join = function () { return 'X'; };"),
    ("an alias held in an object literal", "var fr = Object.freeze; var h = {f: fr}; var o = h.f(Array); o.prototype.join = function () { return 'X'; };"),
    ("a built-in assigned to a member", "var h = {}; h.f = Object.freeze; var o = h.f(Array); o.prototype.join = function () { return 'X'; };"),
    ("a built-in read back into a binding", "var h = {f: Object.freeze}; var g = h.f; var o = g(Array); o.prototype.join = function () { return 'X'; };"),
    ("a class getter returning a built-in", "class C { static get fr() { return Object.freeze; } } var o = C.fr(Array); o.prototype.join = function () { return 'X'; };"),
    ("a prototype defined as a value", "var o = Object.defineProperty({}, 'p', {value: Array.prototype}); o.p.join = function () { return 'X'; };"),
    ("a prototype assigned from a source", "var o = Object.assign({}, {p: Array.prototype}); o.p.join = function () { return 'X'; };"),
    ("a prototype defined in a batch", "var o = Object.defineProperties({}, {p: {value: Array.prototype}}); o.p.join = function () { return 'X'; };"),
    ("a prototype held in frozen data", "var o = Object.freeze({p: Array.prototype}); o.p.join = function () { return 'X'; };"),
    ("a prototype assigned onto a bare object", "var o = Object.assign(Object.create(null), {p: Array.prototype}); o.p.join = function () { return 'X'; };"),
    ("a prototype returned by a defined getter", "var o = Object.defineProperty({}, 'p', {get: function () { return Array.prototype; }}); o.p.join = function () { return 'X'; };"),
    ("a prototype defined under the prototype key", "var o = Object.defineProperty({}, 'prototype', {value: Array.prototype}); o.prototype.join = function () { return 'X'; };"),
    ("a constructor held in an object literal", "var h = {A: Array}; h.A.prototype.join = function () { return 'X'; };"),
    ("a constructor held in an array literal", "var h = [Array]; h[0].prototype.join = function () { return 'X'; };"),
    ("a prototype held in an object literal", "var h = {p: Array.prototype}; h.p.join = function () { return 'X'; };"),
    ("a prototype assigned to a member", "var h = {}; h.p = Array.prototype; h.p.join = function () { return 'X'; };"),
    ("a prototype read back into a binding", "var h = {p: Array.prototype}; var P = h.p; P.join = function () { return 'X'; };"),
    ("a constructor read back into a binding", "var h = {A: Array}; var A = h.A; A.prototype.join = function () { return 'X'; };"),
    ("a prototype held in data and defined on", "var h = {p: Array.prototype}; Object.defineProperty(h.p, 'join', {value: function () { return 'X'; }});"),
    ("a prototype held in data read through destructuring", "var h = {p: Array.prototype}; var {p} = h; p.join = function () { return 'X'; };"),
    ("a prototype set by Reflect", "var h = {}; Reflect.set(h, 'p', Array.prototype); h.p.join = function () { return 'X'; };"),
    ("a prototype held in a returned object", "function mk() { return {}; } var m = mk(); m.p = Array.prototype; m.p.join = function () { return 'X'; };"),
];

const ALIASES_HANDED: [(&str, &str); 6] = [
    ("an alias handed to a callback", "var fr = Object.freeze; var o = [Array].map(fr)[0]; o.prototype.join = function () { return 'X'; };"),
    ("an alias handed to a function", "var fr = Object.freeze; var o = (function (f) { return f(Array); })(fr); o.prototype.join = function () { return 'X'; };"),
    ("an alias bound to call", "var fr = Object.freeze; var c = Function.prototype.call.bind(fr); var o = c(null, Array); o.prototype.join = function () { return 'X'; };"),
    ("an alias handed to new", "var fr = Object.freeze; function C(f) { this.o = f(Array); } var o = new C(fr).o; o.prototype.join = function () { return 'X'; };"),
    ("an alias spread into a call", "var fr = Object.freeze; var o = (function (f) { return f(Array); })(...[fr]); o.prototype.join = function () { return 'X'; };"),
    ("a getPrototypeOf alias handed to a function", "var gp = Object.getPrototypeOf; var p = (function (g) { return g([]); })(gp); p.join = function () { return 'X'; };"),
];

const RETURNED: [(&str, &str); 5] = [
    ("an alias assigned after a function returning its call", "var fr; function g() { return fr(Array); } fr = Object.freeze; var o = g(); o.prototype.join = function () { return 'X'; };"),
    ("an alias's call returned by a function", "var fr = Object.freeze; function g() { return fr(Array); } var o = g(); o.prototype.join = function () { return 'X'; };"),
    ("an alias's call returned by an arrow", "var fr = Object.freeze; var g = () => fr(Array); var o = g(); o.prototype.join = function () { return 'X'; };"),
    ("a getPrototypeOf alias's call returned", "var gp = Object.getPrototypeOf; function g() { return gp([]); } var p = g(); p.join = function () { return 'X'; };"),
    ("a held built-in's call returned", "var h = {f: Object.freeze}; function g() { return h.f(Array); } var o = g(); o.prototype.join = function () { return 'X'; };"),
];

const BOUND_KEYS: [(&str, &str); 2] = [
    (
        "a held prototype key through a binding",
        "var k = 'prototype'; var p = Array[k]; p.join = function () { return 'X'; };",
    ),
    (
        "a held prototype key through a binding and an alias",
        "var k = 'prototype'; var P = Array[k]; var Q = P; Q.join = function () { return 'X'; };",
    ),
];

const HELD: [(&str, &str); 10] = [
    ("a held object's method writing through this", "var h = {p: Array.prototype, m: function () { this.p.join = function () { return 'X'; }; }}; h.m();"),
    ("a held object read through an alias", "var h = {}; h.p = Array.prototype; var g = h; g.p.join = function () { return 'X'; };"),
    ("a held object given a value by defineProperty", "var h = {}; Object.defineProperty(h, 'p', {value: Array.prototype}); h.p.join = function () { return 'X'; };"),
    ("a held object's getter", "var h = {get p() { return Array.prototype; }}; h.p.join = function () { return 'X'; };"),
    ("a returned object assigned onto", "function mk() { return {}; } var m = mk(); Object.assign(m, {p: Array.prototype}); m.p.join = function () { return 'X'; };"),
    ("a held object stored in another", "var h = {p: Array.prototype}; var k = {q: h}; k.q.p.join = function () { return 'X'; };"),
    ("a held object inherited from", "var h = {p: Array.prototype}; var o = Object.create(h); o.p.join = function () { return 'X'; };"),
    ("a held array read through a conditional", "var h = [Array.prototype]; var c = true ? h : 0; c[0].join = function () { return 'X'; };"),
    ("a held object set as a literal's __proto__", "var h = {}; var o = {__proto__: h}; h.p = Array.prototype; o.p.join = function () { return 'X'; };"),
    ("a held object spread into another", "var h = {p: Array.prototype}; var o = {...h}; o.p.join = function () { return 'X'; };"),
];

const ESCAPED: [(&str, &str); 13] = [
    ("a held object returned by a function", "var h = {}; function g() { return h; } h.p = Array.prototype; g().p.join = function () { return 'X'; };"),
    ("a held object returned by a function expression", "var h = {}; var a = function () { return h; }; h.p = Array.prototype; a().p.join = function () { return 'X'; };"),
    ("a held object returned by an arrow", "var h = {}; var a = () => h; h.p = Array.prototype; a().p.join = function () { return 'X'; };"),
    ("a call's result read through the object it returns", "var box = {}; function f() { return box; } var m = f(); m.p = Array.prototype; box.p.join = function () { return 'X'; };"),
    ("a call's result read through another call", "var box = {}; function f() { return box; } var m = f(); m.p = Array.prototype; f().p.join = function () { return 'X'; };"),
    ("a call's result built on the global object", "function f() { return globalThis.box || (globalThis.box = {}); } var m = f(); m.p = Array.prototype; f().p.join = function () { return 'X'; };"),
    ("a held object reached through a conditional assignment", "var h = {}; var c; c = 1 ? h : 0; h.p = Array.prototype; c.p.join = function () { return 'X'; };"),
    ("a held object thrown and caught", "var h = {}; try { throw h; } catch (e) { h.p = Array.prototype; e.p.join = function () { return 'X'; }; }"),
    ("a held object yielded", "var h = {}; function* g() { yield h; } h.p = Array.prototype; g().next().value.p.join = function () { return 'X'; };"),
    ("a held object as a parameter default", "var h = {}; function g(a = h) { return a; } h.p = Array.prototype; g().p.join = function () { return 'X'; };"),
    ("a held object through a reassigned alias", "var h = {}; var e = h; e = 0; h.p = Array.prototype; (function () { return h; })().p.join = function () { return 'X'; };"),
    ("a held object returned by a method", "var h = {}; var e = {m() { return h; }}; h.p = Array.prototype; e.m().p.join = function () { return 'X'; };"),
    ("a binding reassigned to another object", "var h = {}; var e = {}; e.p = Array.prototype; h = e; h.p.join = function () { return 'X'; };"),
];

const FOLDS: [(&str, &str); 10] = [
    ("a definer held in data", "var h = {d: Object.defineProperty}; h.d(String.prototype, 'charCodeAt', {value: function () { return 'X'; }});"),
    ("a definer alias", "var dp = Object.defineProperty; dp(String.prototype, 'charCodeAt', {value: function () { return 'X'; }});"),
    ("a prototype held in data", "var h = {p: String.prototype}; h.p.charCodeAt = function () { return 'X'; };"),
    ("a definer alias through call", "var dp = Object.defineProperty; dp.call(null, String.prototype, 'charCodeAt', {value: function () { return 'X'; }});"),
    ("Reflect.set alias", "var rs = Reflect.set; rs(String.prototype, 'charCodeAt', function () { return 'X'; });"),
    ("a symbol key on Object.prototype through a held key", "var k = 'prototype'; var P = Object[k]; var s = 'sp' + 'lit'; P[Symbol[s]] = function () { return ['X']; };"),
    ("isConcatSpreadable on Object.prototype", "var k = 'prototype'; var P = Object[k]; P[Symbol.isConcatSpreadable] = false;"),
    ("a computed delete on Object.prototype", "var k = 'prototype'; var P = Object[k]; var s = 'to' + 'String'; delete P[s];"),
    ("a computed delete on a spelled Object.prototype", "var P = Object.prototype; var s = 'value' + 'Of'; delete P[s];"),
    ("toString replaced on Object.prototype", "var k = 'prototype'; var P = Object[k]; var t = 'toString'; P[t] = function () { return 'X'; };"),
];

const ESCAPES: [(&str, &str); 6] = [
    ("a prototype returned by a held method", "var ns = {get: function () { return Array.prototype; }}; ns.get().join = function () { return 'X'; };"),
    ("a constructor returned by a function", "function mk() { return Array; } mk().prototype.join = function () { return 'X'; };"),
    ("a constructor handed to a parameter", "(function (A) { A.prototype.join = function () { return 'X'; }; })(Array);"),
    ("a prototype held in data handed to a parameter", "var h = {p: Array.prototype}; (function (P) { P.join = function () { return 'X'; }; })(h.p);"),
    ("a getPrototypeOf result handed to a parameter", "var P = Object.getPrototypeOf([]); (function (Q) { Q.join = function () { return 'X'; }; })(P);"),
    ("a held prototype written by code text", "var h = {}; var e = Promise; h.p = Array.prototype; new Function('h', 'h.p.join = function () { return \"X\"; }')(h);"),
];

const ESCAPED_FOLDS: [(&str, &str); 21] = [
    ("a prototype returned by a held method", "var ns = {get: function () { return Array.prototype; }}; ns.get().join = function () { return 'X'; };"),
    ("a constructor returned by a function", "function mk() { return Array; } mk().prototype.join = function () { return 'X'; };"),
    ("a constructor handed to a parameter", "(function (A) { A.prototype.join = function () { return 'X'; }; })(Array);"),
    ("a prototype held in data handed to a parameter", "var h = {p: Array.prototype}; (function (P) { P.join = function () { return 'X'; }; })(h.p);"),
    ("a getPrototypeOf result handed to a parameter", "var P = Object.getPrototypeOf([]); (function (Q) { Q.join = function () { return 'X'; }; })(P);"),
    ("a held prototype written by code text", "var h = {}; h.p = Array.prototype; new Function('h', 'h.p.join = function () { return \"X\"; }')(h);"),
    ("code text called without new", "var h = {}; h.p = Array.prototype; Function('h', 'h.p.join = function () { return \"X\"; }')(h);"),
    ("a realm name held in a binding", "var k = 'Array'; globalThis[k].prototype.join = function () { return 'X'; };"),
    ("setPrototypeOf through call", "var o = Object.setPrototypeOf.call(null, {}, Array); o.prototype.join = function () { return 'X'; };"),
    ("a setPrototypeOf alias", "var sp = Object.setPrototypeOf; var o = {}; sp(o, Array); o.prototype.join = function () { return 'X'; };"),
    ("a Reflect.setPrototypeOf alias", "var sp = Reflect.setPrototypeOf; var o = {}; sp(o, Array); o.prototype.join = function () { return 'X'; };"),
    ("setPrototypeOf through apply", "var q = {}; Object.setPrototypeOf.apply(null, [q, Array]); q.prototype.join = function () { return 'X'; };"),
    ("Reflect.set with a receiver", "Reflect.set({}, 'join', function () { return 'X'; }, Array.prototype);"),
    ("a Reflect.deleteProperty alias", "var d = Reflect.deleteProperty; d(Array.prototype, 'join');"),
    ("an assign alias with a literal source", "var as = Object.assign; as(Array.prototype, {join: function () { return 'X'; }});"),
    ("__defineGetter__ through call", "var g = Object.prototype.__defineGetter__; g.call(Array.prototype, 'join', function () { return function () { return 'X'; }; });"),
    ("a constructor handed to a declared function", "function f(A) { A.prototype.join = function () { return 'X'; }; } f(Array);"),
    ("a prototype returned into a binding", "function mk() { return Array.prototype; } var P = mk(); P.join = function () { return 'X'; };"),
    ("a data key held in a binding", "var h = {}; h.p = Array.prototype; var k = 'p'; h[k].join = function () { return 'X'; };"),
    ("a held prototype handed to a declared function", "var h = {p: Array.prototype}; function f(P) { P.join = function () { return 'X'; }; } f(h.p);"),
    ("a prototype handed to a built-in's callback", "var P = Array.prototype; [P].forEach(function (Q) { Q.join = function () { return 'X'; }; });"),
];

const REALM_FOLDS: [(&str, &str); 37] = [
    (
        "the global object returned by a returned function",
        r#"var g = (function () { return function () { return globalThis; }; })(); g().String.prototype.charCodeAt = function () { return 7; };"#,
    ),
    (
        "the global object returned through a callback",
        r#"function k(f) { return f(); } k(function () { return globalThis; }).String.prototype.charCodeAt = function () { return 7; };"#,
    ),
    (
        "the global object returned by a function held in an array",
        r#"[function () { return globalThis; }][0]().String.prototype.charCodeAt = function () { return 7; };"#,
    ),
    (
        "a realm name returned through a callback on the returned global object",
        r#"function k(f) { return f(); } var G = k(function () { return globalThis; }); var n = k(function () { return "String"; }); G[n].prototype.charCodeAt = function () { return 7; };"#,
    ),
    (
        "the global object as a sloppy this",
        r#"(0, function () { return this; })().String.prototype.charCodeAt = function () { return 7; };"#,
    ),
    (
        "the global object returned into a binding",
        r#"function k(f) { return f(); } var G = k(function () { return globalThis; }); G.String.prototype.charCodeAt = function () { return 7; };"#,
    ),
    (
        "defineProperty read off the returned global object",
        r#"function k(f) { return f(); } var G = k(function () { return globalThis; }); G.Object.defineProperty(String.prototype, "charCodeAt", {value: function () { return 7; }});"#,
    ),
    (
        "Reflect.set read off the returned global object",
        r#"function k(f) { return f(); } var G = k(function () { return globalThis; }); G.Reflect.set(String.prototype, "charCodeAt", function () { return 7; });"#,
    ),
    (
        "a constructor read off the returned global object into a binding",
        r#"function k(f) { return f(); } var G = k(function () { return globalThis; }); var S = G.String; S.prototype.charCodeAt = function () { return 7; };"#,
    ),
    (
        "Array read off the returned global object",
        r#"function k(f) { return f(); } var G = k(function () { return globalThis; }); G.Array.prototype.join = function () { return "X"; };"#,
    ),
    (
        "Object read off the returned global object into a binding",
        r#"function k(f) { return f(); } var G = k(function () { return globalThis; }); var O = G.Object; O.assign(String.prototype, {charCodeAt: function () { return 7; }});"#,
    ),
    (
        "Object returned by a callback, then defineProperty",
        r#"function k(f) { return f(); } var O = k(function () { return Object; }); O.defineProperty(String.prototype, "charCodeAt", {value: function () { return 7; }});"#,
    ),
    (
        "Object returned by a callback, then assign",
        r#"function k(f) { return f(); } var O = k(function () { return Object; }); O.assign(String.prototype, {charCodeAt: function () { return 7; }});"#,
    ),
    (
        "Object returned by a callback, then setPrototypeOf",
        r#"function k(f) { return f(); } var O = k(function () { return Object; }); O.setPrototypeOf(String.prototype, {[Symbol.split]: function () { return ["z"]; }});"#,
    ),
    (
        "defineProperty read off a returned Object",
        r#"function k(f) { return f(); } k(function () { return Object; }).defineProperty(String.prototype, "charCodeAt", {value: function () { return 7; }});"#,
    ),
    (
        "Object returned by a callback, then defineProperties",
        r#"function k(f) { return f(); } var O = k(function () { return Object; }); O.defineProperties(Array.prototype, {join: {value: function () { return "X"; }}});"#,
    ),
    (
        "Function returned by a callback runs code text",
        r#"function k(f) { return f(); } var F = k(function () { return Function; }); F("String.prototype.charCodeAt = function () { return 7; }")();"#,
    ),
    (
        "Function returned by a callback runs code text under new",
        r#"function k(f) { return f(); } var F = k(function () { return Function; }); new F("String.prototype.charCodeAt = function () { return 7; }")();"#,
    ),
    (
        "Reflect returned by a callback, then set",
        r#"function k(f) { return f(); } var R = k(function () { return Reflect; }); R.set(String.prototype, "charCodeAt", function () { return 7; });"#,
    ),
    (
        "Reflect returned by a callback, then defineProperty",
        r#"function k(f) { return f(); } var R = k(function () { return Reflect; }); R.defineProperty(String.prototype, "charCodeAt", {value: function () { return 7; }});"#,
    ),
    (
        "Reflect returned by a callback, then setPrototypeOf",
        r#"function k(f) { return f(); } var R = k(function () { return Reflect; }); R.setPrototypeOf(String.prototype, {[Symbol.split]: function () { return ["z"]; }});"#,
    ),
    (
        "defineProperty returned by a callback",
        r#"function k(f) { return f(); } var dp = k(function () { return Object.defineProperty; }); dp(String.prototype, "charCodeAt", {value: function () { return 7; }});"#,
    ),
    (
        "assign returned by a callback and called at once",
        r#"function k(f) { return f(); } k(function () { return Object.assign; })(String.prototype, {charCodeAt: function () { return 7; }});"#,
    ),
    (
        "setPrototypeOf returned by a memoised require",
        r#"var sp; var has; function req() { if (has) return sp; has = 1; return sp = Object.setPrototypeOf || function () {}; } var setProto = req(); setProto(String.prototype, {[Symbol.split]: function () { return ["z"]; }});"#,
    ),
    (
        "defineProperty returned by a memoised require",
        r#"var dp; var has; function req() { if (has) return dp; has = 1; return dp = Object.defineProperty; } var def = req(); def(String.prototype, "charCodeAt", {value: function () { return 7; }});"#,
    ),
    (
        "defineProperty returned by a function and called at once",
        r#"function req() { return Object.defineProperty; } req()(String.prototype, "charCodeAt", {value: function () { return 7; }});"#,
    ),
    (
        "Reflect.set returned by a callback",
        r#"function k(f) { return f(); } var rs = k(function () { return Reflect.set; }); rs(String.prototype, "charCodeAt", function () { return 7; });"#,
    ),
    (
        "assign returned by a function into a binding",
        r#"function req() { return Object.assign; } var as = req(); as(String.prototype, {charCodeAt: function () { return 7; }});"#,
    ),
    (
        "defineProperty returned by a callback, called through call",
        r#"function k(f) { return f(); } var dp = k(function () { return Object.defineProperty; }); dp.call(null, String.prototype, "charCodeAt", {value: function () { return 7; }});"#,
    ),
    (
        "a realm name rebound through a parameter key",
        r#"function setg(key, v) { globalThis[key] = v; } setg("String", Array); String.prototype.join = function () { return "X"; };"#,
    ),
    (
        "a realm name rebound under a built key",
        r#"var k = ["Str", "ing"].join(""); globalThis[k] = Array; String.prototype.join = function () { return "X"; };"#,
    ),
    (
        "a realm name defined on the global object under a built key",
        r#"Object.defineProperty(globalThis, ["Str", "ing"].join(""), {value: Array}); String.prototype.join = function () { return "X"; };"#,
    ),
    (
        "the global object returned by a sloppy function",
        r#"var g = (function () { return this; })(); g.String.prototype.charCodeAt = function () { return 7; };"#,
    ),
    (
        "the global object returned by code text",
        r#"var g = Function("return this")(); g.String.prototype.charCodeAt = function () { return 7; };"#,
    ),
    (
        "a prototype held as a prototype's own value, read under an unknown key",
        r#"Array.prototype.x = String.prototype; var k = ["x"][0]; Array.prototype[k].charCodeAt = function () { return 7; };"#,
    ),
    (
        "a constructor destructured off the returned global object",
        r#"function k(f) { return f(); } var {String: S} = k(function () { return globalThis; }); S.prototype.charCodeAt = function () { return 7; };"#,
    ),
    (
        "defineProperty read off a returned Object into a binding",
        r#"function k(f) { return f(); } var O = k(function () { return Object; }); var dp = O.defineProperty; dp(String.prototype, "charCodeAt", {value: function () { return 7; }});"#,
    ),
];

const RESIDUE_FOLDS: [(&str, &str); 4] = [
    (
        "a constructor alias of another root under the same key (bails)",
        "var N = Number; N.prototype.join = function () { return 'X'; };",
    ),
    (
        "a prototype reached through a fresh root (bails)",
        "var g = {}; g.Array = Array; g.Array.prototype.join = function () { return 'X'; };",
    ),
    (
        "a prototype reached through the global object (bails)",
        "globalThis.Number.prototype.join = function () { return 'X'; };",
    ),
    (
        "an unknown key written on the Array constructor (bails)",
        "var k = 'x'; Array[k] = function () {};",
    ),
];

const REBOUND_ROOTS: [(&str, &str); 13] = [
    (
        "a root rebound by assignment",
        r#"String = Array; String.prototype.join = function () { return "X"; };"#,
    ),
    (
        "a root rebound through a helper",
        r#"function f(o, k, v) { o[k] = v; } f(globalThis, "String", Array); String.prototype.join = function () { return "X"; };"#,
    ),
    (
        "a root rebound through a global alias",
        r#"var g = globalThis; g.String = Array; String.prototype.join = function () { return "X"; };"#,
    ),
    (
        "a root rebound by array destructuring",
        r#"[String] = [Array]; String.prototype.join = function () { return "X"; };"#,
    ),
    (
        "a root rebound by object destructuring",
        r#"({ String } = { String: Array }); String.prototype.join = function () { return "X"; };"#,
    ),
    (
        "a root rebound by a logical assignment",
        r#"String &&= Array; String.prototype.join = function () { return "X"; };"#,
    ),
    (
        "a root rebound by assign",
        r#"Object.assign(globalThis, { String: Array }); String.prototype.join = function () { return "X"; };"#,
    ),
    (
        "a root rebound through a sloppy this",
        r#"(function () { this.String = Array; })(); String.prototype.join = function () { return "X"; };"#,
    ),
    (
        "a root rebound through Function's global",
        r#"var gg = Function('return this')(); gg.String = Array; String.prototype.join = function () { return "X"; };"#,
    ),
    (
        "another root rebound to String",
        "Array = String; Array.prototype.charCodeAt = function () { return 7; };",
    ),
    (
        "Object rebound to String",
        "Object = String; Object.prototype.charCodeAt = function () { return 7; };",
    ),
    (
        "a root rebound in a callback",
        r#"[globalThis].forEach(function (g) { g.String = Array; }); String.prototype.join = function () { return "X"; };"#,
    ),
    (
        "a root rebound through a proxy",
        r#"var p = new Proxy(globalThis, {}); p.String = Array; String.prototype.join = function () { return "X"; };"#,
    ),
];

const REGRESSION_SHAPES: &str = include_str!("data/realm_regressions.txt");

const PROBE_FOLDED: &str = "console.log(\"-\".charCodeAt(0), \"a\".concat(\"b\"), [1, 2].join(\"+\"), \"abc\".split(\"\").reverse().join(\"\"));";

const IDENTITY_WRITES: [(&str, &str); 15] = [
    ("an identity write of an own method", "String.prototype.charCodeAt = String.prototype.charCodeAt;"),
    ("an identity write through a helper", "function d(O, k, v) { O[k] = v; } d(String.prototype, 'charCodeAt', String.prototype.charCodeAt);"),
    ("an identity write under computed keys", "String.prototype['charCodeAt'] = String.prototype['charCodeAt'];"),
    ("an identity write under a held key", "var k = 'charCodeAt'; String.prototype[k] = String.prototype[k];"),
    ("another method written over a key", "String.prototype.charCodeAt = Number.prototype.toString;"),
    ("another key read into the write", "String.prototype.charCodeAt = String.prototype.concat;"),
    ("a helper writes another method", "function d(O, k, v) { O[k] = v; } d(String.prototype, 'charCodeAt', String.prototype.trim);"),
    ("an aliased target", "var S = String; S.prototype.charCodeAt = String.prototype.charCodeAt;"),
    ("an identity write followed by a patch", "String.prototype.charCodeAt = String.prototype.charCodeAt, String.prototype.charCodeAt = function () { return 7; };"),
    ("an identity write of an accessor", "Object.defineProperty(String.prototype, 'charCodeAt', { configurable: true, get: function () { return function () { return 7; }; }, set: function () {} }); String.prototype.charCodeAt = String.prototype.charCodeAt;"),
    ("a chained write", "String.prototype.charCodeAt = String.prototype.charCodeAt = function () { return 7; };"),
    ("one helper body, an identity and a patch", "function d(O, k, v) { O[k] = v; } d(String.prototype, 'concat', String.prototype.concat); d(Array.prototype, 'join', String.prototype.concat);"),
    ("a helper fed a held method", "function d(O, k, v) { O[k] = v; } var v = String.prototype.trim; d(String.prototype, 'charCodeAt', v);"),
    ("a helper that reassigns its value", "function d(O, k, v) { v = String.prototype.trim; O[k] = v; } d(String.prototype, 'charCodeAt', String.prototype.charCodeAt);"),
    ("a getter global root alternates", "var real = String; var n = 0; Object.defineProperty(globalThis, 'String', { configurable: true, get: function () { n++; return n % 2 ? real : Number; } }); String.prototype.charCodeAt = String.prototype.charCodeAt; delete globalThis.String; globalThis.String = real;"),
];

const INDEX_WRITERS: [(&str, &str); 5] = [
    ("an index written on Array.prototype", "Array.prototype[0] = 'Z';"),
    ("Array.of with a constructor that returns Array.prototype", "Array.of.call(function () { return Array.prototype; }, 'Z');"),
    ("Array.from with a constructor that returns Array.prototype", "Array.from.call(function () { return Array.prototype; }, ['Z']);"),
    ("a species constructor that returns Array.prototype", "var a = [1]; a.constructor = { [Symbol.species]: function () { return Array.prototype; } }; a.map(function () { return 'Z'; });"),
    ("an index written on Object.prototype", "Object.prototype[1] = 'Q';"),
];

const HOLES: &str = "console.log([, 2].join('+'), [, 1].at(0), [, 1].includes('Z'), [1, , 3].join('-'), [, 'q'].indexOf('Z'));";

const SCRIPT_GLOBALS: [(&str, &str); 16] = [
    ("a global write rebinds a script var", "var k = 'x'; globalThis.k = 'charCodeAt'; String.prototype[k] = function () { return 7; };"),
    ("a top-level this write rebinds a script var", "var k = 'x'; this.k = 'charCodeAt'; String.prototype[k] = function () { return 7; };"),
    ("a helper writes the global object", "var k = 'x'; function set(o, p, v) { o[p] = v; } set(globalThis, 'k', 'charCodeAt'); String.prototype[k] = function () { return 7; };"),
    ("defineProperty on the global object", "var k = 'x'; Object.defineProperty(globalThis, 'k', { value: 'charCodeAt' }); String.prototype[k] = function () { return 7; };"),
    ("a numeric script var rebound", "var i = 0; globalThis.i = 'charCodeAt'; String.prototype[i] = function () { return 7; };"),
    ("an object script var rebound", "var o = { k: 'x' }; globalThis.o = { k: 'charCodeAt' }; String.prototype[o.k] = function () { return 7; };"),
    ("a fresh script var rebound to a prototype", "var o = {}; globalThis.o = String.prototype; o.charCodeAt = function () { return 7; };"),
    ("an object script var written through the global object", "var o = { k: 'x' }; globalThis.o.k = 'charCodeAt'; String.prototype[o.k] = function () { return 7; };"),
    ("a sloppy this rebinds a script var", "var k = 'x'; (function () { this.k = 'charCodeAt'; })(); String.prototype[k] = function () { return 7; };"),
    ("an unknown key on the global object", "var k = 'x'; var t = { k: 'charCodeAt' }; for (var p in t) globalThis[p] = t[p]; String.prototype[k] = function () { return 7; };"),
    ("an object script var read off the global object", "var o = { k: 'x' }; var p = globalThis['o']; p.k = 'charCodeAt'; String.prototype[o.k] = function () { return 7; };"),
    ("a merged object script var rebound", "var o = {}; o.k = 'x'; globalThis.o = { k: 'charCodeAt' }; String.prototype[o.k] = function () { return 7; };"),
    ("Function text rebinds an object script var", "var o = { k: 'x' }; Function('o = { k: \"charCodeAt\" }')(); String.prototype[o.k] = function () { return 7; };"),
    ("the global object's values are enumerated", "var o = { k: 'x' }; Object.values(globalThis).forEach(function (v) { if (v && v.k === 'x') v.k = 'charCodeAt'; }); String.prototype[o.k] = function () { return 7; };"),
    ("a script function rebound to a definer", "function f(a, b, c) { return a; } globalThis.f = Object.defineProperty; f(String.prototype, 'charCodeAt', { value: function () { return 7; } });"),
    ("Function text writes a script let", "let o = { k: 'x' }; Function('o.k = \"charCodeAt\"')(); String.prototype[o.k] = function () { return 7; };"),
];

const TEXT_REBINDS: [(&str, &str); 8] = [
    ("Function text rebinds a root", r#"Function("String = Array")(); String.prototype.join = function () { return "X"; };"#),
    ("Function text rebinds a root in a nested function", r#"Function("function h() { String = Array; } h();")(); String.prototype.join = function () { return "X"; };"#),
    ("Function text runs eval text that rebinds a root", r#"Function("eval('String = Array')")(); String.prototype.join = function () { return "X"; };"#),
    ("Function text rebinds a root through a pattern default", r#"Function("[String = Array] = []")(); String.prototype.join = function () { return "X"; };"#),
    ("a sequence callee of Function", r#"(0, Function)("String = Array")(); String.prototype.join = function () { return "X"; };"#),
    ("the constructor of a function", r#"(function () {}).constructor("String = Array")(); String.prototype.join = function () { return "X"; };"#),
    ("the constructor of an async function", r#"(async function () {}).constructor("String = Array")(); String.prototype.join = function () { return "X"; };"#),
    ("Function text rebinds Object's create", "Function(\"Object = { create: function () { return String.prototype; } }\")(); var o = Object.create(null); o.charCodeAt = function () { return 7; };"),
];

const PROTO_SETTERS: [(&str, &str); 7] = [
    ("the __proto__ setter looked up", "var s = ({}).__lookupSetter__('__proto__'); s.call(String.prototype, { [Symbol.split]: function () { return ['Z']; } });"),
    ("the __proto__ setter looked up by a held key", "var k = '__proto__'; var s = ({}).__lookupSetter__(k); s.call(String.prototype, { [Symbol.split]: function () { return ['Z']; } });"),
    ("the setter lookup reached by a built key", "var s = ({})['__lookup' + 'Setter__']('__proto__'); s.call(String.prototype, { [Symbol.split]: function () { return ['Z']; } });"),
    ("the setter lookup forwarded by call", "var o = {}; var s = o.__lookupSetter__.call(o, '__proto__'); s.call(String.prototype, { [Symbol.split]: function () { return ['Z']; } });"),
    ("the setter lookup forwarded by Reflect.apply", "var s = Reflect.apply(Object.prototype.__lookupSetter__, {}, ['__proto__']); s.call(String.prototype, { [Symbol.split]: function () { return ['Z']; } });"),
    ("the setter lookup forwarded by apply", "var s = ({}).__lookupSetter__.apply({}, ['__proto__']); s.call(String.prototype, { [Symbol.split]: function () { return ['Z']; } });"),
    ("the looked-up setter run by Reflect.apply", "var s = ({}).__lookupSetter__('__proto__'); Reflect.apply(s, String.prototype, [{ [Symbol.split]: function () { return ['Z']; } }]);"),
];

const REALM_DECODES: [(&str, &str); 4] = [
    (
        "Array read off the returned global object",
        r#"function k(f) { return f(); } var G = k(function () { return globalThis; }); G.Array.prototype.join = function () { return "X"; };"#,
    ),
    (
        "Array read off the global object as a sloppy this",
        r#"var g = (function () { return this; })(); g.Array.prototype.join = function () { return "X"; };"#,
    ),
    (
        "defineProperty read off a returned Object",
        r#"function k(f) { return f(); } var O = k(function () { return Object; }); O.defineProperty(Array.prototype, "join", {value: function () { return "X"; }});"#,
    ),
    (
        "set read off a returned Reflect",
        r#"function k(f) { return f(); } var R = k(function () { return Reflect; }); R.set(Array.prototype, "join", function () { return "X"; });"#,
    ),
];

const FOLDED: &str = "console.log('-'.charCodeAt(0), 'a'.concat('b'), [1, 2].join('+'), [1, 2].concat(3).length, 'a,b'.split(','), 'abc'.split('').reverse().join(''));";

fn transformed(options: &Options, source: &str) -> String {
    Pipeline::new(options.clone())
        .run(source)
        .code()
        .to_string()
}

fn prints_what_node_prints(rows: &[(&str, &str)]) {
    let programs: Vec<(String, String)> = rows
        .iter()
        .map(|(label, patch)| (label.to_string(), patched_after_rotator(JOIN_TEST, patch)))
        .collect();
    agrees_with_node(&programs, Some(NODE_PRINTS));
}

fn agrees_with_node(programs: &[(String, String)], expected: Option<&str>) {
    let Some(node) = node_or_skip() else {
        return;
    };
    let safe = Options::default();
    let unsafe_transforms = Options {
        unsafe_transforms: true,
        ..Options::default()
    };
    let run_safe = |source: &str| transformed(&safe, source);
    let run_unsafe = |source: &str| transformed(&unsafe_transforms, source);
    let modes: [Mode<'_>; 2] = [("default", &run_safe), ("unsafe", &run_unsafe)];
    let dir = tempfile::tempdir().unwrap();
    for (label, source) in programs.iter().filter(|_| expected.is_some()) {
        let seen = node_rows::printed(&node, dir.path(), source).unwrap();
        assert!(
            seen.starts_with(expected.unwrap_or_default()),
            "{label}: node printed {seen:?}"
        );
    }
    let wrong = node_rows::mismatches(&node, programs, &modes);
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

#[test]
fn an_optional_chain_is_followed_as_its_plain_form() {
    prints_what_node_prints(&CHAINS);
}

#[test]
fn an_argument_reached_another_way_keeps_the_decode_unfolded() {
    prints_what_node_prints(&ARGUMENTS);
}

#[test]
fn an_identity_built_in_handed_to_a_call_keeps_the_decode_unfolded() {
    prints_what_node_prints(&HANDED);
}

#[test]
fn get_prototype_of_reached_another_way_is_any_prototype() {
    prints_what_node_prints(&PROTOTYPE_OF);
}

#[test]
fn a_replaced_built_in_the_resolver_models_keeps_the_decode_unfolded() {
    prints_what_node_prints(&REPLACED);
}

#[test]
fn a_key_held_in_a_binding_names_the_step_it_spells() {
    prints_what_node_prints(&HELD_KEYS);
}

#[test]
fn a_built_in_or_realm_object_held_in_data_is_followed_by_key() {
    prints_what_node_prints(&DATA);
}

#[test]
fn an_alias_handed_to_a_call_keeps_the_decode_unfolded() {
    prints_what_node_prints(&ALIASES_HANDED);
}

#[test]
fn an_alias_call_returned_by_a_function_keeps_the_decode_unfolded() {
    prints_what_node_prints(&RETURNED);
}

#[test]
fn a_key_held_in_a_binding_names_the_step_through_the_binding() {
    prints_what_node_prints(&BOUND_KEYS);
}

#[test]
fn a_literal_chain_folds_only_as_node_evaluates_it() {
    let programs: Vec<(String, String)> = FOLDS
        .iter()
        .map(|(label, patch)| (label.to_string(), format!("{patch}\n{FOLDED}\n")))
        .collect();
    agrees_with_node(&programs, None);
}

#[test]
fn data_held_by_a_binding_is_read_wherever_the_object_goes() {
    prints_what_node_prints(&HELD);
    prints_what_node_prints(&ESCAPED);
}

#[test]
fn a_realm_object_escaping_to_a_return_a_parameter_or_code_text_keeps_the_decode_unfolded() {
    prints_what_node_prints(&ESCAPES);
}

#[test]
fn a_realm_object_escaping_keeps_its_literal_chains_unfolded() {
    let programs: Vec<(String, String)> = ESCAPED_FOLDS
        .iter()
        .map(|(label, patch)| (label.to_string(), format!("{patch}\n{FOLDED}\n")))
        .collect();
    agrees_with_node(&programs, None);
}

#[test]
fn a_realm_object_reached_through_an_unfollowed_value_keeps_its_literal_chains_unfolded() {
    let programs: Vec<(String, String)> = REALM_FOLDS
        .iter()
        .map(|(label, patch)| (label.to_string(), format!("{patch}\n{FOLDED}\n")))
        .collect();
    agrees_with_node(&programs, None);
}

#[test]
fn a_realm_object_reached_through_an_unfollowed_value_keeps_the_decode_unfolded() {
    prints_what_node_prints(&REALM_DECODES);
}

fn folds_as_node_does(rows: &[(&str, &str)]) {
    let programs: Vec<(String, String)> = rows
        .iter()
        .map(|(label, patch)| (label.to_string(), format!("{patch}\n{FOLDED}\n")))
        .collect();
    agrees_with_node(&programs, None);
}

#[test]
fn a_rebound_root_keeps_its_prototype_writes_imprecise() {
    folds_as_node_does(&REBOUND_ROOTS);
}

#[test]
fn a_looked_up_prototype_setter_keeps_the_literal_chains_unfolded() {
    folds_as_node_does(&PROTO_SETTERS);
}

#[test]
fn code_text_that_rebinds_a_global_keeps_the_literal_chains_unfolded() {
    folds_as_node_does(&TEXT_REBINDS);
}

#[test]
fn every_earlier_realm_regression_folds_as_node_does() {
    let programs: Vec<(String, String)> = REGRESSION_SHAPES
        .lines()
        .filter_map(|line| line.split_once('|'))
        .map(|(label, patch)| (label.to_string(), format!("{patch}\n{PROBE_FOLDED}\n")))
        .collect();
    assert_eq!(programs.len(), 95);
    agrees_with_node(&programs, None);
}

#[test]
fn an_identity_write_leaves_the_fold_and_any_other_write_keeps_it() {
    folds_as_node_does(&IDENTITY_WRITES);
}

#[test]
fn an_index_written_on_a_prototype_leaves_holes_unfolded() {
    let programs: Vec<(String, String)> = INDEX_WRITERS
        .iter()
        .map(|(label, patch)| (label.to_string(), format!("{patch}\n{HOLES}\n")))
        .collect();
    agrees_with_node(&programs, None);
}

#[test]
fn a_classic_script_global_the_global_object_may_reach_keeps_the_literal_chains_unfolded() {
    folds_as_node_does(&SCRIPT_GLOBALS);
}

#[test]
fn a_target_agnostic_precision_residue_bails_to_node_s_value() {
    let programs: Vec<(String, String)> = RESIDUE_FOLDS
        .iter()
        .map(|(label, patch)| (label.to_string(), format!("{patch}\n{FOLDED}\n")))
        .collect();
    agrees_with_node(&programs, None);
}
