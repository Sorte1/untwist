(window["webpackJsonp"] = window["webpackJsonp"] || []).push([[0],[
/* 0 */
/***/ (function(module, __webpack_exports__, __webpack_require__) {

"use strict";
/* harmony export (binding) */ __webpack_require__.d(__webpack_exports__, "a", function() { return shared; });
console.log("eval shared");
const shared = (from) => "shared<" + from + ">";


/***/ }),
/* 1 */
/***/ (function(module, exports) {

console.log("eval tinylib");
exports.pad = function (s) {
  return "[" + s + "]";
};


/***/ }),
/* 2 */
/***/ (function(module, exports) {

console.log("eval cjs");
exports.twice = function (n) {
  return n * 2;
};


/***/ }),
/* 3 */
/***/ (function(module, __webpack_exports__, __webpack_require__) {

"use strict";
__webpack_require__.r(__webpack_exports__);
/* harmony import */ var _cjs_js__WEBPACK_IMPORTED_MODULE_0__ = __webpack_require__(2);
/* harmony import */ var _cjs_js__WEBPACK_IMPORTED_MODULE_0___default = /*#__PURE__*/__webpack_require__.n(_cjs_js__WEBPACK_IMPORTED_MODULE_0__);
/* harmony import */ var _shared_js__WEBPACK_IMPORTED_MODULE_1__ = __webpack_require__(0);
/* harmony import */ var tinylib__WEBPACK_IMPORTED_MODULE_2__ = __webpack_require__(1);
/* harmony import */ var tinylib__WEBPACK_IMPORTED_MODULE_2___default = /*#__PURE__*/__webpack_require__.n(tinylib__WEBPACK_IMPORTED_MODULE_2__);




console.log("eval index " + Object(_cjs_js__WEBPACK_IMPORTED_MODULE_0__["twice"])(21) + " " + Object(_shared_js__WEBPACK_IMPORTED_MODULE_1__[/* shared */ "a"])("index") + " " + Object(tinylib__WEBPACK_IMPORTED_MODULE_2__["pad"])("x"));
async function main() {
  const lazy = await __webpack_require__.e(/* import() */ 3).then(__webpack_require__.bind(null, 4));
  console.log("lazy " + lazy.value + " " + lazy.nested());
  const legacy = await __webpack_require__.e(/* import() */ 4).then(__webpack_require__.t.bind(null, 5, 7));
  console.log("legacy " + legacy.default.kind + " " + typeof legacy.default.run);
  const deep = await lazy.deeper();
  console.log("deeper " + deep.depth);
}
main().then(() => console.log("done"));


/***/ })
],[[3,1]]]);