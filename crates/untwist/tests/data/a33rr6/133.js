(function () {
  function w() { return w.caller === null ? "null" : w.caller.name; }
  var q = (function () { return w(); })();
  var o = { get g() { return w(); }, m: function () { return w(); } };
  var s = (0, o.m)();
  var u = !function () { q += w(); }();
  console.log(q, o.g, o.m(), s);
})();
