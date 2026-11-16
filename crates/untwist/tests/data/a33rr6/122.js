(function () {
  var t, e = { exports: {} };
  function w() { return typeof w.caller.arguments[1]; }
  function r() { if (t) return e.exports; t = 1; (function (n, o) { o = function () {}; n.exports = o; o.m = w(); })(e, e.exports); return e.exports; }
  console.log(r().name, r().m);
})();
