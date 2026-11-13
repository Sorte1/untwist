var __create = Object.create;
var __getProtoOf = Object.getPrototypeOf;
var __defProp = Object.defineProperty;
var __getOwnPropNames = Object.getOwnPropertyNames;
var __hasOwnProp = Object.prototype.hasOwnProperty;
var __toESM = (mod, isNodeMode, target) => {
  target = mod != null ? __create(__getProtoOf(mod)) : {};
  const to = isNodeMode || !mod || !mod.__esModule ? __defProp(target, "default", { value: mod, enumerable: true }) : target;
  for (let key of __getOwnPropNames(mod))
    if (!__hasOwnProp.call(to, key))
      __defProp(to, key, {
        get: () => mod[key],
        enumerable: true
      });
  return to;
};
var __commonJS = (cb, mod) => () => (mod || cb((mod = { exports: {} }).exports, mod), mod.exports);
var __export = (target, all) => {
  for (var name in all)
    __defProp(target, name, {
      get: all[name],
      enumerable: true,
      configurable: true,
      set: (newValue) => all[name] = () => newValue
    });
};
var __esm = (fn, res) => () => (fn && (res = fn(fn = 0)), res);

// cjs.js
var require_cjs = __commonJS((exports2) => {
  console.log("eval cjs");
  exports2.twice = function(n) {
    return n * 2;
  };
});

// lazy.js
var exports_lazy = {};
__export(exports_lazy, {
  value: () => value
});
var value = 42;
var init_lazy = __esm(() => {
  console.log("eval lazy");
});

// index.js
var import_cjs = __toESM(require_cjs());
console.log("eval index " + import_cjs.twice(21));
Promise.resolve().then(() => (init_lazy(), exports_lazy)).then((lazy) => console.log("lazy " + lazy.value));
