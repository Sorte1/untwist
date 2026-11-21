globalThis.x = "G";
function g() {
  const o = { abcde: "a", fghij: function (p, q) { return p + x + q; } };
  function inner() {
    if (o.abcde === "a") { let x = 1; console.log(x); }
    return o.fghij("v", "w");
  }
  return inner();
}
console.log(g());
