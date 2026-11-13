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

eval("module.exports = JSON.parse(\"{\\\"name\\\":\\\"fixture\\\",\\\"items\\\":[\\\"a\\\",\\\"b\\\",\\\"c\\\"]}\");//# sourceURL=[module]\n//# sourceMappingURL=data:application/json;charset=utf-8;base64,eyJ2ZXJzaW9uIjozLCJmaWxlIjoiLi9kYXRhLmpzb24uanMiLCJzb3VyY2VzIjpbXSwibWFwcGluZ3MiOiIiLCJzb3VyY2VSb290IjoiIn0=\n//# sourceURL=webpack-internal:///./data.json\n");

/***/ }),

/***/ "./index.js":
/*!******************!*\
  !*** ./index.js ***!
  \******************/
/*! no exports provided */
/***/ (function(module, __webpack_exports__, __webpack_require__) {

"use strict";
eval("__webpack_require__.r(__webpack_exports__);\n/* harmony import */ var _util_js__WEBPACK_IMPORTED_MODULE_0__ = __webpack_require__(/*! ./util.js */ \"./util.js\");\n/* harmony import */ var _legacy_js__WEBPACK_IMPORTED_MODULE_1__ = __webpack_require__(/*! ./legacy.js */ \"./legacy.js\");\n/* harmony import */ var _legacy_js__WEBPACK_IMPORTED_MODULE_1___default = /*#__PURE__*/__webpack_require__.n(_legacy_js__WEBPACK_IMPORTED_MODULE_1__);\n/* harmony import */ var _strict_cjs_js__WEBPACK_IMPORTED_MODULE_2__ = __webpack_require__(/*! ./strict-cjs.js */ \"./strict-cjs.js\");\n/* harmony import */ var _strict_cjs_js__WEBPACK_IMPORTED_MODULE_2___default = /*#__PURE__*/__webpack_require__.n(_strict_cjs_js__WEBPACK_IMPORTED_MODULE_2__);\n/* harmony import */ var _data_json__WEBPACK_IMPORTED_MODULE_3__ = __webpack_require__(/*! ./data.json */ \"./data.json\");\nvar _data_json__WEBPACK_IMPORTED_MODULE_3___namespace = /*#__PURE__*/__webpack_require__.t(/*! ./data.json */ \"./data.json\", 1);\n/* harmony import */ var _shapes_js__WEBPACK_IMPORTED_MODULE_4__ = __webpack_require__(/*! ./shapes.js */ \"./shapes.js\");\n\n\n\n\n\n\nconsole.log(Object(_util_js__WEBPACK_IMPORTED_MODULE_0__[\"greet\"])(\"webpack\"), Object(_util_js__WEBPACK_IMPORTED_MODULE_0__[\"total\"])([1, 2, 3, 4]));\nconst counter = new _util_js__WEBPACK_IMPORTED_MODULE_0__[\"Counter\"](2);\ncounter.add(3);\nconsole.log(\"counter\", counter.value, _util_js__WEBPACK_IMPORTED_MODULE_0__[\"Counter\"].name);\nconsole.log(\"adders\", Object(_util_js__WEBPACK_IMPORTED_MODULE_0__[\"makeAdders\"])(3).map((add) => add(10)).join(\",\"));\nconsole.log(\"legacy\", _legacy_js__WEBPACK_IMPORTED_MODULE_1___default.a.bump(), _legacy_js__WEBPACK_IMPORTED_MODULE_1___default.a.bump(), _legacy_js__WEBPACK_IMPORTED_MODULE_1___default.a.selfIsExports, _legacy_js__WEBPACK_IMPORTED_MODULE_1___default.a.blockFn());\nconsole.log(\"strict cjs\", _strict_cjs_js__WEBPACK_IMPORTED_MODULE_2___default.a.mode(), _strict_cjs_js__WEBPACK_IMPORTED_MODULE_2___default.a.label);\nconsole.log(\"data\", _data_json__WEBPACK_IMPORTED_MODULE_3__.name, _data_json__WEBPACK_IMPORTED_MODULE_3__.items.length);\nconsole.log(\"shapes\", Object.keys(_shapes_js__WEBPACK_IMPORTED_MODULE_4__).sort().join(\",\"), _shapes_js__WEBPACK_IMPORTED_MODULE_4__[\"area\"](_shapes_js__WEBPACK_IMPORTED_MODULE_4__[\"unit\"]));\n//# sourceURL=[module]\n//# sourceMappingURL=data:application/json;charset=utf-8;base64,eyJ2ZXJzaW9uIjozLCJmaWxlIjoiLi9pbmRleC5qcy5qcyIsInNvdXJjZXMiOlsid2VicGFjazovLy8uL2luZGV4LmpzPzQxZjUiXSwic291cmNlc0NvbnRlbnQiOlsiaW1wb3J0IHsgZ3JlZXQsIHRvdGFsLCBDb3VudGVyLCBtYWtlQWRkZXJzIH0gZnJvbSBcIi4vdXRpbC5qc1wiO1xuaW1wb3J0IGxlZ2FjeSBmcm9tIFwiLi9sZWdhY3kuanNcIjtcbmltcG9ydCBzdHJpY3RDanMgZnJvbSBcIi4vc3RyaWN0LWNqcy5qc1wiO1xuaW1wb3J0IGRhdGEgZnJvbSBcIi4vZGF0YS5qc29uXCI7XG5pbXBvcnQgKiBhcyBzaGFwZXMgZnJvbSBcIi4vc2hhcGVzLmpzXCI7XG5cbmNvbnNvbGUubG9nKGdyZWV0KFwid2VicGFja1wiKSwgdG90YWwoWzEsIDIsIDMsIDRdKSk7XG5jb25zdCBjb3VudGVyID0gbmV3IENvdW50ZXIoMik7XG5jb3VudGVyLmFkZCgzKTtcbmNvbnNvbGUubG9nKFwiY291bnRlclwiLCBjb3VudGVyLnZhbHVlLCBDb3VudGVyLm5hbWUpO1xuY29uc29sZS5sb2coXCJhZGRlcnNcIiwgbWFrZUFkZGVycygzKS5tYXAoKGFkZCkgPT4gYWRkKDEwKSkuam9pbihcIixcIikpO1xuY29uc29sZS5sb2coXCJsZWdhY3lcIiwgbGVnYWN5LmJ1bXAoKSwgbGVnYWN5LmJ1bXAoKSwgbGVnYWN5LnNlbGZJc0V4cG9ydHMsIGxlZ2FjeS5ibG9ja0ZuKCkpO1xuY29uc29sZS5sb2coXCJzdHJpY3QgY2pzXCIsIHN0cmljdENqcy5tb2RlKCksIHN0cmljdENqcy5sYWJlbCk7XG5jb25zb2xlLmxvZyhcImRhdGFcIiwgZGF0YS5uYW1lLCBkYXRhLml0ZW1zLmxlbmd0aCk7XG5jb25zb2xlLmxvZyhcInNoYXBlc1wiLCBPYmplY3Qua2V5cyhzaGFwZXMpLnNvcnQoKS5qb2luKFwiLFwiKSwgc2hhcGVzLmFyZWEoc2hhcGVzLnVuaXQpKTtcbiJdLCJtYXBwaW5ncyI6IkFBQUE7QUFBQTtBQUFBO0FBQUE7QUFBQTtBQUFBO0FBQUE7QUFBQTtBQUFBO0FBQUE7QUFDQTtBQUNBO0FBQ0E7QUFDQTtBQUNBO0FBQ0E7QUFDQTtBQUNBO0FBQ0E7QUFDQTtBQUNBO0FBQ0E7QUFDQTtBQUNBOyIsInNvdXJjZVJvb3QiOiIifQ==\n//# sourceURL=webpack-internal:///./index.js\n");

/***/ }),

/***/ "./legacy.js":
/*!*******************!*\
  !*** ./legacy.js ***!
  \*******************/
/*! no static exports found */
/***/ (function(module, exports) {

eval("var count = 0;\nfunction bump() {\n  count += 1;\n  return count;\n}\nvar selfIsExports = this === module.exports;\nif (count === 0) {\n  function blockFn() {\n    return typeof bump;\n  }\n}\nmodule.exports = { bump: bump, selfIsExports: selfIsExports, blockFn: blockFn };\n//# sourceURL=[module]\n//# sourceMappingURL=data:application/json;charset=utf-8;base64,eyJ2ZXJzaW9uIjozLCJmaWxlIjoiLi9sZWdhY3kuanMuanMiLCJzb3VyY2VzIjpbIndlYnBhY2s6Ly8vLi9sZWdhY3kuanM/YjM4ZiJdLCJzb3VyY2VzQ29udGVudCI6WyJ2YXIgY291bnQgPSAwO1xuZnVuY3Rpb24gYnVtcCgpIHtcbiAgY291bnQgKz0gMTtcbiAgcmV0dXJuIGNvdW50O1xufVxudmFyIHNlbGZJc0V4cG9ydHMgPSB0aGlzID09PSBtb2R1bGUuZXhwb3J0cztcbmlmIChjb3VudCA9PT0gMCkge1xuICBmdW5jdGlvbiBibG9ja0ZuKCkge1xuICAgIHJldHVybiB0eXBlb2YgYnVtcDtcbiAgfVxufVxubW9kdWxlLmV4cG9ydHMgPSB7IGJ1bXA6IGJ1bXAsIHNlbGZJc0V4cG9ydHM6IHNlbGZJc0V4cG9ydHMsIGJsb2NrRm46IGJsb2NrRm4gfTtcbiJdLCJtYXBwaW5ncyI6IkFBQUE7QUFDQTtBQUNBO0FBQ0E7QUFDQTtBQUNBO0FBQ0E7QUFDQTtBQUNBO0FBQ0E7QUFDQTtBQUNBOyIsInNvdXJjZVJvb3QiOiIifQ==\n//# sourceURL=webpack-internal:///./legacy.js\n");

/***/ }),

/***/ "./shapes.js":
/*!*******************!*\
  !*** ./shapes.js ***!
  \*******************/
/*! exports provided: unit, area, default */
/***/ (function(module, __webpack_exports__, __webpack_require__) {

"use strict";
eval("__webpack_require__.r(__webpack_exports__);\n/* harmony export (binding) */ __webpack_require__.d(__webpack_exports__, \"unit\", function() { return unit; });\n/* harmony export (binding) */ __webpack_require__.d(__webpack_exports__, \"area\", function() { return area; });\nconst unit = { width: 2, height: 3 };\nfunction area(shape) {\n  return shape.width * shape.height;\n}\n/* harmony default export */ __webpack_exports__[\"default\"] = (\"shapes\");\n//# sourceURL=[module]\n//# sourceMappingURL=data:application/json;charset=utf-8;base64,eyJ2ZXJzaW9uIjozLCJmaWxlIjoiLi9zaGFwZXMuanMuanMiLCJzb3VyY2VzIjpbIndlYnBhY2s6Ly8vLi9zaGFwZXMuanM/MzBlMyJdLCJzb3VyY2VzQ29udGVudCI6WyJleHBvcnQgY29uc3QgdW5pdCA9IHsgd2lkdGg6IDIsIGhlaWdodDogMyB9O1xuZXhwb3J0IGZ1bmN0aW9uIGFyZWEoc2hhcGUpIHtcbiAgcmV0dXJuIHNoYXBlLndpZHRoICogc2hhcGUuaGVpZ2h0O1xufVxuZXhwb3J0IGRlZmF1bHQgXCJzaGFwZXNcIjtcbiJdLCJtYXBwaW5ncyI6IkFBQUE7QUFBQTtBQUFBO0FBQUE7QUFDQTtBQUNBO0FBQ0E7QUFDQTsiLCJzb3VyY2VSb290IjoiIn0=\n//# sourceURL=webpack-internal:///./shapes.js\n");

/***/ }),

/***/ "./strict-cjs.js":
/*!***********************!*\
  !*** ./strict-cjs.js ***!
  \***********************/
/*! no static exports found */
/***/ (function(module, exports, __webpack_require__) {

"use strict";
eval("\nvar label = \"strict\";\nfunction mode() {\n  return typeof this === \"undefined\" ? \"strict\" : \"sloppy\";\n}\nexports.mode = mode;\nexports.label = label;\n//# sourceURL=[module]\n//# sourceMappingURL=data:application/json;charset=utf-8;base64,eyJ2ZXJzaW9uIjozLCJmaWxlIjoiLi9zdHJpY3QtY2pzLmpzLmpzIiwic291cmNlcyI6WyJ3ZWJwYWNrOi8vLy4vc3RyaWN0LWNqcy5qcz84NDUzIl0sInNvdXJjZXNDb250ZW50IjpbIlwidXNlIHN0cmljdFwiO1xudmFyIGxhYmVsID0gXCJzdHJpY3RcIjtcbmZ1bmN0aW9uIG1vZGUoKSB7XG4gIHJldHVybiB0eXBlb2YgdGhpcyA9PT0gXCJ1bmRlZmluZWRcIiA/IFwic3RyaWN0XCIgOiBcInNsb3BweVwiO1xufVxuZXhwb3J0cy5tb2RlID0gbW9kZTtcbmV4cG9ydHMubGFiZWwgPSBsYWJlbDtcbiJdLCJtYXBwaW5ncyI6IkFBQUE7QUFDQTtBQUNBO0FBQ0E7QUFDQTtBQUNBO0FBQ0E7Iiwic291cmNlUm9vdCI6IiJ9\n//# sourceURL=webpack-internal:///./strict-cjs.js\n");

/***/ }),

/***/ "./util.js":
/*!*****************!*\
  !*** ./util.js ***!
  \*****************/
/*! exports provided: greet, total, Counter, makeAdders */
/***/ (function(module, __webpack_exports__, __webpack_require__) {

"use strict";
eval("__webpack_require__.r(__webpack_exports__);\n/* harmony export (binding) */ __webpack_require__.d(__webpack_exports__, \"greet\", function() { return greet; });\n/* harmony export (binding) */ __webpack_require__.d(__webpack_exports__, \"total\", function() { return total; });\n/* harmony export (binding) */ __webpack_require__.d(__webpack_exports__, \"Counter\", function() { return Counter; });\n/* harmony export (binding) */ __webpack_require__.d(__webpack_exports__, \"makeAdders\", function() { return makeAdders; });\nfunction greet(name) {\n  return \"hello \" + name;\n}\n\nconst total = (list) => list.reduce((sum, value) => sum + value, 0);\n\nlet made = 0;\n\nclass Counter {\n  constructor(start) {\n    made += 1;\n    this.value = start;\n  }\n  add(step) {\n    this.value += step;\n    return this;\n  }\n}\n\nfunction makeAdders(count) {\n  const adders = [];\n  for (let index = 0; index < count; index++) {\n    adders.push((value) => value + index + made);\n  }\n  return adders;\n}\n//# sourceURL=[module]\n//# sourceMappingURL=data:application/json;charset=utf-8;base64,eyJ2ZXJzaW9uIjozLCJmaWxlIjoiLi91dGlsLmpzLmpzIiwic291cmNlcyI6WyJ3ZWJwYWNrOi8vLy4vdXRpbC5qcz85NDE1Il0sInNvdXJjZXNDb250ZW50IjpbImV4cG9ydCBmdW5jdGlvbiBncmVldChuYW1lKSB7XG4gIHJldHVybiBcImhlbGxvIFwiICsgbmFtZTtcbn1cblxuZXhwb3J0IGNvbnN0IHRvdGFsID0gKGxpc3QpID0+IGxpc3QucmVkdWNlKChzdW0sIHZhbHVlKSA9PiBzdW0gKyB2YWx1ZSwgMCk7XG5cbmxldCBtYWRlID0gMDtcblxuZXhwb3J0IGNsYXNzIENvdW50ZXIge1xuICBjb25zdHJ1Y3RvcihzdGFydCkge1xuICAgIG1hZGUgKz0gMTtcbiAgICB0aGlzLnZhbHVlID0gc3RhcnQ7XG4gIH1cbiAgYWRkKHN0ZXApIHtcbiAgICB0aGlzLnZhbHVlICs9IHN0ZXA7XG4gICAgcmV0dXJuIHRoaXM7XG4gIH1cbn1cblxuZXhwb3J0IGZ1bmN0aW9uIG1ha2VBZGRlcnMoY291bnQpIHtcbiAgY29uc3QgYWRkZXJzID0gW107XG4gIGZvciAobGV0IGluZGV4ID0gMDsgaW5kZXggPCBjb3VudDsgaW5kZXgrKykge1xuICAgIGFkZGVycy5wdXNoKCh2YWx1ZSkgPT4gdmFsdWUgKyBpbmRleCArIG1hZGUpO1xuICB9XG4gIHJldHVybiBhZGRlcnM7XG59XG4iXSwibWFwcGluZ3MiOiJBQUFBO0FBQUE7QUFBQTtBQUFBO0FBQUE7QUFBQTtBQUNBO0FBQ0E7QUFDQTtBQUNBO0FBQ0E7QUFDQTtBQUNBO0FBQ0E7QUFDQTtBQUNBO0FBQ0E7QUFDQTtBQUNBO0FBQ0E7QUFDQTtBQUNBO0FBQ0E7QUFDQTtBQUNBO0FBQ0E7QUFDQTtBQUNBO0FBQ0E7QUFDQTtBQUNBOyIsInNvdXJjZVJvb3QiOiIifQ==\n//# sourceURL=webpack-internal:///./util.js\n");

/***/ })

/******/ });