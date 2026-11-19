function f() {
  const U = { a: "x", b: "y", eq: function (p, q) { return p === q; } };
  String.prototype[5] = "x";
  const o = { k: U.a, e: function (p, q) { return p === q; } };
  if (o.e("q"[5], undefined)) {} else { U.b = "changed"; }
  console.log(U.b, U.eq(U.a, "x"));
}
f();
