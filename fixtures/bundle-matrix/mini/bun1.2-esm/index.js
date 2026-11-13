var __defProp = Object.defineProperty;
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

// lazy.js
var exports_lazy = {};
__export(exports_lazy, {
  value: () => value
});
var value = 42;
var init_lazy = __esm(() => {
  console.log("eval lazy");
});

// cjs.js
console.log("eval cjs");
var $twice = function(n) {
  return n * 2;
};

// index.js
console.log("eval index " + $twice(21));
Promise.resolve().then(() => (init_lazy(), exports_lazy)).then((lazy) => console.log("lazy " + lazy.value));
