function f() {
  const U = { a: "x", b: "y", eq: function (p, q) { return p === q; } };
  const o = { k: U.a };
  o.__defineGetter__("k", function () { return "z"; });
  if (o.k !== "x") { U.b = "changed"; }
  console.log(U.b, U.eq(U.a, "x"));
}
f();
