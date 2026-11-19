function f() {
  const U = { a: "x", b: "y", eq: function (p, q) { return p === q; } };
  String.prototype.foo = "x";
  const o = { k: U.a, t: function (p) { return p.foo === undefined; } };
  if (o.t(o.k)) {} else { U.b = "changed"; }
  console.log(U.b, U.eq(U.a, "x"));
}
f();
