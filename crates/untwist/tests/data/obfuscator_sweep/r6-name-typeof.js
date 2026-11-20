function f() {
  const U = { a: "x", b: "y", eq: function (p, q) { return p === q; } };
  
  const o = { k: U.a, t: function undefined(p) { return typeof undefined === p; } };
  
  if (o.t("function")) { U.b = "changed"; }
  console.log(U.b, U.eq(U.a, "x"));
}
f();
