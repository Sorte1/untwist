(function () {
  function h(arguments) { return arguments.caller; }
  function g() { const c = h(g); if (!g.done) { g.done = true; c("str"); } }
  function f(a) { console.log(typeof a); g(); return a; }
  const r = f(1);
  console.log(typeof r);
})();
