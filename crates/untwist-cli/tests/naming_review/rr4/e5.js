"use strict";
var __getOwnPropNames = Object.getOwnPropertyNames;
var __commonJS = (cb, mod) => function __require() {
  return mod || (0, cb[__getOwnPropNames(cb)[0]])((mod = { exports: {} }).exports, mod), mod.exports;
};
var require_a = __commonJS({
  "src/a.js"(e, m) {
    m.exports = { value: 1, q: function () { var exports = function () {}; var module = class {}; function require() {} return [exports, module, require, typeof e, typeof m]; } };
  }
});
var require_b = __commonJS({
  "src/b.js"(e, m) {
    var a = require_a(); m.exports = 2; console.log("pn", a.q().slice(0, 3).map(function (f) { return f.name; }).join(" "));
  }
});
console.log("eval entry");
console.log("a", require_a().value, require_b() && 1);
