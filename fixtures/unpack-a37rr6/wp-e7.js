
(() => {
"use strict";
var __webpack_modules__ = {
1: (module, exports, __webpack_require__) => { for (var i = 0, a = (function () { return this === undefined; })(); i < 1; i++) console.log("m1", a); exports.v = "one"; },
2: (module) => { module.exports = "two"; }
};
var __webpack_module_cache__ = {};
function __webpack_require__(moduleId) { var cachedModule = __webpack_module_cache__[moduleId]; if (cachedModule !== undefined) { return cachedModule.exports; } var module = __webpack_module_cache__[moduleId] = { exports: {} }; __webpack_modules__[moduleId](module, module.exports, __webpack_require__); return module.exports; }
__webpack_require__.o = (obj, prop) => Object.prototype.hasOwnProperty.call(obj, prop);

var __webpack_exports__ = {};

(() => { console.log("entry", __webpack_require__(1).v, __webpack_require__(2)); })();

})();

