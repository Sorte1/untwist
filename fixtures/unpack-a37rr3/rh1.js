(function () {
  function add(a, b) {
    return a + b;
  }
  class K {}
  const base = 10;
  function addBase(n) {
    return add(n, base);
  }
  zz = 3;
  console.log("eval math", zz, typeof K, (function () { return typeof this; })());
  function shout(s) {
    return s.toUpperCase();
  }
  const greeting = "hi";
  function greet() {
    return shout(greeting);
  }
  console.log("eval greet");
  console.log("entry", addBase(1), greet());
})();
