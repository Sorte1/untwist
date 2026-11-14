(function () {
  var hasRequiredE, e;
  function requireE() { if (hasRequiredE) return e; hasRequiredE = 1; e = { v: 1 }; return e; }
  (function () {
    var b, x;
    function requireE() { if (b) return x; b = 1; x = { v: 2 }; return x; }
    console.log(requireE(), e, hasRequiredE);
    (function () {
      var c, y;
      function requireE() { if (c) return y; c = 1; y = { v: 3 }; return y; }
      console.log(requireE(), e, x, b, hasRequiredE);
    })();
  })();
  console.log(requireE());
})();
