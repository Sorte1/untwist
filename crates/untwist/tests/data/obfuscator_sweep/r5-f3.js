function f() {
  const U = { a: "x", b: "y", eq: function (p, q) { return p === q; } };
  var o = { k: U.a };
  { function o() {} }
  if (o.k !== "x") { U.b = "changed"; }
  console.log(U.b, U.eq(U.a, "x"));
}
f();
