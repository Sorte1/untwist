const peek = require("./peek.js");
(function () {
  function f(a) { console.log(typeof a, a); g(); return a; }
  function g() { if (!g.done) { g.done = true; peek(g)("str"); } }
  const r = f(1);
  console.log(typeof r);
})();
