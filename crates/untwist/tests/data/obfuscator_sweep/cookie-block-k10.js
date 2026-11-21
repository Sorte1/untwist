function f() {
  var arr0 = ["two", "three", "one"];
  function r(e) { for (; --e;) arr0.push(arr0.shift()); }
  { r(3); x = 5; var x; }
  return arr0[0] + x;
}
console.log(f());
