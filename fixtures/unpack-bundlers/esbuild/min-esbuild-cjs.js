var __getOwnPropNames = Object.getOwnPropertyNames;
var __commonJS = (cb, mod) => function __require() {
  return mod || (0, cb[__getOwnPropNames(cb)[0]])((mod = { exports: {} }).exports, mod), mod.exports;
};
var require_a = __commonJS({
  "src/a.js"(exports, module) {
    console.log("eval a");
    module.exports = { value: 1 };
  }
});
console.log("eval entry");
console.log("a", require_a().value, require_a() === require_a());
