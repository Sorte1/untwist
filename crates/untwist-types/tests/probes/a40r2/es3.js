(function () {
  "use strict";
  function add(a, b) { return a + b; }
  const base = 10;
  function addBase(n) { return add(n, base); }
  console.log("eval math");
  function shout(s) { return s.toUpperCase(); }
  const greeting = "hi";
  function greet() { return shout(greeting); }
  console.log("eval greet");
  console.log("entry", addBase(1), greet(), (function () { const seen = []; function require(p) { seen.push(p); return seen.length > 3 ? null : p + seen.length; } return require("./chunk_add.js") + require("./chunk_add"); })());
})();
