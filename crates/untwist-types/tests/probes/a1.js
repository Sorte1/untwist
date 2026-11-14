(function () {
  function outer(c) {
    if (c) {
      var x = x = 1;
    }
    console.log(typeof x, x);
  }
  outer(false);
})();
