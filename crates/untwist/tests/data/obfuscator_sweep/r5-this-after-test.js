function f() {
  const U = { a: "x", b: "y", eq: function (p, q) { return p === q; } };
  function g() { this.k = "z"; }
  const o = { k: U.a, t: g };
  for (var i = 0; i < 2; i++) { if (o.k !== "x") { U.b = "changed"; } o.t(); }
  console.log(U.b, U.eq(U.a, "x"));
}
f();
