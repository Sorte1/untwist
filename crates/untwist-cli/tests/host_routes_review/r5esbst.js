import foo from "foo";
var __getOwnPropNames = Object.getOwnPropertyNames;
var __commonJS = (cb, mod) => function __require() {
  return mod || (0, cb[__getOwnPropNames(cb)[0]])((mod = { exports: {} }).exports, mod), mod.exports;
};
var require_a = __commonJS({
  "src/a.js"(exports, module) {
    module.exports = "a";
  }
});
var require_foo = __commonJS({ "node_modules/foo/index.js"(exports, module) { module.exports = "bundled foo"; } });
console.log("entry", require_a(), require_foo(), foo);
export {};
