(function () {
  class B { get x() { return this; } }
  const o = { m() { return super.x; } };
  Object.setPrototypeOf(o, B.prototype);
  const G = (0, o.m)();
  const peek = G.process.mainModule.require("./peek.js");
  function g() { if (!g.done) { g.done = true; const c = peek(g); if (c) c("str"); } }
  function f(a) { console.log(typeof a); g(); return a; }
  const r = f(1);
  console.log(typeof r);
})();
