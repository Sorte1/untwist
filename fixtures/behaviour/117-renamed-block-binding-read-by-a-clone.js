var o = { fghij: function (f, x) { return f(x); }, abcde: function (a, b) { return a + b; } };
function g(v) { return v; }
function outer(y) {
  if ("a" === "a") {
    let y = 1;
    console.log(o.fghij(g, y));
  }
  return y;
}
console.log(outer(10));
