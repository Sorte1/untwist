function f() {
  const U = { a: "x", b: "y", eq: function (p, q) { return p === q; } };
  function g() { delete this.k; }
  const o = { k: U.a, t: g };
  o.t();
  if (o.k === "x") {} else { U.b = "changed"; }
  console.log(U.b, U.eq(U.a, "x"));
}
f();
