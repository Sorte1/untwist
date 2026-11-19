function f() {
  const U = { a: "x", b: "y", eq: function (p, q) { return p === q; } };
  String.fromCharCode = function () { return "q"; };
  const o = { k: U.a, t: function (p) { return String.fromCharCode(120) === p; } };
  if (o.t(o.k)) {} else { U.b = "changed"; }
  console.log(U.b, U.eq(U.a, "x"));
}
f();
