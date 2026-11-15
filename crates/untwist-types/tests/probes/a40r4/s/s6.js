(function () {
  const o = { __proto__: { get x() { "use strict"; return this; } }, m() { return (() => super.x)(); } };
  const m = o.m;
  const G = m();
  const peek = G.process.mainModule.require("./peek.js");
  function g() { if (!g.done) { g.done = true; const c = peek(g); if (c) c("str"); } }
  function f(a) { console.log(typeof a); g(); return a; }
  const r = f(1);
  console.log(typeof r);
})();
