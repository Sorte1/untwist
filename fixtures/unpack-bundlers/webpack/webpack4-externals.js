/******/ (function(modules) { // webpackBootstrap
/******/ 	// The module cache
/******/ 	var installedModules = {};
/******/
/******/ 	// object to store loaded chunks
/******/ 	// "0" means "already loaded"
/******/ 	var installedChunks = {
/******/ 		0: 0
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
/******/ 	return __webpack_require__(__webpack_require__.s = 6);
/******/ })
/************************************************************************/
/******/ ([
/* 0 */
/***/ (function(module, __webpack_exports__, __webpack_require__) {

"use strict";
/* harmony export (binding) */ __webpack_require__.d(__webpack_exports__, "a", function() { return say; });
function say(line) {
  console.log(line);
}
say("eval say");


/***/ }),
/* 1 */
/***/ (function(module, exports, __webpack_require__) {

console.log("eval cycle-a start");
exports.early = "a-early";
const b = __webpack_require__(5);
exports.late = "a-late";
exports.report = function () {
  return b.seen + " " + b.check();
};
console.log("eval cycle-a end");


/***/ }),
/* 2 */
/***/ (function(module) {

module.exports = JSON.parse("{\"name\":\"fixture\",\"items\":[1,2,3]}");

/***/ }),
/* 3 */
/***/ (function(module, exports, __webpack_require__) {

const path = __webpack_require__(4);
console.log("eval cjs");
exports.join = function (a, b) {
  return path.posix.join(a, b);
};
exports.tag = "cjs-" + typeof module;


/***/ }),
/* 4 */
/***/ (function(module, exports) {

module.exports = require("path");

/***/ }),
/* 5 */
/***/ (function(module, exports, __webpack_require__) {

console.log("eval cycle-b");
const a = __webpack_require__(1);
exports.seen = "b saw " + a.early + "," + a.late;
exports.check = function () {
  return "b later " + a.late;
};


/***/ }),
/* 6 */
/***/ (function(module, __webpack_exports__, __webpack_require__) {

"use strict";
// ESM COMPAT FLAG
__webpack_require__.r(__webpack_exports__);

// NAMESPACE OBJECT: ./app/math.js
var math_namespaceObject = {};
__webpack_require__.r(math_namespaceObject);
__webpack_require__.d(math_namespaceObject, "add", function() { return add; });
__webpack_require__.d(math_namespaceObject, "mul", function() { return mul; });
__webpack_require__.d(math_namespaceObject, "triple", function() { return triple; });

// EXTERNAL MODULE: ./app/say.js
var say = __webpack_require__(0);

// CONCATENATED MODULE: ./app/greet.js

Object(say["a" /* say */])("eval greet");
function greet(name) {
  return "hello " + name;
}
function shout(name) {
  return name.toUpperCase() + "!";
}

// CONCATENATED MODULE: ./app/math.js

Object(say["a" /* say */])("eval math");
const add = (a, b) => a + b;
const mul = (a, b) => a * b;
function triple(n) {
  return mul(n, 3);
}

// CONCATENATED MODULE: ./app/live.js

Object(say["a" /* say */])("eval live");
let counter = 0;
function bump() {
  counter += 1;
}

// CONCATENATED MODULE: ./app/reexport.js

Object(say["a" /* say */])("eval reexport");



// EXTERNAL MODULE: ./app/data.json
var data = __webpack_require__(2);

// EXTERNAL MODULE: ./app/cjs.js
var cjs = __webpack_require__(3);
var cjs_default = /*#__PURE__*/__webpack_require__.n(cjs);

// EXTERNAL MODULE: ./app/cycle-a.js
var cycle_a = __webpack_require__(1);
var cycle_a_default = /*#__PURE__*/__webpack_require__.n(cycle_a);

// CONCATENATED MODULE: ./app/index.js









Object(say["a" /* say */])("eval index");
Object(say["a" /* say */])("greet " + greet("ada") + " " + shout("bob"));
Object(say["a" /* say */])("math " + Object.keys(math_namespaceObject).sort().join(",") + " " + add(2, 3));
Object(say["a" /* say */])("live " + counter);
bump();
Object(say["a" /* say */])("live " + counter);
Object(say["a" /* say */])("reexport " + shout("x") + " " + triple(4));
Object(say["a" /* say */])("json " + data.name + " " + data.items.length);
Object(say["a" /* say */])("cjs " + cjs_default.a.join("a", "b") + " " + cjs_default.a.tag);
Object(say["a" /* say */])("cycle " + cycle_a_default.a.report());
Promise.resolve(/* import() */).then(__webpack_require__.t.bind(null, 2, 3)).then((lazy) => Object(say["a" /* say */])("json lazy " + (lazy.default || lazy).name));
Promise.resolve(/* import() */).then(__webpack_require__.bind(null, 7)).then((lazy) => Object(say["a" /* say */])("lazy " + lazy.value + " " + lazy.default));


/***/ }),
/* 7 */
/***/ (function(module, __webpack_exports__, __webpack_require__) {

"use strict";
__webpack_require__.r(__webpack_exports__);
/* harmony export (binding) */ __webpack_require__.d(__webpack_exports__, "value", function() { return value; });
/* harmony import */ var _say_js__WEBPACK_IMPORTED_MODULE_0__ = __webpack_require__(0);

Object(_say_js__WEBPACK_IMPORTED_MODULE_0__[/* say */ "a"])("eval lazy");
const value = 42;
/* harmony default export */ __webpack_exports__["default"] = ("lazy-default");


/***/ })
/******/ ]);