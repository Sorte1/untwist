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

eval("module.exports = JSON.parse(\"{\\\"name\\\":\\\"fixture\\\",\\\"items\\\":[\\\"a\\\",\\\"b\\\",\\\"c\\\"]}\");\n\n//# sourceURL=webpack:///./data.json?");

/***/ }),

/***/ "./index.js":
/*!******************!*\
  !*** ./index.js ***!
  \******************/
/*! no exports provided */
/***/ (function(module, __webpack_exports__, __webpack_require__) {

"use strict";
eval("__webpack_require__.r(__webpack_exports__);\n/* harmony import */ var _util_js__WEBPACK_IMPORTED_MODULE_0__ = __webpack_require__(/*! ./util.js */ \"./util.js\");\n/* harmony import */ var _legacy_js__WEBPACK_IMPORTED_MODULE_1__ = __webpack_require__(/*! ./legacy.js */ \"./legacy.js\");\n/* harmony import */ var _legacy_js__WEBPACK_IMPORTED_MODULE_1___default = /*#__PURE__*/__webpack_require__.n(_legacy_js__WEBPACK_IMPORTED_MODULE_1__);\n/* harmony import */ var _strict_cjs_js__WEBPACK_IMPORTED_MODULE_2__ = __webpack_require__(/*! ./strict-cjs.js */ \"./strict-cjs.js\");\n/* harmony import */ var _strict_cjs_js__WEBPACK_IMPORTED_MODULE_2___default = /*#__PURE__*/__webpack_require__.n(_strict_cjs_js__WEBPACK_IMPORTED_MODULE_2__);\n/* harmony import */ var _data_json__WEBPACK_IMPORTED_MODULE_3__ = __webpack_require__(/*! ./data.json */ \"./data.json\");\nvar _data_json__WEBPACK_IMPORTED_MODULE_3___namespace = /*#__PURE__*/__webpack_require__.t(/*! ./data.json */ \"./data.json\", 1);\n/* harmony import */ var _shapes_js__WEBPACK_IMPORTED_MODULE_4__ = __webpack_require__(/*! ./shapes.js */ \"./shapes.js\");\n\n\n\n\n\n\nconsole.log(Object(_util_js__WEBPACK_IMPORTED_MODULE_0__[\"greet\"])(\"webpack\"), Object(_util_js__WEBPACK_IMPORTED_MODULE_0__[\"total\"])([1, 2, 3, 4]));\nconst counter = new _util_js__WEBPACK_IMPORTED_MODULE_0__[\"Counter\"](2);\ncounter.add(3);\nconsole.log(\"counter\", counter.value, _util_js__WEBPACK_IMPORTED_MODULE_0__[\"Counter\"].name);\nconsole.log(\"adders\", Object(_util_js__WEBPACK_IMPORTED_MODULE_0__[\"makeAdders\"])(3).map((add) => add(10)).join(\",\"));\nconsole.log(\"legacy\", _legacy_js__WEBPACK_IMPORTED_MODULE_1___default.a.bump(), _legacy_js__WEBPACK_IMPORTED_MODULE_1___default.a.bump(), _legacy_js__WEBPACK_IMPORTED_MODULE_1___default.a.selfIsExports, _legacy_js__WEBPACK_IMPORTED_MODULE_1___default.a.blockFn());\nconsole.log(\"strict cjs\", _strict_cjs_js__WEBPACK_IMPORTED_MODULE_2___default.a.mode(), _strict_cjs_js__WEBPACK_IMPORTED_MODULE_2___default.a.label);\nconsole.log(\"data\", _data_json__WEBPACK_IMPORTED_MODULE_3__.name, _data_json__WEBPACK_IMPORTED_MODULE_3__.items.length);\nconsole.log(\"shapes\", Object.keys(_shapes_js__WEBPACK_IMPORTED_MODULE_4__).sort().join(\",\"), _shapes_js__WEBPACK_IMPORTED_MODULE_4__[\"area\"](_shapes_js__WEBPACK_IMPORTED_MODULE_4__[\"unit\"]));\n\n\n//# sourceURL=webpack:///./index.js?");

/***/ }),

/***/ "./legacy.js":
/*!*******************!*\
  !*** ./legacy.js ***!
  \*******************/
/*! no static exports found */
/***/ (function(module, exports) {

eval("var count = 0;\nfunction bump() {\n  count += 1;\n  return count;\n}\nvar selfIsExports = this === module.exports;\nif (count === 0) {\n  function blockFn() {\n    return typeof bump;\n  }\n}\nmodule.exports = { bump: bump, selfIsExports: selfIsExports, blockFn: blockFn };\n\n\n//# sourceURL=webpack:///./legacy.js?");

/***/ }),

/***/ "./shapes.js":
/*!*******************!*\
  !*** ./shapes.js ***!
  \*******************/
/*! exports provided: unit, area, default */
/***/ (function(module, __webpack_exports__, __webpack_require__) {

"use strict";
eval("__webpack_require__.r(__webpack_exports__);\n/* harmony export (binding) */ __webpack_require__.d(__webpack_exports__, \"unit\", function() { return unit; });\n/* harmony export (binding) */ __webpack_require__.d(__webpack_exports__, \"area\", function() { return area; });\nconst unit = { width: 2, height: 3 };\nfunction area(shape) {\n  return shape.width * shape.height;\n}\n/* harmony default export */ __webpack_exports__[\"default\"] = (\"shapes\");\n\n\n//# sourceURL=webpack:///./shapes.js?");

/***/ }),

/***/ "./strict-cjs.js":
/*!***********************!*\
  !*** ./strict-cjs.js ***!
  \***********************/
/*! no static exports found */
/***/ (function(module, exports, __webpack_require__) {

"use strict";
eval("\nvar label = \"strict\";\nfunction mode() {\n  return typeof this === \"undefined\" ? \"strict\" : \"sloppy\";\n}\nexports.mode = mode;\nexports.label = label;\n\n\n//# sourceURL=webpack:///./strict-cjs.js?");

/***/ }),

/***/ "./util.js":
/*!*****************!*\
  !*** ./util.js ***!
  \*****************/
/*! exports provided: greet, total, Counter, makeAdders */
/***/ (function(module, __webpack_exports__, __webpack_require__) {

"use strict";
eval("__webpack_require__.r(__webpack_exports__);\n/* harmony export (binding) */ __webpack_require__.d(__webpack_exports__, \"greet\", function() { return greet; });\n/* harmony export (binding) */ __webpack_require__.d(__webpack_exports__, \"total\", function() { return total; });\n/* harmony export (binding) */ __webpack_require__.d(__webpack_exports__, \"Counter\", function() { return Counter; });\n/* harmony export (binding) */ __webpack_require__.d(__webpack_exports__, \"makeAdders\", function() { return makeAdders; });\nfunction greet(name) {\n  return \"hello \" + name;\n}\n\nconst total = (list) => list.reduce((sum, value) => sum + value, 0);\n\nlet made = 0;\n\nclass Counter {\n  constructor(start) {\n    made += 1;\n    this.value = start;\n  }\n  add(step) {\n    this.value += step;\n    return this;\n  }\n}\n\nfunction makeAdders(count) {\n  const adders = [];\n  for (let index = 0; index < count; index++) {\n    adders.push((value) => value + index + made);\n  }\n  return adders;\n}\n\n\n//# sourceURL=webpack:///./util.js?");

/***/ })

/******/ });