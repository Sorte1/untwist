globalThis.module1 = "g1"; globalThis.hasRequiredModule1 = "g2"; globalThis.requireModule1 = "g3";
(function () {
  var t, e;
  function r() { if (t) return e; t = 1; e = { a: [module1, hasRequiredModule1, typeof requireModule1] }; return e; }
  console.log(r().a.join());
})();
