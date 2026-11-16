(function () {
  var t, e = { exports: {} };
  function r() { if (t) return e.exports; t = 1; (function (n, o) { o = function () {}; n.exports = o; n = () => 2; n.exports = n.name; o.m = n; })(e, e.exports); return e.exports; }
  console.log(r().name, r().m.name);
})();
