/*
 * ATTENTION: The "eval" devtool has been used (maybe by default in mode: "development").
 * This devtool is neither made for production nor for readable output files.
 * It uses "eval()" calls to create a separate source file in the browser devtools.
 * If you are trying to read the output file, select a different devtool (https://webpack.js.org/configuration/devtool/)
 * or disable the default devtool with "devtool: false".
 * If you are looking for production-ready output files, see mode: "production" (https://webpack.js.org/configuration/mode/).
 */
/******/ (() => { // webpackBootstrap
/******/ 	var __webpack_modules__ = ({

/***/ "./index.js":
/*!******************!*\
  !*** ./index.js ***!
  \******************/
/***/ ((__unused_webpack_module, __webpack_exports__, __webpack_require__) => {

"use strict";
eval("__webpack_require__.r(__webpack_exports__);\n/* harmony import */ var _util_js__WEBPACK_IMPORTED_MODULE_0__ = __webpack_require__(/*! ./util.js */ \"./util.js\");\n/* harmony import */ var _legacy_js__WEBPACK_IMPORTED_MODULE_1__ = __webpack_require__(/*! ./legacy.js */ \"./legacy.js\");\n/* harmony import */ var _legacy_js__WEBPACK_IMPORTED_MODULE_1___default = /*#__PURE__*/__webpack_require__.n(_legacy_js__WEBPACK_IMPORTED_MODULE_1__);\n/* harmony import */ var _strict_cjs_js__WEBPACK_IMPORTED_MODULE_2__ = __webpack_require__(/*! ./strict-cjs.js */ \"./strict-cjs.js\");\n/* harmony import */ var _data_json__WEBPACK_IMPORTED_MODULE_3__ = __webpack_require__(/*! ./data.json */ \"./data.json\");\n/* harmony import */ var _shapes_js__WEBPACK_IMPORTED_MODULE_4__ = __webpack_require__(/*! ./shapes.js */ \"./shapes.js\");\n\n\n\n\n\n\nconsole.log((0,_util_js__WEBPACK_IMPORTED_MODULE_0__.greet)(\"webpack\"), (0,_util_js__WEBPACK_IMPORTED_MODULE_0__.total)([1, 2, 3, 4]));\nconst counter = new _util_js__WEBPACK_IMPORTED_MODULE_0__.Counter(2);\ncounter.add(3);\nconsole.log(\"counter\", counter.value, _util_js__WEBPACK_IMPORTED_MODULE_0__.Counter.name);\nconsole.log(\"adders\", (0,_util_js__WEBPACK_IMPORTED_MODULE_0__.makeAdders)(3).map((add) => add(10)).join(\",\"));\nconsole.log(\"legacy\", _legacy_js__WEBPACK_IMPORTED_MODULE_1___default().bump(), _legacy_js__WEBPACK_IMPORTED_MODULE_1___default().bump(), (_legacy_js__WEBPACK_IMPORTED_MODULE_1___default().selfIsExports), _legacy_js__WEBPACK_IMPORTED_MODULE_1___default().blockFn());\nconsole.log(\"strict cjs\", _strict_cjs_js__WEBPACK_IMPORTED_MODULE_2__.mode(), _strict_cjs_js__WEBPACK_IMPORTED_MODULE_2__.label);\nconsole.log(\"data\", _data_json__WEBPACK_IMPORTED_MODULE_3__.name, _data_json__WEBPACK_IMPORTED_MODULE_3__.items.length);\nconsole.log(\"shapes\", Object.keys(_shapes_js__WEBPACK_IMPORTED_MODULE_4__).sort().join(\",\"), _shapes_js__WEBPACK_IMPORTED_MODULE_4__.area(_shapes_js__WEBPACK_IMPORTED_MODULE_4__.unit));\n\n\n//# sourceURL=webpack:///./index.js?");

/***/ }),

/***/ "./legacy.js":
/*!*******************!*\
  !*** ./legacy.js ***!
  \*******************/
/***/ (function(module) {

eval("var count = 0;\nfunction bump() {\n  count += 1;\n  return count;\n}\nvar selfIsExports = this === module.exports;\nif (count === 0) {\n  function blockFn() {\n    return typeof bump;\n  }\n}\nmodule.exports = { bump: bump, selfIsExports: selfIsExports, blockFn: blockFn };\n\n\n//# sourceURL=webpack:///./legacy.js?");

/***/ }),

/***/ "./shapes.js":
/*!*******************!*\
  !*** ./shapes.js ***!
  \*******************/
/***/ ((__unused_webpack_module, __webpack_exports__, __webpack_require__) => {

"use strict";
eval("__webpack_require__.r(__webpack_exports__);\n/* harmony export */ __webpack_require__.d(__webpack_exports__, {\n/* harmony export */   \"area\": () => (/* binding */ area),\n/* harmony export */   \"default\": () => (__WEBPACK_DEFAULT_EXPORT__),\n/* harmony export */   \"unit\": () => (/* binding */ unit)\n/* harmony export */ });\nconst unit = { width: 2, height: 3 };\nfunction area(shape) {\n  return shape.width * shape.height;\n}\n/* harmony default export */ const __WEBPACK_DEFAULT_EXPORT__ = (\"shapes\");\n\n\n//# sourceURL=webpack:///./shapes.js?");

/***/ }),

/***/ "./strict-cjs.js":
/*!***********************!*\
  !*** ./strict-cjs.js ***!
  \***********************/
/***/ ((__unused_webpack_module, exports) => {

"use strict";
eval("\nvar label = \"strict\";\nfunction mode() {\n  return typeof this === \"undefined\" ? \"strict\" : \"sloppy\";\n}\nexports.mode = mode;\nexports.label = label;\n\n\n//# sourceURL=webpack:///./strict-cjs.js?");

/***/ }),

/***/ "./util.js":
/*!*****************!*\
  !*** ./util.js ***!
  \*****************/
/***/ ((__unused_webpack_module, __webpack_exports__, __webpack_require__) => {

"use strict";
eval("__webpack_require__.r(__webpack_exports__);\n/* harmony export */ __webpack_require__.d(__webpack_exports__, {\n/* harmony export */   \"Counter\": () => (/* binding */ Counter),\n/* harmony export */   \"greet\": () => (/* binding */ greet),\n/* harmony export */   \"makeAdders\": () => (/* binding */ makeAdders),\n/* harmony export */   \"total\": () => (/* binding */ total)\n/* harmony export */ });\nfunction greet(name) {\n  return \"hello \" + name;\n}\n\nconst total = (list) => list.reduce((sum, value) => sum + value, 0);\n\nlet made = 0;\n\nclass Counter {\n  constructor(start) {\n    made += 1;\n    this.value = start;\n  }\n  add(step) {\n    this.value += step;\n    return this;\n  }\n}\n\nfunction makeAdders(count) {\n  const adders = [];\n  for (let index = 0; index < count; index++) {\n    adders.push((value) => value + index + made);\n  }\n  return adders;\n}\n\n\n//# sourceURL=webpack:///./util.js?");

/***/ }),

/***/ "./data.json":
/*!*******************!*\
  !*** ./data.json ***!
  \*******************/
/***/ ((module) => {

"use strict";
eval("module.exports = JSON.parse('{\"name\":\"fixture\",\"items\":[\"a\",\"b\",\"c\"]}');\n\n//# sourceURL=webpack:///./data.json?");

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
/******/ 	// This entry module can't be inlined because the eval devtool is used.
/******/ 	var __webpack_exports__ = __webpack_require__("./index.js");
/******/ 	
/******/ })()
;