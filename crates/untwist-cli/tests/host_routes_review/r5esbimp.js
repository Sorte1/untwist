import M from "module";
var __getOwnPropNames = Object.getOwnPropertyNames;
var __commonJS = (cb, mod) => function __require() {
  return mod || (0, cb[__getOwnPropNames(cb)[0]])((mod = { exports: {} }).exports, mod), mod.exports;
};
var require_a = __commonJS({
  "src/a.js"(exports, module) {
    module.exports = "a";
  }
});
console.log("entry", require_a());
try { console.log("got", M.createRequire(import.meta.url)("./src/a.js")); } catch (e) { console.log("threw", e.code || e.message); }
export {};
