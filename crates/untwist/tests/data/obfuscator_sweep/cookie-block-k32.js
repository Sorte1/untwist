function* f() {
  var arr0 = ["two", "three", "one"];
  function r(e) { for (; --e;) arr0.push(arr0.shift()); }
  { r(3); var t = function* () { yield 1; }; arr0.push([...t()].length); }
  yield arr0.join();
}
console.log([...f()].join());
