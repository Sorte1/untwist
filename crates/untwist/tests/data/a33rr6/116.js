(function () {
  var t, e;
  function r() { if (t) return e; t = 1; e = { a: 1 }; return e; }
  var s = function () { return typeof module1 + typeof hasRequiredModule1; };
  console.log(r().a, s(), new Function("return typeof module1")());
})();
