(function () {
  function g() { const k = Object.keys({ caller: 1 })[0]; const c = ({ __proto__: g })[k]; if (!g.done) { g.done = true; c("str"); } }
  function f(a) { console.log(typeof a); g(); return a; }
  const r = f(1);
  console.log(typeof r);
})();
