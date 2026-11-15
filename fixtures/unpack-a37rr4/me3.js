var __getOwnPropNames = Object.getOwnPropertyNames;
var __commonJS = (cb, mod) => function __require() {
  return mod || (0, cb[__getOwnPropNames(cb)[0]])((mod = { exports: {} }).exports, mod), mod.exports;
};
var require_a = __commonJS({ "src/a.js"(exports, module) { console.log("eval a", (function(){ return typeof this })()); module.exports = { value: 1 }; } });
var require_b = __commonJS({ "src/b.js"(exports, module) { console.log("eval b"); module.exports = { value: 2 }; } });
console.log("start");
await null;
console.log("a", require_a().value);
await null;
console.log("b", require_b().value, typeof import.meta.url, (function(){ return typeof this })());
export {};
