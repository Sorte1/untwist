(function() {

// \0rolldown/runtime.js
	var __create = Object.create;
	var __defProp = Object.defineProperty;
	var __getOwnPropDesc = Object.getOwnPropertyDescriptor;
	var __getOwnPropNames = Object.getOwnPropertyNames;
	var __getProtoOf = Object.getPrototypeOf;
	var __hasOwnProp = Object.prototype.hasOwnProperty;
	var __esmMin = (fn, res, err) => () => {
		if (err) throw err[0];
		try {
			return fn && (res = fn(fn = 0)), res;
		} catch (e) {
			throw err = [e], e;
		}
	};
	var __commonJSMin = (cb, mod) => () => (mod || (cb((mod = { exports: {} }).exports, mod, mod.exports), cb = null), mod.exports);
	var __exportAll = (all, flags) => {
		let target = Object.setPrototypeOf({}, null);
		for (var name in all) {
			__defProp(target, name, {
				get: all[name],
				enumerable: true
			});
		}
		if (!(flags & 1)) {
			__defProp(target, Symbol.toStringTag, { value: "Module" });
		}
		return flags & 2 ? target : Object.preventExtensions(target);
	};
	var __copyProps = (to, from, except, desc) => {
		if (from && typeof from === "object" || typeof from === "function") {
			for (var keys = __getOwnPropNames(from), i = 0, n = keys.length, key; i < n; i++) {
				key = keys[i];
				if (!__hasOwnProp.call(to, key) && key !== except) {
					__defProp(to, key, {
						get: ((k) => from[k]).bind(null, key),
						enumerable: !(desc = __getOwnPropDesc(from, key)) || desc.enumerable
					});
				}
			}
		}
		return to;
	};
	var __toESM = (mod, isNodeMode, target) => (target = mod != null ? __create(__getProtoOf(mod)) : {}, __copyProps(isNodeMode || !mod || !mod.__esModule || !__hasOwnProp.call(mod, "default") ? __defProp(target, "default", {
		value: mod,
		enumerable: true
	}) : target, mod));

// node_modules/lodash/_baseSlice.js
	var require__baseSlice = /* @__PURE__ */ __commonJSMin(((exports, module) => {
		/**
		* The base implementation of `_.slice` without an iteratee call guard.
		*
		* @private
		* @param {Array} array The array to slice.
		* @param {number} [start=0] The start position.
		* @param {number} [end=array.length] The end position.
		* @returns {Array} Returns the slice of `array`.
		*/
		function baseSlice(array, start, end) {
			var index = -1, length = array.length;
			if (start < 0) start = -start > length ? 0 : length + start;
			end = end > length ? length : end;
			if (end < 0) end += length;
			length = start > end ? 0 : end - start >>> 0;
			start >>>= 0;
			var result = Array(length);
			while (++index < length) result[index] = array[index + start];
			return result;
		}
		module.exports = baseSlice;
	}));

// node_modules/lodash/eq.js
	var require_eq = /* @__PURE__ */ __commonJSMin(((exports, module) => {
		/**
		* Performs a
		* [`SameValueZero`](http://ecma-international.org/ecma-262/7.0/#sec-samevaluezero)
		* comparison between two values to determine if they are equivalent.
		*
		* @static
		* @memberOf _
		* @since 4.0.0
		* @category Lang
		* @param {*} value The value to compare.
		* @param {*} other The other value to compare.
		* @returns {boolean} Returns `true` if the values are equivalent, else `false`.
		* @example
		*
		* var object = { 'a': 1 };
		* var other = { 'a': 1 };
		*
		* _.eq(object, object);
		* // => true
		*
		* _.eq(object, other);
		* // => false
		*
		* _.eq('a', 'a');
		* // => true
		*
		* _.eq('a', Object('a'));
		* // => false
		*
		* _.eq(NaN, NaN);
		* // => true
		*/
		function eq(value, other) {
			return value === other || value !== value && other !== other;
		}
		module.exports = eq;
	}));

// node_modules/lodash/_freeGlobal.js
	var require__freeGlobal = /* @__PURE__ */ __commonJSMin(((exports, module) => {
		/** Detect free variable `global` from Node.js. */
		var freeGlobal = typeof global == "object" && global && global.Object === Object && global;
		module.exports = freeGlobal;
	}));

// node_modules/lodash/_root.js
	var require__root = /* @__PURE__ */ __commonJSMin(((exports, module) => {
		var freeGlobal = require__freeGlobal();
		/** Detect free variable `self`. */
		var freeSelf = typeof self == "object" && self && self.Object === Object && self;
		/** Used as a reference to the global object. */
		var root = freeGlobal || freeSelf || Function("return this")();
		module.exports = root;
	}));

// node_modules/lodash/_Symbol.js
	var require__Symbol = /* @__PURE__ */ __commonJSMin(((exports, module) => {
		/** Built-in value references. */
		var Symbol = require__root().Symbol;
		module.exports = Symbol;
	}));

// node_modules/lodash/_getRawTag.js
	var require__getRawTag = /* @__PURE__ */ __commonJSMin(((exports, module) => {
		var Symbol = require__Symbol();
		/** Used for built-in method references. */
		var objectProto = Object.prototype;
		/** Used to check objects for own properties. */
		var hasOwnProperty = objectProto.hasOwnProperty;
		/**
		* Used to resolve the
		* [`toStringTag`](http://ecma-international.org/ecma-262/7.0/#sec-object.prototype.tostring)
		* of values.
		*/
		var nativeObjectToString = objectProto.toString;
		/** Built-in value references. */
		var symToStringTag = Symbol ? Symbol.toStringTag : void 0;
		/**
		* A specialized version of `baseGetTag` which ignores `Symbol.toStringTag` values.
		*
		* @private
		* @param {*} value The value to query.
		* @returns {string} Returns the raw `toStringTag`.
		*/
		function getRawTag(value) {
			var isOwn = hasOwnProperty.call(value, symToStringTag), tag = value[symToStringTag];
			try {
				value[symToStringTag] = void 0;
				var unmasked = true;
			} catch (e) {}
			var result = nativeObjectToString.call(value);
			if (unmasked) {
				if (isOwn) value[symToStringTag] = tag;
				else delete value[symToStringTag];
			}
			return result;
		}
		module.exports = getRawTag;
	}));

// node_modules/lodash/_objectToString.js
	var require__objectToString = /* @__PURE__ */ __commonJSMin(((exports, module) => {
		/**
		* Used to resolve the
		* [`toStringTag`](http://ecma-international.org/ecma-262/7.0/#sec-object.prototype.tostring)
		* of values.
		*/
		var nativeObjectToString = Object.prototype.toString;
		/**
		* Converts `value` to a string using `Object.prototype.toString`.
		*
		* @private
		* @param {*} value The value to convert.
		* @returns {string} Returns the converted string.
		*/
		function objectToString(value) {
			return nativeObjectToString.call(value);
		}
		module.exports = objectToString;
	}));

// node_modules/lodash/_baseGetTag.js
	var require__baseGetTag = /* @__PURE__ */ __commonJSMin(((exports, module) => {
		var Symbol = require__Symbol();
		var getRawTag = require__getRawTag();
		var objectToString = require__objectToString();
		/** `Object#toString` result references. */
		var nullTag = "[object Null]";
		var undefinedTag = "[object Undefined]";
		/** Built-in value references. */
		var symToStringTag = Symbol ? Symbol.toStringTag : void 0;
		/**
		* The base implementation of `getTag` without fallbacks for buggy environments.
		*
		* @private
		* @param {*} value The value to query.
		* @returns {string} Returns the `toStringTag`.
		*/
		function baseGetTag(value) {
			if (value == null) return value === void 0 ? undefinedTag : nullTag;
			return symToStringTag && symToStringTag in Object(value) ? getRawTag(value) : objectToString(value);
		}
		module.exports = baseGetTag;
	}));

// node_modules/lodash/isObject.js
	var require_isObject = /* @__PURE__ */ __commonJSMin(((exports, module) => {
		/**
		* Checks if `value` is the
		* [language type](http://www.ecma-international.org/ecma-262/7.0/#sec-ecmascript-language-types)
		* of `Object`. (e.g. arrays, functions, objects, regexes, `new Number(0)`, and `new String('')`)
		*
		* @static
		* @memberOf _
		* @since 0.1.0
		* @category Lang
		* @param {*} value The value to check.
		* @returns {boolean} Returns `true` if `value` is an object, else `false`.
		* @example
		*
		* _.isObject({});
		* // => true
		*
		* _.isObject([1, 2, 3]);
		* // => true
		*
		* _.isObject(_.noop);
		* // => true
		*
		* _.isObject(null);
		* // => false
		*/
		function isObject(value) {
			var type = typeof value;
			return value != null && (type == "object" || type == "function");
		}
		module.exports = isObject;
	}));

// node_modules/lodash/isFunction.js
	var require_isFunction = /* @__PURE__ */ __commonJSMin(((exports, module) => {
		var baseGetTag = require__baseGetTag();
		var isObject = require_isObject();
		/** `Object#toString` result references. */
		var asyncTag = "[object AsyncFunction]";
		var funcTag = "[object Function]";
		var genTag = "[object GeneratorFunction]";
		var proxyTag = "[object Proxy]";
		/**
		* Checks if `value` is classified as a `Function` object.
		*
		* @static
		* @memberOf _
		* @since 0.1.0
		* @category Lang
		* @param {*} value The value to check.
		* @returns {boolean} Returns `true` if `value` is a function, else `false`.
		* @example
		*
		* _.isFunction(_);
		* // => true
		*
		* _.isFunction(/abc/);
		* // => false
		*/
		function isFunction(value) {
			if (!isObject(value)) return false;
			var tag = baseGetTag(value);
			return tag == funcTag || tag == genTag || tag == asyncTag || tag == proxyTag;
		}
		module.exports = isFunction;
	}));

// node_modules/lodash/isLength.js
	var require_isLength = /* @__PURE__ */ __commonJSMin(((exports, module) => {
		/** Used as references for various `Number` constants. */
		var MAX_SAFE_INTEGER = 9007199254740991;
		/**
		* Checks if `value` is a valid array-like length.
		*
		* **Note:** This method is loosely based on
		* [`ToLength`](http://ecma-international.org/ecma-262/7.0/#sec-tolength).
		*
		* @static
		* @memberOf _
		* @since 4.0.0
		* @category Lang
		* @param {*} value The value to check.
		* @returns {boolean} Returns `true` if `value` is a valid length, else `false`.
		* @example
		*
		* _.isLength(3);
		* // => true
		*
		* _.isLength(Number.MIN_VALUE);
		* // => false
		*
		* _.isLength(Infinity);
		* // => false
		*
		* _.isLength('3');
		* // => false
		*/
		function isLength(value) {
			return typeof value == "number" && value > -1 && value % 1 == 0 && value <= MAX_SAFE_INTEGER;
		}
		module.exports = isLength;
	}));

// node_modules/lodash/isArrayLike.js
	var require_isArrayLike = /* @__PURE__ */ __commonJSMin(((exports, module) => {
		var isFunction = require_isFunction();
		var isLength = require_isLength();
		/**
		* Checks if `value` is array-like. A value is considered array-like if it's
		* not a function and has a `value.length` that's an integer greater than or
		* equal to `0` and less than or equal to `Number.MAX_SAFE_INTEGER`.
		*
		* @static
		* @memberOf _
		* @since 4.0.0
		* @category Lang
		* @param {*} value The value to check.
		* @returns {boolean} Returns `true` if `value` is array-like, else `false`.
		* @example
		*
		* _.isArrayLike([1, 2, 3]);
		* // => true
		*
		* _.isArrayLike(document.body.children);
		* // => true
		*
		* _.isArrayLike('abc');
		* // => true
		*
		* _.isArrayLike(_.noop);
		* // => false
		*/
		function isArrayLike(value) {
			return value != null && isLength(value.length) && !isFunction(value);
		}
		module.exports = isArrayLike;
	}));

// node_modules/lodash/_isIndex.js
	var require__isIndex = /* @__PURE__ */ __commonJSMin(((exports, module) => {
		/** Used as references for various `Number` constants. */
		var MAX_SAFE_INTEGER = 9007199254740991;
		/** Used to detect unsigned integer values. */
		var reIsUint = /^(?:0|[1-9]\d*)$/;
		/**
		* Checks if `value` is a valid array-like index.
		*
		* @private
		* @param {*} value The value to check.
		* @param {number} [length=MAX_SAFE_INTEGER] The upper bounds of a valid index.
		* @returns {boolean} Returns `true` if `value` is a valid index, else `false`.
		*/
		function isIndex(value, length) {
			var type = typeof value;
			length = length == null ? MAX_SAFE_INTEGER : length;
			return !!length && (type == "number" || type != "symbol" && reIsUint.test(value)) && value > -1 && value % 1 == 0 && value < length;
		}
		module.exports = isIndex;
	}));

// node_modules/lodash/_isIterateeCall.js
	var require__isIterateeCall = /* @__PURE__ */ __commonJSMin(((exports, module) => {
		var eq = require_eq();
		var isArrayLike = require_isArrayLike();
		var isIndex = require__isIndex();
		var isObject = require_isObject();
		/**
		* Checks if the given arguments are from an iteratee call.
		*
		* @private
		* @param {*} value The potential iteratee value argument.
		* @param {*} index The potential iteratee index or key argument.
		* @param {*} object The potential iteratee object argument.
		* @returns {boolean} Returns `true` if the arguments are from an iteratee call,
		*  else `false`.
		*/
		function isIterateeCall(value, index, object) {
			if (!isObject(object)) return false;
			var type = typeof index;
			if (type == "number" ? isArrayLike(object) && isIndex(index, object.length) : type == "string" && index in object) return eq(object[index], value);
			return false;
		}
		module.exports = isIterateeCall;
	}));

// node_modules/lodash/_trimmedEndIndex.js
	var require__trimmedEndIndex = /* @__PURE__ */ __commonJSMin(((exports, module) => {
		/** Used to match a single whitespace character. */
		var reWhitespace = /\s/;
		/**
		* Used by `_.trim` and `_.trimEnd` to get the index of the last non-whitespace
		* character of `string`.
		*
		* @private
		* @param {string} string The string to inspect.
		* @returns {number} Returns the index of the last non-whitespace character.
		*/
		function trimmedEndIndex(string) {
			var index = string.length;
			while (index-- && reWhitespace.test(string.charAt(index)));
			return index;
		}
		module.exports = trimmedEndIndex;
	}));

// node_modules/lodash/_baseTrim.js
	var require__baseTrim = /* @__PURE__ */ __commonJSMin(((exports, module) => {
		var trimmedEndIndex = require__trimmedEndIndex();
		/** Used to match leading whitespace. */
		var reTrimStart = /^\s+/;
		/**
		* The base implementation of `_.trim`.
		*
		* @private
		* @param {string} string The string to trim.
		* @returns {string} Returns the trimmed string.
		*/
		function baseTrim(string) {
			return string ? string.slice(0, trimmedEndIndex(string) + 1).replace(reTrimStart, "") : string;
		}
		module.exports = baseTrim;
	}));

// node_modules/lodash/isObjectLike.js
	var require_isObjectLike = /* @__PURE__ */ __commonJSMin(((exports, module) => {
		/**
		* Checks if `value` is object-like. A value is object-like if it's not `null`
		* and has a `typeof` result of "object".
		*
		* @static
		* @memberOf _
		* @since 4.0.0
		* @category Lang
		* @param {*} value The value to check.
		* @returns {boolean} Returns `true` if `value` is object-like, else `false`.
		* @example
		*
		* _.isObjectLike({});
		* // => true
		*
		* _.isObjectLike([1, 2, 3]);
		* // => true
		*
		* _.isObjectLike(_.noop);
		* // => false
		*
		* _.isObjectLike(null);
		* // => false
		*/
		function isObjectLike(value) {
			return value != null && typeof value == "object";
		}
		module.exports = isObjectLike;
	}));

// node_modules/lodash/isSymbol.js
	var require_isSymbol = /* @__PURE__ */ __commonJSMin(((exports, module) => {
		var baseGetTag = require__baseGetTag();
		var isObjectLike = require_isObjectLike();
		/** `Object#toString` result references. */
		var symbolTag = "[object Symbol]";
		/**
		* Checks if `value` is classified as a `Symbol` primitive or object.
		*
		* @static
		* @memberOf _
		* @since 4.0.0
		* @category Lang
		* @param {*} value The value to check.
		* @returns {boolean} Returns `true` if `value` is a symbol, else `false`.
		* @example
		*
		* _.isSymbol(Symbol.iterator);
		* // => true
		*
		* _.isSymbol('abc');
		* // => false
		*/
		function isSymbol(value) {
			return typeof value == "symbol" || isObjectLike(value) && baseGetTag(value) == symbolTag;
		}
		module.exports = isSymbol;
	}));

// node_modules/lodash/toNumber.js
	var require_toNumber = /* @__PURE__ */ __commonJSMin(((exports, module) => {
		var baseTrim = require__baseTrim();
		var isObject = require_isObject();
		var isSymbol = require_isSymbol();
		/** Used as references for various `Number` constants. */
		var NAN = NaN;
		/** Used to detect bad signed hexadecimal string values. */
		var reIsBadHex = /^[-+]0x[0-9a-f]+$/i;
		/** Used to detect binary string values. */
		var reIsBinary = /^0b[01]+$/i;
		/** Used to detect octal string values. */
		var reIsOctal = /^0o[0-7]+$/i;
		/** Built-in method references without a dependency on `root`. */
		var freeParseInt = parseInt;
		/**
		* Converts `value` to a number.
		*
		* @static
		* @memberOf _
		* @since 4.0.0
		* @category Lang
		* @param {*} value The value to process.
		* @returns {number} Returns the number.
		* @example
		*
		* _.toNumber(3.2);
		* // => 3.2
		*
		* _.toNumber(Number.MIN_VALUE);
		* // => 5e-324
		*
		* _.toNumber(Infinity);
		* // => Infinity
		*
		* _.toNumber('3.2');
		* // => 3.2
		*/
		function toNumber(value) {
			if (typeof value == "number") return value;
			if (isSymbol(value)) return NAN;
			if (isObject(value)) {
				var other = typeof value.valueOf == "function" ? value.valueOf() : value;
				value = isObject(other) ? other + "" : other;
			}
			if (typeof value != "string") return value === 0 ? value : +value;
			value = baseTrim(value);
			var isBinary = reIsBinary.test(value);
			return isBinary || reIsOctal.test(value) ? freeParseInt(value.slice(2), isBinary ? 2 : 8) : reIsBadHex.test(value) ? NAN : +value;
		}
		module.exports = toNumber;
	}));

// node_modules/lodash/toFinite.js
	var require_toFinite = /* @__PURE__ */ __commonJSMin(((exports, module) => {
		var toNumber = require_toNumber();
		/** Used as references for various `Number` constants. */
		var INFINITY = 1 / 0;
		var MAX_INTEGER = 17976931348623157e292;
		/**
		* Converts `value` to a finite number.
		*
		* @static
		* @memberOf _
		* @since 4.12.0
		* @category Lang
		* @param {*} value The value to convert.
		* @returns {number} Returns the converted number.
		* @example
		*
		* _.toFinite(3.2);
		* // => 3.2
		*
		* _.toFinite(Number.MIN_VALUE);
		* // => 5e-324
		*
		* _.toFinite(Infinity);
		* // => 1.7976931348623157e+308
		*
		* _.toFinite('3.2');
		* // => 3.2
		*/
		function toFinite(value) {
			if (!value) return value === 0 ? value : 0;
			value = toNumber(value);
			if (value === INFINITY || value === -INFINITY) return (value < 0 ? -1 : 1) * MAX_INTEGER;
			return value === value ? value : 0;
		}
		module.exports = toFinite;
	}));

// node_modules/lodash/toInteger.js
	var require_toInteger = /* @__PURE__ */ __commonJSMin(((exports, module) => {
		var toFinite = require_toFinite();
		/**
		* Converts `value` to an integer.
		*
		* **Note:** This method is loosely based on
		* [`ToInteger`](http://www.ecma-international.org/ecma-262/7.0/#sec-tointeger).
		*
		* @static
		* @memberOf _
		* @since 4.0.0
		* @category Lang
		* @param {*} value The value to convert.
		* @returns {number} Returns the converted integer.
		* @example
		*
		* _.toInteger(3.2);
		* // => 3
		*
		* _.toInteger(Number.MIN_VALUE);
		* // => 0
		*
		* _.toInteger(Infinity);
		* // => 1.7976931348623157e+308
		*
		* _.toInteger('3.2');
		* // => 3
		*/
		function toInteger(value) {
			var result = toFinite(value), remainder = result % 1;
			return result === result ? remainder ? result - remainder : result : 0;
		}
		module.exports = toInteger;
	}));

// node_modules/lodash/chunk.js
	var require_chunk = /* @__PURE__ */ __commonJSMin(((exports, module) => {
		var baseSlice = require__baseSlice();
		var isIterateeCall = require__isIterateeCall();
		var toInteger = require_toInteger();
		var nativeCeil = Math.ceil;
		var nativeMax = Math.max;
		/**
		* Creates an array of elements split into groups the length of `size`.
		* If `array` can't be split evenly, the final chunk will be the remaining
		* elements.
		*
		* @static
		* @memberOf _
		* @since 3.0.0
		* @category Array
		* @param {Array} array The array to process.
		* @param {number} [size=1] The length of each chunk
		* @param- {Object} [guard] Enables use as an iteratee for methods like `_.map`.
		* @returns {Array} Returns the new array of chunks.
		* @example
		*
		* _.chunk(['a', 'b', 'c', 'd'], 2);
		* // => [['a', 'b'], ['c', 'd']]
		*
		* _.chunk(['a', 'b', 'c', 'd'], 3);
		* // => [['a', 'b', 'c'], ['d']]
		*/
		function chunk(array, size, guard) {
			if (guard ? isIterateeCall(array, size, guard) : size === void 0) size = 1;
			else size = nativeMax(toInteger(size), 0);
			var length = array == null ? 0 : array.length;
			if (!length || size < 1) return [];
			var index = 0, resIndex = 0, result = Array(nativeCeil(length / size));
			while (index < length) result[resIndex++] = baseSlice(array, index, index += size);
			return result;
		}
		module.exports = chunk;
	}));

// .src-base-esm/data.json
	var data_default = {
		name: "fixture",
		items: [
			1,
			2,
			3,
			4,
			5
		]
	};

// .src-base-esm/cycle-b.js
	var require_cycle_b = /* @__PURE__ */ __commonJSMin(((exports) => {
		console.log("eval cycle-b");
		var a = require_cycle_a();
		exports.seen = "b saw " + a.early + "," + a.late;
		exports.check = function() {
			return "b later " + a.late;
		};
	}));

// .src-base-esm/cycle-a.js
	var require_cycle_a = /* @__PURE__ */ __commonJSMin(((exports) => {
		console.log("eval cycle-a start");
		exports.early = "a-early";
		var b = require_cycle_b();
		exports.late = "a-late";
		exports.report = function() {
			return b.seen + " " + b.check();
		};
		console.log("eval cycle-a end");
	}));

// .src-base-esm/util.js
	var import_chunk = /* @__PURE__ */ __toESM(require_chunk());
	var import_cycle_a = require_cycle_a();
	console.log("eval util");
	const label = (name) => `[${name.padStart(8, ".")}]`;
	function total(list) {
		return list.reduce((sum, n) => sum + n, 0);
	}

// .src-base-esm/lazy.js
	var lazy_exports = /* @__PURE__ */ __exportAll({
		default: () => lazy_default,
		value: () => 42
	});
	var value, lazy_default;
	var init_lazy = __esmMin((() => {
		console.log("eval lazy");
		value = 42;
		lazy_default = "lazy-default";
	}));

// .src-base-esm/index.js
	const items = data_default.items;
	console.log("eval index");
	console.log("last " + items[items.length - 1] + " own " + Object.prototype.hasOwnProperty.call(data_default, "name"));
	console.log("replace " + "a-b-c".split("-").join("+"));
	console.log("chunk " + JSON.stringify((0, import_chunk.default)(items, 2)));
	console.log("cycle " + (0, import_cycle_a.report)());
	console.log("label " + label(data_default.name) + " total " + total(items));
	Promise.resolve().then(() => (init_lazy(), lazy_exports)).then((lazy) => {
		console.log("lazy " + lazy.value + " " + lazy.default);
		return Promise.allSettled([Promise.resolve(1), Promise.reject(/* @__PURE__ */ new Error("no"))]);
	}).then((settled) => console.log("settled " + settled.map((s) => s.status).join(",")));

})();