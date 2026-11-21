function f(c) {
  var arr0 = ["two", "three", "one"];
  function r(e) { for (; --e;) arr0.push(arr0.shift()); }
  { r(3); var q = 5; }
  if (c) q = 6;
  return q + arr0[0];
}
console.log(f(0));
