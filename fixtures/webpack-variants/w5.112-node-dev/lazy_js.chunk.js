"use strict";
exports.ids = ["lazy_js"];
exports.modules = {

/***/ "./lazy.js"
/*!*****************!*\
  !*** ./lazy.js ***!
  \*****************/
(__unused_webpack_module, __webpack_exports__, __webpack_require__) {

__webpack_require__.r(__webpack_exports__);
/* harmony export */ __webpack_require__.d(__webpack_exports__, {
/* harmony export */   deeper: () => (/* binding */ deeper),
/* harmony export */   nested: () => (/* binding */ nested),
/* harmony export */   value: () => (/* binding */ value)
/* harmony export */ });
/* harmony import */ var _shared_js__WEBPACK_IMPORTED_MODULE_0__ = __webpack_require__(/*! ./shared.js */ "./shared.js");
/* harmony import */ var tinylib__WEBPACK_IMPORTED_MODULE_1__ = __webpack_require__(/*! tinylib */ "./node_modules/tinylib/index.js");



console.log("eval lazy " + (0,_shared_js__WEBPACK_IMPORTED_MODULE_0__.shared)("lazy") + " " + (0,tinylib__WEBPACK_IMPORTED_MODULE_1__.pad)("y"));
const value = 42;
function nested() {
  return "nested";
}
const deeper = () => __webpack_require__.e(/*! import() */ "deep_js").then(() => (__webpack_require__(/*! ./deep.js */ "./deep.js")));


/***/ }

};
;