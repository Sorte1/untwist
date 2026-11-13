/******/ (function(modules) { // webpackBootstrap
/******/ 	// The module cache
/******/ 	var installedModules = {};
/******/
/******/ 	// The require function
/******/ 	function __webpack_require__(moduleId) {
/******/
/******/ 		// Check if module is in cache
/******/ 		if(installedModules[moduleId]) {
/******/ 			return installedModules[moduleId].exports;
/******/ 		}
/******/ 		// Create a new module (and put it into the cache)
/******/ 		var module = installedModules[moduleId] = {
/******/ 			i: moduleId,
/******/ 			l: false,
/******/ 			exports: {}
/******/ 		};
/******/
/******/ 		// Execute the module function
/******/ 		modules[moduleId].call(module.exports, module, module.exports, __webpack_require__);
/******/
/******/ 		// Flag the module as loaded
/******/ 		module.l = true;
/******/
/******/ 		// Return the exports of the module
/******/ 		return module.exports;
/******/ 	}
/******/
/******/
/******/ 	// expose the modules object (__webpack_modules__)
/******/ 	__webpack_require__.m = modules;
/******/
/******/ 	// expose the module cache
/******/ 	__webpack_require__.c = installedModules;
/******/
/******/ 	// define getter function for harmony exports
/******/ 	__webpack_require__.d = function(exports, name, getter) {
/******/ 		if(!__webpack_require__.o(exports, name)) {
/******/ 			Object.defineProperty(exports, name, { enumerable: true, get: getter });
/******/ 		}
/******/ 	};
/******/
/******/ 	// define __esModule on exports
/******/ 	__webpack_require__.r = function(exports) {
/******/ 		if(typeof Symbol !== 'undefined' && Symbol.toStringTag) {
/******/ 			Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });
/******/ 		}
/******/ 		Object.defineProperty(exports, '__esModule', { value: true });
/******/ 	};
/******/
/******/ 	// create a fake namespace object
/******/ 	// mode & 1: value is a module id, require it
/******/ 	// mode & 2: merge all properties of value into the ns
/******/ 	// mode & 4: return value when already ns object
/******/ 	// mode & 8|1: behave like require
/******/ 	__webpack_require__.t = function(value, mode) {
/******/ 		if(mode & 1) value = __webpack_require__(value);
/******/ 		if(mode & 8) return value;
/******/ 		if((mode & 4) && typeof value === 'object' && value && value.__esModule) return value;
/******/ 		var ns = Object.create(null);
/******/ 		__webpack_require__.r(ns);
/******/ 		Object.defineProperty(ns, 'default', { enumerable: true, value: value });
/******/ 		if(mode & 2 && typeof value != 'string') for(var key in value) __webpack_require__.d(ns, key, function(key) { return value[key]; }.bind(null, key));
/******/ 		return ns;
/******/ 	};
/******/
/******/ 	// getDefaultExport function for compatibility with non-harmony modules
/******/ 	__webpack_require__.n = function(module) {
/******/ 		var getter = module && module.__esModule ?
/******/ 			function getDefault() { return module['default']; } :
/******/ 			function getModuleExports() { return module; };
/******/ 		__webpack_require__.d(getter, 'a', getter);
/******/ 		return getter;
/******/ 	};
/******/
/******/ 	// Object.prototype.hasOwnProperty.call
/******/ 	__webpack_require__.o = function(object, property) { return Object.prototype.hasOwnProperty.call(object, property); };
/******/
/******/ 	// __webpack_public_path__
/******/ 	__webpack_require__.p = "";
/******/
/******/
/******/ 	// Load entry module and return exports
/******/ 	return __webpack_require__(__webpack_require__.s = "./index.js");
/******/ })
/************************************************************************/
/******/ ({

/***/ "./data.json":
/*!*******************!*\
  !*** ./data.json ***!
  \*******************/
/*! exports provided: name, items, default */
/***/ (function(module) {

eval("module.exports = JSON.parse(\"{\\\"name\\\":\\\"fixture\\\",\\\"items\\\":[\\\"a\\\",\\\"b\\\",\\\"c\\\"]}\");//# sourceURL=[module]\n//# sourceMappingURL=data:application/json;charset=utf-8;base64,eyJ2ZXJzaW9uIjozLCJzb3VyY2VzIjpbXSwibmFtZXMiOltdLCJtYXBwaW5ncyI6IiIsImZpbGUiOiIuL2RhdGEuanNvbi5qcyIsInNvdXJjZVJvb3QiOiIifQ==\n//# sourceURL=webpack-internal:///./data.json\n");

/***/ }),

/***/ "./index.js":
/*!******************!*\
  !*** ./index.js ***!
  \******************/
/*! no exports provided */
/***/ (function(module, __webpack_exports__, __webpack_require__) {

"use strict";
eval("__webpack_require__.r(__webpack_exports__);\n/* harmony import */ var _util_js__WEBPACK_IMPORTED_MODULE_0__ = __webpack_require__(/*! ./util.js */ \"./util.js\");\n/* harmony import */ var _legacy_js__WEBPACK_IMPORTED_MODULE_1__ = __webpack_require__(/*! ./legacy.js */ \"./legacy.js\");\n/* harmony import */ var _legacy_js__WEBPACK_IMPORTED_MODULE_1___default = /*#__PURE__*/__webpack_require__.n(_legacy_js__WEBPACK_IMPORTED_MODULE_1__);\n/* harmony import */ var _strict_cjs_js__WEBPACK_IMPORTED_MODULE_2__ = __webpack_require__(/*! ./strict-cjs.js */ \"./strict-cjs.js\");\n/* harmony import */ var _strict_cjs_js__WEBPACK_IMPORTED_MODULE_2___default = /*#__PURE__*/__webpack_require__.n(_strict_cjs_js__WEBPACK_IMPORTED_MODULE_2__);\n/* harmony import */ var _data_json__WEBPACK_IMPORTED_MODULE_3__ = __webpack_require__(/*! ./data.json */ \"./data.json\");\nvar _data_json__WEBPACK_IMPORTED_MODULE_3___namespace = /*#__PURE__*/__webpack_require__.t(/*! ./data.json */ \"./data.json\", 1);\n/* harmony import */ var _shapes_js__WEBPACK_IMPORTED_MODULE_4__ = __webpack_require__(/*! ./shapes.js */ \"./shapes.js\");\n\n\n\n\n\n\nconsole.log(Object(_util_js__WEBPACK_IMPORTED_MODULE_0__[\"greet\"])(\"webpack\"), Object(_util_js__WEBPACK_IMPORTED_MODULE_0__[\"total\"])([1, 2, 3, 4]));\nconst counter = new _util_js__WEBPACK_IMPORTED_MODULE_0__[\"Counter\"](2);\ncounter.add(3);\nconsole.log(\"counter\", counter.value, _util_js__WEBPACK_IMPORTED_MODULE_0__[\"Counter\"].name);\nconsole.log(\"adders\", Object(_util_js__WEBPACK_IMPORTED_MODULE_0__[\"makeAdders\"])(3).map((add) => add(10)).join(\",\"));\nconsole.log(\"legacy\", _legacy_js__WEBPACK_IMPORTED_MODULE_1___default.a.bump(), _legacy_js__WEBPACK_IMPORTED_MODULE_1___default.a.bump(), _legacy_js__WEBPACK_IMPORTED_MODULE_1___default.a.selfIsExports, _legacy_js__WEBPACK_IMPORTED_MODULE_1___default.a.blockFn());\nconsole.log(\"strict cjs\", _strict_cjs_js__WEBPACK_IMPORTED_MODULE_2___default.a.mode(), _strict_cjs_js__WEBPACK_IMPORTED_MODULE_2___default.a.label);\nconsole.log(\"data\", _data_json__WEBPACK_IMPORTED_MODULE_3__.name, _data_json__WEBPACK_IMPORTED_MODULE_3__.items.length);\nconsole.log(\"shapes\", Object.keys(_shapes_js__WEBPACK_IMPORTED_MODULE_4__).sort().join(\",\"), _shapes_js__WEBPACK_IMPORTED_MODULE_4__[\"area\"](_shapes_js__WEBPACK_IMPORTED_MODULE_4__[\"unit\"]));\n//# sourceURL=[module]\n//# sourceMappingURL=data:application/json;charset=utf-8;base64,eyJ2ZXJzaW9uIjozLCJzb3VyY2VzIjpbIndlYnBhY2s6Ly8vLi9pbmRleC5qcz80MWY1Il0sIm5hbWVzIjpbXSwibWFwcGluZ3MiOiJBQUFBO0FBQUE7QUFBQTtBQUFBO0FBQUE7QUFBQTtBQUFBO0FBQUE7QUFBQTtBQUE4RDtBQUM3QjtBQUNPO0FBQ1Q7QUFDTzs7QUFFdEMsWUFBWSxzREFBSyxhQUFhLHNEQUFLO0FBQ25DLG9CQUFvQixnREFBTztBQUMzQjtBQUNBLHNDQUFzQyxnREFBTztBQUM3QyxzQkFBc0IsMkRBQVU7QUFDaEMsc0JBQXNCLGlEQUFNLFNBQVMsaURBQU0sU0FBUyxpREFBTSxnQkFBZ0IsaURBQU07QUFDaEYsMEJBQTBCLHFEQUFTLFNBQVMscURBQVM7QUFDckQsb0JBQW9CLHVDQUFJLE9BQU8sdUNBQUk7QUFDbkMsa0NBQWtDLHVDQUFNLG9CQUFvQiwrQ0FBVyxDQUFDLCtDQUFXIiwiZmlsZSI6Ii4vaW5kZXguanMuanMiLCJzb3VyY2VzQ29udGVudCI6WyJpbXBvcnQgeyBncmVldCwgdG90YWwsIENvdW50ZXIsIG1ha2VBZGRlcnMgfSBmcm9tIFwiLi91dGlsLmpzXCI7XG5pbXBvcnQgbGVnYWN5IGZyb20gXCIuL2xlZ2FjeS5qc1wiO1xuaW1wb3J0IHN0cmljdENqcyBmcm9tIFwiLi9zdHJpY3QtY2pzLmpzXCI7XG5pbXBvcnQgZGF0YSBmcm9tIFwiLi9kYXRhLmpzb25cIjtcbmltcG9ydCAqIGFzIHNoYXBlcyBmcm9tIFwiLi9zaGFwZXMuanNcIjtcblxuY29uc29sZS5sb2coZ3JlZXQoXCJ3ZWJwYWNrXCIpLCB0b3RhbChbMSwgMiwgMywgNF0pKTtcbmNvbnN0IGNvdW50ZXIgPSBuZXcgQ291bnRlcigyKTtcbmNvdW50ZXIuYWRkKDMpO1xuY29uc29sZS5sb2coXCJjb3VudGVyXCIsIGNvdW50ZXIudmFsdWUsIENvdW50ZXIubmFtZSk7XG5jb25zb2xlLmxvZyhcImFkZGVyc1wiLCBtYWtlQWRkZXJzKDMpLm1hcCgoYWRkKSA9PiBhZGQoMTApKS5qb2luKFwiLFwiKSk7XG5jb25zb2xlLmxvZyhcImxlZ2FjeVwiLCBsZWdhY3kuYnVtcCgpLCBsZWdhY3kuYnVtcCgpLCBsZWdhY3kuc2VsZklzRXhwb3J0cywgbGVnYWN5LmJsb2NrRm4oKSk7XG5jb25zb2xlLmxvZyhcInN0cmljdCBjanNcIiwgc3RyaWN0Q2pzLm1vZGUoKSwgc3RyaWN0Q2pzLmxhYmVsKTtcbmNvbnNvbGUubG9nKFwiZGF0YVwiLCBkYXRhLm5hbWUsIGRhdGEuaXRlbXMubGVuZ3RoKTtcbmNvbnNvbGUubG9nKFwic2hhcGVzXCIsIE9iamVjdC5rZXlzKHNoYXBlcykuc29ydCgpLmpvaW4oXCIsXCIpLCBzaGFwZXMuYXJlYShzaGFwZXMudW5pdCkpO1xuIl0sInNvdXJjZVJvb3QiOiIifQ==\n//# sourceURL=webpack-internal:///./index.js\n");

/***/ }),

/***/ "./legacy.js":
/*!*******************!*\
  !*** ./legacy.js ***!
  \*******************/
/*! no static exports found */
/***/ (function(module, exports) {

eval("var count = 0;\nfunction bump() {\n  count += 1;\n  return count;\n}\nvar selfIsExports = this === module.exports;\nif (count === 0) {\n  function blockFn() {\n    return typeof bump;\n  }\n}\nmodule.exports = { bump: bump, selfIsExports: selfIsExports, blockFn: blockFn };\n//# sourceURL=[module]\n//# sourceMappingURL=data:application/json;charset=utf-8;base64,eyJ2ZXJzaW9uIjozLCJzb3VyY2VzIjpbIndlYnBhY2s6Ly8vLi9sZWdhY3kuanM/YjM4ZiJdLCJuYW1lcyI6W10sIm1hcHBpbmdzIjoiQUFBQTtBQUNBO0FBQ0E7QUFDQTtBQUNBO0FBQ0E7QUFDQTtBQUNBO0FBQ0E7QUFDQTtBQUNBO0FBQ0Esa0JBQWtCIiwiZmlsZSI6Ii4vbGVnYWN5LmpzLmpzIiwic291cmNlc0NvbnRlbnQiOlsidmFyIGNvdW50ID0gMDtcbmZ1bmN0aW9uIGJ1bXAoKSB7XG4gIGNvdW50ICs9IDE7XG4gIHJldHVybiBjb3VudDtcbn1cbnZhciBzZWxmSXNFeHBvcnRzID0gdGhpcyA9PT0gbW9kdWxlLmV4cG9ydHM7XG5pZiAoY291bnQgPT09IDApIHtcbiAgZnVuY3Rpb24gYmxvY2tGbigpIHtcbiAgICByZXR1cm4gdHlwZW9mIGJ1bXA7XG4gIH1cbn1cbm1vZHVsZS5leHBvcnRzID0geyBidW1wOiBidW1wLCBzZWxmSXNFeHBvcnRzOiBzZWxmSXNFeHBvcnRzLCBibG9ja0ZuOiBibG9ja0ZuIH07XG4iXSwic291cmNlUm9vdCI6IiJ9\n//# sourceURL=webpack-internal:///./legacy.js\n");

/***/ }),

/***/ "./shapes.js":
/*!*******************!*\
  !*** ./shapes.js ***!
  \*******************/
/*! exports provided: unit, area, default */
/***/ (function(module, __webpack_exports__, __webpack_require__) {

"use strict";
eval("__webpack_require__.r(__webpack_exports__);\n/* harmony export (binding) */ __webpack_require__.d(__webpack_exports__, \"unit\", function() { return unit; });\n/* harmony export (binding) */ __webpack_require__.d(__webpack_exports__, \"area\", function() { return area; });\nconst unit = { width: 2, height: 3 };\nfunction area(shape) {\n  return shape.width * shape.height;\n}\n/* harmony default export */ __webpack_exports__[\"default\"] = (\"shapes\");\n//# sourceURL=[module]\n//# sourceMappingURL=data:application/json;charset=utf-8;base64,eyJ2ZXJzaW9uIjozLCJzb3VyY2VzIjpbIndlYnBhY2s6Ly8vLi9zaGFwZXMuanM/MzBlMyJdLCJuYW1lcyI6W10sIm1hcHBpbmdzIjoiQUFBQTtBQUFBO0FBQUE7QUFBTyxjQUFjO0FBQ2Q7QUFDUDtBQUNBO0FBQ2UsdUVBQVEsRUFBQyIsImZpbGUiOiIuL3NoYXBlcy5qcy5qcyIsInNvdXJjZXNDb250ZW50IjpbImV4cG9ydCBjb25zdCB1bml0ID0geyB3aWR0aDogMiwgaGVpZ2h0OiAzIH07XG5leHBvcnQgZnVuY3Rpb24gYXJlYShzaGFwZSkge1xuICByZXR1cm4gc2hhcGUud2lkdGggKiBzaGFwZS5oZWlnaHQ7XG59XG5leHBvcnQgZGVmYXVsdCBcInNoYXBlc1wiO1xuIl0sInNvdXJjZVJvb3QiOiIifQ==\n//# sourceURL=webpack-internal:///./shapes.js\n");

/***/ }),

/***/ "./strict-cjs.js":
/*!***********************!*\
  !*** ./strict-cjs.js ***!
  \***********************/
/*! no static exports found */
/***/ (function(module, exports, __webpack_require__) {

"use strict";
eval("\nvar label = \"strict\";\nfunction mode() {\n  return typeof this === \"undefined\" ? \"strict\" : \"sloppy\";\n}\nexports.mode = mode;\nexports.label = label;\n//# sourceURL=[module]\n//# sourceMappingURL=data:application/json;charset=utf-8;base64,eyJ2ZXJzaW9uIjozLCJzb3VyY2VzIjpbIndlYnBhY2s6Ly8vLi9zdHJpY3QtY2pzLmpzPzg0NTMiXSwibmFtZXMiOltdLCJtYXBwaW5ncyI6IkFBQWE7QUFDYjtBQUNBO0FBQ0E7QUFDQTtBQUNBO0FBQ0EiLCJmaWxlIjoiLi9zdHJpY3QtY2pzLmpzLmpzIiwic291cmNlc0NvbnRlbnQiOlsiXCJ1c2Ugc3RyaWN0XCI7XG52YXIgbGFiZWwgPSBcInN0cmljdFwiO1xuZnVuY3Rpb24gbW9kZSgpIHtcbiAgcmV0dXJuIHR5cGVvZiB0aGlzID09PSBcInVuZGVmaW5lZFwiID8gXCJzdHJpY3RcIiA6IFwic2xvcHB5XCI7XG59XG5leHBvcnRzLm1vZGUgPSBtb2RlO1xuZXhwb3J0cy5sYWJlbCA9IGxhYmVsO1xuIl0sInNvdXJjZVJvb3QiOiIifQ==\n//# sourceURL=webpack-internal:///./strict-cjs.js\n");

/***/ }),

/***/ "./util.js":
/*!*****************!*\
  !*** ./util.js ***!
  \*****************/
/*! exports provided: greet, total, Counter, makeAdders */
/***/ (function(module, __webpack_exports__, __webpack_require__) {

"use strict";
eval("__webpack_require__.r(__webpack_exports__);\n/* harmony export (binding) */ __webpack_require__.d(__webpack_exports__, \"greet\", function() { return greet; });\n/* harmony export (binding) */ __webpack_require__.d(__webpack_exports__, \"total\", function() { return total; });\n/* harmony export (binding) */ __webpack_require__.d(__webpack_exports__, \"Counter\", function() { return Counter; });\n/* harmony export (binding) */ __webpack_require__.d(__webpack_exports__, \"makeAdders\", function() { return makeAdders; });\nfunction greet(name) {\n  return \"hello \" + name;\n}\n\nconst total = (list) => list.reduce((sum, value) => sum + value, 0);\n\nlet made = 0;\n\nclass Counter {\n  constructor(start) {\n    made += 1;\n    this.value = start;\n  }\n  add(step) {\n    this.value += step;\n    return this;\n  }\n}\n\nfunction makeAdders(count) {\n  const adders = [];\n  for (let index = 0; index < count; index++) {\n    adders.push((value) => value + index + made);\n  }\n  return adders;\n}\n//# sourceURL=[module]\n//# sourceMappingURL=data:application/json;charset=utf-8;base64,eyJ2ZXJzaW9uIjozLCJzb3VyY2VzIjpbIndlYnBhY2s6Ly8vLi91dGlsLmpzPzk0MTUiXSwibmFtZXMiOltdLCJtYXBwaW5ncyI6IkFBQUE7QUFBQTtBQUFBO0FBQUE7QUFBQTtBQUFPO0FBQ1A7QUFDQTs7QUFFTzs7QUFFUDs7QUFFTztBQUNQO0FBQ0E7QUFDQTtBQUNBO0FBQ0E7QUFDQTtBQUNBO0FBQ0E7QUFDQTs7QUFFTztBQUNQO0FBQ0EscUJBQXFCLGVBQWU7QUFDcEM7QUFDQTtBQUNBO0FBQ0EiLCJmaWxlIjoiLi91dGlsLmpzLmpzIiwic291cmNlc0NvbnRlbnQiOlsiZXhwb3J0IGZ1bmN0aW9uIGdyZWV0KG5hbWUpIHtcbiAgcmV0dXJuIFwiaGVsbG8gXCIgKyBuYW1lO1xufVxuXG5leHBvcnQgY29uc3QgdG90YWwgPSAobGlzdCkgPT4gbGlzdC5yZWR1Y2UoKHN1bSwgdmFsdWUpID0+IHN1bSArIHZhbHVlLCAwKTtcblxubGV0IG1hZGUgPSAwO1xuXG5leHBvcnQgY2xhc3MgQ291bnRlciB7XG4gIGNvbnN0cnVjdG9yKHN0YXJ0KSB7XG4gICAgbWFkZSArPSAxO1xuICAgIHRoaXMudmFsdWUgPSBzdGFydDtcbiAgfVxuICBhZGQoc3RlcCkge1xuICAgIHRoaXMudmFsdWUgKz0gc3RlcDtcbiAgICByZXR1cm4gdGhpcztcbiAgfVxufVxuXG5leHBvcnQgZnVuY3Rpb24gbWFrZUFkZGVycyhjb3VudCkge1xuICBjb25zdCBhZGRlcnMgPSBbXTtcbiAgZm9yIChsZXQgaW5kZXggPSAwOyBpbmRleCA8IGNvdW50OyBpbmRleCsrKSB7XG4gICAgYWRkZXJzLnB1c2goKHZhbHVlKSA9PiB2YWx1ZSArIGluZGV4ICsgbWFkZSk7XG4gIH1cbiAgcmV0dXJuIGFkZGVycztcbn1cbiJdLCJzb3VyY2VSb290IjoiIn0=\n//# sourceURL=webpack-internal:///./util.js\n");

/***/ })

/******/ });