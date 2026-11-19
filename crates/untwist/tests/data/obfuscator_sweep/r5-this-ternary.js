function f() {
  const U = { a: "x", b: "y", eq: function (p, q) { return p === q; } };
  function g() { this.k = "z"; }
  const o = { k: U.a, t: g };
  o.t();
  console.log(o.k !== "x" ? (U.b = "changed") : "kept");
  console.log(U.b, U.eq(U.a, "x"));
}
f();
