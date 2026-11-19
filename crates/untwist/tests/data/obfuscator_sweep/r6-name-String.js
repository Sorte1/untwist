function f() {
  const U = { a: "x", b: "y", eq: function (p, q) { return p === q; } };
  
  const o = { k: U.a, t: function String(p) { return p === "x" ? "" : String("x"); } };
  
  if (o.t("q") === "x") { U.b = "changed"; }
  console.log(U.b, U.eq(U.a, "x"));
}
f();
