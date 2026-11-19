function f() {
  const U = { a: "x", b: "y", eq: function (p, q) { return p === q; } };
  
  const o = { k: U.a, t: function Number(p) { return Number.MAX_SAFE_INTEGER > p; } };
  
  if (o.t(3)) { U.b = "changed"; }
  console.log(U.b, U.eq(U.a, "x"));
}
f();
