(function () {
  function f(a) {
    var a = "s";
    console.log(typeof a, a);
  }
  f(1);
  function g(b) {
    function b() {}
    console.log(typeof b);
  }
  g(2);
})();
