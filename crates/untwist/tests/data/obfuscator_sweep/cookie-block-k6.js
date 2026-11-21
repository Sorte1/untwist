function f() {
  var arr0 = ["two", "three", "one"];
  function r(e) { for (; --e;) arr0.push(arr0.shift()); }
  { r(3); for (var [i] of [[9]]) {} }
  return i + " " + arr0[0];
}
console.log(f());
