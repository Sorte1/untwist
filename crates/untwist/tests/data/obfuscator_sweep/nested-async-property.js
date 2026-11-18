function f() {
  const U = { a: "x", b: "y", eq: function (p, q) { return p === q; } };
  const o = { k: U.a, t: async function (p, q) { return p === q; } };
  if (o.t(U.a, "nope")) { U.b = "changed"; }
  console.log(U.b, U.eq(U.a, "x"));
}
f();
