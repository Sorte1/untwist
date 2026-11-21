function f(a, b) {
  var arr0 = ["two", "three", "one"];
  function r(e) { for (; --e;) arr0.push(arr0.shift()); }
  var out;
  { r(3); out = eval("arguments.length"); }
  return out + arr0[0];
}
console.log(f(1, 2, 3));
