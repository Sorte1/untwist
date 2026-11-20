function f() {
  const U = { a: "x", b: "y", eq: function (p, q) { return p === q; } };
  
  const o = { k: U.a, t: function Math(p) { return Math.PI > p; } };
  
  if (o.t(3)) { U.b = "changed"; }
  console.log(U.b, U.eq(U.a, "x"));
}
f();
