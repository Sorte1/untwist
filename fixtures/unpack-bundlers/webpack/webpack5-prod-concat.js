/******/ (() => { // webpackBootstrap
/******/ 	var __webpack_modules__ = ({

/***/ 3:
/***/ ((module) => {

"use strict";
module.exports = require("path");

/***/ }),

/***/ 106:
/***/ ((module) => {

"use strict";
module.exports = /*#__PURE__*/JSON.parse('{"name":"fixture","items":[1,2,3]}');

/***/ }),

/***/ 444:
/***/ ((__unused_webpack_module, exports, __webpack_require__) => {

console.log("eval cycle-b");
const a = __webpack_require__(501);
exports.seen = "b saw " + a.early + "," + a.late;
exports.check = function () {
  return "b later " + a.late;
};


/***/ }),

/***/ 451:
/***/ ((__unused_webpack_module, exports, __webpack_require__) => {

const path = __webpack_require__(3);
console.log("eval cjs");
exports.join = function (a, b) {
  return path.posix.join(a, b);
};
exports.tag = "cjs-" + "object";


/***/ }),

/***/ 501:
/***/ ((__unused_webpack_module, exports, __webpack_require__) => {

console.log("eval cycle-a start");
exports.early = "a-early";
const b = __webpack_require__(444);
exports.late = "a-late";
exports.report = function () {
  return b.seen + " " + b.check();
};
console.log("eval cycle-a end");


/***/ }),

/***/ 683:
/***/ ((__unused_webpack_module, __webpack_exports__, __webpack_require__) => {

"use strict";
__webpack_require__.r(__webpack_exports__);
/* harmony export */ __webpack_require__.d(__webpack_exports__, {
/* harmony export */   "default": () => (__WEBPACK_DEFAULT_EXPORT__),
/* harmony export */   value: () => (/* binding */ value)
/* harmony export */ });
/* harmony import */ var _say_js__WEBPACK_IMPORTED_MODULE_0__ = __webpack_require__(802);

(0,_say_js__WEBPACK_IMPORTED_MODULE_0__/* .say */ .c)("eval lazy");
const value = 42;
/* harmony default export */ const __WEBPACK_DEFAULT_EXPORT__ = ("lazy-default");


/***/ }),

/***/ 802:
/***/ ((__unused_webpack_module, __webpack_exports__, __webpack_require__) => {

"use strict";
/* harmony export */ __webpack_require__.d(__webpack_exports__, {
/* harmony export */   c: () => (/* binding */ say)
/* harmony export */ });
function say(line) {
  console.log(line);
}
say("eval say");


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

// NAMESPACE OBJECT: ./app/math.js
var math_namespaceObject = {};
__webpack_require__.r(math_namespaceObject);
__webpack_require__.d(math_namespaceObject, {
  add: () => (add),
  mul: () => (mul),
  triple: () => (triple)
});

// EXTERNAL MODULE: ./app/say.js
var say = __webpack_require__(802);
;// ./app/greet.js

(0,say/* say */.c)("eval greet");
function greet(name) {
  return "hello " + name;
}
function shout(name) {
  return name.toUpperCase() + "!";
}

;// ./app/math.js

(0,say/* say */.c)("eval math");
const add = (a, b) => a + b;
const mul = (a, b) => a * b;
function triple(n) {
  return mul(n, 3);
}

;// ./app/live.js

(0,say/* say */.c)("eval live");
let counter = 0;
function bump() {
  counter += 1;
}

;// ./app/reexport.js

(0,say/* say */.c)("eval reexport");



// EXTERNAL MODULE: ./app/data.json
var data = __webpack_require__(106);
// EXTERNAL MODULE: ./app/cjs.js
var cjs = __webpack_require__(451);
// EXTERNAL MODULE: ./app/cycle-a.js
var cycle_a = __webpack_require__(501);
;// ./app/index.js









(0,say/* say */.c)("eval index");
(0,say/* say */.c)("greet " + greet("ada") + " " + shout("bob"));
(0,say/* say */.c)("math " + Object.keys(math_namespaceObject).sort().join(",") + " " + add(2, 3));
(0,say/* say */.c)("live " + counter);
bump();
(0,say/* say */.c)("live " + counter);
(0,say/* say */.c)("reexport " + shout("x") + " " + triple(4));
(0,say/* say */.c)("json " + data.name + " " + data.items.length);
(0,say/* say */.c)("cjs " + cjs.join("a", "b") + " " + cjs.tag);
(0,say/* say */.c)("cycle " + cycle_a.report());
Promise.resolve(/* import() */).then(__webpack_require__.t.bind(__webpack_require__, 106, 19)).then((lazy) => (0,say/* say */.c)("json lazy " + (lazy.default || lazy).name));
Promise.resolve(/* import() */).then(__webpack_require__.bind(__webpack_require__, 683)).then((lazy) => (0,say/* say */.c)("lazy " + lazy.value + " " + lazy.default));

})();

/******/ })()
;