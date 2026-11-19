function f() {
  const U = { a: "x", b: "y", eq: function (p, q) { return p === q; } };
  const G = { s: function () { this.k = "z"; } };
  const o = { k: U.a, t: G.s };
  o.t();
  if (o.k !== "x") { U.b = "changed"; }
  console.log(U.b, U.eq(U.a, "x"));
}
f();
