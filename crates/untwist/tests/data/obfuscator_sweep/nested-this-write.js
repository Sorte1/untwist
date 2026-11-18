function f() {
  const U = { a: "x", b: "y", eq: function (p, q) { return p === q; } };
  const o = { k: U.a, set: function () { this.k = "z"; } };
  o.set();
  if (o.k !== "x") { U.b = "changed"; }
  console.log(U.b, U.eq(U.a, "x"));
}
f();
