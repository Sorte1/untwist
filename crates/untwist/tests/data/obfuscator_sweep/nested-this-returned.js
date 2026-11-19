function f() {
  const U = { a: "x", b: "y", eq: function (p, q) { return p === q; } };
  const o = { k: U.a, self: function () { return this; } };
  o.self().k = "z";
  if (o.k !== "x") { U.b = "changed"; }
  console.log(U.b, U.eq(U.a, "x"));
}
f();
