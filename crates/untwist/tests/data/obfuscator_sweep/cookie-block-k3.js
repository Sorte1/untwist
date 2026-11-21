function f() {
  var h = 1;
  var arr0 = ["two", "three", "one"];
  function r(e) { for (; --e;) arr0.push(arr0.shift()); }
  { function h() { return 7; } r(3); }
  return typeof h + " " + arr0[0];
}
console.log(f());
