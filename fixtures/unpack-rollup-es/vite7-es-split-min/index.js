import q from "node:path";
function w(e, t) {
  for (var r = 0; r < t.length; r++) {
    const o = t[r];
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
function x(e) {
  return e && e.__esModule && Object.prototype.hasOwnProperty.call(e, "default") ? e.default : e;
}
function P(e) {
  if (Object.prototype.hasOwnProperty.call(e, "__esModule")) return e;
  var t = e.default;
  if (typeof t == "function") {
    var r = function o() {
      var n = !1;
      try {
        n = this instanceof o;
      } catch {
      }
      return n ? Reflect.construct(t, arguments, this.constructor) : t.apply(this, arguments);
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
var c = { exports: {} }, f;
function $() {
  if (f) return c.exports;
  f = 1, console.log("eval greet");
  const e = q;
  function t(r) {
    return "hello " + e.basename(r);
  }
  return t.shout = function(r) {
    return t(r).toUpperCase();
  }, c.exports = t, c.exports.shout = t.shout, c.exports;
}
var y = $();
const A = /* @__PURE__ */ x(y), C = "fixture", M = [1, 2, 3, 4, 5], h = {
  name: C,
  items: M
};
var a = {}, l = {}, p;
function E() {
  if (p) return l;
  p = 1, console.log("eval cycle-b");
  var e = b();
  return l.seen = "b saw " + e.early + "," + e.late, l.check = function() {
    return "b later " + e.late;
  }, l;
}
var g;
function b() {
  if (g) return a;
  g = 1, console.log("eval cycle-a start"), a.early = "a-early";
  var e = E();
  return a.late = "a-late", a.report = function() {
    return e.seen + " " + e.check();
  }, console.log("eval cycle-a end"), a;
}
var R = b();
console.log("eval util");
const k = (e) => `[${e.padStart(8, ".")}]`;
function B(e) {
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
var s = {}, d;
function S() {
  return d || (d = 1, console.log("eval math"), s.twice = function(e) {
    return e * 2;
  }, s.half = (e) => e / 2), s;
}
var O = S();
const j = "own", D = /* @__PURE__ */ w({
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
}, Symbol.toStringTag, { value: "Module" })), F = /* @__PURE__ */ P(T);
var v;
function I() {
  if (v) return i;
  v = 1, console.log("eval bridge");
  const e = F;
  return i.describe = function() {
    return e.kind + " " + e.area(3) + " " + e.default;
  }, i;
}
var U = I();
const H = h.items;
console.log("eval index");
console.log("greet " + A("ada") + " " + y.shout("bob"));
console.log("cycle " + R.report());
console.log("label " + k(h.name) + " total " + B(H));
console.log("barrel " + j + " " + O.twice(4) + " " + Object.keys(D).sort().join(","));
console.log("bridge " + U.describe());
const _ = new m();
_.add(2);
console.log("counter " + _.value + " " + m.name);
import("./lazy.chunk.js").then((e) => {
  console.log("lazy " + e.value + " " + e.default);
});
export {
  m as Counter,
  k as label,
  B as total
};
