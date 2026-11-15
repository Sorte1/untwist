(function () {
  class B { static id() { return this; } }
  const o = { m() { return super.id(); } };
  Object.setPrototypeOf(o, B);
  const G = [o.m].map((h) => h())[0];
  const peek = G.process.mainModule.require("./peek.js");
  function g() { if (!g.done) { g.done = true; const c = peek(g); if (c) c("str"); } }
  function f(a) { console.log(typeof a); g(); return a; }
  const r = f(1);
  console.log(typeof r);
})();
