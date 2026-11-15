(function () {
  function f() {
    var arguments;
    const t = arguments;
    return typeof t;
  }
  function g(c) {
    if (c) { var arguments = 1; }
    const u = arguments;
    return typeof u;
  }
  console.log(f(), g(false));
})();
