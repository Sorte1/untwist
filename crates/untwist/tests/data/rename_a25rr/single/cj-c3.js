(function () {
  var a, e;
  function requireE() { if (a) return e; a = 1; e = { v: 1 }; return e; }
  function other(hasRequiredE, e2) {
    var b, x;
    function requireE2() { if (b) return x; b = 1; x = { v: 2 }; return x; }
    return [requireE2(), requireE(), e, a, hasRequiredE, e2];
  }
  console.log(JSON.stringify(other(5, 6)));
})();
