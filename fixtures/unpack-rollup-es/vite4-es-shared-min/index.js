import O from "node:path";
import { l as _, t as j, C as y } from "./util.chunk.js";
function $(e, n) {
  for (var r = 0; r < n.length; r++) {
    const t = n[r];
    if (typeof t != "string" && !Array.isArray(t)) {
      for (const o in t)
        if (o !== "default" && !(o in e)) {
          const s = Object.getOwnPropertyDescriptor(t, o);
          s && Object.defineProperty(e, o, s.get ? s : {
            enumerable: !0,
            get: () => t[o]
          });
        }
    }
  }
  return Object.freeze(Object.defineProperty(e, Symbol.toStringTag, { value: "Module" }));
}
function w(e) {
  return e && e.__esModule && Object.prototype.hasOwnProperty.call(e, "default") ? e.default : e;
}
function C(e) {
  if (e.__esModule)
    return e;
  var n = e.default;
  if (typeof n == "function") {
    var r = function t() {
      return this instanceof t ? Reflect.construct(n, arguments, this.constructor) : n.apply(this, arguments);
    };
    r.prototype = n.prototype;
  } else
    r = {};
  return Object.defineProperty(r, "__esModule", { value: !0 }), Object.keys(e).forEach(function(t) {
    var o = Object.getOwnPropertyDescriptor(e, t);
    Object.defineProperty(r, t, o.get ? o : {
      enumerable: !0,
      get: function() {
        return e[t];
      }
    });
  }), r;
}
var i = { exports: {} };
console.log("eval greet");
const P = O;
function c(e) {
  return "hello " + P.basename(e);
}
c.shout = function(e) {
  return c(e).toUpperCase();
};
i.exports = c;
var q = i.exports.shout = c.shout, A = i.exports;
const x = /* @__PURE__ */ w(A), k = "fixture", M = [
  1,
  2,
  3,
  4,
  5
], d = {
  name: k,
  items: M
};
var a = {}, l = {}, p;
function E() {
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
  var e = E();
  return a.late = "a-late", a.report = function() {
    return e.seen + " " + e.check();
  }, console.log("eval cycle-a end"), a;
}
var S = v(), f = {};
console.log("eval math");
var b = f.twice = function(e) {
  return e * 2;
}, B = f.half = (e) => e / 2;
const h = "own", D = /* @__PURE__ */ $({
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
}, Symbol.toStringTag, { value: "Module" })), F = /* @__PURE__ */ C(T);
console.log("eval bridge");
const u = F;
var U = function() {
  return u.kind + " " + u.area(3) + " " + u.default;
};
const G = d.items;
console.log("eval index");
console.log("greet " + x("ada") + " " + q("bob"));
console.log("cycle " + S.report());
console.log("label " + _(d.name) + " total " + j(G));
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
  _ as label,
  j as total
};
