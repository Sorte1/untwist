function f() {
  var arr0 = ["two", "three", "one"];
  function r(e) { for (; --e;) arr0.push(arr0.shift()); }
  { r(3); try { throw 4; } catch (e) { var z = e; } }
  return z + " " + arr0[0];
}
console.log(f());
