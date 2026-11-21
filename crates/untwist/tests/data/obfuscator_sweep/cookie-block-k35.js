function f(a, b) {
  var arr0 = ["two", "three", "one"];
  function r(e) { for (; --e;) arr0.push(arr0.shift()); }
  { r(3); var g = () => arguments.length; }
  return arr0[0];
}
console.log(f(1, 2, 3));
