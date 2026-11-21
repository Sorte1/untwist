function f() {
  var arr0 = ["two", "three", "one"];
  var al = arr0;
  function r(e) { for (; --e;) al.push(al.shift()); }
  { r(3); arr0 = ["x", "y", "z"]; }
  return arr0[0];
}
console.log(f());
