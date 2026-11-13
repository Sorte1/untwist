/******/ (() => { // webpackBootstrap
/******/ 	"use strict";
/******/ 	var __webpack_modules__ = ([
/* 0 */,
/* 1 */
/***/ ((__unused_webpack___webpack_module__, __webpack_exports__, __webpack_require__) => {

__webpack_require__.r(__webpack_exports__);
/* harmony export */ __webpack_require__.d(__webpack_exports__, {
/* harmony export */   _: () => (/* binding */ _async_to_generator)
/* harmony export */ });
function asyncGeneratorStep(gen, resolve, reject, _next, _throw, key, arg) {
    try {
        var info = gen[key](arg);
        var value = info.value;
    } catch (error) {
        reject(error);
        return;
    }
    if (info.done) resolve(value);
    else Promise.resolve(value).then(_next, _throw);
}
function _async_to_generator(fn) {
    return function() {
        var self = this, args = arguments;

        return new Promise(function(resolve, reject) {
            var gen = fn.apply(self, args);

            function _next(value) {
                asyncGeneratorStep(gen, resolve, reject, _next, _throw, "next", value);
            }

            function _throw(err) {
                asyncGeneratorStep(gen, resolve, reject, _next, _throw, "throw", err);
            }

            _next(undefined);
        });
    };
}



/***/ }),
/* 2 */
/***/ ((__unused_webpack___webpack_module__, __webpack_exports__, __webpack_require__) => {

__webpack_require__.r(__webpack_exports__);
/* harmony export */ __webpack_require__.d(__webpack_exports__, {
/* harmony export */   _: () => (/* binding */ _call_super)
/* harmony export */ });
/* harmony import */ var _get_prototype_of_js__WEBPACK_IMPORTED_MODULE_0__ = __webpack_require__(3);
/* harmony import */ var _is_native_reflect_construct_js__WEBPACK_IMPORTED_MODULE_1__ = __webpack_require__(4);
/* harmony import */ var _possible_constructor_return_js__WEBPACK_IMPORTED_MODULE_2__ = __webpack_require__(5);




function _call_super(_this, derived, args) {
    // Super
    derived = (0,_get_prototype_of_js__WEBPACK_IMPORTED_MODULE_0__._)(derived);
    return (0,_possible_constructor_return_js__WEBPACK_IMPORTED_MODULE_2__._)(
        _this,
        (0,_is_native_reflect_construct_js__WEBPACK_IMPORTED_MODULE_1__._)()
            // NOTE: This doesn't work if this.__proto__.constructor has been modified.
            ? Reflect.construct(derived, args || [], (0,_get_prototype_of_js__WEBPACK_IMPORTED_MODULE_0__._)(_this).constructor)
            : derived.apply(_this, args)
    );
}




/***/ }),
/* 3 */
/***/ ((__unused_webpack___webpack_module__, __webpack_exports__, __webpack_require__) => {

__webpack_require__.r(__webpack_exports__);
/* harmony export */ __webpack_require__.d(__webpack_exports__, {
/* harmony export */   _: () => (/* binding */ _get_prototype_of)
/* harmony export */ });
function _get_prototype_of(o) {
    _get_prototype_of = Object.setPrototypeOf ? Object.getPrototypeOf : function getPrototypeOf(o) {
        return o.__proto__ || Object.getPrototypeOf(o);
    };

    return _get_prototype_of(o);
}



/***/ }),
/* 4 */
/***/ ((__unused_webpack___webpack_module__, __webpack_exports__, __webpack_require__) => {

__webpack_require__.r(__webpack_exports__);
/* harmony export */ __webpack_require__.d(__webpack_exports__, {
/* harmony export */   _: () => (/* binding */ _is_native_reflect_construct)
/* harmony export */ });
function _is_native_reflect_construct() {
    // Since Reflect.construct can't be properly polyfilled, some
    // implementations (e.g. core-js@2) don't set the correct internal slots.
    // Those polyfills don't allow us to subclass built-ins, so we need to
    // use our fallback implementation.
    try {
        // If the internal slots aren't set, this throws an error similar to
        //   TypeError: this is not a Boolean object.
        var result = !Boolean.prototype.valueOf.call(Reflect.construct(Boolean, [], function() {}));
    } catch (_) {}
    return (_is_native_reflect_construct = function() {
        return !!result;
    })();
}




/***/ }),
/* 5 */
/***/ ((__unused_webpack___webpack_module__, __webpack_exports__, __webpack_require__) => {

__webpack_require__.r(__webpack_exports__);
/* harmony export */ __webpack_require__.d(__webpack_exports__, {
/* harmony export */   _: () => (/* binding */ _possible_constructor_return)
/* harmony export */ });
/* harmony import */ var _assert_this_initialized_js__WEBPACK_IMPORTED_MODULE_0__ = __webpack_require__(6);
/* harmony import */ var _type_of_js__WEBPACK_IMPORTED_MODULE_1__ = __webpack_require__(7);



function _possible_constructor_return(self, call) {
    if (call && ((0,_type_of_js__WEBPACK_IMPORTED_MODULE_1__._)(call) === "object" || typeof call === "function")) return call;

    return (0,_assert_this_initialized_js__WEBPACK_IMPORTED_MODULE_0__._)(self);
}



/***/ }),
/* 6 */
/***/ ((__unused_webpack___webpack_module__, __webpack_exports__, __webpack_require__) => {

__webpack_require__.r(__webpack_exports__);
/* harmony export */ __webpack_require__.d(__webpack_exports__, {
/* harmony export */   _: () => (/* binding */ _assert_this_initialized)
/* harmony export */ });
function _assert_this_initialized(self) {
    if (self === void 0) throw new ReferenceError("this hasn't been initialised - super() hasn't been called");

    return self;
}



/***/ }),
/* 7 */
/***/ ((__unused_webpack___webpack_module__, __webpack_exports__, __webpack_require__) => {

__webpack_require__.r(__webpack_exports__);
/* harmony export */ __webpack_require__.d(__webpack_exports__, {
/* harmony export */   _: () => (/* binding */ _type_of)
/* harmony export */ });
function _type_of(obj) {
    "@swc/helpers - typeof";

    return obj && typeof Symbol !== "undefined" && obj.constructor === Symbol ? "symbol" : typeof obj;
}



/***/ }),
/* 8 */
/***/ ((__unused_webpack___webpack_module__, __webpack_exports__, __webpack_require__) => {

__webpack_require__.r(__webpack_exports__);
/* harmony export */ __webpack_require__.d(__webpack_exports__, {
/* harmony export */   _: () => (/* binding */ _class_call_check)
/* harmony export */ });
function _class_call_check(instance, Constructor) {
    if (!(instance instanceof Constructor)) throw new TypeError("Cannot call a class as a function");
}



/***/ }),
/* 9 */
/***/ ((__unused_webpack___webpack_module__, __webpack_exports__, __webpack_require__) => {

__webpack_require__.r(__webpack_exports__);
/* harmony export */ __webpack_require__.d(__webpack_exports__, {
/* harmony export */   _: () => (/* binding */ _create_class)
/* harmony export */ });
function _defineProperties(target, props) {
    for (var i = 0; i < props.length; i++) {
        var descriptor = props[i];
        descriptor.enumerable = descriptor.enumerable || false;
        descriptor.configurable = true;

        if ("value" in descriptor) descriptor.writable = true;

        Object.defineProperty(target, descriptor.key, descriptor);
    }
}
function _create_class(Constructor, protoProps, staticProps) {
    if (protoProps) _defineProperties(Constructor.prototype, protoProps);
    if (staticProps) _defineProperties(Constructor, staticProps);

    return Constructor;
}



/***/ }),
/* 10 */
/***/ ((__unused_webpack___webpack_module__, __webpack_exports__, __webpack_require__) => {

__webpack_require__.r(__webpack_exports__);
/* harmony export */ __webpack_require__.d(__webpack_exports__, {
/* harmony export */   _: () => (/* binding */ _get)
/* harmony export */ });
/* harmony import */ var _super_prop_base_js__WEBPACK_IMPORTED_MODULE_0__ = __webpack_require__(11);


function _get(target, property, receiver) {
    if (typeof Reflect !== "undefined" && Reflect.get) _get = Reflect.get;
    else {
        _get = function get(target, property, receiver) {
            var base = (0,_super_prop_base_js__WEBPACK_IMPORTED_MODULE_0__._)(target, property);

            if (!base) return;

            var desc = Object.getOwnPropertyDescriptor(base, property);

            if (desc.get) return desc.get.call(receiver || target);

            return desc.value;
        };
    }

    return _get(target, property, receiver || target);
}



/***/ }),
/* 11 */
/***/ ((__unused_webpack___webpack_module__, __webpack_exports__, __webpack_require__) => {

__webpack_require__.r(__webpack_exports__);
/* harmony export */ __webpack_require__.d(__webpack_exports__, {
/* harmony export */   _: () => (/* binding */ _super_prop_base)
/* harmony export */ });
/* harmony import */ var _get_prototype_of_js__WEBPACK_IMPORTED_MODULE_0__ = __webpack_require__(3);


function _super_prop_base(object, property) {
    while (!Object.prototype.hasOwnProperty.call(object, property)) {
        object = (0,_get_prototype_of_js__WEBPACK_IMPORTED_MODULE_0__._)(object);
        if (object === null) break;
    }

    return object;
}



/***/ }),
/* 12 */
/***/ ((__unused_webpack___webpack_module__, __webpack_exports__, __webpack_require__) => {

__webpack_require__.r(__webpack_exports__);
/* harmony export */ __webpack_require__.d(__webpack_exports__, {
/* harmony export */   _: () => (/* binding */ _inherits)
/* harmony export */ });
/* harmony import */ var _set_prototype_of_js__WEBPACK_IMPORTED_MODULE_0__ = __webpack_require__(13);


function _inherits(subClass, superClass) {
    if (typeof superClass !== "function" && superClass !== null) {
        throw new TypeError("Super expression must either be null or a function");
    }

    subClass.prototype = Object.create(superClass && superClass.prototype, { constructor: { value: subClass, writable: true, configurable: true } });

    if (superClass) (0,_set_prototype_of_js__WEBPACK_IMPORTED_MODULE_0__._)(subClass, superClass);
}



/***/ }),
/* 13 */
/***/ ((__unused_webpack___webpack_module__, __webpack_exports__, __webpack_require__) => {

__webpack_require__.r(__webpack_exports__);
/* harmony export */ __webpack_require__.d(__webpack_exports__, {
/* harmony export */   _: () => (/* binding */ _set_prototype_of)
/* harmony export */ });
function _set_prototype_of(o, p) {
    _set_prototype_of = Object.setPrototypeOf || function setPrototypeOf(o, p) {
        o.__proto__ = p;

        return o;
    };

    return _set_prototype_of(o, p);
}



/***/ }),
/* 14 */
/***/ ((__unused_webpack___webpack_module__, __webpack_exports__, __webpack_require__) => {

__webpack_require__.r(__webpack_exports__);
/* harmony export */ __webpack_require__.d(__webpack_exports__, {
/* harmony export */   _: () => (/* binding */ _object_without_properties)
/* harmony export */ });
/* harmony import */ var _object_without_properties_loose_js__WEBPACK_IMPORTED_MODULE_0__ = __webpack_require__(15);


function _object_without_properties(source, excluded) {
    if (source == null) return {};

    var target = {}, sourceKeys, key, i;
    if (typeof Reflect !== "undefined" && Reflect.ownKeys) {
        sourceKeys = Reflect.ownKeys(Object(source));
        for (i = 0; i < sourceKeys.length; i++) {
            key = sourceKeys[i];
            if (excluded.indexOf(key) >= 0) continue;
            if (!Object.prototype.propertyIsEnumerable.call(source, key)) continue;
            target[key] = source[key];
        }

        return target;
    }

    target = (0,_object_without_properties_loose_js__WEBPACK_IMPORTED_MODULE_0__._)(source, excluded);
    if (Object.getOwnPropertySymbols) {
        sourceKeys = Object.getOwnPropertySymbols(source);
        for (i = 0; i < sourceKeys.length; i++) {
            key = sourceKeys[i];
            if (excluded.indexOf(key) >= 0) continue;
            if (!Object.prototype.propertyIsEnumerable.call(source, key)) continue;
            target[key] = source[key];
        }
    }

    return target;
}




/***/ }),
/* 15 */
/***/ ((__unused_webpack___webpack_module__, __webpack_exports__, __webpack_require__) => {

__webpack_require__.r(__webpack_exports__);
/* harmony export */ __webpack_require__.d(__webpack_exports__, {
/* harmony export */   _: () => (/* binding */ _object_without_properties_loose)
/* harmony export */ });
function _object_without_properties_loose(source, excluded) {
    if (source == null) return {};

    var target = {}, sourceKeys = Object.getOwnPropertyNames(source), key, i;
    for (i = 0; i < sourceKeys.length; i++) {
        key = sourceKeys[i];
        if (excluded.indexOf(key) >= 0) continue;
        if (!Object.prototype.propertyIsEnumerable.call(source, key)) continue;
        target[key] = source[key];
    }

    return target;
}



/***/ }),
/* 16 */
/***/ ((__unused_webpack___webpack_module__, __webpack_exports__, __webpack_require__) => {

__webpack_require__.r(__webpack_exports__);
/* harmony export */ __webpack_require__.d(__webpack_exports__, {
/* harmony export */   _: () => (/* binding */ _to_consumable_array)
/* harmony export */ });
/* harmony import */ var _array_without_holes_js__WEBPACK_IMPORTED_MODULE_0__ = __webpack_require__(17);
/* harmony import */ var _iterable_to_array_js__WEBPACK_IMPORTED_MODULE_1__ = __webpack_require__(19);
/* harmony import */ var _non_iterable_spread_js__WEBPACK_IMPORTED_MODULE_2__ = __webpack_require__(20);
/* harmony import */ var _unsupported_iterable_to_array_js__WEBPACK_IMPORTED_MODULE_3__ = __webpack_require__(21);





function _to_consumable_array(arr) {
    return (0,_array_without_holes_js__WEBPACK_IMPORTED_MODULE_0__._)(arr) || (0,_iterable_to_array_js__WEBPACK_IMPORTED_MODULE_1__._)(arr) || (0,_unsupported_iterable_to_array_js__WEBPACK_IMPORTED_MODULE_3__._)(arr) || (0,_non_iterable_spread_js__WEBPACK_IMPORTED_MODULE_2__._)();
}



/***/ }),
/* 17 */
/***/ ((__unused_webpack___webpack_module__, __webpack_exports__, __webpack_require__) => {

__webpack_require__.r(__webpack_exports__);
/* harmony export */ __webpack_require__.d(__webpack_exports__, {
/* harmony export */   _: () => (/* binding */ _array_without_holes)
/* harmony export */ });
/* harmony import */ var _array_like_to_array_js__WEBPACK_IMPORTED_MODULE_0__ = __webpack_require__(18);


function _array_without_holes(arr) {
    if (Array.isArray(arr)) return (0,_array_like_to_array_js__WEBPACK_IMPORTED_MODULE_0__._)(arr);
}



/***/ }),
/* 18 */
/***/ ((__unused_webpack___webpack_module__, __webpack_exports__, __webpack_require__) => {

__webpack_require__.r(__webpack_exports__);
/* harmony export */ __webpack_require__.d(__webpack_exports__, {
/* harmony export */   _: () => (/* binding */ _array_like_to_array)
/* harmony export */ });
function _array_like_to_array(arr, len) {
    if (len == null || len > arr.length) len = arr.length;

    for (var i = 0, arr2 = new Array(len); i < len; i++) arr2[i] = arr[i];

    return arr2;
}



/***/ }),
/* 19 */
/***/ ((__unused_webpack___webpack_module__, __webpack_exports__, __webpack_require__) => {

__webpack_require__.r(__webpack_exports__);
/* harmony export */ __webpack_require__.d(__webpack_exports__, {
/* harmony export */   _: () => (/* binding */ _iterable_to_array)
/* harmony export */ });
function _iterable_to_array(iter) {
    if (typeof Symbol !== "undefined" && iter[Symbol.iterator] != null || iter["@@iterator"] != null) {
        return Array.from(iter);
    }
}



/***/ }),
/* 20 */
/***/ ((__unused_webpack___webpack_module__, __webpack_exports__, __webpack_require__) => {

__webpack_require__.r(__webpack_exports__);
/* harmony export */ __webpack_require__.d(__webpack_exports__, {
/* harmony export */   _: () => (/* binding */ _non_iterable_spread)
/* harmony export */ });
function _non_iterable_spread() {
    throw new TypeError("Invalid attempt to spread non-iterable instance.\nIn order to be iterable, non-array objects must have a [Symbol.iterator]() method.");
}



/***/ }),
/* 21 */
/***/ ((__unused_webpack___webpack_module__, __webpack_exports__, __webpack_require__) => {

__webpack_require__.r(__webpack_exports__);
/* harmony export */ __webpack_require__.d(__webpack_exports__, {
/* harmony export */   _: () => (/* binding */ _unsupported_iterable_to_array)
/* harmony export */ });
/* harmony import */ var _array_like_to_array_js__WEBPACK_IMPORTED_MODULE_0__ = __webpack_require__(18);


function _unsupported_iterable_to_array(o, minLen) {
    if (!o) return;
    if (typeof o === "string") return (0,_array_like_to_array_js__WEBPACK_IMPORTED_MODULE_0__._)(o, minLen);

    var n = Object.prototype.toString.call(o).slice(8, -1);

    if (n === "Object" && o.constructor) n = o.constructor.name;
    if (n === "Map" || n === "Set") return Array.from(o);
    if (n === "Arguments" || /^(?:Ui|I)nt(?:8|16|32)(?:Clamped)?Array$/.test(n)) return (0,_array_like_to_array_js__WEBPACK_IMPORTED_MODULE_0__._)(o, minLen);
}



/***/ }),
/* 22 */
/***/ ((__unused_webpack___webpack_module__, __webpack_exports__, __webpack_require__) => {

__webpack_require__.r(__webpack_exports__);
/* harmony export */ __webpack_require__.d(__webpack_exports__, {
/* harmony export */   _: () => (/* binding */ _ts_generator)
/* harmony export */ });
function _ts_generator(thisArg, body) {
    var f, y, t, _ = { label: 0, sent: function () { if (t[0] & 1) throw t[1]; return t[1]; }, trys: [], ops: [] }, g = Object.create((typeof Iterator === "function" ? Iterator : Object).prototype), d = Object.defineProperty;
    return d(g, "next", { value: verb(0) }), d(g, "throw", { value: verb(1) }), d(g, "return", { value: verb(2) }), typeof Symbol === "function" && d(g, Symbol.iterator, { value: function () { return this; } }), g;
    function verb(n) { return function (v) { return step([n, v]); }; }
    function step(op) {
        if (f) throw new TypeError("Generator is already executing.");
        while (g && (g = 0, op[0] && (_ = 0)), _) try {
            if (f = 1, y && (t = op[0] & 2 ? y["return"] : op[0] ? y["throw"] || ((t = y["return"]) && t.call(y), 0) : y.next) && !(t = t.call(y, op[1])).done) return t;
            if (y = 0, t) op = [op[0] & 2, t.value];
            switch (op[0]) {
                case 0: case 1: t = op; break;
                case 4: _.label++; return { value: op[1], done: false };
                case 5: _.label++; y = op[1]; op = [0]; continue;
                case 7: op = _.ops.pop(); _.trys.pop(); continue;
                default:
                    if (!(t = _.trys, t = t.length > 0 && t[t.length - 1]) && (op[0] === 6 || op[0] === 2)) { _ = 0; continue; }
                    if (op[0] === 3 && (!t || (op[1] > t[0] && op[1] < t[3]))) { _.label = op[1]; break; }
                    if (op[0] === 6 && _.label < t[1]) { _.label = t[1]; t = op; break; }
                    if (t && _.label < t[2]) { _.label = t[2]; _.ops.push(op); break; }
                    if (t[2]) _.ops.pop();
                    _.trys.pop(); continue;
            }
            op = body.call(thisArg, _);
        } catch (e) { op = [6, e]; y = 0; } finally { f = t = 0; }
        if (op[0] & 5) throw op[1]; return { value: op[0] ? op[1] : void 0, done: true };
    }
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
/* harmony import */ var _swc_helpers_async_to_generator__WEBPACK_IMPORTED_MODULE_0__ = __webpack_require__(1);
/* harmony import */ var _swc_helpers_call_super__WEBPACK_IMPORTED_MODULE_1__ = __webpack_require__(2);
/* harmony import */ var _swc_helpers_class_call_check__WEBPACK_IMPORTED_MODULE_2__ = __webpack_require__(8);
/* harmony import */ var _swc_helpers_create_class__WEBPACK_IMPORTED_MODULE_3__ = __webpack_require__(9);
/* harmony import */ var _swc_helpers_get__WEBPACK_IMPORTED_MODULE_4__ = __webpack_require__(10);
/* harmony import */ var _swc_helpers_get_prototype_of__WEBPACK_IMPORTED_MODULE_5__ = __webpack_require__(3);
/* harmony import */ var _swc_helpers_inherits__WEBPACK_IMPORTED_MODULE_6__ = __webpack_require__(12);
/* harmony import */ var _swc_helpers_object_without_properties__WEBPACK_IMPORTED_MODULE_7__ = __webpack_require__(14);
/* harmony import */ var _swc_helpers_to_consumable_array__WEBPACK_IMPORTED_MODULE_8__ = __webpack_require__(16);
/* harmony import */ var _swc_helpers_ts_generator__WEBPACK_IMPORTED_MODULE_9__ = __webpack_require__(22);










var Animal = /*#__PURE__*/ function() {
    "use strict";
    function Animal(name) {
        (0,_swc_helpers_class_call_check__WEBPACK_IMPORTED_MODULE_2__._)(this, Animal);
        this.name = name;
    }
    ;(0,_swc_helpers_create_class__WEBPACK_IMPORTED_MODULE_3__._)(Animal, [
        {
            key: "speak",
            value: function speak() {
                return "".concat(this.name, " speaks");
            }
        }
    ], [
        {
            key: "of",
            value: function of(name) {
                return new this(name);
            }
        }
    ]);
    return Animal;
}();
var Dog = /*#__PURE__*/ function(Animal) {
    "use strict";
    (0,_swc_helpers_inherits__WEBPACK_IMPORTED_MODULE_6__._)(Dog, Animal);
    function Dog() {
        (0,_swc_helpers_class_call_check__WEBPACK_IMPORTED_MODULE_2__._)(this, Dog);
        return (0,_swc_helpers_call_super__WEBPACK_IMPORTED_MODULE_1__._)(this, Dog, arguments);
    }
    ;(0,_swc_helpers_create_class__WEBPACK_IMPORTED_MODULE_3__._)(Dog, [
        {
            key: "speak",
            value: function speak() {
                return (0,_swc_helpers_get__WEBPACK_IMPORTED_MODULE_4__._)((0,_swc_helpers_get_prototype_of__WEBPACK_IMPORTED_MODULE_5__._)(Dog.prototype), "speak", this).call(this) + " loudly";
            }
        }
    ]);
    return Dog;
}(Animal);
function count() {
    return (0,_swc_helpers_ts_generator__WEBPACK_IMPORTED_MODULE_9__._)(this, function(_state) {
        switch(_state.label){
            case 0:
                return [
                    4,
                    1
                ];
            case 1:
                _state.sent();
                return [
                    4,
                    2
                ];
            case 2:
                _state.sent();
                return [
                    2
                ];
        }
    });
}
function main() {
    return (0,_swc_helpers_async_to_generator__WEBPACK_IMPORTED_MODULE_0__._)(function() {
        var parts, _ref, a, rest, total, _iteratorNormalCompletion, _didIteratorError, _iteratorError, _iterator, _step, value, err;
        return (0,_swc_helpers_ts_generator__WEBPACK_IMPORTED_MODULE_9__._)(this, function(_state) {
            switch(_state.label){
                case 0:
                    parts = (0,_swc_helpers_to_consumable_array__WEBPACK_IMPORTED_MODULE_8__._)([
                        1,
                        2
                    ]).concat([
                        3
                    ]);
                    _ref = {
                        a: 1,
                        b: 2,
                        c: 3
                    }, a = _ref.a, rest = (0,_swc_helpers_object_without_properties__WEBPACK_IMPORTED_MODULE_7__._)(_ref, [
                        "a"
                    ]);
                    total = 0;
                    _iteratorNormalCompletion = true, _didIteratorError = false, _iteratorError = undefined;
                    _state.label = 1;
                case 1:
                    _state.trys.push([
                        1,
                        6,
                        7,
                        8
                    ]);
                    _iterator = parts[Symbol.iterator]();
                    _state.label = 2;
                case 2:
                    if (!!(_iteratorNormalCompletion = (_step = _iterator.next()).done)) return [
                        3,
                        5
                    ];
                    value = _step.value;
                    return [
                        4,
                        Promise.resolve(value)
                    ];
                case 3:
                    total = total + _state.sent();
                    _state.label = 4;
                case 4:
                    _iteratorNormalCompletion = true;
                    return [
                        3,
                        2
                    ];
                case 5:
                    return [
                        3,
                        8
                    ];
                case 6:
                    err = _state.sent();
                    _didIteratorError = true;
                    _iteratorError = err;
                    return [
                        3,
                        8
                    ];
                case 7:
                    try {
                        if (!_iteratorNormalCompletion && _iterator.return != null) {
                            _iterator.return();
                        }
                    } finally{
                        if (_didIteratorError) {
                            throw _iteratorError;
                        }
                    }
                    return [
                        7
                    ];
                case 8:
                    console.log(Dog.of("rex").speak(), parts.length, a, Object.keys(rest).length, total, (0,_swc_helpers_to_consumable_array__WEBPACK_IMPORTED_MODULE_8__._)(count()).join(","));
                    return [
                        2
                    ];
            }
        });
    })();
}
main();

})();

/******/ })()
;