(function () {
  (function () {
    var b, x;
    function requireE() { if (b) return x; b = 1; x = { v: 2 }; return x; }
    requireE();
    console.log(e, typeof hasRequiredE);
  })();
  var a, e;
  function requireE() { if (a) return e; a = 1; e = { v: 1 }; return e; }
  requireE();
  console.log(e);
})();
