var __getOwnPropNames = Object.getOwnPropertyNames;
var __commonJS = (cb, mod) => function __require() {
  return mod || (0, cb[__getOwnPropNames(cb)[0]])((mod = { exports: {} }).exports, mod), mod.exports;
};
var require_a = __commonJS({ "src/a.js"(exports, module) { console.log("eval a"); module.exports = { value: 1 }; } });
import.meta.shared = 1;
console.log("start", typeof require, typeof module);
if (globalThis.nope) require_a();
console.log("end", import.meta.shared);
export {};
