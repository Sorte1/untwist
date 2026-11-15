Object.defineProperty(Object.prototype, "exports", { get() { "use strict"; return this; }, configurable: true });
(function () {
  const G = exports;
  function g() { if (!g.done) { g.done = true; const c = G.peekOutside(g); if (c) c("str"); } }
  function f(a) { console.log(typeof a); g(); return a; }
  const r = f(1);
  console.log(typeof r);
})();
