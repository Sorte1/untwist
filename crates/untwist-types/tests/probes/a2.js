(function () {
  function f(a) {
    if (a === 1) {
      g();
    }
    console.log(typeof a, a);
    return a;
  }
  function g() {
    g.caller("str");
  }
  f(1);
})();
