function f() {
  const U = { a: "x", b: "y", eq: function (p, q) { return p === q; } };
  
  const o = { k: U.a, t: function undefined(p) { return undefined !== p; } };
  
  if (o.t(void 0)) { U.b = "changed"; }
  console.log(U.b, U.eq(U.a, "x"));
}
f();
