/*
 * ATTENTION: An "eval-source-map" devtool has been used.
 * This devtool is neither made for production nor for readable output files.
 * It uses "eval()" calls to create a separate source file with attached SourceMaps in the browser devtools.
 * If you are trying to read the output file, select a different devtool (https://webpack.js.org/configuration/devtool/)
 * or disable the default devtool with "devtool: false".
 * If you are looking for production-ready output files, see mode: "production" (https://webpack.js.org/configuration/mode/).
 */
/******/ (() => { // webpackBootstrap
/******/ 	"use strict";
/******/ 	var __webpack_modules__ = ({

/***/ "./data.json":
/*!*******************!*\
  !*** ./data.json ***!
  \*******************/
/***/ ((module) => {

module.exports = /*#__PURE__*/JSON.parse('{"name":"fixture","items":["a","b","c"]}');

/***/ }),

/***/ "./index.js":
/*!******************!*\
  !*** ./index.js ***!
  \******************/
/***/ ((__unused_webpack_module, __webpack_exports__, __webpack_require__) => {

eval("__webpack_require__.r(__webpack_exports__);\n/* harmony import */ var _util_js__WEBPACK_IMPORTED_MODULE_0__ = __webpack_require__(/*! ./util.js */ \"./util.js\");\n/* harmony import */ var _data_json__WEBPACK_IMPORTED_MODULE_1__ = __webpack_require__(/*! ./data.json */ \"./data.json\");\n\n\n\nconsole.log((0,_util_js__WEBPACK_IMPORTED_MODULE_0__.greet)(\"webpack\"), (0,_util_js__WEBPACK_IMPORTED_MODULE_0__.total)([1, 2, 3, 4]), _data_json__WEBPACK_IMPORTED_MODULE_1__.name, _data_json__WEBPACK_IMPORTED_MODULE_1__.items.length);\n//# sourceURL=[module]\n//# sourceMappingURL=data:application/json;charset=utf-8;base64,eyJ2ZXJzaW9uIjozLCJmaWxlIjoiLi9pbmRleC5qcyIsIm1hcHBpbmdzIjoiOzs7QUFBeUM7QUFDVjs7QUFFL0IsWUFBWSwrQ0FBSyxhQUFhLCtDQUFLLGdCQUFnQiw0Q0FBUyxFQUFFLDZDQUFVIiwic291cmNlcyI6WyJ3ZWJwYWNrOi8vLy4vaW5kZXguanM/NDFmNSJdLCJzb3VyY2VzQ29udGVudCI6WyJpbXBvcnQgeyBncmVldCwgdG90YWwgfSBmcm9tIFwiLi91dGlsLmpzXCI7XG5pbXBvcnQgZGF0YSBmcm9tIFwiLi9kYXRhLmpzb25cIjtcblxuY29uc29sZS5sb2coZ3JlZXQoXCJ3ZWJwYWNrXCIpLCB0b3RhbChbMSwgMiwgMywgNF0pLCBkYXRhLm5hbWUsIGRhdGEuaXRlbXMubGVuZ3RoKTtcbiJdLCJuYW1lcyI6W10sInNvdXJjZVJvb3QiOiIifQ==\n//# sourceURL=webpack-internal:///./index.js\n");

/***/ }),

/***/ "./util.js":
/*!*****************!*\
  !*** ./util.js ***!
  \*****************/
/***/ ((__unused_webpack_module, __webpack_exports__, __webpack_require__) => {

eval("__webpack_require__.r(__webpack_exports__);\n/* harmony export */ __webpack_require__.d(__webpack_exports__, {\n/* harmony export */   greet: () => (/* binding */ greet),\n/* harmony export */   total: () => (/* binding */ total)\n/* harmony export */ });\nfunction greet(name) {\n  return \"hello \" + name;\n}\n\nconst total = (list) => list.reduce((sum, value) => sum + value, 0);\n//# sourceURL=[module]\n//# sourceMappingURL=data:application/json;charset=utf-8;base64,eyJ2ZXJzaW9uIjozLCJmaWxlIjoiLi91dGlsLmpzIiwibWFwcGluZ3MiOiI7Ozs7O0FBQU87QUFDUDtBQUNBOztBQUVPIiwic291cmNlcyI6WyJ3ZWJwYWNrOi8vLy4vdXRpbC5qcz85NDE1Il0sInNvdXJjZXNDb250ZW50IjpbImV4cG9ydCBmdW5jdGlvbiBncmVldChuYW1lKSB7XG4gIHJldHVybiBcImhlbGxvIFwiICsgbmFtZTtcbn1cblxuZXhwb3J0IGNvbnN0IHRvdGFsID0gKGxpc3QpID0+IGxpc3QucmVkdWNlKChzdW0sIHZhbHVlKSA9PiBzdW0gKyB2YWx1ZSwgMCk7XG4iXSwibmFtZXMiOltdLCJzb3VyY2VSb290IjoiIn0=\n//# sourceURL=webpack-internal:///./util.js\n");

/***/ })

/******/ 	});
/************************************************************************/
/******/ 	// The module cache
/******/ 	var __webpack_module_cache__ = {};
/******/ 	
/******/ 	// The require function
/******/ 	function __webpack_require__(moduleId) {
/******/ 		// Check if module is in cache
/******/ 		var cachedModule = __webpack_module_cache__[moduleId];
/******/ 		if (cachedModule !== undefined) {
/******/ 			return cachedModule.exports;
/******/ 		}
/******/ 		// Create a new module (and put it into the cache)
/******/ 		var module = __webpack_module_cache__[moduleId] = {
/******/ 			// no module.id needed
/******/ 			// no module.loaded needed
/******/ 			exports: {}
/******/ 		};
/******/ 	
/******/ 		// Execute the module function
/******/ 		__webpack_modules__[moduleId](module, module.exports, __webpack_require__);
/******/ 	
/******/ 		// Return the exports of the module
/******/ 		return module.exports;
/******/ 	}
/******/ 	
/************************************************************************/
/******/ 	/* webpack/runtime/define property getters */
/******/ 	(() => {
/******/ 		// define getter functions for harmony exports
/******/ 		__webpack_require__.d = (exports, definition) => {
/******/ 			for(var key in definition) {
/******/ 				if(__webpack_require__.o(definition, key) && !__webpack_require__.o(exports, key)) {
/******/ 					Object.defineProperty(exports, key, { enumerable: true, get: definition[key] });
/******/ 				}
/******/ 			}
/******/ 		};
/******/ 	})();
/******/ 	
/******/ 	/* webpack/runtime/hasOwnProperty shorthand */
/******/ 	(() => {
/******/ 		__webpack_require__.o = (obj, prop) => (Object.prototype.hasOwnProperty.call(obj, prop))
/******/ 	})();
/******/ 	
/******/ 	/* webpack/runtime/make namespace object */
/******/ 	(() => {
/******/ 		// define __esModule on exports
/******/ 		__webpack_require__.r = (exports) => {
/******/ 			if(typeof Symbol !== 'undefined' && Symbol.toStringTag) {
/******/ 				Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });
/******/ 			}
/******/ 			Object.defineProperty(exports, '__esModule', { value: true });
/******/ 		};
/******/ 	})();
/******/ 	
/************************************************************************/
/******/ 	
/******/ 	// startup
/******/ 	// Load entry module and return exports
/******/ 	// This entry module can't be inlined because the eval-source-map devtool is used.
/******/ 	var __webpack_exports__ = __webpack_require__("./index.js");
/******/ 	
/******/ })()
;