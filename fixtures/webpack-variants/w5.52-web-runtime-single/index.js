(self["webpackChunkwpvariants"] = self["webpackChunkwpvariants"] || []).push([[826],{

/***/ 489:
/***/ ((__unused_webpack_module, exports) => {

console.log("eval cjs");
exports.g = function (n) {
  return n * 2;
};


/***/ }),

/***/ 10:
/***/ ((__unused_webpack_module, __unused_webpack___webpack_exports__, __webpack_require__) => {

"use strict";
/* harmony import */ var _cjs_js__WEBPACK_IMPORTED_MODULE_0__ = __webpack_require__(489);
/* harmony import */ var _shared_js__WEBPACK_IMPORTED_MODULE_1__ = __webpack_require__(52);
/* harmony import */ var tinylib__WEBPACK_IMPORTED_MODULE_2__ = __webpack_require__(926);




console.log("eval index " + (0,_cjs_js__WEBPACK_IMPORTED_MODULE_0__/* .twice */ .g)(21) + " " + (0,_shared_js__WEBPACK_IMPORTED_MODULE_1__/* .shared */ .c)("index") + " " + (0,tinylib__WEBPACK_IMPORTED_MODULE_2__/* .pad */ .v)("x"));
async function main() {
  const lazy = await __webpack_require__.e(/* import() */ 401).then(__webpack_require__.bind(__webpack_require__, 401));
  console.log("lazy " + lazy.value + " " + lazy.nested());
  const legacy = await __webpack_require__.e(/* import() */ 103).then(__webpack_require__.t.bind(__webpack_require__, 103, 23));
  console.log("legacy " + legacy.default.kind + " " + typeof legacy.default.run);
  const deep = await lazy.deeper();
  console.log("deeper " + deep.depth);
}
main().then(() => console.log("done"));


/***/ }),

/***/ 926:
/***/ ((__unused_webpack_module, exports) => {

console.log("eval tinylib");
exports.v = function (s) {
  return "[" + s + "]";
};


/***/ }),

/***/ 52:
/***/ ((__unused_webpack_module, __webpack_exports__, __webpack_require__) => {

"use strict";
/* harmony export */ __webpack_require__.d(__webpack_exports__, {
/* harmony export */   "c": () => (/* binding */ shared)
/* harmony export */ });
console.log("eval shared");
const shared = (from) => "shared<" + from + ">";


/***/ })

},
/******/ __webpack_require__ => { // webpackRuntimeModules
/******/ var __webpack_exec__ = (moduleId) => (__webpack_require__(__webpack_require__.s = moduleId))
/******/ var __webpack_exports__ = (__webpack_exec__(10));
/******/ }
]);