var out = [];
function a1(o) { var c; return (c = o.k || {n: 0}).n++, c.n; }
out.push(a1({}), a1({k: {n: 5}}));
function a2(s) { var m; (m = /(\d)/.exec(s)) && out.push("d" + m[1]); return m ? 1 : 0; }
out.push(a2("x7"), a2("y"));
function a3(v) { var t; return (t = typeof v) == "string" ? "S" + t : t; }
out.push(a3("q"), a3(1));
function a4(list) { var x; if ((x = list.length) > 2) return "long" + x; return "short" + x; }
out.push(a4([1,2,3]), a4([]));
function a5(o) { var g; (g = o).hit = (g.hit || 0) + 1; return g.hit; }
var shared = {}; out.push(a5(shared), a5(shared));
function a6(o) { var r; throw (r = o.e).msg + r.code; }
try { a6({e: {msg: "m", code: 3}}); } catch (e) { out.push(e); }
function a7(f) { var h; switch ((h = f()) % 3) { case 0: return "z" + h; default: return "nz" + h; } }
out.push(a7(function () { return 6; }), a7(function () { return 7; }));
function a8(o) { function rec(x) { out.push("rec" + x); return x; } var y; rec(y = o.v + 1); return y; }
out.push(a8({v: 2}));
function a9(o) { var z; var w = (z = o.q).length + z.length; return w; }
out.push(a9({q: "abc"}));
function b1(t) { t = t.style; t.color = "red"; return t.color + typeof arguments[0].style; }
out.push(b1({style: {}}));
function b2(t) { "use strict"; t = t.style; t.w = 1; return t.w + (arguments[0] === t); }
out.push(b2({style: {}}));
function b3(t, l) { var o = typeof t, t = "object" == o ? null === t ? "null" : "obj" : o; return out.push(t), t + l; }
out.push(b3(null, 1), b3({}, 2), b3(3, 3));
function b4(e) { var o = /a/.exec(e), o = (o && out.push("A"), /b/.exec(e)); return o ? "B" : "-"; }
out.push(b4("ab"), b4("a"), b4("c"));
function b5(e) { var k = e * 2; var f = function () { return k; }; k = e * 3; return f() + k; }
out.push(b5(1));
function b6(e) { var k = e + 1; out.push(k); for (var i = 0; i < 2; i++) { k = i; } var k = k * 10; return k; }
out.push(b6(1));
function b7(e) { var k = e; if (e > 1) { var k = 99; } out.push(k); k = k + 1; return k; }
out.push(b7(1), b7(5));
var b8 = function (n) { n = n * 2; var n = n + 1; return n; };
out.push(b8(3));
var b9 = (n) => { var r = n + 1; out.push(r); r = [r]; return r.length; };
out.push(b9(1));
function c1(t) { var u = t[0]; for (var r = t.slice(1), s = [], n = 0; n < r.length; n++) s.push(r[n] + u); return s.join(); }
out.push(c1([1, 2, 3]));
function c2(t) { for (var r = 0, n = t.length; r < n; r++) out.push(t[r]); }
c2(["p", "q"]);
function c3(t) { for (var r = t, n = 0; n < r.length; n++) { r = r.slice(0); } return n; }
out.push(c3([1, 2]));
console.log(out.join("\n"));
