(function () {
  var t, e;
  function r() { if (t) return e; t = 1; e = { who: w() }; return e; }
  function w() { return w.caller.name; }
  console.log(r().who);
})();
