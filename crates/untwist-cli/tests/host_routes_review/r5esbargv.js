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
try { console.log("got", require("fs").existsSync(require("path").join(require("path").dirname(process.argv[1]), "src/a.js"))); } catch (e) { console.log("threw", e.code || e.message); }
