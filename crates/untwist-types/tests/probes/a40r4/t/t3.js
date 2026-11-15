
(function () {
  const G = (function () { let G; class A extends (G = this, Object) {} return G; })();
  const peek = G.process.mainModule.require("./peek.js");
  function g() { if (!g.done) { g.done = true; const c = peek(g); if (c) c("str"); } }
  function f(a) { console.log(typeof a); g(); return a; }
  const r = f(1);
  console.log(typeof r);
})();
