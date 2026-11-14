(function () {
  var a, e;
  function requireE() { if (a) return e; a = 1; e = { v: 1 }; return e; }
  requireE();
  {
    let hasRequiredE = 9, e2 = 3;
    console.log(a, e, hasRequiredE, e2);
    var q = function () { return [a, e, hasRequiredE]; };
  }
  console.log(JSON.stringify(q()));
})();
