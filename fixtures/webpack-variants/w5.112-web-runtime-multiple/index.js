(self["webpackChunkwpvariants"] = self["webpackChunkwpvariants"] || []).push([[57],{

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
/* harmony import */ var _shared_js__WEBPACK_IMPORTED_MODULE_1__ = __webpack_require__(48);
/* harmony import */ var tinylib__WEBPACK_IMPORTED_MODULE_2__ = __webpack_require__(814);




console.log("eval index " + (0,_cjs_js__WEBPACK_IMPORTED_MODULE_0__/* .twice */ .P)(21) + " " + (0,_shared_js__WEBPACK_IMPORTED_MODULE_1__/* .shared */ .u)("index") + " " + (0,tinylib__WEBPACK_IMPORTED_MODULE_2__/* .pad */ .e)("x"));
async function main() {
  const lazy = await __webpack_require__.e(/* import() */ 339).then(() => (__webpack_require__(339)));
  console.log("lazy " + lazy.value + " " + lazy.nested());
  const legacy = await __webpack_require__.e(/* import() */ 542).then(() => (__webpack_require__.t(542, 19)));
  console.log("legacy " + legacy.default.kind + " " + typeof legacy.default.run);
  const deep = await lazy.deeper();
  console.log("deeper " + deep.depth);
}
main().then(() => console.log("done"));


/***/ },

/***/ 814
(__unused_webpack_module, exports) {

console.log("eval tinylib");
exports.e = function (s) {
  return "[" + s + "]";
};


/***/ },

/***/ 48
(__unused_webpack_module, __webpack_exports__, __webpack_require__) {

"use strict";
console.log("eval shared");
const shared = (from) => "shared<" + from + ">";

/* harmony export */ __webpack_require__.d(__webpack_exports__, [
/* harmony export */   "u", 0, /* binding */ shared
/* harmony export */ ]);


/***/ }

},
/******/ __webpack_require__ => { // webpackRuntimeModules
/******/ var __webpack_exec__ = (moduleId) => (__webpack_require__(moduleId))
/******/ var __webpack_exports__ = (__webpack_exec__(237));
/******/ }
]);