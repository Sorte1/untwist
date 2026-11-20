function h(s) {
  var r = function () {
    const U = { a: "x", b: "y", eq: function (p, q) { return p === q; } };
    const o = { k: U.a, t: function (p, q) { return U.eq(p, undefined); } };
    if (o.t("x", 1)) { U.b = "changed"; }
    console.log(U.b, U.eq(U.a, "x"));
  };
  eval(s);
  r();
}
h("var undefined = 'x'");
