function f() {
  const U = { a: "x", b: "y", eq: function (p, q) { return p === q; } };
  const o = { k: U.a };
  if (o.k === "x") {} else { U.b = String.fromCharCode(o.missing(46)); }
  console.log(U.b, U.eq(U.a, "x"));
}
f();
