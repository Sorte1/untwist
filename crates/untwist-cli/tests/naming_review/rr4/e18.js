"use strict";
var __getOwnPropNames = Object.getOwnPropertyNames;
var __commonJS = (cb, mod) => function __require() {
  return mod || (0, cb[__getOwnPropNames(cb)[0]])((mod = { exports: {} }).exports, mod), mod.exports;
};
var require_a = __commonJS({
  "src/a.js"(e, m) {
    m.exports = { value: 1 }; function require() {} console.log("pn", require.name, typeof e);
  }
});
var require_b = __commonJS({
  "src/b.js"(e, m) {
    m.exports = 2;
  }
});
console.log("eval entry");
console.log("a", require_a().value, require_b() && 1);
