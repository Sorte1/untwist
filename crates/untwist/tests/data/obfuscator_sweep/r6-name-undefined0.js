function f() {
  const U = { a: "x", b: "y", eq: function (p, q) { return p === q; } };
  
  const o = { k: U.a, t: function undefined() { return undefined !== void 0; } };
  
  if (o.t()) { U.b = "changed"; }
  console.log(U.b, U.eq(U.a, "x"));
}
f();
