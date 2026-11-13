import q from "node:path";
import { l as w, t as x, C as b } from "./util.chunk.js";
function P(e, r) {
  for (var t = 0; t < r.length; t++) {
    const o = r[t];
    if (typeof o != "string" && !Array.isArray(o)) {
      for (const n in o)
        if (n !== "default" && !(n in e)) {
          const u = Object.getOwnPropertyDescriptor(o, n);
          u && Object.defineProperty(e, n, u.get ? u : {
            enumerable: !0,
            get: () => o[n]
          });
        }
    }
  }
  return Object.freeze(Object.defineProperty(e, Symbol.toStringTag, { value: "Module" }));
}
function C(e) {
  return e && e.__esModule && Object.prototype.hasOwnProperty.call(e, "default") ? e.default : e;
}
function $(e) {
  if (Object.prototype.hasOwnProperty.call(e, "__esModule")) return e;
  var r = e.default;
  if (typeof r == "function") {
    var t = function o() {
      var n = !1;
      try {
        n = this instanceof o;
      } catch {
      }
      return n ? Reflect.construct(r, arguments, this.constructor) : r.apply(this, arguments);
    };
    t.prototype = r.prototype;
  } else t = {};
  return Object.defineProperty(t, "__esModule", { value: !0 }), Object.keys(e).forEach(function(o) {
    var n = Object.getOwnPropertyDescriptor(e, o);
    Object.defineProperty(t, o, n.get ? n : {
      enumerable: !0,
      get: function() {
        return e[o];
      }
    });
  }), t;
}
var c = { exports: {} }, f;
function A() {
  if (f) return c.exports;
  f = 1, console.log("eval greet");
  const e = q;
  function r(t) {
    return "hello " + e.basename(t);
  }
  return r.shout = function(t) {
    return r(t).toUpperCase();
  }, c.exports = r, c.exports.shout = r.shout, c.exports;
}
var v = A();
const M = /* @__PURE__ */ C(v), E = "fixture", R = [1, 2, 3, 4, 5], h = {
  name: E,
  items: R
};
var a = {}, l = {}, p;
function k() {
  if (p) return l;
  p = 1, console.log("eval cycle-b");
  var e = m();
  return l.seen = "b saw " + e.early + "," + e.late, l.check = function() {
    return "b later " + e.late;
  }, l;
}
var g;
function m() {
  if (g) return a;
  g = 1, console.log("eval cycle-a start"), a.early = "a-early";
  var e = k();
  return a.late = "a-late", a.report = function() {
    return e.seen + " " + e.check();
  }, console.log("eval cycle-a end"), a;
}
var B = m(), s = {}, d;
function S() {
  return d || (d = 1, console.log("eval math"), s.twice = function(e) {
    return e * 2;
  }, s.half = (e) => e / 2), s;
}
var O = S();
const j = "own", D = /* @__PURE__ */ P({
  __proto__: null,
  own: j
}, [O]);
var i = {};
console.log("eval shape");
const z = "square";
function G(e) {
  return e * e;
}
const N = "shape-default", T = /* @__PURE__ */ Object.freeze(/* @__PURE__ */ Object.defineProperty({
  __proto__: null,
  area: G,
  default: N,
  kind: z
}, Symbol.toStringTag, { value: "Module" })), F = /* @__PURE__ */ $(T);
var y;
function I() {
  if (y) return i;
  y = 1, console.log("eval bridge");
  const e = F;
  return i.describe = function() {
    return e.kind + " " + e.area(3) + " " + e.default;
  }, i;
}
var U = I();
const H = h.items;
console.log("eval index");
console.log("greet " + M("ada") + " " + v.shout("bob"));
console.log("cycle " + B.report());
console.log("label " + w(h.name) + " total " + x(H));
console.log("barrel " + j + " " + O.twice(4) + " " + Object.keys(D).sort().join(","));
console.log("bridge " + U.describe());
const _ = new b();
_.add(2);
console.log("counter " + _.value + " " + b.name);
import("./lazy.chunk.js").then((e) => {
  console.log("lazy " + e.value + " " + e.default);
});
export {
  b as Counter,
  w as label,
  x as total
};
