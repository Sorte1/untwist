(function () {
  var t, e;
  function r() { if (t) return e; t = 1; e = { who: w() }; return e; }
  function w() { var { caller: c } = w; return c.name; }
  console.log(r().who);
})();
