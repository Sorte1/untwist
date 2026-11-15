"use strict";
const exports = 0;
var __getOwnPropNames = Object.getOwnPropertyNames;
var __commonJS = (cb, mod) => function __require() {
  return mod || (0, cb[__getOwnPropNames(cb)[0]])((mod = { exports: {} }).exports, mod), mod.exports;
};
var require_a = __commonJS({ "src/a.js"(exports, module) { module.exports = { value: (function(){ return typeof this })() }; } });
console.log("a", require_a().value);
