function f() {
  var o = { "9223372036854775807": "wrong", b: "x" };
  console.log(o[1e20], o.b);
  var p = { inf: "wrong", b: "y" };
  console.log(p[1e999], p.b);
  var q = { "1000000000000000000000": "wrong", b: "z" };
  console.log(q[1e21], q.b);
  var r = { "NaN": "wrong", b: "w" };
  console.log(r[1e19], r[0.5], r.b);
  var s = { "0": "zero", b: "v" };
  console.log(s[-0], s[0.0], s.b);
}
f();
