(function () {
  "use strict";
  console.log("entry", twice(2));
  function double(n) {
    return n * 2;
  }
  function twice(n) {
    return double(double(n));
  }
})();
