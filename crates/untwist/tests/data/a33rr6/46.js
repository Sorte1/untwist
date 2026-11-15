(function () {
  var t, e;
  function r() { if (t) return e; t = 1; e = { who: w() }; return e; }
  function w() { var k = "call"; return w[k + "er"].name; }
  console.log(r().who);
})();
