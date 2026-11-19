function h(s) {
  eval(s);
  (function () {
    const U = { a: "x", b: "y", eq: function (p, q) { return p === q; } };
    const o = { k: U.a, t: function (p) { return p === Infinity; } };
    if (o.t("x")) { U.b = "changed"; }
    console.log(U.b, U.eq(U.a, "x"));
  })();
}
h("var Infinity = 'x'");
