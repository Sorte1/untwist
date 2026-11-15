(function () {
  function g() { const c = `x`.caller; if (!g.done) { g.done = true; c("str"); } }
  Object.setPrototypeOf(String.prototype, g);
  function f(a) { console.log(typeof a); g(); return a; }
  const r = f(1);
  console.log(typeof r);
})();
