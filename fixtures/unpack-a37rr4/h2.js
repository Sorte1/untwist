console.log("before");
(() => { "use strict"; var __webpack_modules__ = ({ 1: (__unused_webpack_module, __webpack_exports__, __webpack_require__) => { __webpack_require__.r(__webpack_exports__); __webpack_require__.d(__webpack_exports__, { w: () => w }); var _b = __webpack_require__(2); console.log("m1", _b.v); const w = 1; }, 2: (__unused_webpack_module, __webpack_exports__, __webpack_require__) => { __webpack_require__.r(__webpack_exports__); __webpack_require__.d(__webpack_exports__, { v: () => v }); console.log("m2"); const v = 5; } });
var __webpack_module_cache__ = {};
function __webpack_require__(moduleId) {  var cachedModule = __webpack_module_cache__[moduleId]; if (cachedModule !== undefined) { return cachedModule.exports; } var module = __webpack_module_cache__[moduleId] = { exports: {} }; __webpack_modules__[moduleId](module, module.exports, __webpack_require__); return module.exports; }
(() => { __webpack_require__.d = (exports, definition) => { for (var key in definition) { if (__webpack_require__.o(definition, key) && !__webpack_require__.o(exports, key)) { Object.defineProperty(exports, key, { enumerable: true, get: definition[key] }); } } }; })();
(() => { __webpack_require__.o = (obj, prop) => (Object.prototype.hasOwnProperty.call(obj, prop)); })();
(() => { __webpack_require__.r = (exports) => { if (typeof Symbol !== 'undefined' && Symbol.toStringTag) { Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' }); } Object.defineProperty(exports, '__esModule', { value: true }); }; })();
 var __webpack_exports__ = __webpack_require__(1); })();
console.log("after");
