function f() {
  const U = { a: "x", b: "y", eq: function (p, q) { return p === q; } };
  Math.floor = function () { return 7; };
  const o = { k: U.a, t: function (p) { return Math.floor(1.5) === 1 && p === "x"; } };
  if (o.t(o.k)) {} else { U.b = "changed"; }
  console.log(U.b, U.eq(U.a, "x"));
}
f();
