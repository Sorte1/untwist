/*
 * ATTENTION: An "eval-source-map" devtool has been used.
 * This devtool is neither made for production nor for readable output files.
 * It uses "eval()" calls to create a separate source file with attached SourceMaps in the browser devtools.
 * If you are trying to read the output file, select a different devtool (https://webpack.js.org/configuration/devtool/)
 * or disable the default devtool with "devtool: false".
 * If you are looking for production-ready output files, see mode: "production" (https://webpack.js.org/configuration/mode/).
 */
/******/ (() => { // webpackBootstrap
/******/ 	var __webpack_modules__ = ({

/***/ "./data.json":
/*!*******************!*\
  !*** ./data.json ***!
  \*******************/
/***/ ((module) => {

"use strict";
module.exports = JSON.parse('{"name":"fixture","items":["a","b","c"]}');

/***/ }),

/***/ "./index.js":
/*!******************!*\
  !*** ./index.js ***!
  \******************/
/***/ ((__unused_webpack_module, __webpack_exports__, __webpack_require__) => {

"use strict";
eval("__webpack_require__.r(__webpack_exports__);\n/* harmony import */ var _util_js__WEBPACK_IMPORTED_MODULE_0__ = __webpack_require__(/*! ./util.js */ \"./util.js\");\n/* harmony import */ var _legacy_js__WEBPACK_IMPORTED_MODULE_1__ = __webpack_require__(/*! ./legacy.js */ \"./legacy.js\");\n/* harmony import */ var _legacy_js__WEBPACK_IMPORTED_MODULE_1___default = /*#__PURE__*/__webpack_require__.n(_legacy_js__WEBPACK_IMPORTED_MODULE_1__);\n/* harmony import */ var _strict_cjs_js__WEBPACK_IMPORTED_MODULE_2__ = __webpack_require__(/*! ./strict-cjs.js */ \"./strict-cjs.js\");\n/* harmony import */ var _data_json__WEBPACK_IMPORTED_MODULE_3__ = __webpack_require__(/*! ./data.json */ \"./data.json\");\n/* harmony import */ var _shapes_js__WEBPACK_IMPORTED_MODULE_4__ = __webpack_require__(/*! ./shapes.js */ \"./shapes.js\");\n\n\n\n\n\n\nconsole.log((0,_util_js__WEBPACK_IMPORTED_MODULE_0__.greet)(\"webpack\"), (0,_util_js__WEBPACK_IMPORTED_MODULE_0__.total)([1, 2, 3, 4]));\nconst counter = new _util_js__WEBPACK_IMPORTED_MODULE_0__.Counter(2);\ncounter.add(3);\nconsole.log(\"counter\", counter.value, _util_js__WEBPACK_IMPORTED_MODULE_0__.Counter.name);\nconsole.log(\"adders\", (0,_util_js__WEBPACK_IMPORTED_MODULE_0__.makeAdders)(3).map((add) => add(10)).join(\",\"));\nconsole.log(\"legacy\", _legacy_js__WEBPACK_IMPORTED_MODULE_1___default().bump(), _legacy_js__WEBPACK_IMPORTED_MODULE_1___default().bump(), (_legacy_js__WEBPACK_IMPORTED_MODULE_1___default().selfIsExports), _legacy_js__WEBPACK_IMPORTED_MODULE_1___default().blockFn());\nconsole.log(\"strict cjs\", _strict_cjs_js__WEBPACK_IMPORTED_MODULE_2__.mode(), _strict_cjs_js__WEBPACK_IMPORTED_MODULE_2__.label);\nconsole.log(\"data\", _data_json__WEBPACK_IMPORTED_MODULE_3__.name, _data_json__WEBPACK_IMPORTED_MODULE_3__.items.length);\nconsole.log(\"shapes\", Object.keys(_shapes_js__WEBPACK_IMPORTED_MODULE_4__).sort().join(\",\"), _shapes_js__WEBPACK_IMPORTED_MODULE_4__.area(_shapes_js__WEBPACK_IMPORTED_MODULE_4__.unit));\n//# sourceURL=[module]\n//# sourceMappingURL=data:application/json;charset=utf-8;base64,eyJ2ZXJzaW9uIjozLCJzb3VyY2VzIjpbIndlYnBhY2s6Ly8vLi9pbmRleC5qcz80MWY1Il0sIm5hbWVzIjpbXSwibWFwcGluZ3MiOiI7Ozs7Ozs7QUFBOEQ7QUFDN0I7QUFDTztBQUNUO0FBQ087O0FBRXRDLFlBQVksK0NBQUssYUFBYSwrQ0FBSztBQUNuQyxvQkFBb0IsNkNBQU87QUFDM0I7QUFDQSxzQ0FBc0Msa0RBQVk7QUFDbEQsc0JBQXNCLG9EQUFVO0FBQ2hDLHNCQUFzQixzREFBVyxJQUFJLHNEQUFXLElBQUksaUVBQW9CLEVBQUUseURBQWM7QUFDeEYsMEJBQTBCLGdEQUFjLElBQUksaURBQWU7QUFDM0Qsb0JBQW9CLDRDQUFTLEVBQUUsb0RBQWlCO0FBQ2hELGtDQUFrQyx1Q0FBTSxvQkFBb0IsNENBQVcsQ0FBQyw0Q0FBVyIsImZpbGUiOiIuL2luZGV4LmpzLmpzIiwic291cmNlc0NvbnRlbnQiOlsiaW1wb3J0IHsgZ3JlZXQsIHRvdGFsLCBDb3VudGVyLCBtYWtlQWRkZXJzIH0gZnJvbSBcIi4vdXRpbC5qc1wiO1xuaW1wb3J0IGxlZ2FjeSBmcm9tIFwiLi9sZWdhY3kuanNcIjtcbmltcG9ydCBzdHJpY3RDanMgZnJvbSBcIi4vc3RyaWN0LWNqcy5qc1wiO1xuaW1wb3J0IGRhdGEgZnJvbSBcIi4vZGF0YS5qc29uXCI7XG5pbXBvcnQgKiBhcyBzaGFwZXMgZnJvbSBcIi4vc2hhcGVzLmpzXCI7XG5cbmNvbnNvbGUubG9nKGdyZWV0KFwid2VicGFja1wiKSwgdG90YWwoWzEsIDIsIDMsIDRdKSk7XG5jb25zdCBjb3VudGVyID0gbmV3IENvdW50ZXIoMik7XG5jb3VudGVyLmFkZCgzKTtcbmNvbnNvbGUubG9nKFwiY291bnRlclwiLCBjb3VudGVyLnZhbHVlLCBDb3VudGVyLm5hbWUpO1xuY29uc29sZS5sb2coXCJhZGRlcnNcIiwgbWFrZUFkZGVycygzKS5tYXAoKGFkZCkgPT4gYWRkKDEwKSkuam9pbihcIixcIikpO1xuY29uc29sZS5sb2coXCJsZWdhY3lcIiwgbGVnYWN5LmJ1bXAoKSwgbGVnYWN5LmJ1bXAoKSwgbGVnYWN5LnNlbGZJc0V4cG9ydHMsIGxlZ2FjeS5ibG9ja0ZuKCkpO1xuY29uc29sZS5sb2coXCJzdHJpY3QgY2pzXCIsIHN0cmljdENqcy5tb2RlKCksIHN0cmljdENqcy5sYWJlbCk7XG5jb25zb2xlLmxvZyhcImRhdGFcIiwgZGF0YS5uYW1lLCBkYXRhLml0ZW1zLmxlbmd0aCk7XG5jb25zb2xlLmxvZyhcInNoYXBlc1wiLCBPYmplY3Qua2V5cyhzaGFwZXMpLnNvcnQoKS5qb2luKFwiLFwiKSwgc2hhcGVzLmFyZWEoc2hhcGVzLnVuaXQpKTtcbiJdLCJzb3VyY2VSb290IjoiIn0=\n//# sourceURL=webpack-internal:///./index.js\n");

/***/ }),

/***/ "./legacy.js":
/*!*******************!*\
  !*** ./legacy.js ***!
  \*******************/
/***/ (function(module) {

eval("var count = 0;\nfunction bump() {\n  count += 1;\n  return count;\n}\nvar selfIsExports = this === module.exports;\nif (count === 0) {\n  function blockFn() {\n    return typeof bump;\n  }\n}\nmodule.exports = { bump: bump, selfIsExports: selfIsExports, blockFn: blockFn };\n//# sourceURL=[module]\n//# sourceMappingURL=data:application/json;charset=utf-8;base64,eyJ2ZXJzaW9uIjozLCJzb3VyY2VzIjpbIndlYnBhY2s6Ly8vLi9sZWdhY3kuanM/YjM4ZiJdLCJuYW1lcyI6W10sIm1hcHBpbmdzIjoiQUFBQTtBQUNBO0FBQ0E7QUFDQTtBQUNBO0FBQ0E7QUFDQTtBQUNBO0FBQ0E7QUFDQTtBQUNBO0FBQ0Esa0JBQWtCIiwiZmlsZSI6Ii4vbGVnYWN5LmpzLmpzIiwic291cmNlc0NvbnRlbnQiOlsidmFyIGNvdW50ID0gMDtcbmZ1bmN0aW9uIGJ1bXAoKSB7XG4gIGNvdW50ICs9IDE7XG4gIHJldHVybiBjb3VudDtcbn1cbnZhciBzZWxmSXNFeHBvcnRzID0gdGhpcyA9PT0gbW9kdWxlLmV4cG9ydHM7XG5pZiAoY291bnQgPT09IDApIHtcbiAgZnVuY3Rpb24gYmxvY2tGbigpIHtcbiAgICByZXR1cm4gdHlwZW9mIGJ1bXA7XG4gIH1cbn1cbm1vZHVsZS5leHBvcnRzID0geyBidW1wOiBidW1wLCBzZWxmSXNFeHBvcnRzOiBzZWxmSXNFeHBvcnRzLCBibG9ja0ZuOiBibG9ja0ZuIH07XG4iXSwic291cmNlUm9vdCI6IiJ9\n//# sourceURL=webpack-internal:///./legacy.js\n");

/***/ }),

/***/ "./shapes.js":
/*!*******************!*\
  !*** ./shapes.js ***!
  \*******************/
/***/ ((__unused_webpack_module, __webpack_exports__, __webpack_require__) => {

"use strict";
eval("__webpack_require__.r(__webpack_exports__);\n/* harmony export */ __webpack_require__.d(__webpack_exports__, {\n/* harmony export */   \"unit\": () => (/* binding */ unit),\n/* harmony export */   \"area\": () => (/* binding */ area),\n/* harmony export */   \"default\": () => (__WEBPACK_DEFAULT_EXPORT__)\n/* harmony export */ });\nconst unit = { width: 2, height: 3 };\nfunction area(shape) {\n  return shape.width * shape.height;\n}\n/* harmony default export */ const __WEBPACK_DEFAULT_EXPORT__ = (\"shapes\");\n//# sourceURL=[module]\n//# sourceMappingURL=data:application/json;charset=utf-8;base64,eyJ2ZXJzaW9uIjozLCJzb3VyY2VzIjpbIndlYnBhY2s6Ly8vLi9zaGFwZXMuanM/MzBlMyJdLCJuYW1lcyI6W10sIm1hcHBpbmdzIjoiOzs7Ozs7QUFBTyxjQUFjO0FBQ2Q7QUFDUDtBQUNBO0FBQ0EsaUVBQWUsUUFBUSxFQUFDIiwiZmlsZSI6Ii4vc2hhcGVzLmpzLmpzIiwic291cmNlc0NvbnRlbnQiOlsiZXhwb3J0IGNvbnN0IHVuaXQgPSB7IHdpZHRoOiAyLCBoZWlnaHQ6IDMgfTtcbmV4cG9ydCBmdW5jdGlvbiBhcmVhKHNoYXBlKSB7XG4gIHJldHVybiBzaGFwZS53aWR0aCAqIHNoYXBlLmhlaWdodDtcbn1cbmV4cG9ydCBkZWZhdWx0IFwic2hhcGVzXCI7XG4iXSwic291cmNlUm9vdCI6IiJ9\n//# sourceURL=webpack-internal:///./shapes.js\n");

/***/ }),

/***/ "./strict-cjs.js":
/*!***********************!*\
  !*** ./strict-cjs.js ***!
  \***********************/
/***/ ((__unused_webpack_module, exports) => {

"use strict";
eval("\nvar label = \"strict\";\nfunction mode() {\n  return typeof this === \"undefined\" ? \"strict\" : \"sloppy\";\n}\nexports.mode = mode;\nexports.label = label;\n//# sourceURL=[module]\n//# sourceMappingURL=data:application/json;charset=utf-8;base64,eyJ2ZXJzaW9uIjozLCJzb3VyY2VzIjpbIndlYnBhY2s6Ly8vLi9zdHJpY3QtY2pzLmpzPzg0NTMiXSwibmFtZXMiOltdLCJtYXBwaW5ncyI6IkFBQWE7QUFDYjtBQUNBO0FBQ0E7QUFDQTtBQUNBLFlBQVk7QUFDWixhQUFhIiwiZmlsZSI6Ii4vc3RyaWN0LWNqcy5qcy5qcyIsInNvdXJjZXNDb250ZW50IjpbIlwidXNlIHN0cmljdFwiO1xudmFyIGxhYmVsID0gXCJzdHJpY3RcIjtcbmZ1bmN0aW9uIG1vZGUoKSB7XG4gIHJldHVybiB0eXBlb2YgdGhpcyA9PT0gXCJ1bmRlZmluZWRcIiA/IFwic3RyaWN0XCIgOiBcInNsb3BweVwiO1xufVxuZXhwb3J0cy5tb2RlID0gbW9kZTtcbmV4cG9ydHMubGFiZWwgPSBsYWJlbDtcbiJdLCJzb3VyY2VSb290IjoiIn0=\n//# sourceURL=webpack-internal:///./strict-cjs.js\n");

/***/ }),

/***/ "./util.js":
/*!*****************!*\
  !*** ./util.js ***!
  \*****************/
/***/ ((__unused_webpack_module, __webpack_exports__, __webpack_require__) => {

"use strict";
eval("__webpack_require__.r(__webpack_exports__);\n/* harmony export */ __webpack_require__.d(__webpack_exports__, {\n/* harmony export */   \"greet\": () => (/* binding */ greet),\n/* harmony export */   \"total\": () => (/* binding */ total),\n/* harmony export */   \"Counter\": () => (/* binding */ Counter),\n/* harmony export */   \"makeAdders\": () => (/* binding */ makeAdders)\n/* harmony export */ });\nfunction greet(name) {\n  return \"hello \" + name;\n}\n\nconst total = (list) => list.reduce((sum, value) => sum + value, 0);\n\nlet made = 0;\n\nclass Counter {\n  constructor(start) {\n    made += 1;\n    this.value = start;\n  }\n  add(step) {\n    this.value += step;\n    return this;\n  }\n}\n\nfunction makeAdders(count) {\n  const adders = [];\n  for (let index = 0; index < count; index++) {\n    adders.push((value) => value + index + made);\n  }\n  return adders;\n}\n//# sourceURL=[module]\n//# sourceMappingURL=data:application/json;charset=utf-8;base64,eyJ2ZXJzaW9uIjozLCJzb3VyY2VzIjpbIndlYnBhY2s6Ly8vLi91dGlsLmpzPzk0MTUiXSwibmFtZXMiOltdLCJtYXBwaW5ncyI6Ijs7Ozs7OztBQUFPO0FBQ1A7QUFDQTs7QUFFTzs7QUFFUDs7QUFFTztBQUNQO0FBQ0E7QUFDQTtBQUNBO0FBQ0E7QUFDQTtBQUNBO0FBQ0E7QUFDQTs7QUFFTztBQUNQO0FBQ0EscUJBQXFCLGVBQWU7QUFDcEM7QUFDQTtBQUNBO0FBQ0EiLCJmaWxlIjoiLi91dGlsLmpzLmpzIiwic291cmNlc0NvbnRlbnQiOlsiZXhwb3J0IGZ1bmN0aW9uIGdyZWV0KG5hbWUpIHtcbiAgcmV0dXJuIFwiaGVsbG8gXCIgKyBuYW1lO1xufVxuXG5leHBvcnQgY29uc3QgdG90YWwgPSAobGlzdCkgPT4gbGlzdC5yZWR1Y2UoKHN1bSwgdmFsdWUpID0+IHN1bSArIHZhbHVlLCAwKTtcblxubGV0IG1hZGUgPSAwO1xuXG5leHBvcnQgY2xhc3MgQ291bnRlciB7XG4gIGNvbnN0cnVjdG9yKHN0YXJ0KSB7XG4gICAgbWFkZSArPSAxO1xuICAgIHRoaXMudmFsdWUgPSBzdGFydDtcbiAgfVxuICBhZGQoc3RlcCkge1xuICAgIHRoaXMudmFsdWUgKz0gc3RlcDtcbiAgICByZXR1cm4gdGhpcztcbiAgfVxufVxuXG5leHBvcnQgZnVuY3Rpb24gbWFrZUFkZGVycyhjb3VudCkge1xuICBjb25zdCBhZGRlcnMgPSBbXTtcbiAgZm9yIChsZXQgaW5kZXggPSAwOyBpbmRleCA8IGNvdW50OyBpbmRleCsrKSB7XG4gICAgYWRkZXJzLnB1c2goKHZhbHVlKSA9PiB2YWx1ZSArIGluZGV4ICsgbWFkZSk7XG4gIH1cbiAgcmV0dXJuIGFkZGVycztcbn1cbiJdLCJzb3VyY2VSb290IjoiIn0=\n//# sourceURL=webpack-internal:///./util.js\n");

/***/ })

/******/ 	});
/************************************************************************/
/******/ 	// The module cache
/******/ 	var __webpack_module_cache__ = {};
/******/ 	
/******/ 	// The require function
/******/ 	function __webpack_require__(moduleId) {
/******/ 		// Check if module is in cache
/******/ 		if(__webpack_module_cache__[moduleId]) {
/******/ 			return __webpack_module_cache__[moduleId].exports;
/******/ 		}
/******/ 		// Create a new module (and put it into the cache)
/******/ 		var module = __webpack_module_cache__[moduleId] = {
/******/ 			// no module.id needed
/******/ 			// no module.loaded needed
/******/ 			exports: {}
/******/ 		};
/******/ 	
/******/ 		// Execute the module function
/******/ 		__webpack_modules__[moduleId].call(module.exports, module, module.exports, __webpack_require__);
/******/ 	
/******/ 		// Return the exports of the module
/******/ 		return module.exports;
/******/ 	}
/******/ 	
/************************************************************************/
/******/ 	/* webpack/runtime/compat get default export */
/******/ 	(() => {
/******/ 		// getDefaultExport function for compatibility with non-harmony modules
/******/ 		__webpack_require__.n = (module) => {
/******/ 			var getter = module && module.__esModule ?
/******/ 				() => (module['default']) :
/******/ 				() => (module);
/******/ 			__webpack_require__.d(getter, { a: getter });
/******/ 			return getter;
/******/ 		};
/******/ 	})();
/******/ 	
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