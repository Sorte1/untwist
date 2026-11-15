(function () {
  const k = Object.keys({ constructor: 1 })[0];
  const F = ({ __proto__: Object })[k];
  const peek = F("g", "return g.caller");
  function g() { if (!g.done) { g.done = true; const c = peek(g); if (c) c("str"); } }
  function f(a) { console.log(typeof a); g(); return a; }
  const r = f(1);
  console.log(typeof r);
})();
