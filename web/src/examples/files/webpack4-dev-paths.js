/******/ (function(modules) { // webpackBootstrap
/******/ 	// The module cache
/******/ 	var installedModules = {};
/******/
/******/ 	// object to store loaded chunks
/******/ 	// "0" means "already loaded"
/******/ 	var installedChunks = {
/******/ 		"main": 0
/******/ 	};
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
/******/ 	// The chunk loading function for additional chunks
/******/ 	// Since all referenced chunks are already included
/******/ 	// in this file, this function is empty here.
/******/ 	__webpack_require__.e = function requireEnsure() {
/******/ 		return Promise.resolve();
/******/ 	};
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
/******/ 	// uncaught error handler for webpack runtime
/******/ 	__webpack_require__.oe = function(err) {
/******/ 		process.nextTick(function() {
/******/ 			throw err; // catch this error by using import().catch()
/******/ 		});
/******/ 	};
/******/
/******/
/******/ 	// Load entry module and return exports
/******/ 	return __webpack_require__(__webpack_require__.s = "./app/index.js");
/******/ })
/************************************************************************/
/******/ ({

/***/ "./app/cjs.js":
/*!********************!*\
  !*** ./app/cjs.js ***!
  \********************/
/*! no static exports found */
/***/ (function(module, exports, __webpack_require__) {

const path = __webpack_require__(/*! path */ "path");
console.log("eval cjs");
exports.join = function (a, b) {
  return path.posix.join(a, b);
};
exports.tag = "cjs-" + typeof module;


/***/ }),

/***/ "./app/cycle-a.js":
/*!************************!*\
  !*** ./app/cycle-a.js ***!
  \************************/
/*! no static exports found */
/***/ (function(module, exports, __webpack_require__) {

console.log("eval cycle-a start");
exports.early = "a-early";
const b = __webpack_require__(/*! ./cycle-b.js */ "./app/cycle-b.js");
exports.late = "a-late";
exports.report = function () {
  return b.seen + " " + b.check();
};
console.log("eval cycle-a end");


/***/ }),

/***/ "./app/cycle-b.js":
/*!************************!*\
  !*** ./app/cycle-b.js ***!
  \************************/
/*! no static exports found */
/***/ (function(module, exports, __webpack_require__) {

console.log("eval cycle-b");
const a = __webpack_require__(/*! ./cycle-a.js */ "./app/cycle-a.js");
exports.seen = "b saw " + a.early + "," + a.late;
exports.check = function () {
  return "b later " + a.late;
};


/***/ }),

/***/ "./app/data.json":
/*!***********************!*\
  !*** ./app/data.json ***!
  \***********************/
/*! exports provided: name, items, default */
/***/ (function(module) {

module.exports = JSON.parse("{\"name\":\"fixture\",\"items\":[1,2,3]}");

/***/ }),

/***/ "./app/greet.js":
/*!**********************!*\
  !*** ./app/greet.js ***!
  \**********************/
/*! exports provided: default, shout */
/***/ (function(module, __webpack_exports__, __webpack_require__) {

"use strict";
__webpack_require__.r(__webpack_exports__);
/* harmony export (binding) */ __webpack_require__.d(__webpack_exports__, "default", function() { return greet; });
/* harmony export (binding) */ __webpack_require__.d(__webpack_exports__, "shout", function() { return shout; });
/* harmony import */ var _say_js__WEBPACK_IMPORTED_MODULE_0__ = __webpack_require__(/*! ./say.js */ "./app/say.js");

Object(_say_js__WEBPACK_IMPORTED_MODULE_0__["say"])("eval greet");
function greet(name) {
  return "hello " + name;
}
function shout(name) {
  return name.toUpperCase() + "!";
}


/***/ }),

/***/ "./app/index.js":
/*!**********************!*\
  !*** ./app/index.js ***!
  \**********************/
/*! no exports provided */
/***/ (function(module, __webpack_exports__, __webpack_require__) {

"use strict";
__webpack_require__.r(__webpack_exports__);
/* harmony import */ var _say_js__WEBPACK_IMPORTED_MODULE_0__ = __webpack_require__(/*! ./say.js */ "./app/say.js");
/* harmony import */ var _greet_js__WEBPACK_IMPORTED_MODULE_1__ = __webpack_require__(/*! ./greet.js */ "./app/greet.js");
/* harmony import */ var _math_js__WEBPACK_IMPORTED_MODULE_2__ = __webpack_require__(/*! ./math.js */ "./app/math.js");
/* harmony import */ var _live_js__WEBPACK_IMPORTED_MODULE_3__ = __webpack_require__(/*! ./live.js */ "./app/live.js");
/* harmony import */ var _reexport_js__WEBPACK_IMPORTED_MODULE_4__ = __webpack_require__(/*! ./reexport.js */ "./app/reexport.js");
/* harmony import */ var _data_json__WEBPACK_IMPORTED_MODULE_5__ = __webpack_require__(/*! ./data.json */ "./app/data.json");
var _data_json__WEBPACK_IMPORTED_MODULE_5___namespace = /*#__PURE__*/__webpack_require__.t(/*! ./data.json */ "./app/data.json", 1);
/* harmony import */ var _cjs_js__WEBPACK_IMPORTED_MODULE_6__ = __webpack_require__(/*! ./cjs.js */ "./app/cjs.js");
/* harmony import */ var _cjs_js__WEBPACK_IMPORTED_MODULE_6___default = /*#__PURE__*/__webpack_require__.n(_cjs_js__WEBPACK_IMPORTED_MODULE_6__);
/* harmony import */ var _cycle_a_js__WEBPACK_IMPORTED_MODULE_7__ = __webpack_require__(/*! ./cycle-a.js */ "./app/cycle-a.js");
/* harmony import */ var _cycle_a_js__WEBPACK_IMPORTED_MODULE_7___default = /*#__PURE__*/__webpack_require__.n(_cycle_a_js__WEBPACK_IMPORTED_MODULE_7__);









Object(_say_js__WEBPACK_IMPORTED_MODULE_0__["say"])("eval index");
Object(_say_js__WEBPACK_IMPORTED_MODULE_0__["say"])("greet " + Object(_greet_js__WEBPACK_IMPORTED_MODULE_1__["default"])("ada") + " " + Object(_greet_js__WEBPACK_IMPORTED_MODULE_1__["shout"])("bob"));
Object(_say_js__WEBPACK_IMPORTED_MODULE_0__["say"])("math " + Object.keys(_math_js__WEBPACK_IMPORTED_MODULE_2__).sort().join(",") + " " + _math_js__WEBPACK_IMPORTED_MODULE_2__["add"](2, 3));
Object(_say_js__WEBPACK_IMPORTED_MODULE_0__["say"])("live " + _live_js__WEBPACK_IMPORTED_MODULE_3__["counter"]);
Object(_live_js__WEBPACK_IMPORTED_MODULE_3__["bump"])();
Object(_say_js__WEBPACK_IMPORTED_MODULE_0__["say"])("live " + _live_js__WEBPACK_IMPORTED_MODULE_3__["counter"]);
Object(_say_js__WEBPACK_IMPORTED_MODULE_0__["say"])("reexport " + Object(_reexport_js__WEBPACK_IMPORTED_MODULE_4__["loud"])("x") + " " + Object(_reexport_js__WEBPACK_IMPORTED_MODULE_4__["triple"])(4));
Object(_say_js__WEBPACK_IMPORTED_MODULE_0__["say"])("json " + _data_json__WEBPACK_IMPORTED_MODULE_5__.name + " " + _data_json__WEBPACK_IMPORTED_MODULE_5__.items.length);
Object(_say_js__WEBPACK_IMPORTED_MODULE_0__["say"])("cjs " + _cjs_js__WEBPACK_IMPORTED_MODULE_6___default.a.join("a", "b") + " " + _cjs_js__WEBPACK_IMPORTED_MODULE_6___default.a.tag);
Object(_say_js__WEBPACK_IMPORTED_MODULE_0__["say"])("cycle " + _cycle_a_js__WEBPACK_IMPORTED_MODULE_7___default.a.report());
Promise.resolve(/*! import() */).then(__webpack_require__.t.bind(null, /*! ./data.json */ "./app/data.json", 3)).then((lazy) => Object(_say_js__WEBPACK_IMPORTED_MODULE_0__["say"])("json lazy " + (lazy.default || lazy).name));
Promise.resolve(/*! import() */).then(__webpack_require__.bind(null, /*! ./lazy.js */ "./app/lazy.js")).then((lazy) => Object(_say_js__WEBPACK_IMPORTED_MODULE_0__["say"])("lazy " + lazy.value + " " + lazy.default));


/***/ }),

/***/ "./app/lazy.js":
/*!*********************!*\
  !*** ./app/lazy.js ***!
  \*********************/
/*! exports provided: value, default */
/***/ (function(module, __webpack_exports__, __webpack_require__) {

"use strict";
__webpack_require__.r(__webpack_exports__);
/* harmony export (binding) */ __webpack_require__.d(__webpack_exports__, "value", function() { return value; });
/* harmony import */ var _say_js__WEBPACK_IMPORTED_MODULE_0__ = __webpack_require__(/*! ./say.js */ "./app/say.js");

Object(_say_js__WEBPACK_IMPORTED_MODULE_0__["say"])("eval lazy");
const value = 42;
/* harmony default export */ __webpack_exports__["default"] = ("lazy-default");


/***/ }),

/***/ "./app/live.js":
/*!*********************!*\
  !*** ./app/live.js ***!
  \*********************/
/*! exports provided: counter, bump */
/***/ (function(module, __webpack_exports__, __webpack_require__) {

"use strict";
__webpack_require__.r(__webpack_exports__);
/* harmony export (binding) */ __webpack_require__.d(__webpack_exports__, "counter", function() { return counter; });
/* harmony export (binding) */ __webpack_require__.d(__webpack_exports__, "bump", function() { return bump; });
/* harmony import */ var _say_js__WEBPACK_IMPORTED_MODULE_0__ = __webpack_require__(/*! ./say.js */ "./app/say.js");

Object(_say_js__WEBPACK_IMPORTED_MODULE_0__["say"])("eval live");
let counter = 0;
function bump() {
  counter += 1;
}


/***/ }),

/***/ "./app/math.js":
/*!*********************!*\
  !*** ./app/math.js ***!
  \*********************/
/*! exports provided: add, mul, triple */
/***/ (function(module, __webpack_exports__, __webpack_require__) {

"use strict";
__webpack_require__.r(__webpack_exports__);
/* harmony export (binding) */ __webpack_require__.d(__webpack_exports__, "add", function() { return add; });
/* harmony export (binding) */ __webpack_require__.d(__webpack_exports__, "mul", function() { return mul; });
/* harmony export (binding) */ __webpack_require__.d(__webpack_exports__, "triple", function() { return triple; });
/* harmony import */ var _say_js__WEBPACK_IMPORTED_MODULE_0__ = __webpack_require__(/*! ./say.js */ "./app/say.js");

Object(_say_js__WEBPACK_IMPORTED_MODULE_0__["say"])("eval math");
const add = (a, b) => a + b;
const mul = (a, b) => a * b;
function triple(n) {
  return mul(n, 3);
}


/***/ }),

/***/ "./app/reexport.js":
/*!*************************!*\
  !*** ./app/reexport.js ***!
  \*************************/
/*! exports provided: loud, add, mul, triple */
/***/ (function(module, __webpack_exports__, __webpack_require__) {

"use strict";
__webpack_require__.r(__webpack_exports__);
/* harmony import */ var _say_js__WEBPACK_IMPORTED_MODULE_0__ = __webpack_require__(/*! ./say.js */ "./app/say.js");
/* harmony import */ var _greet_js__WEBPACK_IMPORTED_MODULE_1__ = __webpack_require__(/*! ./greet.js */ "./app/greet.js");
/* harmony reexport (safe) */ __webpack_require__.d(__webpack_exports__, "loud", function() { return _greet_js__WEBPACK_IMPORTED_MODULE_1__["shout"]; });

/* harmony import */ var _math_js__WEBPACK_IMPORTED_MODULE_2__ = __webpack_require__(/*! ./math.js */ "./app/math.js");
/* harmony reexport (safe) */ __webpack_require__.d(__webpack_exports__, "add", function() { return _math_js__WEBPACK_IMPORTED_MODULE_2__["add"]; });

/* harmony reexport (safe) */ __webpack_require__.d(__webpack_exports__, "mul", function() { return _math_js__WEBPACK_IMPORTED_MODULE_2__["mul"]; });

/* harmony reexport (safe) */ __webpack_require__.d(__webpack_exports__, "triple", function() { return _math_js__WEBPACK_IMPORTED_MODULE_2__["triple"]; });


Object(_say_js__WEBPACK_IMPORTED_MODULE_0__["say"])("eval reexport");




/***/ }),

/***/ "./app/say.js":
/*!********************!*\
  !*** ./app/say.js ***!
  \********************/
/*! exports provided: say */
/***/ (function(module, __webpack_exports__, __webpack_require__) {

"use strict";
__webpack_require__.r(__webpack_exports__);
/* harmony export (binding) */ __webpack_require__.d(__webpack_exports__, "say", function() { return say; });
function say(line) {
  console.log(line);
}
say("eval say");


/***/ }),

/***/ "path":
/*!***********************!*\
  !*** external "path" ***!
  \***********************/
/*! no static exports found */
/***/ (function(module, exports) {

module.exports = require("path");

/***/ })

/******/ });