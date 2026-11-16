(function () {
  var t, e;
  function r() { if (t) return e; t = 1; e = { who: w() }; return e; }
  function w() { var s; Error.prepareStackTrace = function (_, sites) { return sites; }; s = new Error().stack; Error.prepareStackTrace = undefined; return s[1].getFunctionName(); }
  console.log(r().who);
})();
