// @bun @bun-cjs
(function(exports, require, module, __filename, __dirname) {var __getOwnPropNames = Object.getOwnPropertyNames;
var __commonJS = (cb, mod) => () => (mod || cb((mod = { exports: {} }).exports, mod), mod.exports);
var require_a = __commonJS((exports2, module2) => {
  console.log("eval a");
  module2.exports = { value: 2 };
});
console.log("eval entry");
console.log("a", require_a().value, typeof require("path").join);
})
