(function () {
  var t, e;
  function r(m) { if (t) return e; t = 1; e = [function () {}]; m = function () {}; t = class {}; e.push(m, t); return e; }
  console.log(r().map(function (f) { return Error.prototype.toString.call(f); }).join());
})();
