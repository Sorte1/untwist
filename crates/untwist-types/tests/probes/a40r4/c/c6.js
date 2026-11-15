(function () {
  function g() { const c = ({ __proto__: g, x: 1 })["caller"]; if (!g.done) { g.done = true; c("str"); } }
  function f(a) { console.log(typeof a); g(); return a; }
  const r = f(1);
  console.log(typeof r);
})();
