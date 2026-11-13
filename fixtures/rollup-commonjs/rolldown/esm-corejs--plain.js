// \0rolldown/runtime.js
var __commonJSMin = (cb, mod) => () => (mod || (cb((mod = { exports: {} }).exports, mod, mod.exports), cb = null), mod.exports);

// node_modules/core-js/internals/global-this.js
var require_global_this = /* @__PURE__ */ __commonJSMin(((exports, module, this$1) => {
	var check = function(it) {
		return it && it.Math === Math && it;
	};
	module.exports = check(typeof globalThis == "object" && globalThis) || check(typeof window == "object" && window) || check(typeof self == "object" && self) || check(typeof global == "object" && global) || check(typeof this$1 == "object" && this$1) || (function() {
		return this;
	})() || Function("return this")();
}));

// node_modules/core-js/internals/is-callable.js
var require_is_callable = /* @__PURE__ */ __commonJSMin(((exports, module) => {
	var documentAll = typeof document == "object" && document.all;
	module.exports = typeof documentAll == "undefined" && documentAll !== void 0 ? function(argument) {
		return typeof argument == "function" || argument === documentAll;
	} : function(argument) {
		return typeof argument == "function";
	};
}));

// node_modules/core-js/internals/get-built-in.js
var require_get_built_in = /* @__PURE__ */ __commonJSMin(((exports, module) => {
	var globalThis = require_global_this();
	var isCallable = require_is_callable();
	var aFunction = function(argument) {
		return isCallable(argument) ? argument : void 0;
	};
	module.exports = function(namespace, method) {
		return arguments.length < 2 ? aFunction(globalThis[namespace]) : globalThis[namespace] && globalThis[namespace][method];
	};
}));

// node_modules/core-js/internals/fails.js
var require_fails = /* @__PURE__ */ __commonJSMin(((exports, module) => {
	module.exports = function(exec) {
		try {
			return !!exec();
		} catch (error) {
			return true;
		}
	};
}));

// node_modules/core-js/internals/function-bind-native.js
var require_function_bind_native = /* @__PURE__ */ __commonJSMin(((exports, module) => {
	var fails = require_fails();
	module.exports = !fails(function() {
		var test = function() {}.bind();
		return typeof test != "function" || test.hasOwnProperty("prototype");
	});
}));

// node_modules/core-js/internals/function-uncurry-this.js
var require_function_uncurry_this = /* @__PURE__ */ __commonJSMin(((exports, module) => {
	var NATIVE_BIND = require_function_bind_native();
	var FunctionPrototype = Function.prototype;
	var call = FunctionPrototype.call;
	var uncurryThisWithBind = NATIVE_BIND && FunctionPrototype.bind.bind(call, call);
	module.exports = NATIVE_BIND ? uncurryThisWithBind : function(fn) {
		return function() {
			return call.apply(fn, arguments);
		};
	};
}));

// node_modules/core-js/internals/is-null-or-undefined.js
var require_is_null_or_undefined = /* @__PURE__ */ __commonJSMin(((exports, module) => {
	module.exports = function(it) {
		return it === null || it === void 0;
	};
}));

// node_modules/core-js/internals/require-object-coercible.js
var require_require_object_coercible = /* @__PURE__ */ __commonJSMin(((exports, module) => {
	var isNullOrUndefined = require_is_null_or_undefined();
	var $TypeError = TypeError;
	module.exports = function(it) {
		if (isNullOrUndefined(it)) throw new $TypeError("Can't call method on " + it);
		return it;
	};
}));

// node_modules/core-js/internals/to-object.js
var require_to_object = /* @__PURE__ */ __commonJSMin(((exports, module) => {
	var requireObjectCoercible = require_require_object_coercible();
	var $Object = Object;
	module.exports = function(argument) {
		return $Object(requireObjectCoercible(argument));
	};
}));

// node_modules/core-js/internals/has-own-property.js
var require_has_own_property = /* @__PURE__ */ __commonJSMin(((exports, module) => {
	var uncurryThis = require_function_uncurry_this();
	var toObject = require_to_object();
	var hasOwnProperty = uncurryThis({}.hasOwnProperty);
	module.exports = Object.hasOwn || function hasOwn(it, key) {
		return hasOwnProperty(toObject(it), key);
	};
}));

// node_modules/core-js/internals/classof-raw.js
var require_classof_raw = /* @__PURE__ */ __commonJSMin(((exports, module) => {
	var uncurryThis = require_function_uncurry_this();
	var toString = uncurryThis({}.toString);
	var stringSlice = uncurryThis("".slice);
	module.exports = function(it) {
		return stringSlice(toString(it), 8, -1);
	};
}));

// node_modules/core-js/internals/indexed-object.js
var require_indexed_object = /* @__PURE__ */ __commonJSMin(((exports, module) => {
	var uncurryThis = require_function_uncurry_this();
	var fails = require_fails();
	var classof = require_classof_raw();
	var $Object = Object;
	var split = uncurryThis("".split);
	module.exports = fails(function() {
		return !$Object("z").propertyIsEnumerable(0);
	}) ? function(it) {
		return classof(it) === "String" ? split(it, "") : $Object(it);
	} : $Object;
}));

// node_modules/core-js/internals/to-indexed-object.js
var require_to_indexed_object = /* @__PURE__ */ __commonJSMin(((exports, module) => {
	var IndexedObject = require_indexed_object();
	var requireObjectCoercible = require_require_object_coercible();
	module.exports = function(it) {
		return IndexedObject(requireObjectCoercible(it));
	};
}));

// node_modules/core-js/internals/math-trunc.js
var require_math_trunc = /* @__PURE__ */ __commonJSMin(((exports, module) => {
	var ceil = Math.ceil;
	var floor = Math.floor;
	module.exports = Math.trunc || function trunc(x) {
		var n = +x;
		return (n > 0 ? floor : ceil)(n);
	};
}));

// node_modules/core-js/internals/to-integer-or-infinity.js
var require_to_integer_or_infinity = /* @__PURE__ */ __commonJSMin(((exports, module) => {
	var trunc = require_math_trunc();
	module.exports = function(argument) {
		var number = +argument;
		return number !== number || number === 0 ? 0 : trunc(number);
	};
}));

// node_modules/core-js/internals/to-absolute-index.js
var require_to_absolute_index = /* @__PURE__ */ __commonJSMin(((exports, module) => {
	var toIntegerOrInfinity = require_to_integer_or_infinity();
	var max = Math.max;
	var min = Math.min;
	module.exports = function(index, length) {
		var integer = toIntegerOrInfinity(index);
		return integer < 0 ? max(integer + length, 0) : min(integer, length);
	};
}));

// node_modules/core-js/internals/to-length.js
var require_to_length = /* @__PURE__ */ __commonJSMin(((exports, module) => {
	var toIntegerOrInfinity = require_to_integer_or_infinity();
	var min = Math.min;
	module.exports = function(argument) {
		var len = toIntegerOrInfinity(argument);
		return len > 0 ? min(len, 9007199254740991) : 0;
	};
}));

// node_modules/core-js/internals/length-of-array-like.js
var require_length_of_array_like = /* @__PURE__ */ __commonJSMin(((exports, module) => {
	var toLength = require_to_length();
	module.exports = function(obj) {
		return toLength(obj.length);
	};
}));

// node_modules/core-js/internals/array-includes.js
var require_array_includes = /* @__PURE__ */ __commonJSMin(((exports, module) => {
	var toIndexedObject = require_to_indexed_object();
	var toAbsoluteIndex = require_to_absolute_index();
	var lengthOfArrayLike = require_length_of_array_like();
	var createMethod = function(IS_INCLUDES) {
		return function($this, el, fromIndex) {
			var O = toIndexedObject($this);
			var length = lengthOfArrayLike(O);
			if (length === 0) return !IS_INCLUDES && -1;
			var index = toAbsoluteIndex(fromIndex, length);
			var value;
			if (IS_INCLUDES && el !== el) while (length > index) {
				value = O[index++];
				if (value !== value) return true;
			}
			else for (; length > index; index++) if ((IS_INCLUDES || index in O) && O[index] === el) return IS_INCLUDES || index || 0;
			return !IS_INCLUDES && -1;
		};
	};
	module.exports = {
		includes: createMethod(true),
		indexOf: createMethod(false)
	};
}));

// node_modules/core-js/internals/hidden-keys.js
var require_hidden_keys = /* @__PURE__ */ __commonJSMin(((exports, module) => {
	module.exports = {};
}));

// node_modules/core-js/internals/object-keys-internal.js
var require_object_keys_internal = /* @__PURE__ */ __commonJSMin(((exports, module) => {
	var uncurryThis = require_function_uncurry_this();
	var hasOwn = require_has_own_property();
	var toIndexedObject = require_to_indexed_object();
	var indexOf = require_array_includes().indexOf;
	var hiddenKeys = require_hidden_keys();
	var push = uncurryThis([].push);
	module.exports = function(object, names) {
		var O = toIndexedObject(object);
		var i = 0;
		var result = [];
		var key;
		for (key in O) !hasOwn(hiddenKeys, key) && hasOwn(O, key) && push(result, key);
		while (names.length > i) if (hasOwn(O, key = names[i++])) ~indexOf(result, key) || push(result, key);
		return result;
	};
}));

// node_modules/core-js/internals/enum-bug-keys.js
var require_enum_bug_keys = /* @__PURE__ */ __commonJSMin(((exports, module) => {
	module.exports = [
		"constructor",
		"hasOwnProperty",
		"isPrototypeOf",
		"propertyIsEnumerable",
		"toLocaleString",
		"toString",
		"valueOf"
	];
}));

// node_modules/core-js/internals/object-get-own-property-names.js
var require_object_get_own_property_names = /* @__PURE__ */ __commonJSMin(((exports) => {
	var internalObjectKeys = require_object_keys_internal();
	var hiddenKeys = require_enum_bug_keys().concat("length", "prototype");
	exports.f = Object.getOwnPropertyNames || function getOwnPropertyNames(O) {
		return internalObjectKeys(O, hiddenKeys);
	};
}));

// node_modules/core-js/internals/object-get-own-property-symbols.js
var require_object_get_own_property_symbols = /* @__PURE__ */ __commonJSMin(((exports) => {
	exports.f = Object.getOwnPropertySymbols;
}));

// node_modules/core-js/internals/is-object.js
var require_is_object = /* @__PURE__ */ __commonJSMin(((exports, module) => {
	var isCallable = require_is_callable();
	module.exports = function(it) {
		return typeof it == "object" ? it !== null : isCallable(it);
	};
}));

// node_modules/core-js/internals/an-object.js
var require_an_object = /* @__PURE__ */ __commonJSMin(((exports, module) => {
	var isObject = require_is_object();
	var $String = String;
	var $TypeError = TypeError;
	module.exports = function(argument) {
		if (isObject(argument)) return argument;
		throw new $TypeError($String(argument) + " is not an object");
	};
}));

// node_modules/core-js/internals/own-keys.js
var require_own_keys = /* @__PURE__ */ __commonJSMin(((exports, module) => {
	var getBuiltIn = require_get_built_in();
	var uncurryThis = require_function_uncurry_this();
	var getOwnPropertyNamesModule = require_object_get_own_property_names();
	var getOwnPropertySymbolsModule = require_object_get_own_property_symbols();
	var anObject = require_an_object();
	var concat = uncurryThis([].concat);
	module.exports = getBuiltIn("Reflect", "ownKeys") || function ownKeys(it) {
		var keys = getOwnPropertyNamesModule.f(anObject(it));
		var getOwnPropertySymbols = getOwnPropertySymbolsModule.f;
		return getOwnPropertySymbols ? concat(keys, getOwnPropertySymbols(it)) : keys;
	};
}));

// apps/corejs/index.js
var ownKeys = require_own_keys();
var getBuiltIn = require_get_built_in();
var keys = ownKeys({
	a: 1,
	b: 2
});
console.log("own-keys " + keys.join(","));
console.log("built-in " + typeof getBuiltIn("Reflect", "ownKeys") + " " + typeof getBuiltIn("Array"));
