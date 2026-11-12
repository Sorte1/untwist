var d = typeof globalThis < "u" ? globalThis : typeof window < "u" ? window : typeof global < "u" ? global : typeof self < "u" ? self : {};
function ge(e) {
  return e && e.__esModule && Object.prototype.hasOwnProperty.call(e, "default") ? e.default : e;
}
var v, $;
function ye() {
  if ($) return v;
  $ = 1;
  function e(r, t, n) {
    var i = -1, a = r.length;
    t < 0 && (t = -t > a ? 0 : a + t), n = n > a ? a : n, n < 0 && (n += a), a = t > n ? 0 : n - t >>> 0, t >>>= 0;
    for (var u = Array(a); ++i < a; )
      u[i] = r[i + t];
    return u;
  }
  return v = e, v;
}
var m, X;
function ve() {
  if (X) return m;
  X = 1;
  function e(r, t) {
    return r === t || r !== r && t !== t;
  }
  return m = e, m;
}
var h, U;
function me() {
  if (U) return h;
  U = 1;
  var e = typeof d == "object" && d && d.Object === Object && d;
  return h = e, h;
}
var q, D;
function he() {
  if (D) return q;
  D = 1;
  var e = me(), r = typeof self == "object" && self && self.Object === Object && self, t = e || r || Function("return this")();
  return q = t, q;
}
var _, H;
function le() {
  if (H) return _;
  H = 1;
  var e = he(), r = e.Symbol;
  return _ = r, _;
}
var T, J;
function qe() {
  if (J) return T;
  J = 1;
  var e = le(), r = Object.prototype, t = r.hasOwnProperty, n = r.toString, i = e ? e.toStringTag : void 0;
  function a(u) {
    var o = t.call(u, i), s = u[i];
    try {
      u[i] = void 0;
      var c = !0;
    } catch {
    }
    var f = n.call(u);
    return c && (o ? u[i] = s : delete u[i]), f;
  }
  return T = a, T;
}
var I, W;
function _e() {
  if (W) return I;
  W = 1;
  var e = Object.prototype, r = e.toString;
  function t(n) {
    return r.call(n);
  }
  return I = t, I;
}
var p, Y;
function be() {
  if (Y) return p;
  Y = 1;
  var e = le(), r = qe(), t = _e(), n = "[object Null]", i = "[object Undefined]", a = e ? e.toStringTag : void 0;
  function u(o) {
    return o == null ? o === void 0 ? i : n : a && a in Object(o) ? r(o) : t(o);
  }
  return p = u, p;
}
var j, K;
function P() {
  if (K) return j;
  K = 1;
  function e(r) {
    var t = typeof r;
    return r != null && (t == "object" || t == "function");
  }
  return j = e, j;
}
var S, Q;
function Te() {
  if (Q) return S;
  Q = 1;
  var e = be(), r = P(), t = "[object AsyncFunction]", n = "[object Function]", i = "[object GeneratorFunction]", a = "[object Proxy]";
  function u(o) {
    if (!r(o))
      return !1;
    var s = e(o);
    return s == n || s == i || s == t || s == a;
  }
  return S = u, S;
}
var O, V;
function Ie() {
  if (V) return O;
  V = 1;
  var e = 9007199254740991;
  function r(t) {
    return typeof t == "number" && t > -1 && t % 1 == 0 && t <= e;
  }
  return O = r, O;
}
var R, Z;
function pe() {
  if (Z) return R;
  Z = 1;
  var e = Te(), r = Ie();
  function t(n) {
    return n != null && r(n.length) && !e(n);
  }
  return R = t, R;
}
var w, z;
function je() {
  if (z) return w;
  z = 1;
  var e = 9007199254740991, r = /^(?:0|[1-9]\d*)$/;
  function t(n, i) {
    var a = typeof n;
    return i = i ?? e, !!i && (a == "number" || a != "symbol" && r.test(n)) && n > -1 && n % 1 == 0 && n < i;
  }
  return w = t, w;
}
var k, ee;
function Se() {
  if (ee) return k;
  ee = 1;
  var e = ve(), r = pe(), t = je(), n = P();
  function i(a, u, o) {
    if (!n(o))
      return !1;
    var s = typeof u;
    return (s == "number" ? r(o) && t(u, o.length) : s == "string" && u in o) ? e(o[u], a) : !1;
  }
  return k = i, k;
}
var A, re;
function Oe() {
  if (re) return A;
  re = 1;
  var e = /\s/;
  function r(t) {
    for (var n = t.length; n-- && e.test(t.charAt(n)); )
      ;
    return n;
  }
  return A = r, A;
}
var E, te;
function Re() {
  if (te) return E;
  te = 1;
  var e = Oe(), r = /^\s+/;
  function t(n) {
    return n && n.slice(0, e(n) + 1).replace(r, "");
  }
  return E = t, E;
}
var x, ne;
function we() {
  if (ne) return x;
  ne = 1;
  function e(r) {
    return r != null && typeof r == "object";
  }
  return x = e, x;
}
var F, ie;
function ke() {
  if (ie) return F;
  ie = 1;
  var e = be(), r = we(), t = "[object Symbol]";
  function n(i) {
    return typeof i == "symbol" || r(i) && e(i) == t;
  }
  return F = n, F;
}
var G, oe;
function Ae() {
  if (oe) return G;
  oe = 1;
  var e = Re(), r = P(), t = ke(), n = NaN, i = /^[-+]0x[0-9a-f]+$/i, a = /^0b[01]+$/i, u = /^0o[0-7]+$/i, o = parseInt;
  function s(c) {
    if (typeof c == "number")
      return c;
    if (t(c))
      return n;
    if (r(c)) {
      var f = typeof c.valueOf == "function" ? c.valueOf() : c;
      c = r(f) ? f + "" : f;
    }
    if (typeof c != "string")
      return c === 0 ? c : +c;
    c = e(c);
    var l = a.test(c);
    return l || u.test(c) ? o(c.slice(2), l ? 2 : 8) : i.test(c) ? n : +c;
  }
  return G = s, G;
}
var L, ae;
function Ee() {
  if (ae) return L;
  ae = 1;
  var e = Ae(), r = 1 / 0, t = 17976931348623157e292;
  function n(i) {
    if (!i)
      return i === 0 ? i : 0;
    if (i = e(i), i === r || i === -r) {
      var a = i < 0 ? -1 : 1;
      return a * t;
    }
    return i === i ? i : 0;
  }
  return L = n, L;
}
var N, ue;
function xe() {
  if (ue) return N;
  ue = 1;
  var e = Ee();
  function r(t) {
    var n = e(t), i = n % 1;
    return n === n ? i ? n - i : n : 0;
  }
  return N = r, N;
}
var C, ce;
function Fe() {
  if (ce) return C;
  ce = 1;
  var e = ye(), r = Se(), t = xe(), n = Math.ceil, i = Math.max;
  function a(u, o, s) {
    (s ? r(u, o, s) : o === void 0) ? o = 1 : o = i(t(o), 0);
    var c = u == null ? 0 : u.length;
    if (!c || o < 1)
      return [];
    for (var f = 0, l = 0, B = Array(n(c / o)); f < c; )
      B[l++] = e(u, f, f += o);
    return B;
  }
  return C = a, C;
}
var Ge = Fe();
const Le = /* @__PURE__ */ ge(Ge), Ne = "fixture", Ce = [1, 2, 3, 4, 5], M = {
  name: Ne,
  items: Ce
};
var b = {}, g = {}, se;
function Pe() {
  if (se) return g;
  se = 1, console.log("eval cycle-b");
  var e = de();
  return g.seen = "b saw " + e.early + "," + e.late, g.check = function() {
    return "b later " + e.late;
  }, g;
}
var fe;
function de() {
  if (fe) return b;
  fe = 1, console.log("eval cycle-a start"), b.early = "a-early";
  var e = Pe();
  return b.late = "a-late", b.report = function() {
    return e.seen + " " + e.check();
  }, console.log("eval cycle-a end"), b;
}
var Me = de();
console.log("eval util");
const Be = (e) => `[${e.padStart(8, ".")}]`;
function $e(e) {
  return e.reduce((r, t) => r + t, 0);
}
const y = M.items;
console.log("eval index");
console.log("last " + y[y.length - 1] + " own " + Object.prototype.hasOwnProperty.call(M, "name"));
console.log("replace " + "a-b-c".split("-").join("+"));
console.log("chunk " + JSON.stringify(Le(y, 2)));
console.log("cycle " + Me.report());
console.log("label " + Be(M.name) + " total " + $e(y));
import("./lazy.chunk.js").then((e) => (console.log("lazy " + e.value + " " + e.default), Promise.allSettled([Promise.resolve(1), Promise.reject(new Error("no"))]))).then((e) => console.log("settled " + e.map((r) => r.status).join(",")));
