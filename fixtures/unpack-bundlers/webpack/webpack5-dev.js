/******/ (() => { // webpackBootstrap
/******/ 	var __webpack_modules__ = ({

/***/ "./app/cjs.js":
/*!********************!*\
  !*** ./app/cjs.js ***!
  \********************/
/***/ ((__unused_webpack_module, exports, __webpack_require__) => {

const path = __webpack_require__(/*! path */ "path");
console.log("eval cjs");
exports.join = function (a, b) {
  return path.posix.join(a, b);
};
exports.tag = "cjs-" + "object";


/***/ }),

/***/ "./app/cycle-a.js":
/*!************************!*\
  !*** ./app/cycle-a.js ***!
  \************************/
/***/ ((__unused_webpack_module, exports, __webpack_require__) => {

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
/***/ ((__unused_webpack_module, exports, __webpack_require__) => {

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
/***/ ((module) => {

"use strict";
module.exports = /*#__PURE__*/JSON.parse('{"name":"fixture","items":[1,2,3]}');

/***/ }),

/***/ "./app/greet.js":
/*!**********************!*\
  !*** ./app/greet.js ***!
  \**********************/
/***/ ((__unused_webpack_module, __webpack_exports__, __webpack_require__) => {

"use strict";
__webpack_require__.r(__webpack_exports__);
/* harmony export */ __webpack_require__.d(__webpack_exports__, {
/* harmony export */   "default": () => (/* binding */ greet),
/* harmony export */   shout: () => (/* binding */ shout)
/* harmony export */ });
/* harmony import */ var _say_js__WEBPACK_IMPORTED_MODULE_0__ = __webpack_require__(/*! ./say.js */ "./app/say.js");

(0,_say_js__WEBPACK_IMPORTED_MODULE_0__.say)("eval greet");
function greet(name) {
  return "hello " + name;
}
function shout(name) {
  return name.toUpperCase() + "!";
}


/***/ }),

/***/ "./app/lazy.js":
/*!*********************!*\
  !*** ./app/lazy.js ***!
  \*********************/
/***/ ((__unused_webpack_module, __webpack_exports__, __webpack_require__) => {

"use strict";
__webpack_require__.r(__webpack_exports__);
/* harmony export */ __webpack_require__.d(__webpack_exports__, {
/* harmony export */   "default": () => (__WEBPACK_DEFAULT_EXPORT__),
/* harmony export */   value: () => (/* binding */ value)
/* harmony export */ });
/* harmony import */ var _say_js__WEBPACK_IMPORTED_MODULE_0__ = __webpack_require__(/*! ./say.js */ "./app/say.js");

(0,_say_js__WEBPACK_IMPORTED_MODULE_0__.say)("eval lazy");
const value = 42;
/* harmony default export */ const __WEBPACK_DEFAULT_EXPORT__ = ("lazy-default");


/***/ }),

/***/ "./app/live.js":
/*!*********************!*\
  !*** ./app/live.js ***!
  \*********************/
/***/ ((__unused_webpack_module, __webpack_exports__, __webpack_require__) => {

"use strict";
__webpack_require__.r(__webpack_exports__);
/* harmony export */ __webpack_require__.d(__webpack_exports__, {
/* harmony export */   bump: () => (/* binding */ bump),
/* harmony export */   counter: () => (/* binding */ counter)
/* harmony export */ });
/* harmony import */ var _say_js__WEBPACK_IMPORTED_MODULE_0__ = __webpack_require__(/*! ./say.js */ "./app/say.js");

(0,_say_js__WEBPACK_IMPORTED_MODULE_0__.say)("eval live");
let counter = 0;
function bump() {
  counter += 1;
}


/***/ }),

/***/ "./app/math.js":
/*!*********************!*\
  !*** ./app/math.js ***!
  \*********************/
/***/ ((__unused_webpack_module, __webpack_exports__, __webpack_require__) => {

"use strict";
__webpack_require__.r(__webpack_exports__);
/* harmony export */ __webpack_require__.d(__webpack_exports__, {
/* harmony export */   add: () => (/* binding */ add),
/* harmony export */   mul: () => (/* binding */ mul),
/* harmony export */   triple: () => (/* binding */ triple)
/* harmony export */ });
/* harmony import */ var _say_js__WEBPACK_IMPORTED_MODULE_0__ = __webpack_require__(/*! ./say.js */ "./app/say.js");

(0,_say_js__WEBPACK_IMPORTED_MODULE_0__.say)("eval math");
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
/***/ ((__unused_webpack_module, __webpack_exports__, __webpack_require__) => {

"use strict";
__webpack_require__.r(__webpack_exports__);
/* harmony export */ __webpack_require__.d(__webpack_exports__, {
/* harmony export */   add: () => (/* reexport safe */ _math_js__WEBPACK_IMPORTED_MODULE_2__.add),
/* harmony export */   loud: () => (/* reexport safe */ _greet_js__WEBPACK_IMPORTED_MODULE_1__.shout),
/* harmony export */   mul: () => (/* reexport safe */ _math_js__WEBPACK_IMPORTED_MODULE_2__.mul),
/* harmony export */   triple: () => (/* reexport safe */ _math_js__WEBPACK_IMPORTED_MODULE_2__.triple)
/* harmony export */ });
/* harmony import */ var _say_js__WEBPACK_IMPORTED_MODULE_0__ = __webpack_require__(/*! ./say.js */ "./app/say.js");
/* harmony import */ var _greet_js__WEBPACK_IMPORTED_MODULE_1__ = __webpack_require__(/*! ./greet.js */ "./app/greet.js");
/* harmony import */ var _math_js__WEBPACK_IMPORTED_MODULE_2__ = __webpack_require__(/*! ./math.js */ "./app/math.js");

(0,_say_js__WEBPACK_IMPORTED_MODULE_0__.say)("eval reexport");




/***/ }),

/***/ "./app/say.js":
/*!********************!*\
  !*** ./app/say.js ***!
  \********************/
/***/ ((__unused_webpack_module, __webpack_exports__, __webpack_require__) => {

"use strict";
__webpack_require__.r(__webpack_exports__);
/* harmony export */ __webpack_require__.d(__webpack_exports__, {
/* harmony export */   say: () => (/* binding */ say)
/* harmony export */ });
function say(line) {
  console.log(line);
}
say("eval say");


/***/ }),

/***/ "path":
/*!***********************!*\
  !*** external "path" ***!
  \***********************/
/***/ ((module) => {

"use strict";
module.exports = require("path");

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
/******/ 	/* webpack/runtime/create fake namespace object */
/******/ 	(() => {
/******/ 		var getProto = Object.getPrototypeOf ? (obj) => (Object.getPrototypeOf(obj)) : (obj) => (obj.__proto__);
/******/ 		var leafPrototypes;
/******/ 		// create a fake namespace object
/******/ 		// mode & 1: value is a module id, require it
/******/ 		// mode & 2: merge all properties of value into the ns
/******/ 		// mode & 4: return value when already ns object
/******/ 		// mode & 16: return value when it's Promise-like
/******/ 		// mode & 8|1: behave like require
/******/ 		__webpack_require__.t = function(value, mode) {
/******/ 			if(mode & 1) value = this(value);
/******/ 			if(mode & 8) return value;
/******/ 			if(typeof value === 'object' && value) {
/******/ 				if((mode & 4) && value.__esModule) return value;
/******/ 				if((mode & 16) && typeof value.then === 'function') return value;
/******/ 			}
/******/ 			var ns = Object.create(null);
/******/ 			__webpack_require__.r(ns);
/******/ 			var def = {};
/******/ 			leafPrototypes = leafPrototypes || [null, getProto({}), getProto([]), getProto(getProto)];
/******/ 			for(var current = mode & 2 && value; typeof current == 'object' && !~leafPrototypes.indexOf(current); current = getProto(current)) {
/******/ 				Object.getOwnPropertyNames(current).forEach((key) => (def[key] = () => (value[key])));
/******/ 			}
/******/ 			def['default'] = () => (value);
/******/ 			__webpack_require__.d(ns, def);
/******/ 			return ns;
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
var __webpack_exports__ = {};
// This entry needs to be wrapped in an IIFE because it needs to be in strict mode.
(() => {
"use strict";
/*!**********************!*\
  !*** ./app/index.js ***!
  \**********************/
__webpack_require__.r(__webpack_exports__);
/* harmony import */ var _say_js__WEBPACK_IMPORTED_MODULE_0__ = __webpack_require__(/*! ./say.js */ "./app/say.js");
/* harmony import */ var _greet_js__WEBPACK_IMPORTED_MODULE_1__ = __webpack_require__(/*! ./greet.js */ "./app/greet.js");
/* harmony import */ var _math_js__WEBPACK_IMPORTED_MODULE_2__ = __webpack_require__(/*! ./math.js */ "./app/math.js");
/* harmony import */ var _live_js__WEBPACK_IMPORTED_MODULE_3__ = __webpack_require__(/*! ./live.js */ "./app/live.js");
/* harmony import */ var _reexport_js__WEBPACK_IMPORTED_MODULE_4__ = __webpack_require__(/*! ./reexport.js */ "./app/reexport.js");
/* harmony import */ var _data_json__WEBPACK_IMPORTED_MODULE_5__ = __webpack_require__(/*! ./data.json */ "./app/data.json");
/* harmony import */ var _cjs_js__WEBPACK_IMPORTED_MODULE_6__ = __webpack_require__(/*! ./cjs.js */ "./app/cjs.js");
/* harmony import */ var _cycle_a_js__WEBPACK_IMPORTED_MODULE_7__ = __webpack_require__(/*! ./cycle-a.js */ "./app/cycle-a.js");









(0,_say_js__WEBPACK_IMPORTED_MODULE_0__.say)("eval index");
(0,_say_js__WEBPACK_IMPORTED_MODULE_0__.say)("greet " + (0,_greet_js__WEBPACK_IMPORTED_MODULE_1__["default"])("ada") + " " + (0,_greet_js__WEBPACK_IMPORTED_MODULE_1__.shout)("bob"));
(0,_say_js__WEBPACK_IMPORTED_MODULE_0__.say)("math " + Object.keys(_math_js__WEBPACK_IMPORTED_MODULE_2__).sort().join(",") + " " + _math_js__WEBPACK_IMPORTED_MODULE_2__.add(2, 3));
(0,_say_js__WEBPACK_IMPORTED_MODULE_0__.say)("live " + _live_js__WEBPACK_IMPORTED_MODULE_3__.counter);
(0,_live_js__WEBPACK_IMPORTED_MODULE_3__.bump)();
(0,_say_js__WEBPACK_IMPORTED_MODULE_0__.say)("live " + _live_js__WEBPACK_IMPORTED_MODULE_3__.counter);
(0,_say_js__WEBPACK_IMPORTED_MODULE_0__.say)("reexport " + (0,_reexport_js__WEBPACK_IMPORTED_MODULE_4__.loud)("x") + " " + (0,_reexport_js__WEBPACK_IMPORTED_MODULE_4__.triple)(4));
(0,_say_js__WEBPACK_IMPORTED_MODULE_0__.say)("json " + _data_json__WEBPACK_IMPORTED_MODULE_5__.name + " " + _data_json__WEBPACK_IMPORTED_MODULE_5__.items.length);
(0,_say_js__WEBPACK_IMPORTED_MODULE_0__.say)("cjs " + _cjs_js__WEBPACK_IMPORTED_MODULE_6__.join("a", "b") + " " + _cjs_js__WEBPACK_IMPORTED_MODULE_6__.tag);
(0,_say_js__WEBPACK_IMPORTED_MODULE_0__.say)("cycle " + _cycle_a_js__WEBPACK_IMPORTED_MODULE_7__.report());
Promise.resolve(/*! import() */).then(__webpack_require__.t.bind(__webpack_require__, /*! ./data.json */ "./app/data.json", 19)).then((lazy) => (0,_say_js__WEBPACK_IMPORTED_MODULE_0__.say)("json lazy " + (lazy.default || lazy).name));
Promise.resolve(/*! import() */).then(__webpack_require__.bind(__webpack_require__, /*! ./lazy.js */ "./app/lazy.js")).then((lazy) => (0,_say_js__WEBPACK_IMPORTED_MODULE_0__.say)("lazy " + lazy.value + " " + lazy.default));

})();

/******/ })()
;