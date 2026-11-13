import O from "node:path";
function _(e, t) {
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
  if (e.__esModule)
    return e;
  var t = e.default;
  if (typeof t == "function") {
    var r = function o() {
      return this instanceof o ? Reflect.construct(t, arguments, this.constructor) : t.apply(this, arguments);
    };
    r.prototype = t.prototype;
  } else
    r = {};
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
const w = O;
function c(e) {
  return "hello " + w.basename(e);
}
c.shout = function(e) {
  return c(e).toUpperCase();
};
i.exports = c;
var P = i.exports.shout = c.shout, q = i.exports;
const A = /* @__PURE__ */ j(q), C = "fixture", x = [
  1,
  2,
  3,
  4,
  5
], d = {
  name: C,
  items: x
};
var a = {}, l = {}, p;
function k() {
  if (p)
    return l;
  p = 1, console.log("eval cycle-b");
  var e = v();
  return l.seen = "b saw " + e.early + "," + e.late, l.check = function() {
    return "b later " + e.late;
  }, l;
}
var g;
function v() {
  if (g)
    return a;
  g = 1, console.log("eval cycle-a start"), a.early = "a-early";
  var e = k();
  return a.late = "a-late", a.report = function() {
    return e.seen + " " + e.check();
  }, console.log("eval cycle-a end"), a;
}
var M = v();
console.log("eval util");
const S = (e) => `[${e.padStart(8, ".")}]`;
function E(e) {
  return e.reduce((t, r) => t + r, 0);
}
class y {
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
}, B = f.half = (e) => e / 2;
const h = "own", D = /* @__PURE__ */ _({
  __proto__: null,
  half: B,
  own: h,
  twice: b
}, [f]);
console.log("eval shape");
const R = "square";
function z(e) {
  return e * e;
}
const N = "shape-default", T = /* @__PURE__ */ Object.freeze(/* @__PURE__ */ Object.defineProperty({
  __proto__: null,
  area: z,
  default: N,
  kind: R
}, Symbol.toStringTag, { value: "Module" })), F = /* @__PURE__ */ $(T);
console.log("eval bridge");
const u = F;
var U = function() {
  return u.kind + " " + u.area(3) + " " + u.default;
};
const G = d.items;
console.log("eval index");
console.log("greet " + A("ada") + " " + P("bob"));
console.log("cycle " + M.report());
console.log("label " + S(d.name) + " total " + E(G));
console.log("barrel " + h + " " + b(4) + " " + Object.keys(D).sort().join(","));
console.log("bridge " + U());
const m = new y();
m.add(2);
console.log("counter " + m.value + " " + y.name);
import("./lazy.chunk.js").then((e) => {
  console.log("lazy " + e.value + " " + e.default);
});
export {
  y as Counter,
  S as label,
  E as total
};
