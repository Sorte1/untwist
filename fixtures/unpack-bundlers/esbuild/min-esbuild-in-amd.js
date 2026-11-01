define([], function () {
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
  var require_b = __commonJS({
    "src/b.js"(exports, module) {
      console.log("eval b", require_a().value);
      module.exports = { value: 2 };
    }
  });
  console.log("eval entry");
  console.log("b", require_b().value, require_a() === require_a());
});
