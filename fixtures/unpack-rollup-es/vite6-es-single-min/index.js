import q from "node:path";
function P(e, t) {
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
function w(e) {
  return e && e.__esModule && Object.prototype.hasOwnProperty.call(e, "default") ? e.default : e;
}
function $(e) {
  if (Object.prototype.hasOwnProperty.call(e, "__esModule")) return e;
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
var l = { exports: {} }, f;
function x() {
  if (f) return l.exports;
  f = 1, console.log("eval greet");
  const e = q;
  function t(r) {
    return "hello " + e.basename(r);
  }
  return t.shout = function(r) {
    return t(r).toUpperCase();
  }, l.exports = t, l.exports.shout = t.shout, l.exports;
}
var v = x();
const M = /* @__PURE__ */ w(v), z = "fixture", A = [1, 2, 3, 4, 5], b = {
  name: z,
  items: A
};
var a = {}, c = {}, p;
function C() {
  if (p) return c;
  p = 1, console.log("eval cycle-b");
  var e = h();
  return c.seen = "b saw " + e.early + "," + e.late, c.check = function() {
    return "b later " + e.late;
  }, c;
}
var d;
function h() {
  if (d) return a;
  d = 1, console.log("eval cycle-a start"), a.early = "a-early";
  var e = C();
  return a.late = "a-late", a.report = function() {
    return e.seen + " " + e.check();
  }, console.log("eval cycle-a end"), a;
}
var S = h();
console.log("eval util");
const E = (e) => `[${e.padStart(8, ".")}]`;
function R(e) {
  return e.reduce((t, r) => t + r, 0);
}
class m {
  constructor() {
    this.value = 0;
  }
  add(t) {
    this.value += t;
  }
}
var u = {}, g;
function k() {
  return g || (g = 1, console.log("eval math"), u.twice = function(e) {
    return e * 2;
  }, u.half = (e) => e / 2), u;
}
var O = k();
const _ = "own", B = /* @__PURE__ */ P({
  __proto__: null,
  own: _
}, [O]);
var i = {};
console.log("eval shape");
const D = "square";
function T(e) {
  return e * e;
}
const G = "shape-default", N = /* @__PURE__ */ Object.freeze(/* @__PURE__ */ Object.defineProperty({
  __proto__: null,
  area: T,
  default: G,
  kind: D
}, Symbol.toStringTag, { value: "Module" })), F = /* @__PURE__ */ $(N);
var y;
function U() {
  if (y) return i;
  y = 1, console.log("eval bridge");
  const e = F;
  return i.describe = function() {
    return e.kind + " " + e.area(3) + " " + e.default;
  }, i;
}
var H = U();
const I = b.items;
console.log("eval index");
console.log("greet " + M("ada") + " " + v.shout("bob"));
console.log("cycle " + S.report());
console.log("label " + E(b.name) + " total " + R(I));
console.log("barrel " + _ + " " + O.twice(4) + " " + Object.keys(B).sort().join(","));
console.log("bridge " + H.describe());
const j = new m();
j.add(2);
console.log("counter " + j.value + " " + m.name);
Promise.resolve().then(() => L).then((e) => {
  console.log("lazy " + e.value + " " + e.default);
});
console.log("eval lazy");
const J = 42, K = "lazy-default", L = /* @__PURE__ */ Object.freeze(/* @__PURE__ */ Object.defineProperty({
  __proto__: null,
  default: K,
  value: J
}, Symbol.toStringTag, { value: "Module" }));
export {
  m as Counter,
  E as label,
  R as total
};
