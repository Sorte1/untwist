Object.defineProperty(Object.prototype, "exports", { set(v) { "use strict"; v.G = this; }, configurable: true });
(function () {
  const box = {};
  exports = box;
  const G = box.G;
  function g() { if (!g.done) { g.done = true; const c = G.peekOutside(g); if (c) c("str"); } }
  function f(a) { console.log(typeof a); g(); return a; }
  const r = f(1);
  console.log(typeof r);
})();
