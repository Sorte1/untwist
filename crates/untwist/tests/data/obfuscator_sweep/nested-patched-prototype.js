function f() {
  const U = { a: "x", b: "y", eq: function (p, q) { return p === q; } };
  Object.prototype.zz = function () { this.k = "z"; };
  const o = { k: U.a };
  o.zz();
  if (o.k !== "x") { U.b = "changed"; }
  console.log(U.b, U.eq(U.a, "x"));
}
f();
