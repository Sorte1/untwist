function f() {
  var arr0 = ["two", "three", "one"];
  { function r(e) { for (; --e;) arr0.push(arr0.shift()); } r(3); }
  return typeof r + " " + arr0[0];
}
console.log(f());
