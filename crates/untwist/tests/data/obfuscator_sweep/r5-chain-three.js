function f() {
  const U = { a: "x", b: "y", eq: function (p, q) { return p === q; } };
  function g() { this.v = "z"; }
  const p = { v: U.a, t: g };
  const o = { k: p.v };
  p.t();
  const o2 = { k: p.v };
  if (o2.k !== "x") { U.b = "changed"; }
  console.log(U.b, U.eq(U.a, "x"));
}
f();
