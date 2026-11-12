(function () {
	'use strict';

	var commonjsGlobal = typeof globalThis !== 'undefined' ? globalThis : typeof window !== 'undefined' ? window : typeof global !== 'undefined' ? global : typeof self !== 'undefined' ? self : {};

	var es_array_at = {};

	var globalThis_1;
	var hasRequiredGlobalThis;

	function requireGlobalThis () {
		if (hasRequiredGlobalThis) return globalThis_1;
		hasRequiredGlobalThis = 1;
		var check = function (it) {
		  return it && it.Math === Math && it;
		};

		// https://github.com/zloirock/core-js/issues/86#issuecomment-115759028
		globalThis_1 =
		  // eslint-disable-next-line es/no-global-this -- safe
		  check(typeof globalThis == 'object' && globalThis) ||
		  check(typeof window == 'object' && window) ||
		  // eslint-disable-next-line no-restricted-globals -- safe
		  check(typeof self == 'object' && self) ||
		  check(typeof commonjsGlobal == 'object' && commonjsGlobal) ||
		  check(typeof commonjsGlobal == 'object' && commonjsGlobal) ||
		  // eslint-disable-next-line no-new-func -- fallback
		  (function () { return this; })() || Function('return this')();
		return globalThis_1;
	}

	var objectGetOwnPropertyDescriptor = {};

	var fails;
	var hasRequiredFails;

	function requireFails () {
		if (hasRequiredFails) return fails;
		hasRequiredFails = 1;
		fails = function (exec) {
		  try {
		    return !!exec();
		  } catch (error) {
		    return true;
		  }
		};
		return fails;
	}

	var descriptors;
	var hasRequiredDescriptors;

	function requireDescriptors () {
		if (hasRequiredDescriptors) return descriptors;
		hasRequiredDescriptors = 1;
		var fails = requireFails();

		// Detect IE8's incomplete defineProperty implementation
		descriptors = !fails(function () {
		  // eslint-disable-next-line es/no-object-defineproperty -- required for testing
		  return Object.defineProperty({}, 1, { get: function () { return 7; } })[1] !== 7;
		});
		return descriptors;
	}

	var functionBindNative;
	var hasRequiredFunctionBindNative;

	function requireFunctionBindNative () {
		if (hasRequiredFunctionBindNative) return functionBindNative;
		hasRequiredFunctionBindNative = 1;
		var fails = requireFails();

		functionBindNative = !fails(function () {
		  // eslint-disable-next-line es/no-function-prototype-bind -- safe
		  var test = function () { /* empty */ }.bind();
		  // eslint-disable-next-line no-prototype-builtins -- safe
		  return typeof test != 'function' || test.hasOwnProperty('prototype');
		});
		return functionBindNative;
	}

	var functionCall;
	var hasRequiredFunctionCall;

	function requireFunctionCall () {
		if (hasRequiredFunctionCall) return functionCall;
		hasRequiredFunctionCall = 1;
		var NATIVE_BIND = requireFunctionBindNative();

		var call = Function.prototype.call;
		// eslint-disable-next-line es/no-function-prototype-bind -- safe
		functionCall = NATIVE_BIND ? call.bind(call) : function () {
		  return call.apply(call, arguments);
		};
		return functionCall;
	}

	var objectPropertyIsEnumerable = {};

	var hasRequiredObjectPropertyIsEnumerable;

	function requireObjectPropertyIsEnumerable () {
		if (hasRequiredObjectPropertyIsEnumerable) return objectPropertyIsEnumerable;
		hasRequiredObjectPropertyIsEnumerable = 1;
		var $propertyIsEnumerable = {}.propertyIsEnumerable;
		// eslint-disable-next-line es/no-object-getownpropertydescriptor -- safe
		var getOwnPropertyDescriptor = Object.getOwnPropertyDescriptor;

		// Nashorn ~ JDK8 bug
		var NASHORN_BUG = getOwnPropertyDescriptor && !$propertyIsEnumerable.call({ 1: 2 }, 1);

		// `Object.prototype.propertyIsEnumerable` method implementation
		// https://tc39.es/ecma262/#sec-object.prototype.propertyisenumerable
		objectPropertyIsEnumerable.f = NASHORN_BUG ? function propertyIsEnumerable(V) {
		  var descriptor = getOwnPropertyDescriptor(this, V);
		  return !!descriptor && descriptor.enumerable;
		} : $propertyIsEnumerable;
		return objectPropertyIsEnumerable;
	}

	var createPropertyDescriptor;
	var hasRequiredCreatePropertyDescriptor;

	function requireCreatePropertyDescriptor () {
		if (hasRequiredCreatePropertyDescriptor) return createPropertyDescriptor;
		hasRequiredCreatePropertyDescriptor = 1;
		createPropertyDescriptor = function (bitmap, value) {
		  return {
		    enumerable: !(bitmap & 1),
		    configurable: !(bitmap & 2),
		    writable: !(bitmap & 4),
		    value: value
		  };
		};
		return createPropertyDescriptor;
	}

	var functionUncurryThis;
	var hasRequiredFunctionUncurryThis;

	function requireFunctionUncurryThis () {
		if (hasRequiredFunctionUncurryThis) return functionUncurryThis;
		hasRequiredFunctionUncurryThis = 1;
		var NATIVE_BIND = requireFunctionBindNative();

		var FunctionPrototype = Function.prototype;
		var call = FunctionPrototype.call;
		// eslint-disable-next-line es/no-function-prototype-bind -- safe
		var uncurryThisWithBind = NATIVE_BIND && FunctionPrototype.bind.bind(call, call);

		functionUncurryThis = NATIVE_BIND ? uncurryThisWithBind : function (fn) {
		  return function () {
		    return call.apply(fn, arguments);
		  };
		};
		return functionUncurryThis;
	}

	var classofRaw;
	var hasRequiredClassofRaw;

	function requireClassofRaw () {
		if (hasRequiredClassofRaw) return classofRaw;
		hasRequiredClassofRaw = 1;
		var uncurryThis = requireFunctionUncurryThis();

		var toString = uncurryThis({}.toString);
		var stringSlice = uncurryThis(''.slice);

		classofRaw = function (it) {
		  return stringSlice(toString(it), 8, -1);
		};
		return classofRaw;
	}

	var indexedObject;
	var hasRequiredIndexedObject;

	function requireIndexedObject () {
		if (hasRequiredIndexedObject) return indexedObject;
		hasRequiredIndexedObject = 1;
		var uncurryThis = requireFunctionUncurryThis();
		var fails = requireFails();
		var classof = requireClassofRaw();

		var $Object = Object;
		var split = uncurryThis(''.split);

		// fallback for non-array-like ES3 and non-enumerable old V8 strings
		indexedObject = fails(function () {
		  // throws an error in rhino, see https://github.com/mozilla/rhino/issues/346
		  // eslint-disable-next-line no-prototype-builtins -- safe
		  return !$Object('z').propertyIsEnumerable(0);
		}) ? function (it) {
		  return classof(it) === 'String' ? split(it, '') : $Object(it);
		} : $Object;
		return indexedObject;
	}

	var isNullOrUndefined;
	var hasRequiredIsNullOrUndefined;

	function requireIsNullOrUndefined () {
		if (hasRequiredIsNullOrUndefined) return isNullOrUndefined;
		hasRequiredIsNullOrUndefined = 1;
		// we can't use just `it == null` since of `document.all` special case
		// https://tc39.es/ecma262/#sec-IsHTMLDDA-internal-slot-aec
		isNullOrUndefined = function (it) {
		  return it === null || it === undefined;
		};
		return isNullOrUndefined;
	}

	var requireObjectCoercible;
	var hasRequiredRequireObjectCoercible;

	function requireRequireObjectCoercible () {
		if (hasRequiredRequireObjectCoercible) return requireObjectCoercible;
		hasRequiredRequireObjectCoercible = 1;
		var isNullOrUndefined = requireIsNullOrUndefined();

		var $TypeError = TypeError;

		// `RequireObjectCoercible` abstract operation
		// https://tc39.es/ecma262/#sec-requireobjectcoercible
		requireObjectCoercible = function (it) {
		  if (isNullOrUndefined(it)) throw new $TypeError("Can't call method on " + it);
		  return it;
		};
		return requireObjectCoercible;
	}

	var toIndexedObject;
	var hasRequiredToIndexedObject;

	function requireToIndexedObject () {
		if (hasRequiredToIndexedObject) return toIndexedObject;
		hasRequiredToIndexedObject = 1;
		// toObject with fallback for non-array-like ES3 strings
		var IndexedObject = requireIndexedObject();
		var requireObjectCoercible = requireRequireObjectCoercible();

		toIndexedObject = function (it) {
		  return IndexedObject(requireObjectCoercible(it));
		};
		return toIndexedObject;
	}

	var isCallable;
	var hasRequiredIsCallable;

	function requireIsCallable () {
		if (hasRequiredIsCallable) return isCallable;
		hasRequiredIsCallable = 1;
		// https://tc39.es/ecma262/#sec-IsHTMLDDA-internal-slot
		var documentAll = typeof document == 'object' && document.all;

		// `IsCallable` abstract operation
		// https://tc39.es/ecma262/#sec-iscallable
		// eslint-disable-next-line unicorn/no-typeof-undefined -- required for testing
		isCallable = typeof documentAll == 'undefined' && documentAll !== undefined ? function (argument) {
		  return typeof argument == 'function' || argument === documentAll;
		} : function (argument) {
		  return typeof argument == 'function';
		};
		return isCallable;
	}

	var isObject;
	var hasRequiredIsObject;

	function requireIsObject () {
		if (hasRequiredIsObject) return isObject;
		hasRequiredIsObject = 1;
		var isCallable = requireIsCallable();

		isObject = function (it) {
		  return typeof it == 'object' ? it !== null : isCallable(it);
		};
		return isObject;
	}

	var getBuiltIn;
	var hasRequiredGetBuiltIn;

	function requireGetBuiltIn () {
		if (hasRequiredGetBuiltIn) return getBuiltIn;
		hasRequiredGetBuiltIn = 1;
		var globalThis = requireGlobalThis();
		var isCallable = requireIsCallable();

		var aFunction = function (argument) {
		  return isCallable(argument) ? argument : undefined;
		};

		getBuiltIn = function (namespace, method) {
		  return arguments.length < 2 ? aFunction(globalThis[namespace]) : globalThis[namespace] && globalThis[namespace][method];
		};
		return getBuiltIn;
	}

	var objectIsPrototypeOf;
	var hasRequiredObjectIsPrototypeOf;

	function requireObjectIsPrototypeOf () {
		if (hasRequiredObjectIsPrototypeOf) return objectIsPrototypeOf;
		hasRequiredObjectIsPrototypeOf = 1;
		var uncurryThis = requireFunctionUncurryThis();

		objectIsPrototypeOf = uncurryThis({}.isPrototypeOf);
		return objectIsPrototypeOf;
	}

	var environmentUserAgent;
	var hasRequiredEnvironmentUserAgent;

	function requireEnvironmentUserAgent () {
		if (hasRequiredEnvironmentUserAgent) return environmentUserAgent;
		hasRequiredEnvironmentUserAgent = 1;
		var globalThis = requireGlobalThis();

		var navigator = globalThis.navigator;
		var userAgent = navigator && navigator.userAgent;

		environmentUserAgent = userAgent ? String(userAgent) : '';
		return environmentUserAgent;
	}

	var environmentV8Version;
	var hasRequiredEnvironmentV8Version;

	function requireEnvironmentV8Version () {
		if (hasRequiredEnvironmentV8Version) return environmentV8Version;
		hasRequiredEnvironmentV8Version = 1;
		var globalThis = requireGlobalThis();
		var userAgent = requireEnvironmentUserAgent();

		var process = globalThis.process;
		var Deno = globalThis.Deno;
		var versions = process && process.versions || Deno && Deno.version;
		var v8 = versions && versions.v8;
		var match, version;

		if (v8) {
		  match = v8.split('.');
		  // in old Chrome, versions of V8 isn't V8 = Chrome / 10
		  // but their correct versions are not interesting for us
		  version = match[0] > 0 && match[0] < 4 ? 1 : +(match[0] + match[1]);
		}

		// BrowserFS NodeJS `process` polyfill incorrectly set `.v8` to `0.0`
		// so check `userAgent` even if `.v8` exists, but 0
		if (!version && userAgent) {
		  match = userAgent.match(/Edge\/(\d+)/);
		  if (!match || match[1] >= 74) {
		    match = userAgent.match(/Chrome\/(\d+)/);
		    if (match) version = +match[1];
		  }
		}

		environmentV8Version = version;
		return environmentV8Version;
	}

	var symbolConstructorDetection;
	var hasRequiredSymbolConstructorDetection;

	function requireSymbolConstructorDetection () {
		if (hasRequiredSymbolConstructorDetection) return symbolConstructorDetection;
		hasRequiredSymbolConstructorDetection = 1;
		/* eslint-disable es/no-symbol -- required for testing */
		var V8_VERSION = requireEnvironmentV8Version();
		var fails = requireFails();
		var globalThis = requireGlobalThis();

		var $String = globalThis.String;

		// eslint-disable-next-line es/no-object-getownpropertysymbols -- required for testing
		symbolConstructorDetection = !!Object.getOwnPropertySymbols && !fails(function () {
		  var symbol = Symbol('symbol detection');
		  // Chrome 38 Symbol has incorrect toString conversion
		  // `get-own-property-symbols` polyfill symbols converted to object are not Symbol instances
		  // nb: Do not call `String` directly to avoid this being optimized out to `symbol+''` which will,
		  // of course, fail.
		  return !$String(symbol) || !(Object(symbol) instanceof Symbol) ||
		    // Chrome 38-40 symbols are not inherited from DOM collections prototypes to instances
		    !Symbol.sham && V8_VERSION && V8_VERSION < 41;
		});
		return symbolConstructorDetection;
	}

	var useSymbolAsUid;
	var hasRequiredUseSymbolAsUid;

	function requireUseSymbolAsUid () {
		if (hasRequiredUseSymbolAsUid) return useSymbolAsUid;
		hasRequiredUseSymbolAsUid = 1;
		/* eslint-disable es/no-symbol -- required for testing */
		var NATIVE_SYMBOL = requireSymbolConstructorDetection();

		useSymbolAsUid = NATIVE_SYMBOL &&
		  !Symbol.sham &&
		  typeof Symbol.iterator == 'symbol';
		return useSymbolAsUid;
	}

	var isSymbol;
	var hasRequiredIsSymbol;

	function requireIsSymbol () {
		if (hasRequiredIsSymbol) return isSymbol;
		hasRequiredIsSymbol = 1;
		var getBuiltIn = requireGetBuiltIn();
		var isCallable = requireIsCallable();
		var isPrototypeOf = requireObjectIsPrototypeOf();
		var USE_SYMBOL_AS_UID = requireUseSymbolAsUid();

		var $Object = Object;

		isSymbol = USE_SYMBOL_AS_UID ? function (it) {
		  return typeof it == 'symbol';
		} : function (it) {
		  var $Symbol = getBuiltIn('Symbol');
		  return isCallable($Symbol) && isPrototypeOf($Symbol.prototype, $Object(it));
		};
		return isSymbol;
	}

	var tryToString;
	var hasRequiredTryToString;

	function requireTryToString () {
		if (hasRequiredTryToString) return tryToString;
		hasRequiredTryToString = 1;
		var $String = String;

		tryToString = function (argument) {
		  try {
		    return $String(argument);
		  } catch (error) {
		    return 'Object';
		  }
		};
		return tryToString;
	}

	var aCallable;
	var hasRequiredACallable;

	function requireACallable () {
		if (hasRequiredACallable) return aCallable;
		hasRequiredACallable = 1;
		var isCallable = requireIsCallable();
		var tryToString = requireTryToString();

		var $TypeError = TypeError;

		// `Assert: IsCallable(argument) is true`
		aCallable = function (argument) {
		  if (isCallable(argument)) return argument;
		  throw new $TypeError(tryToString(argument) + ' is not a function');
		};
		return aCallable;
	}

	var getMethod;
	var hasRequiredGetMethod;

	function requireGetMethod () {
		if (hasRequiredGetMethod) return getMethod;
		hasRequiredGetMethod = 1;
		var aCallable = requireACallable();
		var isNullOrUndefined = requireIsNullOrUndefined();

		// `GetMethod` abstract operation
		// https://tc39.es/ecma262/#sec-getmethod
		getMethod = function (V, P) {
		  var func = V[P];
		  return isNullOrUndefined(func) ? undefined : aCallable(func);
		};
		return getMethod;
	}

	var ordinaryToPrimitive;
	var hasRequiredOrdinaryToPrimitive;

	function requireOrdinaryToPrimitive () {
		if (hasRequiredOrdinaryToPrimitive) return ordinaryToPrimitive;
		hasRequiredOrdinaryToPrimitive = 1;
		var call = requireFunctionCall();
		var isCallable = requireIsCallable();
		var isObject = requireIsObject();

		var $TypeError = TypeError;

		// `OrdinaryToPrimitive` abstract operation
		// https://tc39.es/ecma262/#sec-ordinarytoprimitive
		ordinaryToPrimitive = function (input, pref) {
		  var fn, val;
		  if (pref === 'string' && isCallable(fn = input.toString) && !isObject(val = call(fn, input))) return val;
		  if (isCallable(fn = input.valueOf) && !isObject(val = call(fn, input))) return val;
		  if (pref !== 'string' && isCallable(fn = input.toString) && !isObject(val = call(fn, input))) return val;
		  throw new $TypeError("Can't convert object to primitive value");
		};
		return ordinaryToPrimitive;
	}

	var sharedStore = {exports: {}};

	var isPure;
	var hasRequiredIsPure;

	function requireIsPure () {
		if (hasRequiredIsPure) return isPure;
		hasRequiredIsPure = 1;
		isPure = false;
		return isPure;
	}

	var defineGlobalProperty;
	var hasRequiredDefineGlobalProperty;

	function requireDefineGlobalProperty () {
		if (hasRequiredDefineGlobalProperty) return defineGlobalProperty;
		hasRequiredDefineGlobalProperty = 1;
		var globalThis = requireGlobalThis();

		// eslint-disable-next-line es/no-object-defineproperty -- safe
		var defineProperty = Object.defineProperty;

		defineGlobalProperty = function (key, value) {
		  try {
		    defineProperty(globalThis, key, { value: value, configurable: true, writable: true });
		  } catch (error) {
		    globalThis[key] = value;
		  } return value;
		};
		return defineGlobalProperty;
	}

	var hasRequiredSharedStore;

	function requireSharedStore () {
		if (hasRequiredSharedStore) return sharedStore.exports;
		hasRequiredSharedStore = 1;
		var IS_PURE = requireIsPure();
		var globalThis = requireGlobalThis();
		var defineGlobalProperty = requireDefineGlobalProperty();

		var SHARED = '__core-js_shared__';
		var store = sharedStore.exports = globalThis[SHARED] || defineGlobalProperty(SHARED, {});

		(store.versions || (store.versions = [])).push({
		  version: '3.51.0',
		  mode: IS_PURE ? 'pure' : 'global',
		  copyright: '© 2013–2025 Denis Pushkarev (zloirock.ru), 2025–2026 CoreJS Company (core-js.io). All rights reserved.',
		  license: 'https://github.com/zloirock/core-js/blob/v3.51.0/LICENSE',
		  source: 'https://github.com/zloirock/core-js'
		});
		return sharedStore.exports;
	}

	var shared;
	var hasRequiredShared;

	function requireShared () {
		if (hasRequiredShared) return shared;
		hasRequiredShared = 1;
		var store = requireSharedStore();
		// eslint-disable-next-line es/no-object-create -- safe
		var create = Object.create || Object;

		shared = function (key, value) {
		  return store[key] || (store[key] = value || create(null));
		};
		return shared;
	}

	var toObject;
	var hasRequiredToObject;

	function requireToObject () {
		if (hasRequiredToObject) return toObject;
		hasRequiredToObject = 1;
		var requireObjectCoercible = requireRequireObjectCoercible();

		var $Object = Object;

		// `ToObject` abstract operation
		// https://tc39.es/ecma262/#sec-toobject
		toObject = function (argument) {
		  return $Object(requireObjectCoercible(argument));
		};
		return toObject;
	}

	var hasOwnProperty_1;
	var hasRequiredHasOwnProperty;

	function requireHasOwnProperty () {
		if (hasRequiredHasOwnProperty) return hasOwnProperty_1;
		hasRequiredHasOwnProperty = 1;
		var uncurryThis = requireFunctionUncurryThis();
		var toObject = requireToObject();

		var hasOwnProperty = uncurryThis({}.hasOwnProperty);

		// `HasOwnProperty` abstract operation
		// https://tc39.es/ecma262/#sec-hasownproperty
		// eslint-disable-next-line es/no-object-hasown -- safe
		hasOwnProperty_1 = Object.hasOwn || function hasOwn(it, key) {
		  return hasOwnProperty(toObject(it), key);
		};
		return hasOwnProperty_1;
	}

	var uid;
	var hasRequiredUid;

	function requireUid () {
		if (hasRequiredUid) return uid;
		hasRequiredUid = 1;
		var uncurryThis = requireFunctionUncurryThis();

		var id = 0;
		var postfix = Math.random();
		var toString = uncurryThis(1.1.toString);

		uid = function (key) {
		  return 'Symbol(' + (key === undefined ? '' : key) + ')_' + toString(++id + postfix, 36);
		};
		return uid;
	}

	var wellKnownSymbol;
	var hasRequiredWellKnownSymbol;

	function requireWellKnownSymbol () {
		if (hasRequiredWellKnownSymbol) return wellKnownSymbol;
		hasRequiredWellKnownSymbol = 1;
		var globalThis = requireGlobalThis();
		var shared = requireShared();
		var hasOwn = requireHasOwnProperty();
		var uid = requireUid();
		var NATIVE_SYMBOL = requireSymbolConstructorDetection();
		var USE_SYMBOL_AS_UID = requireUseSymbolAsUid();

		var Symbol = globalThis.Symbol;
		var WellKnownSymbolsStore = shared('wks');
		var createWellKnownSymbol = USE_SYMBOL_AS_UID ? Symbol['for'] || Symbol : Symbol && Symbol.withoutSetter || uid;

		wellKnownSymbol = function (name) {
		  if (!hasOwn(WellKnownSymbolsStore, name)) {
		    WellKnownSymbolsStore[name] = NATIVE_SYMBOL && hasOwn(Symbol, name)
		      ? Symbol[name]
		      : createWellKnownSymbol('Symbol.' + name);
		  } return WellKnownSymbolsStore[name];
		};
		return wellKnownSymbol;
	}

	var toPrimitive;
	var hasRequiredToPrimitive;

	function requireToPrimitive () {
		if (hasRequiredToPrimitive) return toPrimitive;
		hasRequiredToPrimitive = 1;
		var call = requireFunctionCall();
		var isObject = requireIsObject();
		var isSymbol = requireIsSymbol();
		var getMethod = requireGetMethod();
		var ordinaryToPrimitive = requireOrdinaryToPrimitive();
		var wellKnownSymbol = requireWellKnownSymbol();

		var $TypeError = TypeError;
		var TO_PRIMITIVE = wellKnownSymbol('toPrimitive');

		// `ToPrimitive` abstract operation
		// https://tc39.es/ecma262/#sec-toprimitive
		toPrimitive = function (input, pref) {
		  if (!isObject(input) || isSymbol(input)) return input;
		  var exoticToPrim = getMethod(input, TO_PRIMITIVE);
		  var result;
		  if (exoticToPrim) {
		    if (pref === undefined) pref = 'default';
		    result = call(exoticToPrim, input, pref);
		    if (!isObject(result) || isSymbol(result)) return result;
		    throw new $TypeError("Can't convert object to primitive value");
		  }
		  if (pref === undefined) pref = 'number';
		  return ordinaryToPrimitive(input, pref);
		};
		return toPrimitive;
	}

	var toPropertyKey;
	var hasRequiredToPropertyKey;

	function requireToPropertyKey () {
		if (hasRequiredToPropertyKey) return toPropertyKey;
		hasRequiredToPropertyKey = 1;
		var toPrimitive = requireToPrimitive();
		var isSymbol = requireIsSymbol();

		// `ToPropertyKey` abstract operation
		// https://tc39.es/ecma262/#sec-topropertykey
		toPropertyKey = function (argument) {
		  var key = toPrimitive(argument, 'string');
		  return isSymbol(key) ? key : key + '';
		};
		return toPropertyKey;
	}

	var documentCreateElement;
	var hasRequiredDocumentCreateElement;

	function requireDocumentCreateElement () {
		if (hasRequiredDocumentCreateElement) return documentCreateElement;
		hasRequiredDocumentCreateElement = 1;
		var globalThis = requireGlobalThis();
		var isObject = requireIsObject();

		var document = globalThis.document;
		// typeof document.createElement is 'object' in old IE
		var EXISTS = isObject(document) && isObject(document.createElement);

		documentCreateElement = function (it) {
		  return EXISTS ? document.createElement(it) : {};
		};
		return documentCreateElement;
	}

	var ie8DomDefine;
	var hasRequiredIe8DomDefine;

	function requireIe8DomDefine () {
		if (hasRequiredIe8DomDefine) return ie8DomDefine;
		hasRequiredIe8DomDefine = 1;
		var DESCRIPTORS = requireDescriptors();
		var fails = requireFails();
		var createElement = requireDocumentCreateElement();

		// Thanks to IE8 for its funny defineProperty
		ie8DomDefine = !DESCRIPTORS && !fails(function () {
		  // eslint-disable-next-line es/no-object-defineproperty -- required for testing
		  return Object.defineProperty(createElement('div'), 'a', {
		    get: function () { return 7; }
		  }).a !== 7;
		});
		return ie8DomDefine;
	}

	var hasRequiredObjectGetOwnPropertyDescriptor;

	function requireObjectGetOwnPropertyDescriptor () {
		if (hasRequiredObjectGetOwnPropertyDescriptor) return objectGetOwnPropertyDescriptor;
		hasRequiredObjectGetOwnPropertyDescriptor = 1;
		var DESCRIPTORS = requireDescriptors();
		var call = requireFunctionCall();
		var propertyIsEnumerableModule = requireObjectPropertyIsEnumerable();
		var createPropertyDescriptor = requireCreatePropertyDescriptor();
		var toIndexedObject = requireToIndexedObject();
		var toPropertyKey = requireToPropertyKey();
		var hasOwn = requireHasOwnProperty();
		var IE8_DOM_DEFINE = requireIe8DomDefine();

		// eslint-disable-next-line es/no-object-getownpropertydescriptor -- safe
		var $getOwnPropertyDescriptor = Object.getOwnPropertyDescriptor;

		// `Object.getOwnPropertyDescriptor` method
		// https://tc39.es/ecma262/#sec-object.getownpropertydescriptor
		objectGetOwnPropertyDescriptor.f = DESCRIPTORS ? $getOwnPropertyDescriptor : function getOwnPropertyDescriptor(O, P) {
		  O = toIndexedObject(O);
		  P = toPropertyKey(P);
		  if (IE8_DOM_DEFINE) try {
		    return $getOwnPropertyDescriptor(O, P);
		  } catch (error) { /* empty */ }
		  if (hasOwn(O, P)) return createPropertyDescriptor(!call(propertyIsEnumerableModule.f, O, P), O[P]);
		};
		return objectGetOwnPropertyDescriptor;
	}

	var objectDefineProperty = {};

	var v8PrototypeDefineBug;
	var hasRequiredV8PrototypeDefineBug;

	function requireV8PrototypeDefineBug () {
		if (hasRequiredV8PrototypeDefineBug) return v8PrototypeDefineBug;
		hasRequiredV8PrototypeDefineBug = 1;
		var DESCRIPTORS = requireDescriptors();
		var fails = requireFails();

		// V8 ~ Chrome 36-
		// https://bugs.chromium.org/p/v8/issues/detail?id=3334
		v8PrototypeDefineBug = DESCRIPTORS && fails(function () {
		  // eslint-disable-next-line es/no-object-defineproperty -- required for testing
		  return Object.defineProperty(function () { /* empty */ }, 'prototype', {
		    value: 42,
		    writable: false
		  }).prototype !== 42;
		});
		return v8PrototypeDefineBug;
	}

	var anObject;
	var hasRequiredAnObject;

	function requireAnObject () {
		if (hasRequiredAnObject) return anObject;
		hasRequiredAnObject = 1;
		var isObject = requireIsObject();

		var $String = String;
		var $TypeError = TypeError;

		// `Assert: Type(argument) is Object`
		anObject = function (argument) {
		  if (isObject(argument)) return argument;
		  throw new $TypeError($String(argument) + ' is not an object');
		};
		return anObject;
	}

	var hasRequiredObjectDefineProperty;

	function requireObjectDefineProperty () {
		if (hasRequiredObjectDefineProperty) return objectDefineProperty;
		hasRequiredObjectDefineProperty = 1;
		var DESCRIPTORS = requireDescriptors();
		var IE8_DOM_DEFINE = requireIe8DomDefine();
		var V8_PROTOTYPE_DEFINE_BUG = requireV8PrototypeDefineBug();
		var anObject = requireAnObject();
		var toPropertyKey = requireToPropertyKey();

		var $TypeError = TypeError;
		// eslint-disable-next-line es/no-object-defineproperty -- safe
		var $defineProperty = Object.defineProperty;
		// eslint-disable-next-line es/no-object-getownpropertydescriptor -- safe
		var $getOwnPropertyDescriptor = Object.getOwnPropertyDescriptor;
		var ENUMERABLE = 'enumerable';
		var CONFIGURABLE = 'configurable';
		var WRITABLE = 'writable';

		// `Object.defineProperty` method
		// https://tc39.es/ecma262/#sec-object.defineproperty
		objectDefineProperty.f = DESCRIPTORS ? V8_PROTOTYPE_DEFINE_BUG ? function defineProperty(O, P, Attributes) {
		  anObject(O);
		  P = toPropertyKey(P);
		  anObject(Attributes);
		  if (typeof O === 'function' && P === 'prototype' && 'value' in Attributes && WRITABLE in Attributes && !Attributes[WRITABLE]) {
		    var current = $getOwnPropertyDescriptor(O, P);
		    if (current && current[WRITABLE]) {
		      O[P] = Attributes.value;
		      Attributes = {
		        configurable: CONFIGURABLE in Attributes ? Attributes[CONFIGURABLE] : current[CONFIGURABLE],
		        enumerable: ENUMERABLE in Attributes ? Attributes[ENUMERABLE] : current[ENUMERABLE],
		        writable: false
		      };
		    }
		  } return $defineProperty(O, P, Attributes);
		} : $defineProperty : function defineProperty(O, P, Attributes) {
		  anObject(O);
		  P = toPropertyKey(P);
		  anObject(Attributes);
		  if (IE8_DOM_DEFINE) try {
		    return $defineProperty(O, P, Attributes);
		  } catch (error) { /* empty */ }
		  if ('get' in Attributes || 'set' in Attributes) throw new $TypeError('Accessors not supported');
		  if ('value' in Attributes) O[P] = Attributes.value;
		  return O;
		};
		return objectDefineProperty;
	}

	var createNonEnumerableProperty;
	var hasRequiredCreateNonEnumerableProperty;

	function requireCreateNonEnumerableProperty () {
		if (hasRequiredCreateNonEnumerableProperty) return createNonEnumerableProperty;
		hasRequiredCreateNonEnumerableProperty = 1;
		var DESCRIPTORS = requireDescriptors();
		var definePropertyModule = requireObjectDefineProperty();
		var createPropertyDescriptor = requireCreatePropertyDescriptor();

		createNonEnumerableProperty = DESCRIPTORS ? function (object, key, value) {
		  return definePropertyModule.f(object, key, createPropertyDescriptor(1, value));
		} : function (object, key, value) {
		  object[key] = value;
		  return object;
		};
		return createNonEnumerableProperty;
	}

	var makeBuiltIn = {exports: {}};

	var functionName;
	var hasRequiredFunctionName;

	function requireFunctionName () {
		if (hasRequiredFunctionName) return functionName;
		hasRequiredFunctionName = 1;
		var DESCRIPTORS = requireDescriptors();
		var hasOwn = requireHasOwnProperty();

		var FunctionPrototype = Function.prototype;
		// eslint-disable-next-line es/no-object-getownpropertydescriptor -- safe
		var getDescriptor = DESCRIPTORS && Object.getOwnPropertyDescriptor;

		var EXISTS = hasOwn(FunctionPrototype, 'name');
		// additional protection from minified / mangled / dropped function names
		var PROPER = EXISTS && function something() { /* empty */ }.name === 'something';
		var CONFIGURABLE = EXISTS && (!DESCRIPTORS || (DESCRIPTORS && getDescriptor(FunctionPrototype, 'name').configurable));

		functionName = {
		  EXISTS: EXISTS,
		  PROPER: PROPER,
		  CONFIGURABLE: CONFIGURABLE
		};
		return functionName;
	}

	var inspectSource;
	var hasRequiredInspectSource;

	function requireInspectSource () {
		if (hasRequiredInspectSource) return inspectSource;
		hasRequiredInspectSource = 1;
		var uncurryThis = requireFunctionUncurryThis();
		var isCallable = requireIsCallable();
		var store = requireSharedStore();

		var functionToString = uncurryThis(Function.toString);

		// this helper broken in `core-js@3.4.1-3.4.4`, so we can't use `shared` helper
		if (!isCallable(store.inspectSource)) {
		  store.inspectSource = function (it) {
		    return functionToString(it);
		  };
		}

		inspectSource = store.inspectSource;
		return inspectSource;
	}

	var weakMapBasicDetection;
	var hasRequiredWeakMapBasicDetection;

	function requireWeakMapBasicDetection () {
		if (hasRequiredWeakMapBasicDetection) return weakMapBasicDetection;
		hasRequiredWeakMapBasicDetection = 1;
		var globalThis = requireGlobalThis();
		var isCallable = requireIsCallable();

		var WeakMap = globalThis.WeakMap;

		weakMapBasicDetection = isCallable(WeakMap) && /native code/.test(String(WeakMap));
		return weakMapBasicDetection;
	}

	var sharedKey;
	var hasRequiredSharedKey;

	function requireSharedKey () {
		if (hasRequiredSharedKey) return sharedKey;
		hasRequiredSharedKey = 1;
		var shared = requireShared();
		var uid = requireUid();

		var keys = shared('keys');

		sharedKey = function (key) {
		  return keys[key] || (keys[key] = uid(key));
		};
		return sharedKey;
	}

	var hiddenKeys;
	var hasRequiredHiddenKeys;

	function requireHiddenKeys () {
		if (hasRequiredHiddenKeys) return hiddenKeys;
		hasRequiredHiddenKeys = 1;
		hiddenKeys = {};
		return hiddenKeys;
	}

	var internalState;
	var hasRequiredInternalState;

	function requireInternalState () {
		if (hasRequiredInternalState) return internalState;
		hasRequiredInternalState = 1;
		var NATIVE_WEAK_MAP = requireWeakMapBasicDetection();
		var globalThis = requireGlobalThis();
		var isObject = requireIsObject();
		var createNonEnumerableProperty = requireCreateNonEnumerableProperty();
		var hasOwn = requireHasOwnProperty();
		var shared = requireSharedStore();
		var sharedKey = requireSharedKey();
		var hiddenKeys = requireHiddenKeys();

		var OBJECT_ALREADY_INITIALIZED = 'Object already initialized';
		var TypeError = globalThis.TypeError;
		var WeakMap = globalThis.WeakMap;
		var set, get, has;

		var enforce = function (it) {
		  return has(it) ? get(it) : set(it, {});
		};

		var getterFor = function (TYPE) {
		  return function (it) {
		    var state;
		    if (!isObject(it) || (state = get(it)).type !== TYPE) {
		      throw new TypeError('Incompatible receiver, ' + TYPE + ' required');
		    } return state;
		  };
		};

		if (NATIVE_WEAK_MAP || shared.state) {
		  var store = shared.state || (shared.state = new WeakMap());
		  /* eslint-disable no-self-assign -- prototype methods protection */
		  store.get = store.get;
		  store.has = store.has;
		  store.set = store.set;
		  /* eslint-enable no-self-assign -- prototype methods protection */
		  set = function (it, metadata) {
		    if (store.has(it)) throw new TypeError(OBJECT_ALREADY_INITIALIZED);
		    metadata.facade = it;
		    store.set(it, metadata);
		    return metadata;
		  };
		  get = function (it) {
		    return store.get(it) || {};
		  };
		  has = function (it) {
		    return store.has(it);
		  };
		} else {
		  var STATE = sharedKey('state');
		  hiddenKeys[STATE] = true;
		  set = function (it, metadata) {
		    if (hasOwn(it, STATE)) throw new TypeError(OBJECT_ALREADY_INITIALIZED);
		    metadata.facade = it;
		    createNonEnumerableProperty(it, STATE, metadata);
		    return metadata;
		  };
		  get = function (it) {
		    return hasOwn(it, STATE) ? it[STATE] : {};
		  };
		  has = function (it) {
		    return hasOwn(it, STATE);
		  };
		}

		internalState = {
		  set: set,
		  get: get,
		  has: has,
		  enforce: enforce,
		  getterFor: getterFor
		};
		return internalState;
	}

	var hasRequiredMakeBuiltIn;

	function requireMakeBuiltIn () {
		if (hasRequiredMakeBuiltIn) return makeBuiltIn.exports;
		hasRequiredMakeBuiltIn = 1;
		var uncurryThis = requireFunctionUncurryThis();
		var fails = requireFails();
		var isCallable = requireIsCallable();
		var hasOwn = requireHasOwnProperty();
		var DESCRIPTORS = requireDescriptors();
		var CONFIGURABLE_FUNCTION_NAME = requireFunctionName().CONFIGURABLE;
		var inspectSource = requireInspectSource();
		var InternalStateModule = requireInternalState();

		var enforceInternalState = InternalStateModule.enforce;
		var getInternalState = InternalStateModule.get;
		var $String = String;
		// eslint-disable-next-line es/no-object-defineproperty -- safe
		var defineProperty = Object.defineProperty;
		var stringSlice = uncurryThis(''.slice);
		var replace = uncurryThis(''.replace);
		var join = uncurryThis([].join);

		var CONFIGURABLE_LENGTH = DESCRIPTORS && !fails(function () {
		  return defineProperty(function () { /* empty */ }, 'length', { value: 8 }).length !== 8;
		});

		var TEMPLATE = String(String).split('String');

		var makeBuiltIn$1 = makeBuiltIn.exports = function (value, name, options) {
		  if (stringSlice($String(name), 0, 7) === 'Symbol(') {
		    name = '[' + replace($String(name), /^Symbol\(([^)]*)\).*$/, '$1') + ']';
		  }
		  if (options && options.getter) name = 'get ' + name;
		  if (options && options.setter) name = 'set ' + name;
		  if (!hasOwn(value, 'name') || (CONFIGURABLE_FUNCTION_NAME && value.name !== name)) {
		    if (DESCRIPTORS) defineProperty(value, 'name', { value: name, configurable: true });
		    else value.name = name;
		  }
		  if (CONFIGURABLE_LENGTH && options && hasOwn(options, 'arity') && value.length !== options.arity) {
		    defineProperty(value, 'length', { value: options.arity });
		  }
		  try {
		    if (options && hasOwn(options, 'constructor') && options.constructor) {
		      if (DESCRIPTORS) defineProperty(value, 'prototype', { writable: false });
		    // in V8 ~ Chrome 53, prototypes of some methods, like `Array.prototype.values`, are non-writable
		    } else if (value.prototype) value.prototype = undefined;
		  } catch (error) { /* empty */ }
		  var state = enforceInternalState(value);
		  if (!hasOwn(state, 'source')) {
		    state.source = join(TEMPLATE, typeof name == 'string' ? name : '');
		  } return value;
		};

		// add fake Function#toString for correct work wrapped methods / constructors with methods like LoDash isNative
		// eslint-disable-next-line no-extend-native -- required
		Function.prototype.toString = makeBuiltIn$1(function toString() {
		  return isCallable(this) && getInternalState(this).source || inspectSource(this);
		}, 'toString');
		return makeBuiltIn.exports;
	}

	var defineBuiltIn;
	var hasRequiredDefineBuiltIn;

	function requireDefineBuiltIn () {
		if (hasRequiredDefineBuiltIn) return defineBuiltIn;
		hasRequiredDefineBuiltIn = 1;
		var isCallable = requireIsCallable();
		var definePropertyModule = requireObjectDefineProperty();
		var makeBuiltIn = requireMakeBuiltIn();
		var defineGlobalProperty = requireDefineGlobalProperty();

		defineBuiltIn = function (O, key, value, options) {
		  if (!options) options = {};
		  var simple = options.enumerable;
		  var name = options.name !== undefined ? options.name : key;
		  if (isCallable(value)) makeBuiltIn(value, name, options);
		  if (options.global) {
		    if (simple) O[key] = value;
		    else defineGlobalProperty(key, value);
		  } else {
		    try {
		      if (!options.unsafe) delete O[key];
		      else if (O[key]) simple = true;
		    } catch (error) { /* empty */ }
		    if (simple) O[key] = value;
		    else definePropertyModule.f(O, key, {
		      value: value,
		      enumerable: false,
		      configurable: !options.nonConfigurable,
		      writable: !options.nonWritable
		    });
		  } return O;
		};
		return defineBuiltIn;
	}

	var objectGetOwnPropertyNames = {};

	var mathTrunc;
	var hasRequiredMathTrunc;

	function requireMathTrunc () {
		if (hasRequiredMathTrunc) return mathTrunc;
		hasRequiredMathTrunc = 1;
		var ceil = Math.ceil;
		var floor = Math.floor;

		// `Math.trunc` method
		// https://tc39.es/ecma262/#sec-math.trunc
		// eslint-disable-next-line es/no-math-trunc -- safe
		mathTrunc = Math.trunc || function trunc(x) {
		  var n = +x;
		  return (n > 0 ? floor : ceil)(n);
		};
		return mathTrunc;
	}

	var toIntegerOrInfinity;
	var hasRequiredToIntegerOrInfinity;

	function requireToIntegerOrInfinity () {
		if (hasRequiredToIntegerOrInfinity) return toIntegerOrInfinity;
		hasRequiredToIntegerOrInfinity = 1;
		var trunc = requireMathTrunc();

		// `ToIntegerOrInfinity` abstract operation
		// https://tc39.es/ecma262/#sec-tointegerorinfinity
		toIntegerOrInfinity = function (argument) {
		  var number = +argument;
		  // eslint-disable-next-line no-self-compare -- NaN check
		  return number !== number || number === 0 ? 0 : trunc(number);
		};
		return toIntegerOrInfinity;
	}

	var toAbsoluteIndex;
	var hasRequiredToAbsoluteIndex;

	function requireToAbsoluteIndex () {
		if (hasRequiredToAbsoluteIndex) return toAbsoluteIndex;
		hasRequiredToAbsoluteIndex = 1;
		var toIntegerOrInfinity = requireToIntegerOrInfinity();

		var max = Math.max;
		var min = Math.min;

		// Helper for a popular repeating case of the spec:
		// Let integer be ? ToInteger(index).
		// If integer < 0, let result be max((length + integer), 0); else let result be min(integer, length).
		toAbsoluteIndex = function (index, length) {
		  var integer = toIntegerOrInfinity(index);
		  return integer < 0 ? max(integer + length, 0) : min(integer, length);
		};
		return toAbsoluteIndex;
	}

	var toLength;
	var hasRequiredToLength;

	function requireToLength () {
		if (hasRequiredToLength) return toLength;
		hasRequiredToLength = 1;
		var toIntegerOrInfinity = requireToIntegerOrInfinity();

		var min = Math.min;

		// `ToLength` abstract operation
		// https://tc39.es/ecma262/#sec-tolength
		toLength = function (argument) {
		  var len = toIntegerOrInfinity(argument);
		  return len > 0 ? min(len, 0x1FFFFFFFFFFFFF) : 0; // 2 ** 53 - 1 == 9007199254740991
		};
		return toLength;
	}

	var lengthOfArrayLike;
	var hasRequiredLengthOfArrayLike;

	function requireLengthOfArrayLike () {
		if (hasRequiredLengthOfArrayLike) return lengthOfArrayLike;
		hasRequiredLengthOfArrayLike = 1;
		var toLength = requireToLength();

		// `LengthOfArrayLike` abstract operation
		// https://tc39.es/ecma262/#sec-lengthofarraylike
		lengthOfArrayLike = function (obj) {
		  return toLength(obj.length);
		};
		return lengthOfArrayLike;
	}

	var arrayIncludes;
	var hasRequiredArrayIncludes;

	function requireArrayIncludes () {
		if (hasRequiredArrayIncludes) return arrayIncludes;
		hasRequiredArrayIncludes = 1;
		var toIndexedObject = requireToIndexedObject();
		var toAbsoluteIndex = requireToAbsoluteIndex();
		var lengthOfArrayLike = requireLengthOfArrayLike();

		// `Array.prototype.{ indexOf, includes }` methods implementation
		var createMethod = function (IS_INCLUDES) {
		  return function ($this, el, fromIndex) {
		    var O = toIndexedObject($this);
		    var length = lengthOfArrayLike(O);
		    if (length === 0) return !IS_INCLUDES && -1;
		    var index = toAbsoluteIndex(fromIndex, length);
		    var value;
		    // Array#includes uses SameValueZero equality algorithm
		    // eslint-disable-next-line no-self-compare -- NaN check
		    if (IS_INCLUDES && el !== el) while (length > index) {
		      value = O[index++];
		      // eslint-disable-next-line no-self-compare -- NaN check
		      if (value !== value) return true;
		    // Array#indexOf ignores holes, Array#includes - not
		    } else for (;length > index; index++) {
		      if ((IS_INCLUDES || index in O) && O[index] === el) return IS_INCLUDES || index || 0;
		    } return !IS_INCLUDES && -1;
		  };
		};

		arrayIncludes = {
		  // `Array.prototype.includes` method
		  // https://tc39.es/ecma262/#sec-array.prototype.includes
		  includes: createMethod(true),
		  // `Array.prototype.indexOf` method
		  // https://tc39.es/ecma262/#sec-array.prototype.indexof
		  indexOf: createMethod(false)
		};
		return arrayIncludes;
	}

	var objectKeysInternal;
	var hasRequiredObjectKeysInternal;

	function requireObjectKeysInternal () {
		if (hasRequiredObjectKeysInternal) return objectKeysInternal;
		hasRequiredObjectKeysInternal = 1;
		var uncurryThis = requireFunctionUncurryThis();
		var hasOwn = requireHasOwnProperty();
		var toIndexedObject = requireToIndexedObject();
		var indexOf = requireArrayIncludes().indexOf;
		var hiddenKeys = requireHiddenKeys();

		var push = uncurryThis([].push);

		objectKeysInternal = function (object, names) {
		  var O = toIndexedObject(object);
		  var i = 0;
		  var result = [];
		  var key;
		  for (key in O) !hasOwn(hiddenKeys, key) && hasOwn(O, key) && push(result, key);
		  // Don't enum bug & hidden keys
		  while (names.length > i) if (hasOwn(O, key = names[i++])) {
		    ~indexOf(result, key) || push(result, key);
		  }
		  return result;
		};
		return objectKeysInternal;
	}

	var enumBugKeys;
	var hasRequiredEnumBugKeys;

	function requireEnumBugKeys () {
		if (hasRequiredEnumBugKeys) return enumBugKeys;
		hasRequiredEnumBugKeys = 1;
		// IE8- don't enum bug keys
		enumBugKeys = [
		  'constructor',
		  'hasOwnProperty',
		  'isPrototypeOf',
		  'propertyIsEnumerable',
		  'toLocaleString',
		  'toString',
		  'valueOf'
		];
		return enumBugKeys;
	}

	var hasRequiredObjectGetOwnPropertyNames;

	function requireObjectGetOwnPropertyNames () {
		if (hasRequiredObjectGetOwnPropertyNames) return objectGetOwnPropertyNames;
		hasRequiredObjectGetOwnPropertyNames = 1;
		var internalObjectKeys = requireObjectKeysInternal();
		var enumBugKeys = requireEnumBugKeys();

		var hiddenKeys = enumBugKeys.concat('length', 'prototype');

		// `Object.getOwnPropertyNames` method
		// https://tc39.es/ecma262/#sec-object.getownpropertynames
		// eslint-disable-next-line es/no-object-getownpropertynames -- safe
		objectGetOwnPropertyNames.f = Object.getOwnPropertyNames || function getOwnPropertyNames(O) {
		  return internalObjectKeys(O, hiddenKeys);
		};
		return objectGetOwnPropertyNames;
	}

	var objectGetOwnPropertySymbols = {};

	var hasRequiredObjectGetOwnPropertySymbols;

	function requireObjectGetOwnPropertySymbols () {
		if (hasRequiredObjectGetOwnPropertySymbols) return objectGetOwnPropertySymbols;
		hasRequiredObjectGetOwnPropertySymbols = 1;
		// eslint-disable-next-line es/no-object-getownpropertysymbols -- safe
		objectGetOwnPropertySymbols.f = Object.getOwnPropertySymbols;
		return objectGetOwnPropertySymbols;
	}

	var ownKeys;
	var hasRequiredOwnKeys;

	function requireOwnKeys () {
		if (hasRequiredOwnKeys) return ownKeys;
		hasRequiredOwnKeys = 1;
		var getBuiltIn = requireGetBuiltIn();
		var uncurryThis = requireFunctionUncurryThis();
		var getOwnPropertyNamesModule = requireObjectGetOwnPropertyNames();
		var getOwnPropertySymbolsModule = requireObjectGetOwnPropertySymbols();
		var anObject = requireAnObject();

		var concat = uncurryThis([].concat);

		// all object keys, includes non-enumerable and symbols
		ownKeys = getBuiltIn('Reflect', 'ownKeys') || function ownKeys(it) {
		  var keys = getOwnPropertyNamesModule.f(anObject(it));
		  var getOwnPropertySymbols = getOwnPropertySymbolsModule.f;
		  return getOwnPropertySymbols ? concat(keys, getOwnPropertySymbols(it)) : keys;
		};
		return ownKeys;
	}

	var copyConstructorProperties;
	var hasRequiredCopyConstructorProperties;

	function requireCopyConstructorProperties () {
		if (hasRequiredCopyConstructorProperties) return copyConstructorProperties;
		hasRequiredCopyConstructorProperties = 1;
		var hasOwn = requireHasOwnProperty();
		var ownKeys = requireOwnKeys();
		var getOwnPropertyDescriptorModule = requireObjectGetOwnPropertyDescriptor();
		var definePropertyModule = requireObjectDefineProperty();

		copyConstructorProperties = function (target, source, exceptions) {
		  var keys = ownKeys(source);
		  var defineProperty = definePropertyModule.f;
		  var getOwnPropertyDescriptor = getOwnPropertyDescriptorModule.f;
		  for (var i = 0; i < keys.length; i++) {
		    var key = keys[i];
		    if (!hasOwn(target, key) && !(exceptions && hasOwn(exceptions, key))) {
		      defineProperty(target, key, getOwnPropertyDescriptor(source, key));
		    }
		  }
		};
		return copyConstructorProperties;
	}

	var isForced_1;
	var hasRequiredIsForced;

	function requireIsForced () {
		if (hasRequiredIsForced) return isForced_1;
		hasRequiredIsForced = 1;
		var fails = requireFails();
		var isCallable = requireIsCallable();

		var replacement = /#|\.prototype\./;

		var isForced = function (feature, detection) {
		  var value = data[normalize(feature)];
		  return value === POLYFILL ? true
		    : value === NATIVE ? false
		    : isCallable(detection) ? fails(detection)
		    : !!detection;
		};

		var normalize = isForced.normalize = function (string) {
		  return String(string).replace(replacement, '.').toLowerCase();
		};

		var data = isForced.data = {};
		var NATIVE = isForced.NATIVE = 'N';
		var POLYFILL = isForced.POLYFILL = 'P';

		isForced_1 = isForced;
		return isForced_1;
	}

	var _export;
	var hasRequired_export;

	function require_export () {
		if (hasRequired_export) return _export;
		hasRequired_export = 1;
		var globalThis = requireGlobalThis();
		var getOwnPropertyDescriptor = requireObjectGetOwnPropertyDescriptor().f;
		var createNonEnumerableProperty = requireCreateNonEnumerableProperty();
		var defineBuiltIn = requireDefineBuiltIn();
		var defineGlobalProperty = requireDefineGlobalProperty();
		var copyConstructorProperties = requireCopyConstructorProperties();
		var isForced = requireIsForced();

		/*
		  options.target         - name of the target object
		  options.global         - target is the global object
		  options.stat           - export as static methods of target
		  options.proto          - export as prototype methods of target
		  options.real           - real prototype method for the `pure` version
		  options.forced         - export even if the native feature is available
		  options.bind           - bind methods to the target, required for the `pure` version
		  options.wrap           - wrap constructors to preventing global pollution, required for the `pure` version
		  options.unsafe         - use the simple assignment of property instead of delete + defineProperty
		  options.sham           - add a flag to not completely full polyfills
		  options.enumerable     - export as enumerable property
		  options.dontCallGetSet - prevent calling a getter on target
		  options.name           - the .name of the function if it does not match the key
		*/
		_export = function (options, source) {
		  var TARGET = options.target;
		  var GLOBAL = options.global;
		  var STATIC = options.stat;
		  var FORCED, target, key, targetProperty, sourceProperty, descriptor;
		  if (GLOBAL) {
		    target = globalThis;
		  } else if (STATIC) {
		    target = globalThis[TARGET] || defineGlobalProperty(TARGET, {});
		  } else {
		    target = globalThis[TARGET] && globalThis[TARGET].prototype;
		  }
		  if (target) for (key in source) {
		    sourceProperty = source[key];
		    if (options.dontCallGetSet) {
		      descriptor = getOwnPropertyDescriptor(target, key);
		      targetProperty = descriptor && descriptor.value;
		    } else targetProperty = target[key];
		    FORCED = isForced(GLOBAL ? key : TARGET + (STATIC ? '.' : '#') + key, options.forced);
		    // contained in target
		    if (!FORCED && targetProperty !== undefined) {
		      if (typeof sourceProperty == typeof targetProperty) continue;
		      copyConstructorProperties(sourceProperty, targetProperty);
		    }
		    // add a flag to not completely full polyfills
		    if (options.sham || (targetProperty && targetProperty.sham)) {
		      createNonEnumerableProperty(sourceProperty, 'sham', true);
		    }
		    defineBuiltIn(target, key, sourceProperty, options);
		  }
		};
		return _export;
	}

	var objectDefineProperties = {};

	var objectKeys;
	var hasRequiredObjectKeys;

	function requireObjectKeys () {
		if (hasRequiredObjectKeys) return objectKeys;
		hasRequiredObjectKeys = 1;
		var internalObjectKeys = requireObjectKeysInternal();
		var enumBugKeys = requireEnumBugKeys();

		// `Object.keys` method
		// https://tc39.es/ecma262/#sec-object.keys
		// eslint-disable-next-line es/no-object-keys -- safe
		objectKeys = Object.keys || function keys(O) {
		  return internalObjectKeys(O, enumBugKeys);
		};
		return objectKeys;
	}

	var hasRequiredObjectDefineProperties;

	function requireObjectDefineProperties () {
		if (hasRequiredObjectDefineProperties) return objectDefineProperties;
		hasRequiredObjectDefineProperties = 1;
		var DESCRIPTORS = requireDescriptors();
		var V8_PROTOTYPE_DEFINE_BUG = requireV8PrototypeDefineBug();
		var definePropertyModule = requireObjectDefineProperty();
		var anObject = requireAnObject();
		var toIndexedObject = requireToIndexedObject();
		var objectKeys = requireObjectKeys();

		// `Object.defineProperties` method
		// https://tc39.es/ecma262/#sec-object.defineproperties
		// eslint-disable-next-line es/no-object-defineproperties -- safe
		objectDefineProperties.f = DESCRIPTORS && !V8_PROTOTYPE_DEFINE_BUG ? Object.defineProperties : function defineProperties(O, Properties) {
		  anObject(O);
		  var props = toIndexedObject(Properties);
		  var keys = objectKeys(Properties);
		  var length = keys.length;
		  var index = 0;
		  var key;
		  while (length > index) definePropertyModule.f(O, key = keys[index++], props[key]);
		  return O;
		};
		return objectDefineProperties;
	}

	var html;
	var hasRequiredHtml;

	function requireHtml () {
		if (hasRequiredHtml) return html;
		hasRequiredHtml = 1;
		var getBuiltIn = requireGetBuiltIn();

		html = getBuiltIn('document', 'documentElement');
		return html;
	}

	var objectCreate;
	var hasRequiredObjectCreate;

	function requireObjectCreate () {
		if (hasRequiredObjectCreate) return objectCreate;
		hasRequiredObjectCreate = 1;
		/* global ActiveXObject -- old IE, WSH */
		var anObject = requireAnObject();
		var definePropertiesModule = requireObjectDefineProperties();
		var enumBugKeys = requireEnumBugKeys();
		var hiddenKeys = requireHiddenKeys();
		var html = requireHtml();
		var documentCreateElement = requireDocumentCreateElement();
		var sharedKey = requireSharedKey();

		var GT = '>';
		var LT = '<';
		var PROTOTYPE = 'prototype';
		var SCRIPT = 'script';
		var IE_PROTO = sharedKey('IE_PROTO');

		var EmptyConstructor = function () { /* empty */ };

		var scriptTag = function (content) {
		  return LT + SCRIPT + GT + content + LT + '/' + SCRIPT + GT;
		};

		// Create object with fake `null` prototype: use ActiveX Object with cleared prototype
		var NullProtoObjectViaActiveX = function (activeXDocument) {
		  activeXDocument.write(scriptTag(''));
		  activeXDocument.close();
		  var temp = activeXDocument.parentWindow.Object;
		  // eslint-disable-next-line no-useless-assignment -- avoid memory leak
		  activeXDocument = null;
		  return temp;
		};

		// Create object with fake `null` prototype: use iframe Object with cleared prototype
		var NullProtoObjectViaIFrame = function () {
		  // Thrash, waste and sodomy: IE GC bug
		  var iframe = documentCreateElement('iframe');
		  var JS = 'java' + SCRIPT + ':';
		  var iframeDocument;
		  iframe.style.display = 'none';
		  html.appendChild(iframe);
		  // https://github.com/zloirock/core-js/issues/475
		  iframe.src = String(JS);
		  iframeDocument = iframe.contentWindow.document;
		  iframeDocument.open();
		  iframeDocument.write(scriptTag('document.F=Object'));
		  iframeDocument.close();
		  return iframeDocument.F;
		};

		// Check for document.domain and active x support
		// No need to use active x approach when document.domain is not set
		// see https://github.com/es-shims/es5-shim/issues/150
		// variation of https://github.com/kitcambridge/es5-shim/commit/4f738ac066346
		// avoid IE GC bug
		var activeXDocument;
		var NullProtoObject = function () {
		  try {
		    activeXDocument = new ActiveXObject('htmlfile');
		  } catch (error) { /* ignore */ }
		  NullProtoObject = typeof document != 'undefined'
		    ? document.domain && activeXDocument
		      ? NullProtoObjectViaActiveX(activeXDocument) // old IE
		      : NullProtoObjectViaIFrame()
		    : NullProtoObjectViaActiveX(activeXDocument); // WSH
		  var length = enumBugKeys.length;
		  while (length--) delete NullProtoObject[PROTOTYPE][enumBugKeys[length]];
		  return NullProtoObject();
		};

		hiddenKeys[IE_PROTO] = true;

		// `Object.create` method
		// https://tc39.es/ecma262/#sec-object.create
		// eslint-disable-next-line es/no-object-create -- safe
		objectCreate = Object.create || function create(O, Properties) {
		  var result;
		  if (O !== null) {
		    EmptyConstructor[PROTOTYPE] = anObject(O);
		    result = new EmptyConstructor();
		    EmptyConstructor[PROTOTYPE] = null;
		    // add "__proto__" for Object.getPrototypeOf polyfill
		    result[IE_PROTO] = O;
		  } else result = NullProtoObject();
		  return Properties === undefined ? result : definePropertiesModule.f(result, Properties);
		};
		return objectCreate;
	}

	var addToUnscopables;
	var hasRequiredAddToUnscopables;

	function requireAddToUnscopables () {
		if (hasRequiredAddToUnscopables) return addToUnscopables;
		hasRequiredAddToUnscopables = 1;
		var wellKnownSymbol = requireWellKnownSymbol();
		var create = requireObjectCreate();
		var defineProperty = requireObjectDefineProperty().f;

		var UNSCOPABLES = wellKnownSymbol('unscopables');
		var ArrayPrototype = Array.prototype;

		// Array.prototype[@@unscopables]
		// https://tc39.es/ecma262/#sec-array.prototype-@@unscopables
		if (ArrayPrototype[UNSCOPABLES] === undefined) {
		  defineProperty(ArrayPrototype, UNSCOPABLES, {
		    configurable: true,
		    value: create(null)
		  });
		}

		// add a key to Array.prototype[@@unscopables]
		addToUnscopables = function (key) {
		  ArrayPrototype[UNSCOPABLES][key] = true;
		};
		return addToUnscopables;
	}

	var hasRequiredEs_array_at;

	function requireEs_array_at () {
		if (hasRequiredEs_array_at) return es_array_at;
		hasRequiredEs_array_at = 1;
		var $ = require_export();
		var toObject = requireToObject();
		var lengthOfArrayLike = requireLengthOfArrayLike();
		var toIntegerOrInfinity = requireToIntegerOrInfinity();
		var addToUnscopables = requireAddToUnscopables();

		// `Array.prototype.at` method
		// https://tc39.es/ecma262/#sec-array.prototype.at
		$({ target: 'Array', proto: true }, {
		  at: function at(index) {
		    var O = toObject(this);
		    var len = lengthOfArrayLike(O);
		    var relativeIndex = toIntegerOrInfinity(index);
		    var k = relativeIndex >= 0 ? relativeIndex : len + relativeIndex;
		    return (k < 0 || k >= len) ? undefined : O[k];
		  }
		});

		addToUnscopables('at');
		return es_array_at;
	}

	requireEs_array_at();

	var es_object_hasOwn = {};

	var hasRequiredEs_object_hasOwn;

	function requireEs_object_hasOwn () {
		if (hasRequiredEs_object_hasOwn) return es_object_hasOwn;
		hasRequiredEs_object_hasOwn = 1;
		var $ = require_export();
		var hasOwn = requireHasOwnProperty();

		// `Object.hasOwn` method
		// https://tc39.es/ecma262/#sec-object.hasown
		$({ target: 'Object', stat: true }, {
		  hasOwn: hasOwn
		});
		return es_object_hasOwn;
	}

	requireEs_object_hasOwn();

	var es_string_replaceAll = {};

	var isRegexp;
	var hasRequiredIsRegexp;

	function requireIsRegexp () {
		if (hasRequiredIsRegexp) return isRegexp;
		hasRequiredIsRegexp = 1;
		var isObject = requireIsObject();
		var classof = requireClassofRaw();
		var wellKnownSymbol = requireWellKnownSymbol();

		var MATCH = wellKnownSymbol('match');

		// `IsRegExp` abstract operation
		// https://tc39.es/ecma262/#sec-isregexp
		isRegexp = function (it) {
		  var isRegExp;
		  return isObject(it) && ((isRegExp = it[MATCH]) !== undefined ? !!isRegExp : classof(it) === 'RegExp');
		};
		return isRegexp;
	}

	var toStringTagSupport;
	var hasRequiredToStringTagSupport;

	function requireToStringTagSupport () {
		if (hasRequiredToStringTagSupport) return toStringTagSupport;
		hasRequiredToStringTagSupport = 1;
		var wellKnownSymbol = requireWellKnownSymbol();

		var TO_STRING_TAG = wellKnownSymbol('toStringTag');
		var test = {};
		// eslint-disable-next-line unicorn/no-immediate-mutation -- ES3 syntax limitation
		test[TO_STRING_TAG] = 'z';

		toStringTagSupport = String(test) === '[object z]';
		return toStringTagSupport;
	}

	var classof;
	var hasRequiredClassof;

	function requireClassof () {
		if (hasRequiredClassof) return classof;
		hasRequiredClassof = 1;
		var TO_STRING_TAG_SUPPORT = requireToStringTagSupport();
		var isCallable = requireIsCallable();
		var classofRaw = requireClassofRaw();
		var wellKnownSymbol = requireWellKnownSymbol();

		var TO_STRING_TAG = wellKnownSymbol('toStringTag');
		var $Object = Object;

		// ES3 wrong here
		var CORRECT_ARGUMENTS = classofRaw(function () { return arguments; }()) === 'Arguments';

		// fallback for IE11 Script Access Denied error
		var tryGet = function (it, key) {
		  try {
		    return it[key];
		  } catch (error) { /* empty */ }
		};

		// getting tag from ES6+ `Object.prototype.toString`
		classof = TO_STRING_TAG_SUPPORT ? classofRaw : function (it) {
		  var O, tag, result;
		  return it === undefined ? 'Undefined' : it === null ? 'Null'
		    // @@toStringTag case
		    : typeof (tag = tryGet(O = $Object(it), TO_STRING_TAG)) == 'string' ? tag
		    // builtinTag case
		    : CORRECT_ARGUMENTS ? classofRaw(O)
		    // ES3 arguments fallback
		    : (result = classofRaw(O)) === 'Object' && isCallable(O.callee) ? 'Arguments' : result;
		};
		return classof;
	}

	var toString;
	var hasRequiredToString;

	function requireToString () {
		if (hasRequiredToString) return toString;
		hasRequiredToString = 1;
		var classof = requireClassof();

		var $String = String;

		toString = function (argument) {
		  if (classof(argument) === 'Symbol') throw new TypeError('Cannot convert a Symbol value to a string');
		  return $String(argument);
		};
		return toString;
	}

	var regexpFlagsDetection;
	var hasRequiredRegexpFlagsDetection;

	function requireRegexpFlagsDetection () {
		if (hasRequiredRegexpFlagsDetection) return regexpFlagsDetection;
		hasRequiredRegexpFlagsDetection = 1;
		var globalThis = requireGlobalThis();
		var fails = requireFails();

		// babel-minify and Closure Compiler transpiles RegExp('.', 'd') -> /./d and it causes SyntaxError
		var RegExp = globalThis.RegExp;

		var FLAGS_GETTER_IS_CORRECT = !fails(function () {
		  var INDICES_SUPPORT = true;
		  try {
		    RegExp('.', 'd');
		  } catch (error) {
		    INDICES_SUPPORT = false;
		  }

		  var O = {};
		  // modern V8 bug
		  var calls = '';
		  var expected = INDICES_SUPPORT ? 'dgimsy' : 'gimsy';

		  var addGetter = function (key, chr) {
		    // eslint-disable-next-line es/no-object-defineproperty -- safe
		    Object.defineProperty(O, key, { get: function () {
		      calls += chr;
		      return true;
		    } });
		  };

		  var pairs = {
		    dotAll: 's',
		    global: 'g',
		    ignoreCase: 'i',
		    multiline: 'm',
		    sticky: 'y'
		  };

		  if (INDICES_SUPPORT) pairs.hasIndices = 'd';

		  for (var key in pairs) addGetter(key, pairs[key]);

		  // eslint-disable-next-line es/no-object-getownpropertydescriptor -- safe
		  var result = Object.getOwnPropertyDescriptor(RegExp.prototype, 'flags').get.call(O);

		  return result !== expected || calls !== expected;
		});

		regexpFlagsDetection = { correct: FLAGS_GETTER_IS_CORRECT };
		return regexpFlagsDetection;
	}

	var regexpFlags;
	var hasRequiredRegexpFlags;

	function requireRegexpFlags () {
		if (hasRequiredRegexpFlags) return regexpFlags;
		hasRequiredRegexpFlags = 1;
		var anObject = requireAnObject();

		// `RegExp.prototype.flags` getter implementation
		// https://tc39.es/ecma262/#sec-get-regexp.prototype.flags
		regexpFlags = function () {
		  var that = anObject(this);
		  var result = '';
		  if (that.hasIndices) result += 'd';
		  if (that.global) result += 'g';
		  if (that.ignoreCase) result += 'i';
		  if (that.multiline) result += 'm';
		  if (that.dotAll) result += 's';
		  if (that.unicode) result += 'u';
		  if (that.unicodeSets) result += 'v';
		  if (that.sticky) result += 'y';
		  return result;
		};
		return regexpFlags;
	}

	var regexpGetFlags;
	var hasRequiredRegexpGetFlags;

	function requireRegexpGetFlags () {
		if (hasRequiredRegexpGetFlags) return regexpGetFlags;
		hasRequiredRegexpGetFlags = 1;
		var call = requireFunctionCall();
		var hasOwn = requireHasOwnProperty();
		var isPrototypeOf = requireObjectIsPrototypeOf();
		var regExpFlagsDetection = requireRegexpFlagsDetection();
		var regExpFlagsGetterImplementation = requireRegexpFlags();

		var RegExpPrototype = RegExp.prototype;

		regexpGetFlags = regExpFlagsDetection.correct ? function (it) {
		  return it.flags;
		} : function (it) {
		  return (!regExpFlagsDetection.correct && isPrototypeOf(RegExpPrototype, it) && !hasOwn(it, 'flags'))
		    ? call(regExpFlagsGetterImplementation, it)
		    : it.flags;
		};
		return regexpGetFlags;
	}

	var getSubstitution;
	var hasRequiredGetSubstitution;

	function requireGetSubstitution () {
		if (hasRequiredGetSubstitution) return getSubstitution;
		hasRequiredGetSubstitution = 1;
		var uncurryThis = requireFunctionUncurryThis();
		var toObject = requireToObject();

		var floor = Math.floor;
		var charAt = uncurryThis(''.charAt);
		var replace = uncurryThis(''.replace);
		var stringSlice = uncurryThis(''.slice);
		// eslint-disable-next-line redos/no-vulnerable -- safe
		var SUBSTITUTION_SYMBOLS = /\$([$&'`]|\d{1,2}|<[^>]*>)/g;
		var SUBSTITUTION_SYMBOLS_NO_NAMED = /\$([$&'`]|\d{1,2})/g;

		// `GetSubstitution` abstract operation
		// https://tc39.es/ecma262/#sec-getsubstitution
		getSubstitution = function (matched, str, position, captures, namedCaptures, replacement) {
		  var tailPos = position + matched.length;
		  var m = captures.length;
		  var symbols = SUBSTITUTION_SYMBOLS_NO_NAMED;
		  if (namedCaptures !== undefined) {
		    namedCaptures = toObject(namedCaptures);
		    symbols = SUBSTITUTION_SYMBOLS;
		  }
		  return replace(replacement, symbols, function (match, ch) {
		    var capture;
		    switch (charAt(ch, 0)) {
		      case '$': return '$';
		      case '&': return matched;
		      case '`': return stringSlice(str, 0, position);
		      case "'": return stringSlice(str, tailPos);
		      case '<':
		        capture = namedCaptures[stringSlice(ch, 1, -1)];
		        break;
		      default: // \d\d?
		        var n = +ch;
		        if (n === 0) return match;
		        if (n > m) {
		          var f = floor(n / 10);
		          if (f === 0) return match;
		          if (f <= m) return captures[f - 1] === undefined ? charAt(ch, 1) : captures[f - 1] + charAt(ch, 1);
		          return match;
		        }
		        capture = captures[n - 1];
		    }
		    return capture === undefined ? '' : capture;
		  });
		};
		return getSubstitution;
	}

	var hasRequiredEs_string_replaceAll;

	function requireEs_string_replaceAll () {
		if (hasRequiredEs_string_replaceAll) return es_string_replaceAll;
		hasRequiredEs_string_replaceAll = 1;
		var $ = require_export();
		var call = requireFunctionCall();
		var uncurryThis = requireFunctionUncurryThis();
		var requireObjectCoercible = requireRequireObjectCoercible();
		var isCallable = requireIsCallable();
		var isObject = requireIsObject();
		var isRegExp = requireIsRegexp();
		var toString = requireToString();
		var getMethod = requireGetMethod();
		var getRegExpFlags = requireRegexpGetFlags();
		var getSubstitution = requireGetSubstitution();
		var wellKnownSymbol = requireWellKnownSymbol();
		var IS_PURE = requireIsPure();

		var REPLACE = wellKnownSymbol('replace');
		var $TypeError = TypeError;
		var indexOf = uncurryThis(''.indexOf);
		var replace = uncurryThis(''.replace);
		var stringSlice = uncurryThis(''.slice);
		var max = Math.max;

		// `String.prototype.replaceAll` method
		// https://tc39.es/ecma262/#sec-string.prototype.replaceall
		$({ target: 'String', proto: true }, {
		  replaceAll: function replaceAll(searchValue, replaceValue) {
		    var O = requireObjectCoercible(this);
		    var IS_REG_EXP, flags, replacer, string, searchString, functionalReplace, searchLength, advanceBy, position, replacement;
		    var endOfLastMatch = 0;
		    var result = '';
		    if (isObject(searchValue)) {
		      IS_REG_EXP = isRegExp(searchValue);
		      if (IS_REG_EXP) {
		        flags = toString(requireObjectCoercible(getRegExpFlags(searchValue)));
		        if (!~indexOf(flags, 'g')) throw new $TypeError('`.replaceAll` does not allow non-global regexes');
		      }
		      replacer = getMethod(searchValue, REPLACE);
		      if (replacer) return call(replacer, searchValue, O, replaceValue);
		      if (IS_PURE && IS_REG_EXP) return replace(toString(O), searchValue, replaceValue);
		    }
		    string = toString(O);
		    searchString = toString(searchValue);
		    functionalReplace = isCallable(replaceValue);
		    if (!functionalReplace) replaceValue = toString(replaceValue);
		    searchLength = searchString.length;
		    advanceBy = max(1, searchLength);
		    position = indexOf(string, searchString);
		    while (position !== -1) {
		      replacement = functionalReplace
		        ? toString(replaceValue(searchString, position, string))
		        : getSubstitution(searchString, string, position, [], undefined, replaceValue);
		      result += stringSlice(string, endOfLastMatch, position) + replacement;
		      endOfLastMatch = position + searchLength;
		      position = position + advanceBy > string.length ? -1 : indexOf(string, searchString, position + advanceBy);
		    }
		    if (endOfLastMatch < string.length) {
		      result += stringSlice(string, endOfLastMatch);
		    }
		    return result;
		  }
		});
		return es_string_replaceAll;
	}

	requireEs_string_replaceAll();

	var name = "corejs";
	var items = [
		1,
		2,
		3
	];
	var data = {
		name: name,
		items: items
	};

	var describe = {};

	var keys;
	var hasRequiredKeys;

	function requireKeys () {
		if (hasRequiredKeys) return keys;
		hasRequiredKeys = 1;
		requireEs_string_replaceAll();
		console.log("eval keys");
		keys = function keys(value) {
		  return Object.keys(value).map(function (key) {
		    return key.replaceAll("a", "A");
		  });
		};
		return keys;
	}

	var hasRequiredDescribe;

	function requireDescribe () {
		if (hasRequiredDescribe) return describe;
		hasRequiredDescribe = 1;
		requireEs_object_hasOwn();
		var keys = requireKeys();
		console.log("eval describe");
		describe.describe = function (value) {
		  return keys(value).join("|") + " " + Object.hasOwn(value, "items");
		};
		return describe;
	}

	var describeExports = requireDescribe();

	console.log("eval index");
	console.log("at " + data.items.at(-1) + " hasOwn " + Object.hasOwn(data, "name"));
	console.log("replaceAll " + "a-b-c".replaceAll("-", "+"));
	console.log("describe " + describeExports.describe(data));

})();
