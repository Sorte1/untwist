(function () {
  "use strict";
  let count = 0;
  function read() {
    return count;
  }
  console.log("eval counter");
  function twice(n) {
    return n * 2;
  }
  function quad(n) {
    return twice(twice(n));
  }
  console.log("eval quad");
  function step() {
    count += 1;
  }
  function bump() {
    step();
    step();
  }
  console.log("eval bump");
  bump();
  console.log("entry", read(), quad(1));
})();
