var __defProp = Object.defineProperty;
var __getOwnPropNames = Object.getOwnPropertyNames;
var __esm = (fn, res) => function __init() {
  return fn && (res = (0, fn[__getOwnPropNames(fn)[0]])(fn = 0)), res;
};
var __export = (target, all) => {
  for (var name in all)
    __defProp(target, name, { get: all[name], enumerable: true });
};
var lib_exports = {};
__export(lib_exports, {
  value: () => value
});
var value;
var init_lib = __esm({
  "src/lib.js"() {
    console.log("eval lib");
    value = 7;
  }
});
console.log("eval entry");
init_lib();
console.log("lib", lib_exports.value);
