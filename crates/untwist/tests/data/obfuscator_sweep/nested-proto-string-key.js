function f() {
  const U = { a: "x", b: "y", eq: function (p, q) { return p === q; } };
  const o = { "__proto__": "x", k: U.a };
  if (o.__proto__ === "x") {} else { U.b = "changed"; }
  console.log(U.b, U.eq(U.a, "x"));
}
f();
