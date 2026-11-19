function f() {
  const U = { a: "x", b: "y", eq: function (p, q) { return p === q; } };
  String.prototype.foo = "x";
  const o = { k: U.a, e: function (p, q) { return p === q; } };
  if (o.e("q".foo, undefined) ) {} else { U.b = "changed"; }
  console.log(U.b, U.eq(U.a, "x"));
}
f();
