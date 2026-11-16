"use strict";var __getOwnPropNames = Object.getOwnPropertyNames;
var __commonJS = (cb, mod) => function __require() {
  return mod || (0, cb[__getOwnPropNames(cb)[0]])((mod = { exports: {} }).exports, mod), mod.exports;
};
var require_a = __commonJS({
  "src/a.js"(p, q) {
"use strict";function f(){var exports=1;try{throw 2}catch(exports){p.k=exports;return p.k+exports}}console.log(f())
  }
});
require_a();
