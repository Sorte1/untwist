var app = (function () {
	'use strict';

	var commonjsGlobal = typeof globalThis !== 'undefined' ? globalThis : typeof window !== 'undefined' ? window : typeof global !== 'undefined' ? global : typeof self !== 'undefined' ? self : {};

	var polyfill = {};

	var check = function (it) {
	  return it && it.Math === Math && it;
	};

	// https://github.com/zloirock/core-js/issues/86#issuecomment-115759028
	var globalThis_1 =
	  // eslint-disable-next-line es/no-global-this -- safe
	  check(typeof globalThis == 'object' && globalThis) ||
	  check(typeof window == 'object' && window) ||
	  // eslint-disable-next-line no-restricted-globals -- safe
	  check(typeof self == 'object' && self) ||
	  check(typeof commonjsGlobal == 'object' && commonjsGlobal) ||
	  check(typeof commonjsGlobal == 'object' && commonjsGlobal) ||
	  // eslint-disable-next-line no-new-func -- fallback
	  (function () { return this; })() || Function('return this')();

	var objectGetOwnPropertyDescriptor = {};

	var fails$9 = function (exec) {
	  try {
	    return !!exec();
	  } catch (error) {
	    return true;
	  }
	};

	var fails$8 = fails$9;

	// Detect IE8's incomplete defineProperty implementation
	var descriptors = !fails$8(function () {
	  // eslint-disable-next-line es/no-object-defineproperty -- required for testing
	  return Object.defineProperty({}, 1, { get: function () { return 7; } })[1] !== 7;
	});

	var fails$7 = fails$9;

	var functionBindNative = !fails$7(function () {
	  // eslint-disable-next-line es/no-function-prototype-bind -- safe
	  var test = function () { /* empty */ }.bind();
	  // eslint-disable-next-line no-prototype-builtins -- safe
	  return typeof test != 'function' || test.hasOwnProperty('prototype');
	});

	var NATIVE_BIND$2 = functionBindNative;

	var call$7 = Function.prototype.call;
	// eslint-disable-next-line es/no-function-prototype-bind -- safe
	var functionCall = NATIVE_BIND$2 ? call$7.bind(call$7) : function () {
	  return call$7.apply(call$7, arguments);
	};

	var objectPropertyIsEnumerable = {};

	var $propertyIsEnumerable = {}.propertyIsEnumerable;
	// eslint-disable-next-line es/no-object-getownpropertydescriptor -- safe
	var getOwnPropertyDescriptor$1 = Object.getOwnPropertyDescriptor;

	// Nashorn ~ JDK8 bug
	var NASHORN_BUG = getOwnPropertyDescriptor$1 && !$propertyIsEnumerable.call({ 1: 2 }, 1);

	// `Object.prototype.propertyIsEnumerable` method implementation
	// https://tc39.es/ecma262/#sec-object.prototype.propertyisenumerable
	objectPropertyIsEnumerable.f = NASHORN_BUG ? function propertyIsEnumerable(V) {
	  var descriptor = getOwnPropertyDescriptor$1(this, V);
	  return !!descriptor && descriptor.enumerable;
	} : $propertyIsEnumerable;

	var createPropertyDescriptor$3 = function (bitmap, value) {
	  return {
	    enumerable: !(bitmap & 1),
	    configurable: !(bitmap & 2),
	    writable: !(bitmap & 4),
	    value: value
	  };
	};

	var NATIVE_BIND$1 = functionBindNative;

	var FunctionPrototype$1 = Function.prototype;
	var call$6 = FunctionPrototype$1.call;
	// eslint-disable-next-line es/no-function-prototype-bind -- safe
	var uncurryThisWithBind = NATIVE_BIND$1 && FunctionPrototype$1.bind.bind(call$6, call$6);

	var functionUncurryThis = NATIVE_BIND$1 ? uncurryThisWithBind : function (fn) {
	  return function () {
	    return call$6.apply(fn, arguments);
	  };
	};

	var uncurryThis$c = functionUncurryThis;

	var toString$1 = uncurryThis$c({}.toString);
	var stringSlice$1 = uncurryThis$c(''.slice);

	var classofRaw$1 = function (it) {
	  return stringSlice$1(toString$1(it), 8, -1);
	};

	var uncurryThis$b = functionUncurryThis;
	var fails$6 = fails$9;
	var classof$1 = classofRaw$1;

	var $Object$2 = Object;
	var split = uncurryThis$b(''.split);

	// fallback for non-array-like ES3 and non-enumerable old V8 strings
	var indexedObject = fails$6(function () {
	  // throws an error in rhino, see https://github.com/mozilla/rhino/issues/346
	  // eslint-disable-next-line no-prototype-builtins -- safe
	  return !$Object$2('z').propertyIsEnumerable(0);
	}) ? function (it) {
	  return classof$1(it) === 'String' ? split(it, '') : $Object$2(it);
	} : $Object$2;

	// we can't use just `it == null` since of `document.all` special case
	// https://tc39.es/ecma262/#sec-IsHTMLDDA-internal-slot-aec
	var isNullOrUndefined$3 = function (it) {
	  return it === null || it === undefined;
	};

	var isNullOrUndefined$2 = isNullOrUndefined$3;

	var $TypeError$8 = TypeError;

	// `RequireObjectCoercible` abstract operation
	// https://tc39.es/ecma262/#sec-requireobjectcoercible
	var requireObjectCoercible$3 = function (it) {
	  if (isNullOrUndefined$2(it)) throw new $TypeError$8("Can't call method on " + it);
	  return it;
	};

	// toObject with fallback for non-array-like ES3 strings
	var IndexedObject = indexedObject;
	var requireObjectCoercible$2 = requireObjectCoercible$3;

	var toIndexedObject$4 = function (it) {
	  return IndexedObject(requireObjectCoercible$2(it));
	};

	// https://tc39.es/ecma262/#sec-IsHTMLDDA-internal-slot
	var documentAll = typeof document == 'object' && document.all;

	// `IsCallable` abstract operation
	// https://tc39.es/ecma262/#sec-iscallable
	// eslint-disable-next-line unicorn/no-typeof-undefined -- required for testing
	var isCallable$b = typeof documentAll == 'undefined' && documentAll !== undefined ? function (argument) {
	  return typeof argument == 'function' || argument === documentAll;
	} : function (argument) {
	  return typeof argument == 'function';
	};

	var isCallable$a = isCallable$b;

	var isObject$5 = function (it) {
	  return typeof it == 'object' ? it !== null : isCallable$a(it);
	};

	var globalThis$d = globalThis_1;
	var isCallable$9 = isCallable$b;

	var aFunction = function (argument) {
	  return isCallable$9(argument) ? argument : undefined;
	};

	var getBuiltIn$4 = function (namespace, method) {
	  return arguments.length < 2 ? aFunction(globalThis$d[namespace]) : globalThis$d[namespace] && globalThis$d[namespace][method];
	};

	var uncurryThis$a = functionUncurryThis;

	var objectIsPrototypeOf = uncurryThis$a({}.isPrototypeOf);

	var globalThis$c = globalThis_1;

	var navigator = globalThis$c.navigator;
	var userAgent$1 = navigator && navigator.userAgent;

	var environmentUserAgent = userAgent$1 ? String(userAgent$1) : '';

	var globalThis$b = globalThis_1;
	var userAgent = environmentUserAgent;

	var process = globalThis$b.process;
	var Deno = globalThis$b.Deno;
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

	var environmentV8Version = version;

	/* eslint-disable es/no-symbol -- required for testing */
	var V8_VERSION = environmentV8Version;
	var fails$5 = fails$9;
	var globalThis$a = globalThis_1;

	var $String$3 = globalThis$a.String;

	// eslint-disable-next-line es/no-object-getownpropertysymbols -- required for testing
	var symbolConstructorDetection = !!Object.getOwnPropertySymbols && !fails$5(function () {
	  var symbol = Symbol('symbol detection');
	  // Chrome 38 Symbol has incorrect toString conversion
	  // `get-own-property-symbols` polyfill symbols converted to object are not Symbol instances
	  // nb: Do not call `String` directly to avoid this being optimized out to `symbol+''` which will,
	  // of course, fail.
	  return !$String$3(symbol) || !(Object(symbol) instanceof Symbol) ||
	    // Chrome 38-40 symbols are not inherited from DOM collections prototypes to instances
	    !Symbol.sham && V8_VERSION && V8_VERSION < 41;
	});

	/* eslint-disable es/no-symbol -- required for testing */
	var NATIVE_SYMBOL$1 = symbolConstructorDetection;

	var useSymbolAsUid = NATIVE_SYMBOL$1 &&
	  !Symbol.sham &&
	  typeof Symbol.iterator == 'symbol';

	var getBuiltIn$3 = getBuiltIn$4;
	var isCallable$8 = isCallable$b;
	var isPrototypeOf$1 = objectIsPrototypeOf;
	var USE_SYMBOL_AS_UID$1 = useSymbolAsUid;

	var $Object$1 = Object;

	var isSymbol$2 = USE_SYMBOL_AS_UID$1 ? function (it) {
	  return typeof it == 'symbol';
	} : function (it) {
	  var $Symbol = getBuiltIn$3('Symbol');
	  return isCallable$8($Symbol) && isPrototypeOf$1($Symbol.prototype, $Object$1(it));
	};

	var $String$2 = String;

	var tryToString$3 = function (argument) {
	  try {
	    return $String$2(argument);
	  } catch (error) {
	    return 'Object';
	  }
	};

	var isCallable$7 = isCallable$b;
	var tryToString$2 = tryToString$3;

	var $TypeError$7 = TypeError;

	// `Assert: IsCallable(argument) is true`
	var aCallable$3 = function (argument) {
	  if (isCallable$7(argument)) return argument;
	  throw new $TypeError$7(tryToString$2(argument) + ' is not a function');
	};

	var aCallable$2 = aCallable$3;
	var isNullOrUndefined$1 = isNullOrUndefined$3;

	// `GetMethod` abstract operation
	// https://tc39.es/ecma262/#sec-getmethod
	var getMethod$3 = function (V, P) {
	  var func = V[P];
	  return isNullOrUndefined$1(func) ? undefined : aCallable$2(func);
	};

	var call$5 = functionCall;
	var isCallable$6 = isCallable$b;
	var isObject$4 = isObject$5;

	var $TypeError$6 = TypeError;

	// `OrdinaryToPrimitive` abstract operation
	// https://tc39.es/ecma262/#sec-ordinarytoprimitive
	var ordinaryToPrimitive$1 = function (input, pref) {
	  var fn, val;
	  if (pref === 'string' && isCallable$6(fn = input.toString) && !isObject$4(val = call$5(fn, input))) return val;
	  if (isCallable$6(fn = input.valueOf) && !isObject$4(val = call$5(fn, input))) return val;
	  if (pref !== 'string' && isCallable$6(fn = input.toString) && !isObject$4(val = call$5(fn, input))) return val;
	  throw new $TypeError$6("Can't convert object to primitive value");
	};

	var sharedStore = {exports: {}};

	var globalThis$9 = globalThis_1;

	// eslint-disable-next-line es/no-object-defineproperty -- safe
	var defineProperty$2 = Object.defineProperty;

	var defineGlobalProperty$3 = function (key, value) {
	  try {
	    defineProperty$2(globalThis$9, key, { value: value, configurable: true, writable: true });
	  } catch (error) {
	    globalThis$9[key] = value;
	  } return value;
	};

	var globalThis$8 = globalThis_1;
	var defineGlobalProperty$2 = defineGlobalProperty$3;

	var SHARED = '__core-js_shared__';
	var store$3 = sharedStore.exports = globalThis$8[SHARED] || defineGlobalProperty$2(SHARED, {});

	(store$3.versions || (store$3.versions = [])).push({
	  version: '3.51.0',
	  mode: 'global',
	  copyright: '© 2013–2025 Denis Pushkarev (zloirock.ru), 2025–2026 CoreJS Company (core-js.io). All rights reserved.',
	  license: 'https://github.com/zloirock/core-js/blob/v3.51.0/LICENSE',
	  source: 'https://github.com/zloirock/core-js'
	});

	var store$2 = sharedStore.exports;
	// eslint-disable-next-line es/no-object-create -- safe
	var create$3 = Object.create || Object;

	var shared$3 = function (key, value) {
	  return store$2[key] || (store$2[key] = value || create$3(null));
	};

	var requireObjectCoercible$1 = requireObjectCoercible$3;

	var $Object = Object;

	// `ToObject` abstract operation
	// https://tc39.es/ecma262/#sec-toobject
	var toObject$2 = function (argument) {
	  return $Object(requireObjectCoercible$1(argument));
	};

	var uncurryThis$9 = functionUncurryThis;
	var toObject$1 = toObject$2;

	var hasOwnProperty = uncurryThis$9({}.hasOwnProperty);

	// `HasOwnProperty` abstract operation
	// https://tc39.es/ecma262/#sec-hasownproperty
	// eslint-disable-next-line es/no-object-hasown -- safe
	var hasOwnProperty_1 = Object.hasOwn || function hasOwn(it, key) {
	  return hasOwnProperty(toObject$1(it), key);
	};

	var uncurryThis$8 = functionUncurryThis;

	var id = 0;
	var postfix = Math.random();
	var toString = uncurryThis$8(1.1.toString);

	var uid$2 = function (key) {
	  return 'Symbol(' + (key === undefined ? '' : key) + ')_' + toString(++id + postfix, 36);
	};

	var globalThis$7 = globalThis_1;
	var shared$2 = shared$3;
	var hasOwn$6 = hasOwnProperty_1;
	var uid$1 = uid$2;
	var NATIVE_SYMBOL = symbolConstructorDetection;
	var USE_SYMBOL_AS_UID = useSymbolAsUid;

	var Symbol$1 = globalThis$7.Symbol;
	var WellKnownSymbolsStore = shared$2('wks');
	var createWellKnownSymbol = USE_SYMBOL_AS_UID ? Symbol$1['for'] || Symbol$1 : Symbol$1 && Symbol$1.withoutSetter || uid$1;

	var wellKnownSymbol$4 = function (name) {
	  if (!hasOwn$6(WellKnownSymbolsStore, name)) {
	    WellKnownSymbolsStore[name] = NATIVE_SYMBOL && hasOwn$6(Symbol$1, name)
	      ? Symbol$1[name]
	      : createWellKnownSymbol('Symbol.' + name);
	  } return WellKnownSymbolsStore[name];
	};

	var call$4 = functionCall;
	var isObject$3 = isObject$5;
	var isSymbol$1 = isSymbol$2;
	var getMethod$2 = getMethod$3;
	var ordinaryToPrimitive = ordinaryToPrimitive$1;
	var wellKnownSymbol$3 = wellKnownSymbol$4;

	var $TypeError$5 = TypeError;
	var TO_PRIMITIVE = wellKnownSymbol$3('toPrimitive');

	// `ToPrimitive` abstract operation
	// https://tc39.es/ecma262/#sec-toprimitive
	var toPrimitive$1 = function (input, pref) {
	  if (!isObject$3(input) || isSymbol$1(input)) return input;
	  var exoticToPrim = getMethod$2(input, TO_PRIMITIVE);
	  var result;
	  if (exoticToPrim) {
	    if (pref === undefined) pref = 'default';
	    result = call$4(exoticToPrim, input, pref);
	    if (!isObject$3(result) || isSymbol$1(result)) return result;
	    throw new $TypeError$5("Can't convert object to primitive value");
	  }
	  if (pref === undefined) pref = 'number';
	  return ordinaryToPrimitive(input, pref);
	};

	var toPrimitive = toPrimitive$1;
	var isSymbol = isSymbol$2;

	// `ToPropertyKey` abstract operation
	// https://tc39.es/ecma262/#sec-topropertykey
	var toPropertyKey$3 = function (argument) {
	  var key = toPrimitive(argument, 'string');
	  return isSymbol(key) ? key : key + '';
	};

	var globalThis$6 = globalThis_1;
	var isObject$2 = isObject$5;

	var document$1 = globalThis$6.document;
	// typeof document.createElement is 'object' in old IE
	var EXISTS$1 = isObject$2(document$1) && isObject$2(document$1.createElement);

	var documentCreateElement$1 = function (it) {
	  return EXISTS$1 ? document$1.createElement(it) : {};
	};

	var DESCRIPTORS$9 = descriptors;
	var fails$4 = fails$9;
	var createElement = documentCreateElement$1;

	// Thanks to IE8 for its funny defineProperty
	var ie8DomDefine = !DESCRIPTORS$9 && !fails$4(function () {
	  // eslint-disable-next-line es/no-object-defineproperty -- required for testing
	  return Object.defineProperty(createElement('div'), 'a', {
	    get: function () { return 7; }
	  }).a !== 7;
	});

	var DESCRIPTORS$8 = descriptors;
	var call$3 = functionCall;
	var propertyIsEnumerableModule = objectPropertyIsEnumerable;
	var createPropertyDescriptor$2 = createPropertyDescriptor$3;
	var toIndexedObject$3 = toIndexedObject$4;
	var toPropertyKey$2 = toPropertyKey$3;
	var hasOwn$5 = hasOwnProperty_1;
	var IE8_DOM_DEFINE$1 = ie8DomDefine;

	// eslint-disable-next-line es/no-object-getownpropertydescriptor -- safe
	var $getOwnPropertyDescriptor$1 = Object.getOwnPropertyDescriptor;

	// `Object.getOwnPropertyDescriptor` method
	// https://tc39.es/ecma262/#sec-object.getownpropertydescriptor
	objectGetOwnPropertyDescriptor.f = DESCRIPTORS$8 ? $getOwnPropertyDescriptor$1 : function getOwnPropertyDescriptor(O, P) {
	  O = toIndexedObject$3(O);
	  P = toPropertyKey$2(P);
	  if (IE8_DOM_DEFINE$1) try {
	    return $getOwnPropertyDescriptor$1(O, P);
	  } catch (error) { /* empty */ }
	  if (hasOwn$5(O, P)) return createPropertyDescriptor$2(!call$3(propertyIsEnumerableModule.f, O, P), O[P]);
	};

	var objectDefineProperty = {};

	var DESCRIPTORS$7 = descriptors;
	var fails$3 = fails$9;

	// V8 ~ Chrome 36-
	// https://bugs.chromium.org/p/v8/issues/detail?id=3334
	var v8PrototypeDefineBug = DESCRIPTORS$7 && fails$3(function () {
	  // eslint-disable-next-line es/no-object-defineproperty -- required for testing
	  return Object.defineProperty(function () { /* empty */ }, 'prototype', {
	    value: 42,
	    writable: false
	  }).prototype !== 42;
	});

	var isObject$1 = isObject$5;

	var $String$1 = String;
	var $TypeError$4 = TypeError;

	// `Assert: Type(argument) is Object`
	var anObject$7 = function (argument) {
	  if (isObject$1(argument)) return argument;
	  throw new $TypeError$4($String$1(argument) + ' is not an object');
	};

	var DESCRIPTORS$6 = descriptors;
	var IE8_DOM_DEFINE = ie8DomDefine;
	var V8_PROTOTYPE_DEFINE_BUG$1 = v8PrototypeDefineBug;
	var anObject$6 = anObject$7;
	var toPropertyKey$1 = toPropertyKey$3;

	var $TypeError$3 = TypeError;
	// eslint-disable-next-line es/no-object-defineproperty -- safe
	var $defineProperty = Object.defineProperty;
	// eslint-disable-next-line es/no-object-getownpropertydescriptor -- safe
	var $getOwnPropertyDescriptor = Object.getOwnPropertyDescriptor;
	var ENUMERABLE = 'enumerable';
	var CONFIGURABLE$1 = 'configurable';
	var WRITABLE = 'writable';

	// `Object.defineProperty` method
	// https://tc39.es/ecma262/#sec-object.defineproperty
	objectDefineProperty.f = DESCRIPTORS$6 ? V8_PROTOTYPE_DEFINE_BUG$1 ? function defineProperty(O, P, Attributes) {
	  anObject$6(O);
	  P = toPropertyKey$1(P);
	  anObject$6(Attributes);
	  if (typeof O === 'function' && P === 'prototype' && 'value' in Attributes && WRITABLE in Attributes && !Attributes[WRITABLE]) {
	    var current = $getOwnPropertyDescriptor(O, P);
	    if (current && current[WRITABLE]) {
	      O[P] = Attributes.value;
	      Attributes = {
	        configurable: CONFIGURABLE$1 in Attributes ? Attributes[CONFIGURABLE$1] : current[CONFIGURABLE$1],
	        enumerable: ENUMERABLE in Attributes ? Attributes[ENUMERABLE] : current[ENUMERABLE],
	        writable: false
	      };
	    }
	  } return $defineProperty(O, P, Attributes);
	} : $defineProperty : function defineProperty(O, P, Attributes) {
	  anObject$6(O);
	  P = toPropertyKey$1(P);
	  anObject$6(Attributes);
	  if (IE8_DOM_DEFINE) try {
	    return $defineProperty(O, P, Attributes);
	  } catch (error) { /* empty */ }
	  if ('get' in Attributes || 'set' in Attributes) throw new $TypeError$3('Accessors not supported');
	  if ('value' in Attributes) O[P] = Attributes.value;
	  return O;
	};

	var DESCRIPTORS$5 = descriptors;
	var definePropertyModule$4 = objectDefineProperty;
	var createPropertyDescriptor$1 = createPropertyDescriptor$3;

	var createNonEnumerableProperty$2 = DESCRIPTORS$5 ? function (object, key, value) {
	  return definePropertyModule$4.f(object, key, createPropertyDescriptor$1(1, value));
	} : function (object, key, value) {
	  object[key] = value;
	  return object;
	};

	var makeBuiltIn$2 = {exports: {}};

	var DESCRIPTORS$4 = descriptors;
	var hasOwn$4 = hasOwnProperty_1;

	var FunctionPrototype = Function.prototype;
	// eslint-disable-next-line es/no-object-getownpropertydescriptor -- safe
	var getDescriptor = DESCRIPTORS$4 && Object.getOwnPropertyDescriptor;

	var EXISTS = hasOwn$4(FunctionPrototype, 'name');
	// additional protection from minified / mangled / dropped function names
	var PROPER = EXISTS && function something() { /* empty */ }.name === 'something';
	var CONFIGURABLE = EXISTS && (!DESCRIPTORS$4 || (DESCRIPTORS$4 && getDescriptor(FunctionPrototype, 'name').configurable));

	var functionName = {
	  EXISTS: EXISTS,
	  PROPER: PROPER,
	  CONFIGURABLE: CONFIGURABLE
	};

	var uncurryThis$7 = functionUncurryThis;
	var isCallable$5 = isCallable$b;
	var store$1 = sharedStore.exports;

	var functionToString = uncurryThis$7(Function.toString);

	// this helper broken in `core-js@3.4.1-3.4.4`, so we can't use `shared` helper
	if (!isCallable$5(store$1.inspectSource)) {
	  store$1.inspectSource = function (it) {
	    return functionToString(it);
	  };
	}

	var inspectSource$1 = store$1.inspectSource;

	var globalThis$5 = globalThis_1;
	var isCallable$4 = isCallable$b;

	var WeakMap$1 = globalThis$5.WeakMap;

	var weakMapBasicDetection = isCallable$4(WeakMap$1) && /native code/.test(String(WeakMap$1));

	var shared$1 = shared$3;
	var uid = uid$2;

	var keys = shared$1('keys');

	var sharedKey$2 = function (key) {
	  return keys[key] || (keys[key] = uid(key));
	};

	var hiddenKeys$4 = {};

	var NATIVE_WEAK_MAP = weakMapBasicDetection;
	var globalThis$4 = globalThis_1;
	var isObject = isObject$5;
	var createNonEnumerableProperty$1 = createNonEnumerableProperty$2;
	var hasOwn$3 = hasOwnProperty_1;
	var shared = sharedStore.exports;
	var sharedKey$1 = sharedKey$2;
	var hiddenKeys$3 = hiddenKeys$4;

	var OBJECT_ALREADY_INITIALIZED = 'Object already initialized';
	var TypeError$1 = globalThis$4.TypeError;
	var WeakMap = globalThis$4.WeakMap;
	var set, get, has;

	var enforce = function (it) {
	  return has(it) ? get(it) : set(it, {});
	};

	var getterFor = function (TYPE) {
	  return function (it) {
	    var state;
	    if (!isObject(it) || (state = get(it)).type !== TYPE) {
	      throw new TypeError$1('Incompatible receiver, ' + TYPE + ' required');
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
	    if (store.has(it)) throw new TypeError$1(OBJECT_ALREADY_INITIALIZED);
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
	  var STATE = sharedKey$1('state');
	  hiddenKeys$3[STATE] = true;
	  set = function (it, metadata) {
	    if (hasOwn$3(it, STATE)) throw new TypeError$1(OBJECT_ALREADY_INITIALIZED);
	    metadata.facade = it;
	    createNonEnumerableProperty$1(it, STATE, metadata);
	    return metadata;
	  };
	  get = function (it) {
	    return hasOwn$3(it, STATE) ? it[STATE] : {};
	  };
	  has = function (it) {
	    return hasOwn$3(it, STATE);
	  };
	}

	var internalState = {
	  set: set,
	  get: get,
	  has: has,
	  enforce: enforce,
	  getterFor: getterFor
	};

	var uncurryThis$6 = functionUncurryThis;
	var fails$2 = fails$9;
	var isCallable$3 = isCallable$b;
	var hasOwn$2 = hasOwnProperty_1;
	var DESCRIPTORS$3 = descriptors;
	var CONFIGURABLE_FUNCTION_NAME = functionName.CONFIGURABLE;
	var inspectSource = inspectSource$1;
	var InternalStateModule = internalState;

	var enforceInternalState = InternalStateModule.enforce;
	var getInternalState = InternalStateModule.get;
	var $String = String;
	// eslint-disable-next-line es/no-object-defineproperty -- safe
	var defineProperty$1 = Object.defineProperty;
	var stringSlice = uncurryThis$6(''.slice);
	var replace = uncurryThis$6(''.replace);
	var join = uncurryThis$6([].join);

	var CONFIGURABLE_LENGTH = DESCRIPTORS$3 && !fails$2(function () {
	  return defineProperty$1(function () { /* empty */ }, 'length', { value: 8 }).length !== 8;
	});

	var TEMPLATE = String(String).split('String');

	var makeBuiltIn$1 = makeBuiltIn$2.exports = function (value, name, options) {
	  if (stringSlice($String(name), 0, 7) === 'Symbol(') {
	    name = '[' + replace($String(name), /^Symbol\(([^)]*)\).*$/, '$1') + ']';
	  }
	  if (options && options.getter) name = 'get ' + name;
	  if (options && options.setter) name = 'set ' + name;
	  if (!hasOwn$2(value, 'name') || (CONFIGURABLE_FUNCTION_NAME && value.name !== name)) {
	    if (DESCRIPTORS$3) defineProperty$1(value, 'name', { value: name, configurable: true });
	    else value.name = name;
	  }
	  if (CONFIGURABLE_LENGTH && options && hasOwn$2(options, 'arity') && value.length !== options.arity) {
	    defineProperty$1(value, 'length', { value: options.arity });
	  }
	  try {
	    if (options && hasOwn$2(options, 'constructor') && options.constructor) {
	      if (DESCRIPTORS$3) defineProperty$1(value, 'prototype', { writable: false });
	    // in V8 ~ Chrome 53, prototypes of some methods, like `Array.prototype.values`, are non-writable
	    } else if (value.prototype) value.prototype = undefined;
	  } catch (error) { /* empty */ }
	  var state = enforceInternalState(value);
	  if (!hasOwn$2(state, 'source')) {
	    state.source = join(TEMPLATE, typeof name == 'string' ? name : '');
	  } return value;
	};

	// add fake Function#toString for correct work wrapped methods / constructors with methods like LoDash isNative
	// eslint-disable-next-line no-extend-native -- required
	Function.prototype.toString = makeBuiltIn$1(function toString() {
	  return isCallable$3(this) && getInternalState(this).source || inspectSource(this);
	}, 'toString');

	var isCallable$2 = isCallable$b;
	var definePropertyModule$3 = objectDefineProperty;
	var makeBuiltIn = makeBuiltIn$2.exports;
	var defineGlobalProperty$1 = defineGlobalProperty$3;

	var defineBuiltIn$1 = function (O, key, value, options) {
	  if (!options) options = {};
	  var simple = options.enumerable;
	  var name = options.name !== undefined ? options.name : key;
	  if (isCallable$2(value)) makeBuiltIn(value, name, options);
	  if (options.global) {
	    if (simple) O[key] = value;
	    else defineGlobalProperty$1(key, value);
	  } else {
	    try {
	      if (!options.unsafe) delete O[key];
	      else if (O[key]) simple = true;
	    } catch (error) { /* empty */ }
	    if (simple) O[key] = value;
	    else definePropertyModule$3.f(O, key, {
	      value: value,
	      enumerable: false,
	      configurable: !options.nonConfigurable,
	      writable: !options.nonWritable
	    });
	  } return O;
	};

	var objectGetOwnPropertyNames = {};

	var ceil = Math.ceil;
	var floor = Math.floor;

	// `Math.trunc` method
	// https://tc39.es/ecma262/#sec-math.trunc
	// eslint-disable-next-line es/no-math-trunc -- safe
	var mathTrunc = Math.trunc || function trunc(x) {
	  var n = +x;
	  return (n > 0 ? floor : ceil)(n);
	};

	var trunc = mathTrunc;

	// `ToIntegerOrInfinity` abstract operation
	// https://tc39.es/ecma262/#sec-tointegerorinfinity
	var toIntegerOrInfinity$3 = function (argument) {
	  var number = +argument;
	  // eslint-disable-next-line no-self-compare -- NaN check
	  return number !== number || number === 0 ? 0 : trunc(number);
	};

	var toIntegerOrInfinity$2 = toIntegerOrInfinity$3;

	var max = Math.max;
	var min$1 = Math.min;

	// Helper for a popular repeating case of the spec:
	// Let integer be ? ToInteger(index).
	// If integer < 0, let result be max((length + integer), 0); else let result be min(integer, length).
	var toAbsoluteIndex$1 = function (index, length) {
	  var integer = toIntegerOrInfinity$2(index);
	  return integer < 0 ? max(integer + length, 0) : min$1(integer, length);
	};

	var toIntegerOrInfinity$1 = toIntegerOrInfinity$3;

	var min = Math.min;

	// `ToLength` abstract operation
	// https://tc39.es/ecma262/#sec-tolength
	var toLength$1 = function (argument) {
	  var len = toIntegerOrInfinity$1(argument);
	  return len > 0 ? min(len, 0x1FFFFFFFFFFFFF) : 0; // 2 ** 53 - 1 == 9007199254740991
	};

	var toLength = toLength$1;

	// `LengthOfArrayLike` abstract operation
	// https://tc39.es/ecma262/#sec-lengthofarraylike
	var lengthOfArrayLike$3 = function (obj) {
	  return toLength(obj.length);
	};

	var toIndexedObject$2 = toIndexedObject$4;
	var toAbsoluteIndex = toAbsoluteIndex$1;
	var lengthOfArrayLike$2 = lengthOfArrayLike$3;

	// `Array.prototype.{ indexOf, includes }` methods implementation
	var createMethod = function (IS_INCLUDES) {
	  return function ($this, el, fromIndex) {
	    var O = toIndexedObject$2($this);
	    var length = lengthOfArrayLike$2(O);
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

	var arrayIncludes = {
	  // `Array.prototype.includes` method
	  // https://tc39.es/ecma262/#sec-array.prototype.includes
	  includes: createMethod(true),
	  // `Array.prototype.indexOf` method
	  // https://tc39.es/ecma262/#sec-array.prototype.indexof
	  indexOf: createMethod(false)
	};

	var uncurryThis$5 = functionUncurryThis;
	var hasOwn$1 = hasOwnProperty_1;
	var toIndexedObject$1 = toIndexedObject$4;
	var indexOf = arrayIncludes.indexOf;
	var hiddenKeys$2 = hiddenKeys$4;

	var push$1 = uncurryThis$5([].push);

	var objectKeysInternal = function (object, names) {
	  var O = toIndexedObject$1(object);
	  var i = 0;
	  var result = [];
	  var key;
	  for (key in O) !hasOwn$1(hiddenKeys$2, key) && hasOwn$1(O, key) && push$1(result, key);
	  // Don't enum bug & hidden keys
	  while (names.length > i) if (hasOwn$1(O, key = names[i++])) {
	    ~indexOf(result, key) || push$1(result, key);
	  }
	  return result;
	};

	// IE8- don't enum bug keys
	var enumBugKeys$3 = [
	  'constructor',
	  'hasOwnProperty',
	  'isPrototypeOf',
	  'propertyIsEnumerable',
	  'toLocaleString',
	  'toString',
	  'valueOf'
	];

	var internalObjectKeys$1 = objectKeysInternal;
	var enumBugKeys$2 = enumBugKeys$3;

	var hiddenKeys$1 = enumBugKeys$2.concat('length', 'prototype');

	// `Object.getOwnPropertyNames` method
	// https://tc39.es/ecma262/#sec-object.getownpropertynames
	// eslint-disable-next-line es/no-object-getownpropertynames -- safe
	objectGetOwnPropertyNames.f = Object.getOwnPropertyNames || function getOwnPropertyNames(O) {
	  return internalObjectKeys$1(O, hiddenKeys$1);
	};

	var objectGetOwnPropertySymbols = {};

	// eslint-disable-next-line es/no-object-getownpropertysymbols -- safe
	objectGetOwnPropertySymbols.f = Object.getOwnPropertySymbols;

	var getBuiltIn$2 = getBuiltIn$4;
	var uncurryThis$4 = functionUncurryThis;
	var getOwnPropertyNamesModule = objectGetOwnPropertyNames;
	var getOwnPropertySymbolsModule = objectGetOwnPropertySymbols;
	var anObject$5 = anObject$7;

	var concat = uncurryThis$4([].concat);

	// all object keys, includes non-enumerable and symbols
	var ownKeys$1 = getBuiltIn$2('Reflect', 'ownKeys') || function ownKeys(it) {
	  var keys = getOwnPropertyNamesModule.f(anObject$5(it));
	  var getOwnPropertySymbols = getOwnPropertySymbolsModule.f;
	  return getOwnPropertySymbols ? concat(keys, getOwnPropertySymbols(it)) : keys;
	};

	var hasOwn = hasOwnProperty_1;
	var ownKeys = ownKeys$1;
	var getOwnPropertyDescriptorModule = objectGetOwnPropertyDescriptor;
	var definePropertyModule$2 = objectDefineProperty;

	var copyConstructorProperties$1 = function (target, source, exceptions) {
	  var keys = ownKeys(source);
	  var defineProperty = definePropertyModule$2.f;
	  var getOwnPropertyDescriptor = getOwnPropertyDescriptorModule.f;
	  for (var i = 0; i < keys.length; i++) {
	    var key = keys[i];
	    if (!hasOwn(target, key) && !(exceptions && hasOwn(exceptions, key))) {
	      defineProperty(target, key, getOwnPropertyDescriptor(source, key));
	    }
	  }
	};

	var fails$1 = fails$9;
	var isCallable$1 = isCallable$b;

	var replacement = /#|\.prototype\./;

	var isForced$1 = function (feature, detection) {
	  var value = data[normalize(feature)];
	  return value === POLYFILL ? true
	    : value === NATIVE ? false
	    : isCallable$1(detection) ? fails$1(detection)
	    : !!detection;
	};

	var normalize = isForced$1.normalize = function (string) {
	  return String(string).replace(replacement, '.').toLowerCase();
	};

	var data = isForced$1.data = {};
	var NATIVE = isForced$1.NATIVE = 'N';
	var POLYFILL = isForced$1.POLYFILL = 'P';

	var isForced_1 = isForced$1;

	var globalThis$3 = globalThis_1;
	var getOwnPropertyDescriptor = objectGetOwnPropertyDescriptor.f;
	var createNonEnumerableProperty = createNonEnumerableProperty$2;
	var defineBuiltIn = defineBuiltIn$1;
	var defineGlobalProperty = defineGlobalProperty$3;
	var copyConstructorProperties = copyConstructorProperties$1;
	var isForced = isForced_1;

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
	var _export = function (options, source) {
	  var TARGET = options.target;
	  var GLOBAL = options.global;
	  var STATIC = options.stat;
	  var FORCED, target, key, targetProperty, sourceProperty, descriptor;
	  if (GLOBAL) {
	    target = globalThis$3;
	  } else if (STATIC) {
	    target = globalThis$3[TARGET] || defineGlobalProperty(TARGET, {});
	  } else {
	    target = globalThis$3[TARGET] && globalThis$3[TARGET].prototype;
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

	var objectDefineProperties = {};

	var internalObjectKeys = objectKeysInternal;
	var enumBugKeys$1 = enumBugKeys$3;

	// `Object.keys` method
	// https://tc39.es/ecma262/#sec-object.keys
	// eslint-disable-next-line es/no-object-keys -- safe
	var objectKeys$1 = Object.keys || function keys(O) {
	  return internalObjectKeys(O, enumBugKeys$1);
	};

	var DESCRIPTORS$2 = descriptors;
	var V8_PROTOTYPE_DEFINE_BUG = v8PrototypeDefineBug;
	var definePropertyModule$1 = objectDefineProperty;
	var anObject$4 = anObject$7;
	var toIndexedObject = toIndexedObject$4;
	var objectKeys = objectKeys$1;

	// `Object.defineProperties` method
	// https://tc39.es/ecma262/#sec-object.defineproperties
	// eslint-disable-next-line es/no-object-defineproperties -- safe
	objectDefineProperties.f = DESCRIPTORS$2 && !V8_PROTOTYPE_DEFINE_BUG ? Object.defineProperties : function defineProperties(O, Properties) {
	  anObject$4(O);
	  var props = toIndexedObject(Properties);
	  var keys = objectKeys(Properties);
	  var length = keys.length;
	  var index = 0;
	  var key;
	  while (length > index) definePropertyModule$1.f(O, key = keys[index++], props[key]);
	  return O;
	};

	var getBuiltIn$1 = getBuiltIn$4;

	var html$1 = getBuiltIn$1('document', 'documentElement');

	/* global ActiveXObject -- old IE, WSH */
	var anObject$3 = anObject$7;
	var definePropertiesModule = objectDefineProperties;
	var enumBugKeys = enumBugKeys$3;
	var hiddenKeys = hiddenKeys$4;
	var html = html$1;
	var documentCreateElement = documentCreateElement$1;
	var sharedKey = sharedKey$2;

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
	var objectCreate = Object.create || function create(O, Properties) {
	  var result;
	  if (O !== null) {
	    EmptyConstructor[PROTOTYPE] = anObject$3(O);
	    result = new EmptyConstructor();
	    EmptyConstructor[PROTOTYPE] = null;
	    // add "__proto__" for Object.getPrototypeOf polyfill
	    result[IE_PROTO] = O;
	  } else result = NullProtoObject();
	  return Properties === undefined ? result : definePropertiesModule.f(result, Properties);
	};

	var wellKnownSymbol$2 = wellKnownSymbol$4;
	var create$2 = objectCreate;
	var defineProperty = objectDefineProperty.f;

	var UNSCOPABLES = wellKnownSymbol$2('unscopables');
	var ArrayPrototype$2 = Array.prototype;

	// Array.prototype[@@unscopables]
	// https://tc39.es/ecma262/#sec-array.prototype-@@unscopables
	if (ArrayPrototype$2[UNSCOPABLES] === undefined) {
	  defineProperty(ArrayPrototype$2, UNSCOPABLES, {
	    configurable: true,
	    value: create$2(null)
	  });
	}

	// add a key to Array.prototype[@@unscopables]
	var addToUnscopables$1 = function (key) {
	  ArrayPrototype$2[UNSCOPABLES][key] = true;
	};

	var $$2 = _export;
	var toObject = toObject$2;
	var lengthOfArrayLike$1 = lengthOfArrayLike$3;
	var toIntegerOrInfinity = toIntegerOrInfinity$3;
	var addToUnscopables = addToUnscopables$1;

	// `Array.prototype.at` method
	// https://tc39.es/ecma262/#sec-array.prototype.at
	$$2({ target: 'Array', proto: true }, {
	  at: function at(index) {
	    var O = toObject(this);
	    var len = lengthOfArrayLike$1(O);
	    var relativeIndex = toIntegerOrInfinity(index);
	    var k = relativeIndex >= 0 ? relativeIndex : len + relativeIndex;
	    return (k < 0 || k >= len) ? undefined : O[k];
	  }
	});

	addToUnscopables('at');

	var globalThis$2 = globalThis_1;
	var uncurryThis$3 = functionUncurryThis;

	var entryUnbind$1 = function (CONSTRUCTOR, METHOD) {
	  return uncurryThis$3(globalThis$2[CONSTRUCTOR].prototype[METHOD]);
	};

	var entryUnbind = entryUnbind$1;

	entryUnbind('Array', 'at');

	// TODO: Remove from `core-js@4`
	var $$1 = _export;
	var DESCRIPTORS$1 = descriptors;
	var create$1 = objectCreate;

	// `Object.create` method
	// https://tc39.es/ecma262/#sec-object.create
	$$1({ target: 'Object', stat: true, sham: !DESCRIPTORS$1 }, {
	  create: create$1
	});

	var DESCRIPTORS = descriptors;
	var definePropertyModule = objectDefineProperty;
	var createPropertyDescriptor = createPropertyDescriptor$3;

	var createProperty$1 = function (object, key, value) {
	  if (DESCRIPTORS) definePropertyModule.f(object, key, createPropertyDescriptor(0, value));
	  else object[key] = value;
	};

	var classofRaw = classofRaw$1;
	var uncurryThis$2 = functionUncurryThis;

	var functionUncurryThisClause = function (fn) {
	  // Nashorn bug:
	  //   https://github.com/zloirock/core-js/issues/1128
	  //   https://github.com/zloirock/core-js/issues/1130
	  if (classofRaw(fn) === 'Function') return uncurryThis$2(fn);
	};

	var uncurryThis$1 = functionUncurryThisClause;
	var aCallable$1 = aCallable$3;
	var NATIVE_BIND = functionBindNative;

	var bind$1 = uncurryThis$1(uncurryThis$1.bind);

	// optional / simple context binding
	var functionBindContext = function (fn, that) {
	  aCallable$1(fn);
	  return that === undefined ? fn : NATIVE_BIND ? bind$1(fn, that) : function (/* ...args */) {
	    return fn.apply(that, arguments);
	  };
	};

	var iterators = Object.create ? Object.create(null) : {};

	var wellKnownSymbol$1 = wellKnownSymbol$4;
	var Iterators = iterators;

	var ITERATOR$1 = wellKnownSymbol$1('iterator');
	var ArrayPrototype$1 = Array.prototype;

	// check on default Array iterator
	var isArrayIteratorMethod$1 = function (it) {
	  return it !== undefined && (Iterators.Array === it || ArrayPrototype$1[ITERATOR$1] === it);
	};

	var classof = classofRaw$1;
	var isNullOrUndefined = isNullOrUndefined$3;
	var getMethod$1 = getMethod$3;
	var wellKnownSymbol = wellKnownSymbol$4;

	var ITERATOR = wellKnownSymbol('iterator');
	var ArrayPrototype = Array.prototype;

	var getIteratorMethodInternal = function (it) {
	  if (!isNullOrUndefined(it)) return getMethod$1(it, ITERATOR)
	    || getMethod$1(it, '@@iterator')
	    || (classof(it) === 'Arguments' ? ArrayPrototype[ITERATOR] : undefined);
	};

	var call$2 = functionCall;
	var isCallable = isCallable$b;
	var anObject$2 = anObject$7;
	var tryToString$1 = tryToString$3;
	var getIteratorMethod$1 = getIteratorMethodInternal;

	var $TypeError$2 = TypeError;

	var getIteratorInternal = function (argument, usingIterator) {
	  var iteratorMethod = arguments.length < 2 ? getIteratorMethod$1(argument) : usingIterator;
	  if (isCallable(iteratorMethod)) return anObject$2(call$2(iteratorMethod, argument));
	  throw new $TypeError$2(tryToString$1(argument) + ' is not iterable');
	};

	var call$1 = functionCall;
	var anObject$1 = anObject$7;
	var getMethod = getMethod$3;

	var iteratorClose$1 = function (iterator, kind, value) {
	  var innerResult, innerError;
	  anObject$1(iterator);
	  try {
	    innerResult = getMethod(iterator, 'return');
	    if (!innerResult) {
	      if (kind === 'throw') throw value;
	      return value;
	    }
	    innerResult = call$1(innerResult, iterator);
	  } catch (error) {
	    innerError = true;
	    innerResult = error;
	  }
	  if (kind === 'throw') throw value;
	  if (innerError) throw innerResult;
	  anObject$1(innerResult);
	  return value;
	};

	var bind = functionBindContext;
	var call = functionCall;
	var anObject = anObject$7;
	var tryToString = tryToString$3;
	var isArrayIteratorMethod = isArrayIteratorMethod$1;
	var lengthOfArrayLike = lengthOfArrayLike$3;
	var isPrototypeOf = objectIsPrototypeOf;
	var getIterator = getIteratorInternal;
	var getIteratorMethod = getIteratorMethodInternal;
	var iteratorClose = iteratorClose$1;

	var $TypeError$1 = TypeError;

	var Result = function (stopped, result) {
	  this.stopped = stopped;
	  this.result = result;
	};

	var ResultPrototype = Result.prototype;

	var iterate$1 = function (iterable, unboundFunction, options) {
	  var that = options && options.that;
	  var AS_ENTRIES = !!(options && options.AS_ENTRIES);
	  var IS_RECORD = !!(options && options.IS_RECORD);
	  var IS_ITERATOR = !!(options && options.IS_ITERATOR);
	  var INTERRUPTED = !!(options && options.INTERRUPTED);
	  var fn = bind(unboundFunction, that);
	  var iterator, iterFn, index, length, result, next, step;

	  var stop = function (condition) {
	    var $iterator = iterator;
	    iterator = undefined;
	    if ($iterator) iteratorClose($iterator, 'normal');
	    return new Result(true, condition);
	  };

	  var callFn = function (value) {
	    if (AS_ENTRIES) {
	      anObject(value);
	      return INTERRUPTED ? fn(value[0], value[1], stop) : fn(value[0], value[1]);
	    } return INTERRUPTED ? fn(value, stop) : fn(value);
	  };

	  if (IS_RECORD) {
	    iterator = iterable.iterator;
	  } else if (IS_ITERATOR) {
	    iterator = iterable;
	  } else {
	    iterFn = getIteratorMethod(iterable);
	    if (!iterFn) throw new $TypeError$1(tryToString(iterable) + ' is not iterable');
	    // optimisation for array iterators
	    if (isArrayIteratorMethod(iterFn)) {
	      for (index = 0, length = lengthOfArrayLike(iterable); length > index; index++) {
	        result = callFn(iterable[index]);
	        if (result && isPrototypeOf(ResultPrototype, result)) return result;
	      } return new Result(false);
	    }
	    iterator = getIterator(iterable, iterFn);
	  }

	  next = IS_RECORD ? iterable.next : iterator.next;
	  while (!(step = call(next, iterator)).done) {
	    // `IteratorValue` errors should propagate without closing the iterator
	    var value = step.value;
	    try {
	      result = callFn(value);
	    } catch (error) {
	      if (iterator) iteratorClose(iterator, 'throw', error);
	      else throw error;
	    }
	    if (typeof result == 'object' && result && isPrototypeOf(ResultPrototype, result)) return result;
	  } return new Result(false);
	};

	var $TypeError = TypeError;
	var MAX_SAFE_INTEGER = 0x1FFFFFFFFFFFFF; // 2 ** 53 - 1 == 9007199254740991

	var doesNotExceedSafeInteger$1 = function (it) {
	  if (it > MAX_SAFE_INTEGER) throw new $TypeError('Maximum allowed index exceeded');
	  return it;
	};

	var $ = _export;
	var createProperty = createProperty$1;
	var getBuiltIn = getBuiltIn$4;
	var uncurryThis = functionUncurryThis;
	var aCallable = aCallable$3;
	var requireObjectCoercible = requireObjectCoercible$3;
	var toPropertyKey = toPropertyKey$3;
	var iterate = iterate$1;
	var doesNotExceedSafeInteger = doesNotExceedSafeInteger$1;
	var fails = fails$9;

	// eslint-disable-next-line es/no-object-groupby -- testing
	var nativeGroupBy = Object.groupBy;
	var create = getBuiltIn('Object', 'create');
	var push = uncurryThis([].push);

	// https://bugs.webkit.org/show_bug.cgi?id=271524
	var DOES_NOT_WORK_WITH_PRIMITIVES = !nativeGroupBy || fails(function () {
	  return nativeGroupBy('ab', function (it) {
	    return it;
	  }).a.length !== 1;
	});

	// `Object.groupBy` method
	// https://tc39.es/ecma262/#sec-object.groupby
	$({ target: 'Object', stat: true, forced: DOES_NOT_WORK_WITH_PRIMITIVES }, {
	  groupBy: function groupBy(items, callbackfn) {
	    requireObjectCoercible(items);
	    aCallable(callbackfn);
	    var obj = create(null);
	    var k = 0;
	    iterate(items, function (value) {
	      doesNotExceedSafeInteger(k);
	      var key = toPropertyKey(callbackfn(value, k++));
	      // in some IE versions, `hasOwnProperty` returns incorrect result on integer keys
	      // but since it's a `null` prototype object, we can safely use `in`
	      if (key in obj) push(obj[key], value);
	      else createProperty(obj, key, [value]);
	    });
	    return obj;
	  }
	});

	var globalThis$1 = globalThis_1;

	var path$1 = globalThis$1;

	var path = path$1;

	path.Object.groupBy;

	console.log("at " + [1, 2, 3].at(-1));
	var groups = Object.groupBy([1, 2, 3, 4], function (n) {
	  return n % 2 ? "odd" : "even";
	});
	console.log("group " + groups.odd.join(",") + " " + groups.even.join(","));

	return polyfill;

})();
