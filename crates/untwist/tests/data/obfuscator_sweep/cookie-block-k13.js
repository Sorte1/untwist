function f() {
  var q = 1;
  var arr0 = ["two", "three", "one"];
  function r(e) { for (; --e;) arr0.push(arr0.shift()); }
  { var [q] = [5]; r(3); }
  return q + " " + arr0[0];
}
console.log(f());
