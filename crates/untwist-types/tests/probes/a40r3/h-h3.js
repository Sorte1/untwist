function g() { if (!g.done) { g.done = true; const c = this.peekOutside(g); if (c) c("str"); } }
(function () {
  function f(a) { console.log(typeof a); g(); return a; }
  const r = f(1);
  console.log(typeof r);
})();
