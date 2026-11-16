
module.exports = (() => {
"use strict";
var __webpack_modules__ = {
1: (module, exports, __webpack_require__) => { console.log("m1"); exports.v = "one"; },
2: (module) => { module.exports = "two"; }
};
var __webpack_module_cache__ = {};
function __webpack_require__(moduleId) { var cachedModule = __webpack_module_cache__[moduleId]; if (cachedModule !== undefined) { return cachedModule.exports; } var module = __webpack_module_cache__[moduleId] = { exports: {} }; __webpack_modules__[moduleId](module, module.exports, __webpack_require__); return module.exports; }
__webpack_require__.o = (obj, prop) => Object.prototype.hasOwnProperty.call(obj, prop);

var other = {}; var __webpack_exports__ = {};

(() => { other.k = 1; __webpack_exports__.j = __webpack_require__(1).v; console.log("entry", other); })();
return __webpack_exports__;
})();
console.log("end", module.exports);
