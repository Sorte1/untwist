function f() {
  const U = { a: "x", b: "y", eq: function (p, q) { return p === q; } };
  parseInt = function () { return 7; };
  const o = { k: U.a, t: function (p) { return parseInt("1" + p) === 1; } };
  if (o.t(o.k)) {} else { U.b = "changed"; }
  console.log(U.b, U.eq(U.a, "x"));
}
f();
