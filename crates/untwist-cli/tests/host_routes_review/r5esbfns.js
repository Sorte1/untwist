"use strict";
var __getOwnPropNames = Object.getOwnPropertyNames;
var __commonJS = (cb, mod) => function __require() {
  return mod || (0, cb[__getOwnPropNames(cb)[0]])((mod = { exports: {} }).exports, mod), mod.exports;
};
var require_a = __commonJS({
  "src/a.js"(exports, module) {
    module.exports = "a";
  }
});
var require_b = __commonJS({ "src/b.js"(exports, module) { Function("return import('./a.js')")().then(function(m){console.log("got", m.default||m)}, function(e){console.log("threw", e.code||e.message)}); } });
console.log("entry", require_a());
require_b();
