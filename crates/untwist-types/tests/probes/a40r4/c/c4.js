(function () {
  function g() { let c; { let arguments = g; c = arguments.caller; } if (!g.done) { g.done = true; c("str"); } }
  function f(a) { console.log(typeof a); g(); return a; }
  const r = f(1);
  console.log(typeof r);
})();
