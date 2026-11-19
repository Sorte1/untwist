function f() {
  const U = { a: "x", b: "y", eq: function (p, q) { return p === q; } };
  (function (g) {
    const o = { k: U.a, t: g };
    o.t();
    if (o.k !== "x") { U.b = "changed"; }
  })(function () { this.k = "z"; });
  console.log(U.b, U.eq(U.a, "x"));
}
f();
