function f() {
  var arr0 = ["two", "three", "one"];
  function r(e) { for (; --e;) arr0.push(arr0.shift()); }
  lab: { r(3); }
  return arr0[0];
}
console.log(f());
