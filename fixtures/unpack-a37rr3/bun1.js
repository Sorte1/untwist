// @bun @bun-cjs
(function(exports, require, module, __filename, __dirname) {var __getOwnPropNames = Object.getOwnPropertyNames;
var __commonJS = (cb, mod) => () => (mod || cb((mod = { exports: {} }).exports, mod), mod.exports);
var require_a = __commonJS((exports2, module2) => {
  console.log("eval a");
  module2.exports = { value: 2 };
});
class K {}
yy = 4;
console.log("eval entry", yy, typeof K, (function () { return typeof this; })());
console.log("a", require_a().value, typeof require("path").join);
})
