globalThis.x = "G";
function g() {
  const o = { abcde: "a", fghij: function (p, q) { return p + q; }, klmno: function (p) { return p(); } };
  function inner() {
    if (o.abcde === "a") { let x = 1; console.log(x); }
    return o.fghij("v", o.klmno(function () { return x; }));
  }
  return inner();
}
console.log(g());
