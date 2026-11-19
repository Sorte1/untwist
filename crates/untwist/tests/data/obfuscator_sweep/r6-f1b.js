function f() {
  const U = { a: "x", b: "y", eq: function (p, q) { return p === q; } };
  const o = { k: U.a, t: function parseInt(p) { return p === "5" ? "zz" : parseInt("5"); } };
  if (o.t("q") === "zz") { U.b = "changed"; }
  console.log(U.b, U.eq(U.a, "x"));
}
f();
