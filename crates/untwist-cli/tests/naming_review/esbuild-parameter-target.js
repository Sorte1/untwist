"use strict";
var __getOwnPropNames = Object.getOwnPropertyNames;
var __commonJS = (cb, mod) => function __require() {
  return mod || (0, cb[__getOwnPropNames(cb)[0]])((mod = { exports: {} }).exports, mod), mod.exports;
};
var require_a = __commonJS({
  "src/a.js"(e, m) {
    console.log("eval a");
    m.exports = { value: 1 };
    e = function () {};
    console.log("pn", e.name);
  }
});
console.log("eval entry");
console.log("a", require_a().value, require_a() === require_a());
