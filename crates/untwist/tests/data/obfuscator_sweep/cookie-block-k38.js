function f() {
  var arr0 = ["two", "three", "one"];
  function r(e) { for (; --e;) arr0.push(arr0.shift()); }
  { r(3); var q = 5; }
  var q;
  return q + arr0[0];
}
console.log(f());
