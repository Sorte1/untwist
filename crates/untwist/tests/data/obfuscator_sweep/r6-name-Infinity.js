function f() {
  const U = { a: "x", b: "y", eq: function (p, q) { return p === q; } };
  
  const o = { k: U.a, t: function Infinity(p) { return Infinity < p; } };
  
  if (o.t(1)) { U.b = "changed"; }
  console.log(U.b, U.eq(U.a, "x"));
}
f();
