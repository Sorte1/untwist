var __getOwnPropNames = Object.getOwnPropertyNames;
var __commonJS = (cb, mod) => function __require() {
  "use strict";
  return mod || (0, cb[__getOwnPropNames(cb)[0]])((mod = { exports: {} }).exports, mod), mod.exports;
};
var require_a = __commonJS({
  "src/a.js"(exports, module) {
    "use strict";
    module.exports = { value: 1 };
  }
});
function k() { "use strict"; }
y = 5;
console.log("a", require_a().value, y);
