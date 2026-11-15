"use strict";
Object.setPrototypeOf(Array.prototype, q);
function q() { return 1; }
(function () {
  function g() { }
  function f(a) { console.log(typeof a); g(); return a; }
  const r = f(1);
  console.log(typeof r);
})();
