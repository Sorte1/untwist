(function () {
  "use strict";
  function add(a, b) {
    return a + b;
  }
  const base = 10;
  function addBase(n) {
    return add(n, base);
  }
  console.log("eval math", typeof module);
  console.log("entry", addBase(1));
})();
