function f() {
  String.prototype.foo = "x";
  const U = { a: "x", b: "y", eq: function (p, q) { return p === q; } };
  if (U.eq("q".foo, undefined)) {} else { U.b = "changed"; }
  console.log(U.b, U.eq(U.a, "x"));
}
f();
