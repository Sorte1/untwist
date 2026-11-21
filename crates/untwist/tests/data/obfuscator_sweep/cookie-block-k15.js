function f() {
  var arr0 = ["two", "three", "one"];
  function r(e) { for (; --e;) arr0.push(arr0.shift()); }
  { r(3); if (arr0.length) { function g() { return 7; } } }
  return typeof g + " " + arr0[0];
}
console.log(f());
