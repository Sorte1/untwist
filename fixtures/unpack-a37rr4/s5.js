"use strict";
console.log("before");
void (() => { "use strict"; var __webpack_modules__ = {1: (module, exports, __webpack_require__) => { console.log("m1"); module.exports = __webpack_require__(2); }, 2: (module) => { module.exports = "two"; }}; var __webpack_module_cache__ = {}; function __webpack_require__(moduleId) { var cachedModule = __webpack_module_cache__[moduleId]; if (cachedModule !== undefined) { return cachedModule.exports; } var module = __webpack_module_cache__[moduleId] = { exports: {} }; __webpack_modules__[moduleId](module, module.exports, __webpack_require__); return module.exports; } var __webpack_exports__ = __webpack_require__(1); })();
console.log("after");
