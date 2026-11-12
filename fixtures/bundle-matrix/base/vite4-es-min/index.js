var l = typeof globalThis < "u" ? globalThis : typeof window < "u" ? window : typeof global < "u" ? global : typeof self < "u" ? self : {};
function I(e) {
  return e && e.__esModule && Object.prototype.hasOwnProperty.call(e, "default") ? e.default : e;
}
function w(e, r, t) {
  var o = -1, n = e.length;
  r < 0 && (r = -r > n ? 0 : n + r), t = t > n ? n : t, t < 0 && (t += n), n = r > t ? 0 : t - r >>> 0, r >>>= 0;
  for (var i = Array(n); ++o < n; )
    i[o] = e[o + r];
  return i;
}
var A = w;
function E(e, r) {
  return e === r || e !== e && r !== r;
}
var F = E, x = typeof l == "object" && l && l.Object === Object && l, k = x, G = k, N = typeof self == "object" && self && self.Object === Object && self, P = G || N || Function("return this")(), C = P, L = C, R = L.Symbol, S = R, g = S, _ = Object.prototype, q = _.hasOwnProperty, M = _.toString, a = g ? g.toStringTag : void 0;
function B(e) {
  var r = q.call(e, a), t = e[a];
  try {
    e[a] = void 0;
    var o = !0;
  } catch {
  }
  var n = M.call(e);
  return o && (r ? e[a] = t : delete e[a]), n;
}
var X = B, U = Object.prototype, D = U.toString;
function H(e) {
  return D.call(e);
}
var J = H, m = S, W = X, Y = J, K = "[object Null]", Q = "[object Undefined]", d = m ? m.toStringTag : void 0;
function V(e) {
  return e == null ? e === void 0 ? Q : K : d && d in Object(e) ? W(e) : Y(e);
}
var h = V;
function Z(e) {
  var r = typeof e;
  return e != null && (r == "object" || r == "function");
}
var b = Z, z = h, ee = b, re = "[object AsyncFunction]", te = "[object Function]", ne = "[object GeneratorFunction]", oe = "[object Proxy]";
function ie(e) {
  if (!ee(e))
    return !1;
  var r = z(e);
  return r == te || r == ne || r == re || r == oe;
}
var ae = ie, ce = 9007199254740991;
function le(e) {
  return typeof e == "number" && e > -1 && e % 1 == 0 && e <= ce;
}
var se = le, fe = ae, be = se;
function ue(e) {
  return e != null && be(e.length) && !fe(e);
}
var ye = ue, ge = 9007199254740991, me = /^(?:0|[1-9]\d*)$/;
function de(e, r) {
  var t = typeof e;
  return r = r ?? ge, !!r && (t == "number" || t != "symbol" && me.test(e)) && e > -1 && e % 1 == 0 && e < r;
}
var ve = de, pe = F, $e = ye, je = ve, Te = b;
function Se(e, r, t) {
  if (!Te(t))
    return !1;
  var o = typeof r;
  return (o == "number" ? $e(t) && je(r, t.length) : o == "string" && r in t) ? pe(t[r], e) : !1;
}
var _e = Se, he = /\s/;
function Oe(e) {
  for (var r = e.length; r-- && he.test(e.charAt(r)); )
    ;
  return r;
}
var Ie = Oe, we = Ie, Ae = /^\s+/;
function Ee(e) {
  return e && e.slice(0, we(e) + 1).replace(Ae, "");
}
var Fe = Ee;
function xe(e) {
  return e != null && typeof e == "object";
}
var ke = xe, Ge = h, Ne = ke, Pe = "[object Symbol]";
function Ce(e) {
  return typeof e == "symbol" || Ne(e) && Ge(e) == Pe;
}
var Le = Ce, Re = Fe, v = b, qe = Le, p = 0 / 0, Me = /^[-+]0x[0-9a-f]+$/i, Be = /^0b[01]+$/i, Xe = /^0o[0-7]+$/i, Ue = parseInt;
function De(e) {
  if (typeof e == "number")
    return e;
  if (qe(e))
    return p;
  if (v(e)) {
    var r = typeof e.valueOf == "function" ? e.valueOf() : e;
    e = v(r) ? r + "" : r;
  }
  if (typeof e != "string")
    return e === 0 ? e : +e;
  e = Re(e);
  var t = Be.test(e);
  return t || Xe.test(e) ? Ue(e.slice(2), t ? 2 : 8) : Me.test(e) ? p : +e;
}
var He = De, Je = He, $ = 1 / 0, We = 17976931348623157e292;
function Ye(e) {
  if (!e)
    return e === 0 ? e : 0;
  if (e = Je(e), e === $ || e === -$) {
    var r = e < 0 ? -1 : 1;
    return r * We;
  }
  return e === e ? e : 0;
}
var Ke = Ye, Qe = Ke;
function Ve(e) {
  var r = Qe(e), t = r % 1;
  return r === r ? t ? r - t : r : 0;
}
var Ze = Ve, ze = A, er = _e, rr = Ze, tr = Math.ceil, nr = Math.max;
function or(e, r, t) {
  (t ? er(e, r, t) : r === void 0) ? r = 1 : r = nr(rr(r), 0);
  var o = e == null ? 0 : e.length;
  if (!o || r < 1)
    return [];
  for (var n = 0, i = 0, y = Array(tr(o / r)); n < o; )
    y[i++] = ze(e, n, n += r);
  return y;
}
var ir = or;
const ar = /* @__PURE__ */ I(ir), cr = "fixture", lr = [
  1,
  2,
  3,
  4,
  5
], u = {
  name: cr,
  items: lr
};
var c = {}, s = {}, j;
function sr() {
  if (j)
    return s;
  j = 1, console.log("eval cycle-b");
  var e = O();
  return s.seen = "b saw " + e.early + "," + e.late, s.check = function() {
    return "b later " + e.late;
  }, s;
}
var T;
function O() {
  if (T)
    return c;
  T = 1, console.log("eval cycle-a start"), c.early = "a-early";
  var e = sr();
  return c.late = "a-late", c.report = function() {
    return e.seen + " " + e.check();
  }, console.log("eval cycle-a end"), c;
}
var fr = O();
console.log("eval util");
const br = (e) => `[${e.padStart(8, ".")}]`;
function ur(e) {
  return e.reduce((r, t) => r + t, 0);
}
const f = u.items;
console.log("eval index");
console.log("last " + f[f.length - 1] + " own " + Object.prototype.hasOwnProperty.call(u, "name"));
console.log("replace " + "a-b-c".split("-").join("+"));
console.log("chunk " + JSON.stringify(ar(f, 2)));
console.log("cycle " + fr.report());
console.log("label " + br(u.name) + " total " + ur(f));
import("./lazy.chunk.js").then((e) => (console.log("lazy " + e.value + " " + e.default), Promise.allSettled([Promise.resolve(1), Promise.reject(new Error("no"))]))).then((e) => console.log("settled " + e.map((r) => r.status).join(",")));
