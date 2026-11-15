/******/ (() => { // webpackBootstrap
/******/ 	"use strict";
/******/ 	var __webpack_modules__ = ([
/* 0 */,
/* 1 */
/***/ ((__unused_webpack___webpack_module__, __webpack_exports__, __webpack_require__) => {

__webpack_require__.r(__webpack_exports__);
/* harmony export */ __webpack_require__.d(__webpack_exports__, {
/* harmony export */   "default": () => (/* binding */ greet)
/* harmony export */ });
function greet(name) {
  return `Hello, ${name}!`;
}


/***/ }),
/* 2 */
/***/ ((__unused_webpack___webpack_module__, __webpack_exports__, __webpack_require__) => {

__webpack_require__.r(__webpack_exports__);
/* harmony export */ __webpack_require__.d(__webpack_exports__, {
/* harmony export */   Cart: () => (/* binding */ Cart)
/* harmony export */ });
class Cart {
  constructor() {
    this.items = [];
  }

  add(name, price, quantity = 1) {
    this.items.push({ name, price, quantity });
    return this;
  }

  count() {
    return this.items.reduce((sum, item) => sum + item.quantity, 0);
  }

  total() {
    return this.items.reduce((sum, item) => sum + item.price * item.quantity, 0);
  }
}


/***/ }),
/* 3 */
/***/ ((__unused_webpack___webpack_module__, __webpack_exports__, __webpack_require__) => {

__webpack_require__.r(__webpack_exports__);
/* harmony export */ __webpack_require__.d(__webpack_exports__, {
/* harmony export */   CURRENCY: () => (/* binding */ CURRENCY),
/* harmony export */   formatPrice: () => (/* binding */ formatPrice)
/* harmony export */ });
const CURRENCY = "EUR";

function formatPrice(value, currency = CURRENCY) {
  return `${value.toFixed(2)} ${currency}`;
}


/***/ }),
/* 4 */
/***/ ((__unused_webpack___webpack_module__, __webpack_exports__, __webpack_require__) => {

__webpack_require__.r(__webpack_exports__);
/* harmony export */ __webpack_require__.d(__webpack_exports__, {
/* harmony export */   RATE: () => (/* binding */ RATE),
/* harmony export */   withTax: () => (/* binding */ withTax)
/* harmony export */ });
const RATE = 0.2;

function withTax(value) {
  return Math.round(value * (1 + RATE) * 100) / 100;
}


/***/ })
/******/ 	]);
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
/******/ 		__webpack_modules__[moduleId](module, module.exports, __webpack_require__);
/******/ 	
/******/ 		// Return the exports of the module
/******/ 		return module.exports;
/******/ 	}
/******/ 	
/************************************************************************/
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
let __webpack_exports__ = {};
// This entry needs to be wrapped in an IIFE because it needs to be isolated against other modules in the chunk.
(() => {
__webpack_require__.r(__webpack_exports__);
/* harmony import */ var _greet_js__WEBPACK_IMPORTED_MODULE_0__ = __webpack_require__(1);
/* harmony import */ var _shop_cart_js__WEBPACK_IMPORTED_MODULE_1__ = __webpack_require__(2);
/* harmony import */ var _shop_format_js__WEBPACK_IMPORTED_MODULE_2__ = __webpack_require__(3);
/* harmony import */ var _shop_tax_js__WEBPACK_IMPORTED_MODULE_3__ = __webpack_require__(4);





const cart = new _shop_cart_js__WEBPACK_IMPORTED_MODULE_1__.Cart();
cart.add("apple", 0.5, 4);
cart.add("bread", 2.25, 1);
console.log((0,_greet_js__WEBPACK_IMPORTED_MODULE_0__["default"])("shopper"));
console.log(`items ${cart.count()} subtotal ${(0,_shop_format_js__WEBPACK_IMPORTED_MODULE_2__.formatPrice)(cart.total())}`);
console.log(`with tax ${(0,_shop_format_js__WEBPACK_IMPORTED_MODULE_2__.formatPrice)(_shop_tax_js__WEBPACK_IMPORTED_MODULE_3__.withTax(cart.total()))} at ${_shop_tax_js__WEBPACK_IMPORTED_MODULE_3__.RATE * 100}%`);

})();

/******/ })()
;