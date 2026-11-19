function f() {
  const U = { a: "x", b: "y", eq: function (p, q) { return p === q; } };
  String.prototype.foo = "x";
  const o = { k: U.a, j: typeof "q".foo };
  if (o.j === "undefined") {} else { U.b = "changed"; }
  console.log(U.b, U.eq(U.a, "x"));
}
f();
