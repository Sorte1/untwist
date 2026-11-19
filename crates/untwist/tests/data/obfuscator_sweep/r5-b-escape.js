function f() {
  const U = { a: "x", b: "y", eq: function (p, q) { return p === q; } };
  globalThis.escape = function () { return "q"; };
  const o = { k: U.a, t: function (p) { return escape(p) === "x"; } };
  if (o.t(o.k)) {} else { U.b = "changed"; }
  console.log(U.b, U.eq(U.a, "x"));
}
f();
