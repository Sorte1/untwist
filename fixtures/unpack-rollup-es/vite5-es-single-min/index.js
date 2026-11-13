import _ from "node:path";
function O(e, t) {
  for (var r = 0; r < t.length; r++) {
    const o = t[r];
    if (typeof o != "string" && !Array.isArray(o)) {
      for (const n in o)
        if (n !== "default" && !(n in e)) {
          const s = Object.getOwnPropertyDescriptor(o, n);
          s && Object.defineProperty(e, n, s.get ? s : {
            enumerable: !0,
            get: () => o[n]
          });
        }
    }
  }
  return Object.freeze(Object.defineProperty(e, Symbol.toStringTag, { value: "Module" }));
}
function j(e) {
  return e && e.__esModule && Object.prototype.hasOwnProperty.call(e, "default") ? e.default : e;
}
function $(e) {
  if (e.__esModule) return e;
  var t = e.default;
  if (typeof t == "function") {
    var r = function o() {
      return this instanceof o ? Reflect.construct(t, arguments, this.constructor) : t.apply(this, arguments);
    };
    r.prototype = t.prototype;
  } else r = {};
  return Object.defineProperty(r, "__esModule", { value: !0 }), Object.keys(e).forEach(function(o) {
    var n = Object.getOwnPropertyDescriptor(e, o);
    Object.defineProperty(r, o, n.get ? n : {
      enumerable: !0,
      get: function() {
        return e[o];
      }
    });
  }), r;
}
var i = { exports: {} };
console.log("eval greet");
const P = _;
function c(e) {
  return "hello " + P.basename(e);
}
c.shout = function(e) {
  return c(e).toUpperCase();
};
i.exports = c;
var w = i.exports.shout = c.shout, q = i.exports;
const z = /* @__PURE__ */ j(q), A = "fixture", C = [
  1,
  2,
  3,
  4,
  5
], d = {
  name: A,
  items: C
};
var l = {}, a = {}, p;
function S() {
  if (p) return a;
  p = 1, console.log("eval cycle-b");
  var e = y();
  return a.seen = "b saw " + e.early + "," + e.late, a.check = function() {
    return "b later " + e.late;
  }, a;
}
var g;
function y() {
  if (g) return l;
  g = 1, console.log("eval cycle-a start"), l.early = "a-early";
  var e = S();
  return l.late = "a-late", l.report = function() {
    return e.seen + " " + e.check();
  }, console.log("eval cycle-a end"), l;
}
var x = y();
console.log("eval util");
const M = (e) => `[${e.padStart(8, ".")}]`;
function k(e) {
  return e.reduce((t, r) => t + r, 0);
}
class v {
  constructor() {
    this.value = 0;
  }
  add(t) {
    this.value += t;
  }
}
var f = {};
console.log("eval math");
var b = f.twice = function(e) {
  return e * 2;
}, E = f.half = (e) => e / 2;
const h = "own", B = /* @__PURE__ */ O({
  __proto__: null,
  half: E,
  own: h,
  twice: b
}, [f]);
console.log("eval shape");
const D = "square";
function R(e) {
  return e * e;
}
const T = "shape-default", N = /* @__PURE__ */ Object.freeze(/* @__PURE__ */ Object.defineProperty({
  __proto__: null,
  area: R,
  default: T,
  kind: D
}, Symbol.toStringTag, { value: "Module" })), F = /* @__PURE__ */ $(N);
console.log("eval bridge");
const u = F;
var U = function() {
  return u.kind + " " + u.area(3) + " " + u.default;
};
const G = d.items;
console.log("eval index");
console.log("greet " + z("ada") + " " + w("bob"));
console.log("cycle " + x.report());
console.log("label " + M(d.name) + " total " + k(G));
console.log("barrel " + h + " " + b(4) + " " + Object.keys(B).sort().join(","));
console.log("bridge " + U());
const m = new v();
m.add(2);
console.log("counter " + m.value + " " + v.name);
Promise.resolve().then(() => J).then((e) => {
  console.log("lazy " + e.value + " " + e.default);
});
console.log("eval lazy");
const H = 42, I = "lazy-default", J = /* @__PURE__ */ Object.freeze(/* @__PURE__ */ Object.defineProperty({
  __proto__: null,
  default: I,
  value: H
}, Symbol.toStringTag, { value: "Module" }));
export {
  v as Counter,
  M as label,
  k as total
};
