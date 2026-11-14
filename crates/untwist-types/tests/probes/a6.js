(function () {
  function f() {
    try {
      throw 0;
    } catch (e) {
      var e = "s";
    }
    console.log(typeof e, e);
  }
  f();
})();
