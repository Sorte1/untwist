function f(c) {
  var arr0 = ["two", "three", "one"];
  function r(e) { for (; --e;) arr0.push(arr0.shift()); }
  { r(3); var q = 5; globalThis.peek9 = function () { return q; }; }
  var q = 6;
  return peek9() + arr0[0];
}
console.log(f(0));
