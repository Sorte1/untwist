(self["webpackChunk"] = self["webpackChunk"] || []).push([[57],{

/***/ 275
(__unused_webpack_module, exports) {

console.log("eval cjs");
exports.P = function (n) {
  return n * 2;
};


/***/ },

/***/ 237
(__unused_webpack_module, __unused_webpack___webpack_exports__, __webpack_require__) {

"use strict";
/* harmony import */ var _cjs_js__WEBPACK_IMPORTED_MODULE_0__ = __webpack_require__(275);

console.log("eval index " + (0,_cjs_js__WEBPACK_IMPORTED_MODULE_0__/* .twice */ .P)(21));
__webpack_require__.e(/* import() */ 339).then(() => (__webpack_require__(339))).then(lazy => console.log("lazy " + lazy.value));


/***/ }

},
/******/ __webpack_require__ => { // webpackRuntimeModules
/******/ var __webpack_exec__ = (moduleId) => (__webpack_require__(moduleId))
/******/ var __webpack_exports__ = (__webpack_exec__(237));
/******/ }
]);