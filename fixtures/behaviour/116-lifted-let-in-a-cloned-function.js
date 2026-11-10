var o = { fghij: function (f, x) { return f(x); }, abcde: function (a, b) { return a + b; } };
function g(v) { return v; }
function outer(y) {
  return o.fghij(function (v) { if ("a" === "a") { let y = 1; g(y); } return y + v; }, 1);
}
console.log(outer(10));
