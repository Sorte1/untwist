import { createRequire as e } from "node:module";
//#region \0rolldown/runtime.js
var t = Object.create, n = Object.defineProperty, r = Object.getOwnPropertyDescriptor, i = Object.getOwnPropertyNames, a = Object.getPrototypeOf, o = Object.prototype.hasOwnProperty, s = (e, t) => () => (e && (t = e(e = 0)), t), c = (e, t) => () => (t || (e((t = { exports: {} }).exports, t), e = null), t.exports), l = (e, t) => {
	let r = {};
	for (var i in e) n(r, i, {
		get: e[i],
		enumerable: !0
	});
	return t || n(r, Symbol.toStringTag, { value: "Module" }), r;
}, u = (e, t, a, s) => {
	if (t && typeof t == "object" || typeof t == "function") for (var c = i(t), l = 0, u = c.length, d; l < u; l++) d = c[l], !o.call(e, d) && d !== a && n(e, d, {
		get: ((e) => t[e]).bind(null, d),
		enumerable: !(s = r(t, d)) || s.enumerable
	});
	return e;
}, d = (e, t, n) => (u(e, t, "default"), n && u(n, t, "default")), f = (e, r, i) => (i = e == null ? {} : t(a(e)), u(r || !e || !e.__esModule ? n(i, "default", {
	value: e,
	enumerable: !0
}) : i, e)), p = (e) => o.call(e, "module.exports") ? e["module.exports"] : u(n({}, "__esModule", { value: !0 }), e), m = /* @__PURE__ */ e(import.meta.url), h = /* @__PURE__ */ f((/* @__PURE__ */ c(((e, t) => {
	console.log("eval greet");
	var n = m("node:path");
	function r(e) {
		return "hello " + n.basename(e);
	}
	r.shout = function(e) {
		return r(e).toUpperCase();
	}, t.exports = r, t.exports.shout = r.shout;
})))()), g = {
	name: "fixture",
	items: [
		1,
		2,
		3,
		4,
		5
	]
}, _ = /* @__PURE__ */ c(((e) => {
	console.log("eval cycle-b");
	var t = v();
	e.seen = "b saw " + t.early + "," + t.late, e.check = function() {
		return "b later " + t.late;
	};
})), v = /* @__PURE__ */ c(((e) => {
	console.log("eval cycle-a start"), e.early = "a-early";
	var t = _();
	e.late = "a-late", e.report = function() {
		return t.seen + " " + t.check();
	}, console.log("eval cycle-a end");
})), y = v();
console.log("eval util");
var b = (e) => `[${e.padStart(8, ".")}]`;
function x(e) {
	return e.reduce((e, t) => e + t, 0);
}
var S = class {
	constructor() {
		this.value = 0;
	}
	add(e) {
		this.value += e;
	}
}, C = /* @__PURE__ */ c(((e) => {
	console.log("eval math"), e.twice = function(e) {
		return e * 2;
	}, e.half = (e) => e / 2;
})), w = /* @__PURE__ */ l({ own: () => "own" });
d(w, /* @__PURE__ */ f(C()));
//#endregion
//#region tools/rolldown/app/shape.js
var T = /* @__PURE__ */ l({
	area: () => E,
	default: () => O,
	kind: () => D
});
function E(e) {
	return e * e;
}
var D, O, k = s((() => {
	console.log("eval shape"), D = "square", O = "shape-default";
})), A = /* @__PURE__ */ c(((e) => {
	console.log("eval bridge");
	var t = (k(), p(T));
	e.describe = function() {
		return t.kind + " " + t.area(3) + " " + t.default;
	};
})), j = /* @__PURE__ */ l({
	default: () => M,
	value: () => 42
}), M, N = s((() => {
	console.log("eval lazy"), M = "lazy-default";
})), P = A(), F = g.items;
console.log("eval index"), console.log("greet " + (0, h.default)("ada") + " " + (0, h.shout)("bob")), console.log("cycle " + (0, y.report)()), console.log("label " + b(g.name) + " total " + x(F)), console.log("barrel own " + w.twice(4) + " " + Object.keys(w).sort().join(",")), console.log("bridge " + (0, P.describe)());
var I = new S();
I.add(2), console.log("counter " + I.value + " " + S.name), Promise.resolve().then(() => (N(), j)).then((e) => {
	console.log("lazy " + e.value + " " + e.default);
});
//#endregion
export {};
