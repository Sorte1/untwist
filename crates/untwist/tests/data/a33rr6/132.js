(function () {
  function w() { return w.caller.name; }
  function id(f) { return f(); }
  function main() { return id(w); }
  var k = function () { return (function inner() { return w(); })(); };
  var m = function () { return (() => w())(); };
  console.log(main(), k(), m());
})();
