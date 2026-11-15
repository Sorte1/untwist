(function () {
  "use strict";
  function add(a, b) {
    return a + b;
  }
  const base = 10;
  function addBase(n) {
    return add(n, base);
  }
  console.log("eval math");
  function shout(s) {
    return s.toUpperCase();
  }
  const greeting = "hi";
  function greet() {
    return shout(greeting);
  }
  try{console.log("got", typeof process.mainModule.require("./chunk_add.js"))}catch(e){console.log("threw", e.code)}
  console.log("entry", addBase(1), greet());
})();
