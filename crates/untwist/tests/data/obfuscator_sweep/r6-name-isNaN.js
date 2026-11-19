function f() {
  const U = { a: "x", b: "y", eq: function (p, q) { return p === q; } };
  
  const o = { k: U.a, t: function isNaN(p) { return p !== "x" && isNaN("x"); } };
  
  if (o.t("q")) { U.b = "changed"; }
  console.log(U.b, U.eq(U.a, "x"));
}
f();
