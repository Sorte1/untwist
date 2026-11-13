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

/***/ "./index.js"
/*!******************!*\
  !*** ./index.js ***!
  \******************/
(__unused_webpack_module, __webpack_exports__, __webpack_require__) {

"use strict";
eval("{__webpack_require__.r(__webpack_exports__);\n/* harmony import */ var _util_js__WEBPACK_IMPORTED_MODULE_0__ = __webpack_require__(/*! ./util.js */ \"./util.js\");\n/* harmony import */ var _legacy_js__WEBPACK_IMPORTED_MODULE_1__ = __webpack_require__(/*! ./legacy.js */ \"./legacy.js\");\n/* harmony import */ var _legacy_js__WEBPACK_IMPORTED_MODULE_1___default = /*#__PURE__*/__webpack_require__.n(_legacy_js__WEBPACK_IMPORTED_MODULE_1__);\n/* harmony import */ var _strict_cjs_js__WEBPACK_IMPORTED_MODULE_2__ = __webpack_require__(/*! ./strict-cjs.js */ \"./strict-cjs.js\");\n/* harmony import */ var _data_json__WEBPACK_IMPORTED_MODULE_3__ = __webpack_require__(/*! ./data.json */ \"./data.json\");\n/* harmony import */ var _shapes_js__WEBPACK_IMPORTED_MODULE_4__ = __webpack_require__(/*! ./shapes.js */ \"./shapes.js\");\n\n\n\n\n\n\nconsole.log((0,_util_js__WEBPACK_IMPORTED_MODULE_0__.greet)(\"webpack\"), (0,_util_js__WEBPACK_IMPORTED_MODULE_0__.total)([1, 2, 3, 4]));\nconst counter = new _util_js__WEBPACK_IMPORTED_MODULE_0__.Counter(2);\ncounter.add(3);\nconsole.log(\"counter\", counter.value, _util_js__WEBPACK_IMPORTED_MODULE_0__.Counter.name);\nconsole.log(\"adders\", (0,_util_js__WEBPACK_IMPORTED_MODULE_0__.makeAdders)(3).map((add) => add(10)).join(\",\"));\nconsole.log(\"legacy\", _legacy_js__WEBPACK_IMPORTED_MODULE_1___default().bump(), _legacy_js__WEBPACK_IMPORTED_MODULE_1___default().bump(), (_legacy_js__WEBPACK_IMPORTED_MODULE_1___default().selfIsExports), _legacy_js__WEBPACK_IMPORTED_MODULE_1___default().blockFn());\nconsole.log(\"strict cjs\", _strict_cjs_js__WEBPACK_IMPORTED_MODULE_2__.mode(), _strict_cjs_js__WEBPACK_IMPORTED_MODULE_2__.label);\nconsole.log(\"data\", _data_json__WEBPACK_IMPORTED_MODULE_3__.name, _data_json__WEBPACK_IMPORTED_MODULE_3__.items.length);\nconsole.log(\"shapes\", Object.keys(_shapes_js__WEBPACK_IMPORTED_MODULE_4__).sort().join(\",\"), _shapes_js__WEBPACK_IMPORTED_MODULE_4__.area(_shapes_js__WEBPACK_IMPORTED_MODULE_4__.unit));\n//# sourceURL=[module]\n//# sourceMappingURL=data:application/json;charset=utf-8;base64,eyJ2ZXJzaW9uIjozLCJmaWxlIjoiLi9pbmRleC5qcyIsIm1hcHBpbmdzIjoiOzs7Ozs7O0FBQUE7QUFDQTtBQUNBO0FBQ0E7QUFDQTtBQUNBO0FBQ0E7QUFDQTtBQUNBO0FBQ0E7QUFDQTtBQUNBO0FBQ0E7QUFDQTtBQUNBIiwic291cmNlcyI6WyJ3ZWJwYWNrOi8vLy4vaW5kZXguanM/NGE4YiJdLCJuYW1lcyI6W10sInNvdXJjZVJvb3QiOiIifQ==\n//# sourceURL=webpack-internal:///./index.js\n\n}");

/***/ },

/***/ "./legacy.js"
/*!*******************!*\
  !*** ./legacy.js ***!
  \*******************/
(module) {

eval("{var count = 0;\nfunction bump() {\n  count += 1;\n  return count;\n}\nvar selfIsExports = this === module.exports;\nif (count === 0) {\n  function blockFn() {\n    return typeof bump;\n  }\n}\nmodule.exports = { bump: bump, selfIsExports: selfIsExports, blockFn: blockFn };\n//# sourceURL=[module]\n//# sourceMappingURL=data:application/json;charset=utf-8;base64,eyJ2ZXJzaW9uIjozLCJmaWxlIjoiLi9sZWdhY3kuanMiLCJtYXBwaW5ncyI6IkFBQUE7QUFDQTtBQUNBO0FBQ0E7QUFDQTtBQUNBO0FBQ0E7QUFDQTtBQUNBO0FBQ0E7QUFDQTtBQUNBIiwic291cmNlcyI6WyJ3ZWJwYWNrOi8vLy4vbGVnYWN5LmpzP2IwMzAiXSwibmFtZXMiOltdLCJzb3VyY2VSb290IjoiIn0=\n//# sourceURL=webpack-internal:///./legacy.js\n\n}");

/***/ },

/***/ "./shapes.js"
/*!*******************!*\
  !*** ./shapes.js ***!
  \*******************/
(__unused_webpack_module, __webpack_exports__, __webpack_require__) {

"use strict";
eval("{__webpack_require__.r(__webpack_exports__);\n/* harmony export */ __webpack_require__.d(__webpack_exports__, {\n/* harmony export */   area: () => (/* binding */ area),\n/* harmony export */   \"default\": () => (__WEBPACK_DEFAULT_EXPORT__),\n/* harmony export */   unit: () => (/* binding */ unit)\n/* harmony export */ });\nconst unit = { width: 2, height: 3 };\nfunction area(shape) {\n  return shape.width * shape.height;\n}\n/* harmony default export */ const __WEBPACK_DEFAULT_EXPORT__ = (\"shapes\");\n//# sourceURL=[module]\n//# sourceMappingURL=data:application/json;charset=utf-8;base64,eyJ2ZXJzaW9uIjozLCJmaWxlIjoiLi9zaGFwZXMuanMiLCJtYXBwaW5ncyI6Ijs7Ozs7O0FBQUE7QUFDQTtBQUNBO0FBQ0E7QUFDQSIsInNvdXJjZXMiOlsid2VicGFjazovLy8uL3NoYXBlcy5qcz9jM2IwIl0sIm5hbWVzIjpbXSwic291cmNlUm9vdCI6IiJ9\n//# sourceURL=webpack-internal:///./shapes.js\n\n}");

/***/ },

/***/ "./strict-cjs.js"
/*!***********************!*\
  !*** ./strict-cjs.js ***!
  \***********************/
(__unused_webpack_module, exports) {

"use strict";
eval("{\nvar label = \"strict\";\nfunction mode() {\n  return typeof this === \"undefined\" ? \"strict\" : \"sloppy\";\n}\nexports.mode = mode;\nexports.label = label;\n//# sourceURL=[module]\n//# sourceMappingURL=data:application/json;charset=utf-8;base64,eyJ2ZXJzaW9uIjozLCJmaWxlIjoiLi9zdHJpY3QtY2pzLmpzIiwibWFwcGluZ3MiOiJBQUFBO0FBQ0E7QUFDQTtBQUNBO0FBQ0E7QUFDQTtBQUNBIiwic291cmNlcyI6WyJ3ZWJwYWNrOi8vLy4vc3RyaWN0LWNqcy5qcz85YzgxIl0sIm5hbWVzIjpbXSwic291cmNlUm9vdCI6IiJ9\n//# sourceURL=webpack-internal:///./strict-cjs.js\n\n}");

/***/ },

/***/ "./util.js"
/*!*****************!*\
  !*** ./util.js ***!
  \*****************/
(__unused_webpack_module, __webpack_exports__, __webpack_require__) {

"use strict";
eval("{__webpack_require__.r(__webpack_exports__);\n/* harmony export */ __webpack_require__.d(__webpack_exports__, {\n/* harmony export */   Counter: () => (/* binding */ Counter),\n/* harmony export */   greet: () => (/* binding */ greet),\n/* harmony export */   makeAdders: () => (/* binding */ makeAdders),\n/* harmony export */   total: () => (/* binding */ total)\n/* harmony export */ });\nfunction greet(name) {\n  return \"hello \" + name;\n}\n\nconst total = (list) => list.reduce((sum, value) => sum + value, 0);\n\nlet made = 0;\n\nclass Counter {\n  constructor(start) {\n    made += 1;\n    this.value = start;\n  }\n  add(step) {\n    this.value += step;\n    return this;\n  }\n}\n\nfunction makeAdders(count) {\n  const adders = [];\n  for (let index = 0; index < count; index++) {\n    adders.push((value) => value + index + made);\n  }\n  return adders;\n}\n//# sourceURL=[module]\n//# sourceMappingURL=data:application/json;charset=utf-8;base64,eyJ2ZXJzaW9uIjozLCJmaWxlIjoiLi91dGlsLmpzIiwibWFwcGluZ3MiOiI7Ozs7Ozs7QUFBQTtBQUNBO0FBQ0E7QUFDQTtBQUNBO0FBQ0E7QUFDQTtBQUNBO0FBQ0E7QUFDQTtBQUNBO0FBQ0E7QUFDQTtBQUNBO0FBQ0E7QUFDQTtBQUNBO0FBQ0E7QUFDQTtBQUNBO0FBQ0E7QUFDQTtBQUNBO0FBQ0E7QUFDQTtBQUNBIiwic291cmNlcyI6WyJ3ZWJwYWNrOi8vLy4vdXRpbC5qcz8wMWNmIl0sIm5hbWVzIjpbXSwic291cmNlUm9vdCI6IiJ9\n//# sourceURL=webpack-internal:///./util.js\n\n}");

/***/ },

/***/ "./data.json"
/*!*******************!*\
  !*** ./data.json ***!
  \*******************/
(module) {

"use strict";
module.exports = /*#__PURE__*/JSON.parse('{"name":"fixture","items":["a","b","c"]}');

/***/ }

/******/ 	});
/************************************************************************/
/******/ 	// The module cache
/******/ 	const __webpack_module_cache__ = {};
/******/ 	
/******/ 	// The require function
/******/ 	function __webpack_require__(moduleId) {
/******/ 		// Check if module is in cache
/******/ 		const cachedModule = __webpack_module_cache__[moduleId];
/******/ 		if (cachedModule !== undefined) {
/******/ 			return cachedModule.exports;
/******/ 		}
/******/ 		// Create a new module (and put it into the cache)
/******/ 		const module = __webpack_module_cache__[moduleId] = {
/******/ 			// no module.id needed
/******/ 			// no module.loaded needed
/******/ 			exports: {}
/******/ 		};
/******/ 	
/******/ 		// Execute the module function
/******/ 		if (!(moduleId in __webpack_modules__)) {
/******/ 			delete __webpack_module_cache__[moduleId];
/******/ 			const e = new Error("Cannot find module '" + moduleId + "'");
/******/ 			e.code = 'MODULE_NOT_FOUND';
/******/ 			throw e;
/******/ 		}
/******/ 		__webpack_modules__[moduleId].call(module.exports, module, module.exports, __webpack_require__);
/******/ 	
/******/ 		// Return the exports of the module
/******/ 		return module.exports;
/******/ 	}
/******/ 	
/************************************************************************/
/******/ 	/* webpack/runtime/compat get default export */
/******/ 	// getDefaultExport function for compatibility with non-harmony modules
/******/ 	__webpack_require__.n = (module) => {
/******/ 		const getter = module && module.__esModule ?
/******/ 			() => (module['default']) :
/******/ 			() => (module);
/******/ 		__webpack_require__.d(getter, { a: getter });
/******/ 		return getter;
/******/ 	};
/******/ 	
/******/ 	/* webpack/runtime/define property getters */
/******/ 	// define getter/value functions for harmony exports
/******/ 	__webpack_require__.d = (exports, definition) => {
/******/ 		for(var key in definition) {
/******/ 			if(__webpack_require__.o(definition, key) && !__webpack_require__.o(exports, key)) {
/******/ 				Object.defineProperty(exports, key, { enumerable: true, get: definition[key] });
/******/ 			}
/******/ 		}
/******/ 	};
/******/ 	
/******/ 	/* webpack/runtime/hasOwnProperty shorthand */
/******/ 	__webpack_require__.o = (obj, prop) => (Object.prototype.hasOwnProperty.call(obj, prop));
/******/ 	
/******/ 	/* webpack/runtime/make namespace object */
/******/ 	// define __esModule on exports
/******/ 	__webpack_require__.r = (exports) => {
/******/ 		Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });
/******/ 		Object.defineProperty(exports, '__esModule', { value: true });
/******/ 	};
/******/ 	
/************************************************************************/
/******/ 	
/******/ 	// startup
/******/ 	// Load entry module and return exports
/******/ 	// This entry module can't be inlined because the eval-source-map devtool is used.
/******/ 	let __webpack_exports__ = __webpack_require__("./index.js");
/******/ 	
/******/ })()
;