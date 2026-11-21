function f() {
  var arr0 = ["two", "three", "one"];
  function r(e) { for (; --e;) arr0.push(arr0.shift()); }
  { function g() { return 7; } r(3); }
  return typeof g + " " + arr0[0];
}
console.log(f());
