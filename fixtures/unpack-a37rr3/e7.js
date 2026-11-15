(() => {
"use strict";
var __getOwnPropNames = Object.getOwnPropertyNames;
var __commonJS = (cb, mod) => function __require() {
  return mod || (0, cb[__getOwnPropNames(cb)[0]])((mod = { exports: {} }).exports, mod), mod.exports;
};
var require_a = __commonJS({
  "src/a.js"(exports, module) {
    console.log("a this", typeof this, this === exports, (() => typeof this)());
    module.exports = { value: 1 };
  }
});
console.log("a", require_a().value);
})();
