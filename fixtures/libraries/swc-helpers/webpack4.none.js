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
/******/ 	return __webpack_require__(__webpack_require__.s = 0);
/******/ })
/************************************************************************/
/******/ ([
/* 0 */
/***/ (function(module, __webpack_exports__, __webpack_require__) {

"use strict";
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
        Object(_swc_helpers_class_call_check__WEBPACK_IMPORTED_MODULE_2__["_"])(this, Animal);
        this.name = name;
    }
    Object(_swc_helpers_create_class__WEBPACK_IMPORTED_MODULE_3__["_"])(Animal, [
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
    Object(_swc_helpers_inherits__WEBPACK_IMPORTED_MODULE_6__["_"])(Dog, Animal);
    function Dog() {
        Object(_swc_helpers_class_call_check__WEBPACK_IMPORTED_MODULE_2__["_"])(this, Dog);
        return Object(_swc_helpers_call_super__WEBPACK_IMPORTED_MODULE_1__["_"])(this, Dog, arguments);
    }
    Object(_swc_helpers_create_class__WEBPACK_IMPORTED_MODULE_3__["_"])(Dog, [
        {
            key: "speak",
            value: function speak() {
                return Object(_swc_helpers_get__WEBPACK_IMPORTED_MODULE_4__["_"])(Object(_swc_helpers_get_prototype_of__WEBPACK_IMPORTED_MODULE_5__["_"])(Dog.prototype), "speak", this).call(this) + " loudly";
            }
        }
    ]);
    return Dog;
}(Animal);
function count() {
    return Object(_swc_helpers_ts_generator__WEBPACK_IMPORTED_MODULE_9__["_"])(this, function(_state) {
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
    return Object(_swc_helpers_async_to_generator__WEBPACK_IMPORTED_MODULE_0__["_"])(function() {
        var parts, _ref, a, rest, total, _iteratorNormalCompletion, _didIteratorError, _iteratorError, _iterator, _step, value, err;
        return Object(_swc_helpers_ts_generator__WEBPACK_IMPORTED_MODULE_9__["_"])(this, function(_state) {
            switch(_state.label){
                case 0:
                    parts = Object(_swc_helpers_to_consumable_array__WEBPACK_IMPORTED_MODULE_8__["_"])([
                        1,
                        2
                    ]).concat([
                        3
                    ]);
                    _ref = {
                        a: 1,
                        b: 2,
                        c: 3
                    }, a = _ref.a, rest = Object(_swc_helpers_object_without_properties__WEBPACK_IMPORTED_MODULE_7__["_"])(_ref, [
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
                    console.log(Dog.of("rex").speak(), parts.length, a, Object.keys(rest).length, total, Object(_swc_helpers_to_consumable_array__WEBPACK_IMPORTED_MODULE_8__["_"])(count()).join(","));
                    return [
                        2
                    ];
            }
        });
    })();
}
main();


/***/ }),
/* 1 */
/***/ (function(module, __webpack_exports__, __webpack_require__) {

"use strict";
__webpack_require__.r(__webpack_exports__);
/* harmony export (binding) */ __webpack_require__.d(__webpack_exports__, "_", function() { return _async_to_generator; });
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
/***/ (function(module, __webpack_exports__, __webpack_require__) {

"use strict";
__webpack_require__.r(__webpack_exports__);
/* harmony export (binding) */ __webpack_require__.d(__webpack_exports__, "_", function() { return _call_super; });
/* harmony import */ var _get_prototype_of_js__WEBPACK_IMPORTED_MODULE_0__ = __webpack_require__(3);
/* harmony import */ var _is_native_reflect_construct_js__WEBPACK_IMPORTED_MODULE_1__ = __webpack_require__(4);
/* harmony import */ var _possible_constructor_return_js__WEBPACK_IMPORTED_MODULE_2__ = __webpack_require__(5);




function _call_super(_this, derived, args) {
    // Super
    derived = Object(_get_prototype_of_js__WEBPACK_IMPORTED_MODULE_0__["_"])(derived);
    return Object(_possible_constructor_return_js__WEBPACK_IMPORTED_MODULE_2__["_"])(
        _this,
        Object(_is_native_reflect_construct_js__WEBPACK_IMPORTED_MODULE_1__["_"])()
            // NOTE: This doesn't work if this.__proto__.constructor has been modified.
            ? Reflect.construct(derived, args || [], Object(_get_prototype_of_js__WEBPACK_IMPORTED_MODULE_0__["_"])(_this).constructor)
            : derived.apply(_this, args)
    );
}




/***/ }),
/* 3 */
/***/ (function(module, __webpack_exports__, __webpack_require__) {

"use strict";
__webpack_require__.r(__webpack_exports__);
/* harmony export (binding) */ __webpack_require__.d(__webpack_exports__, "_", function() { return _get_prototype_of; });
function _get_prototype_of(o) {
    _get_prototype_of = Object.setPrototypeOf ? Object.getPrototypeOf : function getPrototypeOf(o) {
        return o.__proto__ || Object.getPrototypeOf(o);
    };

    return _get_prototype_of(o);
}



/***/ }),
/* 4 */
/***/ (function(module, __webpack_exports__, __webpack_require__) {

"use strict";
__webpack_require__.r(__webpack_exports__);
/* harmony export (binding) */ __webpack_require__.d(__webpack_exports__, "_", function() { return _is_native_reflect_construct; });
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
/***/ (function(module, __webpack_exports__, __webpack_require__) {

"use strict";
__webpack_require__.r(__webpack_exports__);
/* harmony export (binding) */ __webpack_require__.d(__webpack_exports__, "_", function() { return _possible_constructor_return; });
/* harmony import */ var _assert_this_initialized_js__WEBPACK_IMPORTED_MODULE_0__ = __webpack_require__(6);
/* harmony import */ var _type_of_js__WEBPACK_IMPORTED_MODULE_1__ = __webpack_require__(7);



function _possible_constructor_return(self, call) {
    if (call && (Object(_type_of_js__WEBPACK_IMPORTED_MODULE_1__["_"])(call) === "object" || typeof call === "function")) return call;

    return Object(_assert_this_initialized_js__WEBPACK_IMPORTED_MODULE_0__["_"])(self);
}



/***/ }),
/* 6 */
/***/ (function(module, __webpack_exports__, __webpack_require__) {

"use strict";
__webpack_require__.r(__webpack_exports__);
/* harmony export (binding) */ __webpack_require__.d(__webpack_exports__, "_", function() { return _assert_this_initialized; });
function _assert_this_initialized(self) {
    if (self === void 0) throw new ReferenceError("this hasn't been initialised - super() hasn't been called");

    return self;
}



/***/ }),
/* 7 */
/***/ (function(module, __webpack_exports__, __webpack_require__) {

"use strict";
__webpack_require__.r(__webpack_exports__);
/* harmony export (binding) */ __webpack_require__.d(__webpack_exports__, "_", function() { return _type_of; });
function _type_of(obj) {
    "@swc/helpers - typeof";

    return obj && typeof Symbol !== "undefined" && obj.constructor === Symbol ? "symbol" : typeof obj;
}



/***/ }),
/* 8 */
/***/ (function(module, __webpack_exports__, __webpack_require__) {

"use strict";
__webpack_require__.r(__webpack_exports__);
/* harmony export (binding) */ __webpack_require__.d(__webpack_exports__, "_", function() { return _class_call_check; });
function _class_call_check(instance, Constructor) {
    if (!(instance instanceof Constructor)) throw new TypeError("Cannot call a class as a function");
}



/***/ }),
/* 9 */
/***/ (function(module, __webpack_exports__, __webpack_require__) {

"use strict";
__webpack_require__.r(__webpack_exports__);
/* harmony export (binding) */ __webpack_require__.d(__webpack_exports__, "_", function() { return _create_class; });
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
/***/ (function(module, __webpack_exports__, __webpack_require__) {

"use strict";
__webpack_require__.r(__webpack_exports__);
/* harmony export (binding) */ __webpack_require__.d(__webpack_exports__, "_", function() { return _get; });
/* harmony import */ var _super_prop_base_js__WEBPACK_IMPORTED_MODULE_0__ = __webpack_require__(11);


function _get(target, property, receiver) {
    if (typeof Reflect !== "undefined" && Reflect.get) _get = Reflect.get;
    else {
        _get = function get(target, property, receiver) {
            var base = Object(_super_prop_base_js__WEBPACK_IMPORTED_MODULE_0__["_"])(target, property);

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
/***/ (function(module, __webpack_exports__, __webpack_require__) {

"use strict";
__webpack_require__.r(__webpack_exports__);
/* harmony export (binding) */ __webpack_require__.d(__webpack_exports__, "_", function() { return _super_prop_base; });
/* harmony import */ var _get_prototype_of_js__WEBPACK_IMPORTED_MODULE_0__ = __webpack_require__(3);


function _super_prop_base(object, property) {
    while (!Object.prototype.hasOwnProperty.call(object, property)) {
        object = Object(_get_prototype_of_js__WEBPACK_IMPORTED_MODULE_0__["_"])(object);
        if (object === null) break;
    }

    return object;
}



/***/ }),
/* 12 */
/***/ (function(module, __webpack_exports__, __webpack_require__) {

"use strict";
__webpack_require__.r(__webpack_exports__);
/* harmony export (binding) */ __webpack_require__.d(__webpack_exports__, "_", function() { return _inherits; });
/* harmony import */ var _set_prototype_of_js__WEBPACK_IMPORTED_MODULE_0__ = __webpack_require__(13);


function _inherits(subClass, superClass) {
    if (typeof superClass !== "function" && superClass !== null) {
        throw new TypeError("Super expression must either be null or a function");
    }

    subClass.prototype = Object.create(superClass && superClass.prototype, { constructor: { value: subClass, writable: true, configurable: true } });

    if (superClass) Object(_set_prototype_of_js__WEBPACK_IMPORTED_MODULE_0__["_"])(subClass, superClass);
}



/***/ }),
/* 13 */
/***/ (function(module, __webpack_exports__, __webpack_require__) {

"use strict";
__webpack_require__.r(__webpack_exports__);
/* harmony export (binding) */ __webpack_require__.d(__webpack_exports__, "_", function() { return _set_prototype_of; });
function _set_prototype_of(o, p) {
    _set_prototype_of = Object.setPrototypeOf || function setPrototypeOf(o, p) {
        o.__proto__ = p;

        return o;
    };

    return _set_prototype_of(o, p);
}



/***/ }),
/* 14 */
/***/ (function(module, __webpack_exports__, __webpack_require__) {

"use strict";
__webpack_require__.r(__webpack_exports__);
/* harmony export (binding) */ __webpack_require__.d(__webpack_exports__, "_", function() { return _object_without_properties; });
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

    target = Object(_object_without_properties_loose_js__WEBPACK_IMPORTED_MODULE_0__["_"])(source, excluded);
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
/***/ (function(module, __webpack_exports__, __webpack_require__) {

"use strict";
__webpack_require__.r(__webpack_exports__);
/* harmony export (binding) */ __webpack_require__.d(__webpack_exports__, "_", function() { return _object_without_properties_loose; });
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
/***/ (function(module, __webpack_exports__, __webpack_require__) {

"use strict";
__webpack_require__.r(__webpack_exports__);
/* harmony export (binding) */ __webpack_require__.d(__webpack_exports__, "_", function() { return _to_consumable_array; });
/* harmony import */ var _array_without_holes_js__WEBPACK_IMPORTED_MODULE_0__ = __webpack_require__(17);
/* harmony import */ var _iterable_to_array_js__WEBPACK_IMPORTED_MODULE_1__ = __webpack_require__(19);
/* harmony import */ var _non_iterable_spread_js__WEBPACK_IMPORTED_MODULE_2__ = __webpack_require__(20);
/* harmony import */ var _unsupported_iterable_to_array_js__WEBPACK_IMPORTED_MODULE_3__ = __webpack_require__(21);





function _to_consumable_array(arr) {
    return Object(_array_without_holes_js__WEBPACK_IMPORTED_MODULE_0__["_"])(arr) || Object(_iterable_to_array_js__WEBPACK_IMPORTED_MODULE_1__["_"])(arr) || Object(_unsupported_iterable_to_array_js__WEBPACK_IMPORTED_MODULE_3__["_"])(arr) || Object(_non_iterable_spread_js__WEBPACK_IMPORTED_MODULE_2__["_"])();
}



/***/ }),
/* 17 */
/***/ (function(module, __webpack_exports__, __webpack_require__) {

"use strict";
__webpack_require__.r(__webpack_exports__);
/* harmony export (binding) */ __webpack_require__.d(__webpack_exports__, "_", function() { return _array_without_holes; });
/* harmony import */ var _array_like_to_array_js__WEBPACK_IMPORTED_MODULE_0__ = __webpack_require__(18);


function _array_without_holes(arr) {
    if (Array.isArray(arr)) return Object(_array_like_to_array_js__WEBPACK_IMPORTED_MODULE_0__["_"])(arr);
}



/***/ }),
/* 18 */
/***/ (function(module, __webpack_exports__, __webpack_require__) {

"use strict";
__webpack_require__.r(__webpack_exports__);
/* harmony export (binding) */ __webpack_require__.d(__webpack_exports__, "_", function() { return _array_like_to_array; });
function _array_like_to_array(arr, len) {
    if (len == null || len > arr.length) len = arr.length;

    for (var i = 0, arr2 = new Array(len); i < len; i++) arr2[i] = arr[i];

    return arr2;
}



/***/ }),
/* 19 */
/***/ (function(module, __webpack_exports__, __webpack_require__) {

"use strict";
__webpack_require__.r(__webpack_exports__);
/* harmony export (binding) */ __webpack_require__.d(__webpack_exports__, "_", function() { return _iterable_to_array; });
function _iterable_to_array(iter) {
    if (typeof Symbol !== "undefined" && iter[Symbol.iterator] != null || iter["@@iterator"] != null) {
        return Array.from(iter);
    }
}



/***/ }),
/* 20 */
/***/ (function(module, __webpack_exports__, __webpack_require__) {

"use strict";
__webpack_require__.r(__webpack_exports__);
/* harmony export (binding) */ __webpack_require__.d(__webpack_exports__, "_", function() { return _non_iterable_spread; });
function _non_iterable_spread() {
    throw new TypeError("Invalid attempt to spread non-iterable instance.\nIn order to be iterable, non-array objects must have a [Symbol.iterator]() method.");
}



/***/ }),
/* 21 */
/***/ (function(module, __webpack_exports__, __webpack_require__) {

"use strict";
__webpack_require__.r(__webpack_exports__);
/* harmony export (binding) */ __webpack_require__.d(__webpack_exports__, "_", function() { return _unsupported_iterable_to_array; });
/* harmony import */ var _array_like_to_array_js__WEBPACK_IMPORTED_MODULE_0__ = __webpack_require__(18);


function _unsupported_iterable_to_array(o, minLen) {
    if (!o) return;
    if (typeof o === "string") return Object(_array_like_to_array_js__WEBPACK_IMPORTED_MODULE_0__["_"])(o, minLen);

    var n = Object.prototype.toString.call(o).slice(8, -1);

    if (n === "Object" && o.constructor) n = o.constructor.name;
    if (n === "Map" || n === "Set") return Array.from(o);
    if (n === "Arguments" || /^(?:Ui|I)nt(?:8|16|32)(?:Clamped)?Array$/.test(n)) return Object(_array_like_to_array_js__WEBPACK_IMPORTED_MODULE_0__["_"])(o, minLen);
}



/***/ }),
/* 22 */
/***/ (function(module, __webpack_exports__, __webpack_require__) {

"use strict";
__webpack_require__.r(__webpack_exports__);
/* harmony export (binding) */ __webpack_require__.d(__webpack_exports__, "_", function() { return _ts_generator; });
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
/******/ ]);