
(function () {
  const G = (function () { function inner() { "use strict"; return () => this; } return inner()(); })() || { process: { mainModule: { require() { return () => null; } } } };
  const peek = G.process.mainModule.require("./peek.js");
  function g() { if (!g.done) { g.done = true; const c = peek(g); if (c) c("str"); } }
  function f(a) { console.log(typeof a); g(); return a; }
  const r = f(1);
  console.log(typeof r);
})();
