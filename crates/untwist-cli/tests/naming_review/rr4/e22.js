"use strict";
var __getOwnPropNames = Object.getOwnPropertyNames;
var __commonJS = (cb, mod) => function __require() {
  return mod || (0, cb[__getOwnPropNames(cb)[0]])((mod = { exports: {} }).exports, mod), mod.exports;
};
var require_a = __commonJS({
  "src/a.js"(e, m) {
    m.exports = { value: 1 }; var q = function () { var out = []; for (var exports = function () {}, i = 0; i < 1; i++) out.push(exports.name); for (let module = class {}; out.length < 2;) out.push(module.name); return out.concat(typeof e, typeof m).join(" "); }; console.log("pn", q());
  }
});
var require_b = __commonJS({
  "src/b.js"(e, m) {
    m.exports = 2;
  }
});
console.log("eval entry");
console.log("a", require_a().value, require_b() && 1);
