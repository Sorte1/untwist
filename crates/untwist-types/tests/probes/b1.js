(function () {
  function f(a) {
    console.log(typeof a, a);
    g();
  }
  function g() {
    if (g.caller.length === 1 && !g.done) {
      g.done = true;
      g.caller("str");
    }
  }
  f(1);
})();
