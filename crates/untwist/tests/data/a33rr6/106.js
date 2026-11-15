(function () {
  var t, e;
  function r() { if (t) return e; t = 1; e = { a: arguments.callee.name, b: r.caller && r.caller.name, c: w() }; return e; }
  function w() { return w.caller.name; }
  function z() { return r(); }
  console.log(JSON.stringify(z()));
})();
