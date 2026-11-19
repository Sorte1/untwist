function h() {
  eval("var undefined = 'x'");
  (function () {
    const U = { a: "x", b: "y", eq: function (p, q) { return p === q; } };
    const o = { k: U.a, j: undefined };
    if (o.j === "x") { U.b = "changed"; }
    console.log(U.b, U.eq(U.a, "x"));
  })();
}
h();
