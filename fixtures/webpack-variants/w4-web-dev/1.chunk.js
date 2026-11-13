(window["webpackJsonp"] = window["webpackJsonp"] || []).push([[1],{

/***/ "./lazy.js":
/*!*****************!*\
  !*** ./lazy.js ***!
  \*****************/
/*! exports provided: value, nested, deeper */
/***/ (function(module, __webpack_exports__, __webpack_require__) {

"use strict";
__webpack_require__.r(__webpack_exports__);
/* harmony export (binding) */ __webpack_require__.d(__webpack_exports__, "value", function() { return value; });
/* harmony export (binding) */ __webpack_require__.d(__webpack_exports__, "nested", function() { return nested; });
/* harmony export (binding) */ __webpack_require__.d(__webpack_exports__, "deeper", function() { return deeper; });
/* harmony import */ var _shared_js__WEBPACK_IMPORTED_MODULE_0__ = __webpack_require__(/*! ./shared.js */ "./shared.js");
/* harmony import */ var tinylib__WEBPACK_IMPORTED_MODULE_1__ = __webpack_require__(/*! tinylib */ "./node_modules/tinylib/index.js");
/* harmony import */ var tinylib__WEBPACK_IMPORTED_MODULE_1___default = /*#__PURE__*/__webpack_require__.n(tinylib__WEBPACK_IMPORTED_MODULE_1__);



console.log("eval lazy " + Object(_shared_js__WEBPACK_IMPORTED_MODULE_0__["shared"])("lazy") + " " + Object(tinylib__WEBPACK_IMPORTED_MODULE_1__["pad"])("y"));
const value = 42;
function nested() {
  return "nested";
}
const deeper = () => __webpack_require__.e(/*! import() */ 0).then(__webpack_require__.bind(null, /*! ./deep.js */ "./deep.js"));


/***/ })

}]);