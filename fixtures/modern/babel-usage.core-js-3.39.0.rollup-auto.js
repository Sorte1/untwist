(function () {
	'use strict';

	var commonjsGlobal = typeof globalThis !== 'undefined' ? globalThis : typeof window !== 'undefined' ? window : typeof global !== 'undefined' ? global : typeof self !== 'undefined' ? self : {};

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
	  check(typeof globalThis_1 == 'object' && globalThis_1) ||
	  // eslint-disable-next-line no-new-func -- fallback
	  (function () { return this; })() || Function('return this')();

	var objectGetOwnPropertyDescriptor = {};

	var fails$S = function (exec) {
	  try {
	    return !!exec();
	  } catch (error) {
	    return true;
	  }
	};

	var fails$R = fails$S;

	// Detect IE8's incomplete defineProperty implementation
	var descriptors = !fails$R(function () {
	  // eslint-disable-next-line es/no-object-defineproperty -- required for testing
	  return Object.defineProperty({}, 1, { get: function () { return 7; } })[1] !== 7;
	});

	var fails$Q = fails$S;

	var functionBindNative = !fails$Q(function () {
	  // eslint-disable-next-line es/no-function-prototype-bind -- safe
	  var test = (function () { /* empty */ }).bind();
	  // eslint-disable-next-line no-prototype-builtins -- safe
	  return typeof test != 'function' || test.hasOwnProperty('prototype');
	});

	var NATIVE_BIND$3 = functionBindNative;

	var call$x = Function.prototype.call;

	var functionCall = NATIVE_BIND$3 ? call$x.bind(call$x) : function () {
	  return call$x.apply(call$x, arguments);
	};

	var objectPropertyIsEnumerable = {};

	var $propertyIsEnumerable$2 = {}.propertyIsEnumerable;
	// eslint-disable-next-line es/no-object-getownpropertydescriptor -- safe
	var getOwnPropertyDescriptor$6 = Object.getOwnPropertyDescriptor;

	// Nashorn ~ JDK8 bug
	var NASHORN_BUG = getOwnPropertyDescriptor$6 && !$propertyIsEnumerable$2.call({ 1: 2 }, 1);

	// `Object.prototype.propertyIsEnumerable` method implementation
	// https://tc39.es/ecma262/#sec-object.prototype.propertyisenumerable
	objectPropertyIsEnumerable.f = NASHORN_BUG ? function propertyIsEnumerable(V) {
	  var descriptor = getOwnPropertyDescriptor$6(this, V);
	  return !!descriptor && descriptor.enumerable;
	} : $propertyIsEnumerable$2;

	var createPropertyDescriptor$a = function (bitmap, value) {
	  return {
	    enumerable: !(bitmap & 1),
	    configurable: !(bitmap & 2),
	    writable: !(bitmap & 4),
	    value: value
	  };
	};

	var NATIVE_BIND$2 = functionBindNative;

	var FunctionPrototype$3 = Function.prototype;
	var call$w = FunctionPrototype$3.call;
	var uncurryThisWithBind = NATIVE_BIND$2 && FunctionPrototype$3.bind.bind(call$w, call$w);

	var functionUncurryThis = NATIVE_BIND$2 ? uncurryThisWithBind : function (fn) {
	  return function () {
	    return call$w.apply(fn, arguments);
	  };
	};

	var uncurryThis$X = functionUncurryThis;

	var toString$q = uncurryThis$X({}.toString);
	var stringSlice$d = uncurryThis$X(''.slice);

	var classofRaw$2 = function (it) {
	  return stringSlice$d(toString$q(it), 8, -1);
	};

	var uncurryThis$W = functionUncurryThis;
	var fails$P = fails$S;
	var classof$e = classofRaw$2;

	var $Object$5 = Object;
	var split$3 = uncurryThis$W(''.split);

	// fallback for non-array-like ES3 and non-enumerable old V8 strings
	var indexedObject = fails$P(function () {
	  // throws an error in rhino, see https://github.com/mozilla/rhino/issues/346
	  // eslint-disable-next-line no-prototype-builtins -- safe
	  return !$Object$5('z').propertyIsEnumerable(0);
	}) ? function (it) {
	  return classof$e(it) === 'String' ? split$3(it, '') : $Object$5(it);
	} : $Object$5;

	// we can't use just `it == null` since of `document.all` special case
	// https://tc39.es/ecma262/#sec-IsHTMLDDA-internal-slot-aec
	var isNullOrUndefined$b = function (it) {
	  return it === null || it === undefined;
	};

	var isNullOrUndefined$a = isNullOrUndefined$b;

	var $TypeError$m = TypeError;

	// `RequireObjectCoercible` abstract operation
	// https://tc39.es/ecma262/#sec-requireobjectcoercible
	var requireObjectCoercible$g = function (it) {
	  if (isNullOrUndefined$a(it)) throw new $TypeError$m("Can't call method on " + it);
	  return it;
	};

	// toObject with fallback for non-array-like ES3 strings
	var IndexedObject$4 = indexedObject;
	var requireObjectCoercible$f = requireObjectCoercible$g;

	var toIndexedObject$d = function (it) {
	  return IndexedObject$4(requireObjectCoercible$f(it));
	};

	// https://tc39.es/ecma262/#sec-IsHTMLDDA-internal-slot
	var documentAll = typeof document == 'object' && document.all;

	// `IsCallable` abstract operation
	// https://tc39.es/ecma262/#sec-iscallable
	// eslint-disable-next-line unicorn/no-typeof-undefined -- required for testing
	var isCallable$w = typeof documentAll == 'undefined' && documentAll !== undefined ? function (argument) {
	  return typeof argument == 'function' || argument === documentAll;
	} : function (argument) {
	  return typeof argument == 'function';
	};

	var isCallable$v = isCallable$w;

	var isObject$n = function (it) {
	  return typeof it == 'object' ? it !== null : isCallable$v(it);
	};

	var globalThis$F = globalThis_1;
	var isCallable$u = isCallable$w;

	var aFunction = function (argument) {
	  return isCallable$u(argument) ? argument : undefined;
	};

	var getBuiltIn$k = function (namespace, method) {
	  return arguments.length < 2 ? aFunction(globalThis$F[namespace]) : globalThis$F[namespace] && globalThis$F[namespace][method];
	};

	var uncurryThis$V = functionUncurryThis;

	var objectIsPrototypeOf = uncurryThis$V({}.isPrototypeOf);

	var globalThis$E = globalThis_1;

	var navigator = globalThis$E.navigator;
	var userAgent$8 = navigator && navigator.userAgent;

	var environmentUserAgent = userAgent$8 ? String(userAgent$8) : '';

	var globalThis$D = globalThis_1;
	var userAgent$7 = environmentUserAgent;

	var process$3 = globalThis$D.process;
	var Deno$1 = globalThis$D.Deno;
	var versions = process$3 && process$3.versions || Deno$1 && Deno$1.version;
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
	if (!version && userAgent$7) {
	  match = userAgent$7.match(/Edge\/(\d+)/);
	  if (!match || match[1] >= 74) {
	    match = userAgent$7.match(/Chrome\/(\d+)/);
	    if (match) version = +match[1];
	  }
	}

	var environmentV8Version = version;

	/* eslint-disable es/no-symbol -- required for testing */
	var V8_VERSION$2 = environmentV8Version;
	var fails$O = fails$S;
	var globalThis$C = globalThis_1;

	var $String$7 = globalThis$C.String;

	// eslint-disable-next-line es/no-object-getownpropertysymbols -- required for testing
	var symbolConstructorDetection = !!Object.getOwnPropertySymbols && !fails$O(function () {
	  var symbol = Symbol('symbol detection');
	  // Chrome 38 Symbol has incorrect toString conversion
	  // `get-own-property-symbols` polyfill symbols converted to object are not Symbol instances
	  // nb: Do not call `String` directly to avoid this being optimized out to `symbol+''` which will,
	  // of course, fail.
	  return !$String$7(symbol) || !(Object(symbol) instanceof Symbol) ||
	    // Chrome 38-40 symbols are not inherited from DOM collections prototypes to instances
	    !Symbol.sham && V8_VERSION$2 && V8_VERSION$2 < 41;
	});

	/* eslint-disable es/no-symbol -- required for testing */
	var NATIVE_SYMBOL$6 = symbolConstructorDetection;

	var useSymbolAsUid = NATIVE_SYMBOL$6 &&
	  !Symbol.sham &&
	  typeof Symbol.iterator == 'symbol';

	var getBuiltIn$j = getBuiltIn$k;
	var isCallable$t = isCallable$w;
	var isPrototypeOf$8 = objectIsPrototypeOf;
	var USE_SYMBOL_AS_UID$1 = useSymbolAsUid;

	var $Object$4 = Object;

	var isSymbol$6 = USE_SYMBOL_AS_UID$1 ? function (it) {
	  return typeof it == 'symbol';
	} : function (it) {
	  var $Symbol = getBuiltIn$j('Symbol');
	  return isCallable$t($Symbol) && isPrototypeOf$8($Symbol.prototype, $Object$4(it));
	};

	var $String$6 = String;

	var tryToString$6 = function (argument) {
	  try {
	    return $String$6(argument);
	  } catch (error) {
	    return 'Object';
	  }
	};

	var isCallable$s = isCallable$w;
	var tryToString$5 = tryToString$6;

	var $TypeError$l = TypeError;

	// `Assert: IsCallable(argument) is true`
	var aCallable$i = function (argument) {
	  if (isCallable$s(argument)) return argument;
	  throw new $TypeError$l(tryToString$5(argument) + ' is not a function');
	};

	var aCallable$h = aCallable$i;
	var isNullOrUndefined$9 = isNullOrUndefined$b;

	// `GetMethod` abstract operation
	// https://tc39.es/ecma262/#sec-getmethod
	var getMethod$7 = function (V, P) {
	  var func = V[P];
	  return isNullOrUndefined$9(func) ? undefined : aCallable$h(func);
	};

	var call$v = functionCall;
	var isCallable$r = isCallable$w;
	var isObject$m = isObject$n;

	var $TypeError$k = TypeError;

	// `OrdinaryToPrimitive` abstract operation
	// https://tc39.es/ecma262/#sec-ordinarytoprimitive
	var ordinaryToPrimitive$2 = function (input, pref) {
	  var fn, val;
	  if (pref === 'string' && isCallable$r(fn = input.toString) && !isObject$m(val = call$v(fn, input))) return val;
	  if (isCallable$r(fn = input.valueOf) && !isObject$m(val = call$v(fn, input))) return val;
	  if (pref !== 'string' && isCallable$r(fn = input.toString) && !isObject$m(val = call$v(fn, input))) return val;
	  throw new $TypeError$k("Can't convert object to primitive value");
	};

	var sharedStore = {exports: {}};

	var isPure = false;

	var globalThis$B = globalThis_1;

	// eslint-disable-next-line es/no-object-defineproperty -- safe
	var defineProperty$c = Object.defineProperty;

	var defineGlobalProperty$3 = function (key, value) {
	  try {
	    defineProperty$c(globalThis$B, key, { value: value, configurable: true, writable: true });
	  } catch (error) {
	    globalThis$B[key] = value;
	  } return value;
	};

	var globalThis$A = globalThis_1;
	var defineGlobalProperty$2 = defineGlobalProperty$3;

	var SHARED = '__core-js_shared__';
	var store$3 = sharedStore.exports = globalThis$A[SHARED] || defineGlobalProperty$2(SHARED, {});

	(store$3.versions || (store$3.versions = [])).push({
	  version: '3.39.0',
	  mode: 'global',
	  copyright: '© 2014-2024 Denis Pushkarev (zloirock.ru)',
	  license: 'https://github.com/zloirock/core-js/blob/v3.39.0/LICENSE',
	  source: 'https://github.com/zloirock/core-js'
	});

	var sharedStoreExports = sharedStore.exports;

	var store$2 = sharedStoreExports;

	var shared$7 = function (key, value) {
	  return store$2[key] || (store$2[key] = value || {});
	};

	var requireObjectCoercible$e = requireObjectCoercible$g;

	var $Object$3 = Object;

	// `ToObject` abstract operation
	// https://tc39.es/ecma262/#sec-toobject
	var toObject$g = function (argument) {
	  return $Object$3(requireObjectCoercible$e(argument));
	};

	var uncurryThis$U = functionUncurryThis;
	var toObject$f = toObject$g;

	var hasOwnProperty = uncurryThis$U({}.hasOwnProperty);

	// `HasOwnProperty` abstract operation
	// https://tc39.es/ecma262/#sec-hasownproperty
	// eslint-disable-next-line es/no-object-hasown -- safe
	var hasOwnProperty_1 = Object.hasOwn || function hasOwn(it, key) {
	  return hasOwnProperty(toObject$f(it), key);
	};

	var uncurryThis$T = functionUncurryThis;

	var id$2 = 0;
	var postfix = Math.random();
	var toString$p = uncurryThis$T(1.0.toString);

	var uid$5 = function (key) {
	  return 'Symbol(' + (key === undefined ? '' : key) + ')_' + toString$p(++id$2 + postfix, 36);
	};

	var globalThis$z = globalThis_1;
	var shared$6 = shared$7;
	var hasOwn$r = hasOwnProperty_1;
	var uid$4 = uid$5;
	var NATIVE_SYMBOL$5 = symbolConstructorDetection;
	var USE_SYMBOL_AS_UID = useSymbolAsUid;

	var Symbol$2 = globalThis$z.Symbol;
	var WellKnownSymbolsStore$1 = shared$6('wks');
	var createWellKnownSymbol = USE_SYMBOL_AS_UID ? Symbol$2['for'] || Symbol$2 : Symbol$2 && Symbol$2.withoutSetter || uid$4;

	var wellKnownSymbol$w = function (name) {
	  if (!hasOwn$r(WellKnownSymbolsStore$1, name)) {
	    WellKnownSymbolsStore$1[name] = NATIVE_SYMBOL$5 && hasOwn$r(Symbol$2, name)
	      ? Symbol$2[name]
	      : createWellKnownSymbol('Symbol.' + name);
	  } return WellKnownSymbolsStore$1[name];
	};

	var call$u = functionCall;
	var isObject$l = isObject$n;
	var isSymbol$5 = isSymbol$6;
	var getMethod$6 = getMethod$7;
	var ordinaryToPrimitive$1 = ordinaryToPrimitive$2;
	var wellKnownSymbol$v = wellKnownSymbol$w;

	var $TypeError$j = TypeError;
	var TO_PRIMITIVE$1 = wellKnownSymbol$v('toPrimitive');

	// `ToPrimitive` abstract operation
	// https://tc39.es/ecma262/#sec-toprimitive
	var toPrimitive$2 = function (input, pref) {
	  if (!isObject$l(input) || isSymbol$5(input)) return input;
	  var exoticToPrim = getMethod$6(input, TO_PRIMITIVE$1);
	  var result;
	  if (exoticToPrim) {
	    if (pref === undefined) pref = 'default';
	    result = call$u(exoticToPrim, input, pref);
	    if (!isObject$l(result) || isSymbol$5(result)) return result;
	    throw new $TypeError$j("Can't convert object to primitive value");
	  }
	  if (pref === undefined) pref = 'number';
	  return ordinaryToPrimitive$1(input, pref);
	};

	var toPrimitive$1 = toPrimitive$2;
	var isSymbol$4 = isSymbol$6;

	// `ToPropertyKey` abstract operation
	// https://tc39.es/ecma262/#sec-topropertykey
	var toPropertyKey$4 = function (argument) {
	  var key = toPrimitive$1(argument, 'string');
	  return isSymbol$4(key) ? key : key + '';
	};

	var globalThis$y = globalThis_1;
	var isObject$k = isObject$n;

	var document$3 = globalThis$y.document;
	// typeof document.createElement is 'object' in old IE
	var EXISTS$1 = isObject$k(document$3) && isObject$k(document$3.createElement);

	var documentCreateElement$2 = function (it) {
	  return EXISTS$1 ? document$3.createElement(it) : {};
	};

	var DESCRIPTORS$t = descriptors;
	var fails$N = fails$S;
	var createElement$1 = documentCreateElement$2;

	// Thanks to IE8 for its funny defineProperty
	var ie8DomDefine = !DESCRIPTORS$t && !fails$N(function () {
	  // eslint-disable-next-line es/no-object-defineproperty -- required for testing
	  return Object.defineProperty(createElement$1('div'), 'a', {
	    get: function () { return 7; }
	  }).a !== 7;
	});

	var DESCRIPTORS$s = descriptors;
	var call$t = functionCall;
	var propertyIsEnumerableModule$2 = objectPropertyIsEnumerable;
	var createPropertyDescriptor$9 = createPropertyDescriptor$a;
	var toIndexedObject$c = toIndexedObject$d;
	var toPropertyKey$3 = toPropertyKey$4;
	var hasOwn$q = hasOwnProperty_1;
	var IE8_DOM_DEFINE$1 = ie8DomDefine;

	// eslint-disable-next-line es/no-object-getownpropertydescriptor -- safe
	var $getOwnPropertyDescriptor$2 = Object.getOwnPropertyDescriptor;

	// `Object.getOwnPropertyDescriptor` method
	// https://tc39.es/ecma262/#sec-object.getownpropertydescriptor
	objectGetOwnPropertyDescriptor.f = DESCRIPTORS$s ? $getOwnPropertyDescriptor$2 : function getOwnPropertyDescriptor(O, P) {
	  O = toIndexedObject$c(O);
	  P = toPropertyKey$3(P);
	  if (IE8_DOM_DEFINE$1) try {
	    return $getOwnPropertyDescriptor$2(O, P);
	  } catch (error) { /* empty */ }
	  if (hasOwn$q(O, P)) return createPropertyDescriptor$9(!call$t(propertyIsEnumerableModule$2.f, O, P), O[P]);
	};

	var objectDefineProperty = {};

	var DESCRIPTORS$r = descriptors;
	var fails$M = fails$S;

	// V8 ~ Chrome 36-
	// https://bugs.chromium.org/p/v8/issues/detail?id=3334
	var v8PrototypeDefineBug = DESCRIPTORS$r && fails$M(function () {
	  // eslint-disable-next-line es/no-object-defineproperty -- required for testing
	  return Object.defineProperty(function () { /* empty */ }, 'prototype', {
	    value: 42,
	    writable: false
	  }).prototype !== 42;
	});

	var isObject$j = isObject$n;

	var $String$5 = String;
	var $TypeError$i = TypeError;

	// `Assert: Type(argument) is Object`
	var anObject$q = function (argument) {
	  if (isObject$j(argument)) return argument;
	  throw new $TypeError$i($String$5(argument) + ' is not an object');
	};

	var DESCRIPTORS$q = descriptors;
	var IE8_DOM_DEFINE = ie8DomDefine;
	var V8_PROTOTYPE_DEFINE_BUG$1 = v8PrototypeDefineBug;
	var anObject$p = anObject$q;
	var toPropertyKey$2 = toPropertyKey$4;

	var $TypeError$h = TypeError;
	// eslint-disable-next-line es/no-object-defineproperty -- safe
	var $defineProperty$1 = Object.defineProperty;
	// eslint-disable-next-line es/no-object-getownpropertydescriptor -- safe
	var $getOwnPropertyDescriptor$1 = Object.getOwnPropertyDescriptor;
	var ENUMERABLE = 'enumerable';
	var CONFIGURABLE$1 = 'configurable';
	var WRITABLE = 'writable';

	// `Object.defineProperty` method
	// https://tc39.es/ecma262/#sec-object.defineproperty
	objectDefineProperty.f = DESCRIPTORS$q ? V8_PROTOTYPE_DEFINE_BUG$1 ? function defineProperty(O, P, Attributes) {
	  anObject$p(O);
	  P = toPropertyKey$2(P);
	  anObject$p(Attributes);
	  if (typeof O === 'function' && P === 'prototype' && 'value' in Attributes && WRITABLE in Attributes && !Attributes[WRITABLE]) {
	    var current = $getOwnPropertyDescriptor$1(O, P);
	    if (current && current[WRITABLE]) {
	      O[P] = Attributes.value;
	      Attributes = {
	        configurable: CONFIGURABLE$1 in Attributes ? Attributes[CONFIGURABLE$1] : current[CONFIGURABLE$1],
	        enumerable: ENUMERABLE in Attributes ? Attributes[ENUMERABLE] : current[ENUMERABLE],
	        writable: false
	      };
	    }
	  } return $defineProperty$1(O, P, Attributes);
	} : $defineProperty$1 : function defineProperty(O, P, Attributes) {
	  anObject$p(O);
	  P = toPropertyKey$2(P);
	  anObject$p(Attributes);
	  if (IE8_DOM_DEFINE) try {
	    return $defineProperty$1(O, P, Attributes);
	  } catch (error) { /* empty */ }
	  if ('get' in Attributes || 'set' in Attributes) throw new $TypeError$h('Accessors not supported');
	  if ('value' in Attributes) O[P] = Attributes.value;
	  return O;
	};

	var DESCRIPTORS$p = descriptors;
	var definePropertyModule$5 = objectDefineProperty;
	var createPropertyDescriptor$8 = createPropertyDescriptor$a;

	var createNonEnumerableProperty$b = DESCRIPTORS$p ? function (object, key, value) {
	  return definePropertyModule$5.f(object, key, createPropertyDescriptor$8(1, value));
	} : function (object, key, value) {
	  object[key] = value;
	  return object;
	};

	var makeBuiltIn$3 = {exports: {}};

	var DESCRIPTORS$o = descriptors;
	var hasOwn$p = hasOwnProperty_1;

	var FunctionPrototype$2 = Function.prototype;
	// eslint-disable-next-line es/no-object-getownpropertydescriptor -- safe
	var getDescriptor = DESCRIPTORS$o && Object.getOwnPropertyDescriptor;

	var EXISTS = hasOwn$p(FunctionPrototype$2, 'name');
	// additional protection from minified / mangled / dropped function names
	var PROPER = EXISTS && (function something() { /* empty */ }).name === 'something';
	var CONFIGURABLE = EXISTS && (!DESCRIPTORS$o || (DESCRIPTORS$o && getDescriptor(FunctionPrototype$2, 'name').configurable));

	var functionName = {
	  EXISTS: EXISTS,
	  PROPER: PROPER,
	  CONFIGURABLE: CONFIGURABLE
	};

	var uncurryThis$S = functionUncurryThis;
	var isCallable$q = isCallable$w;
	var store$1 = sharedStoreExports;

	var functionToString$1 = uncurryThis$S(Function.toString);

	// this helper broken in `core-js@3.4.1-3.4.4`, so we can't use `shared` helper
	if (!isCallable$q(store$1.inspectSource)) {
	  store$1.inspectSource = function (it) {
	    return functionToString$1(it);
	  };
	}

	var inspectSource$3 = store$1.inspectSource;

	var globalThis$x = globalThis_1;
	var isCallable$p = isCallable$w;

	var WeakMap$2 = globalThis$x.WeakMap;

	var weakMapBasicDetection = isCallable$p(WeakMap$2) && /native code/.test(String(WeakMap$2));

	var shared$5 = shared$7;
	var uid$3 = uid$5;

	var keys$1 = shared$5('keys');

	var sharedKey$4 = function (key) {
	  return keys$1[key] || (keys$1[key] = uid$3(key));
	};

	var hiddenKeys$6 = {};

	var NATIVE_WEAK_MAP$1 = weakMapBasicDetection;
	var globalThis$w = globalThis_1;
	var isObject$i = isObject$n;
	var createNonEnumerableProperty$a = createNonEnumerableProperty$b;
	var hasOwn$o = hasOwnProperty_1;
	var shared$4 = sharedStoreExports;
	var sharedKey$3 = sharedKey$4;
	var hiddenKeys$5 = hiddenKeys$6;

	var OBJECT_ALREADY_INITIALIZED = 'Object already initialized';
	var TypeError$7 = globalThis$w.TypeError;
	var WeakMap$1 = globalThis$w.WeakMap;
	var set$5, get$4, has$c;

	var enforce = function (it) {
	  return has$c(it) ? get$4(it) : set$5(it, {});
	};

	var getterFor$1 = function (TYPE) {
	  return function (it) {
	    var state;
	    if (!isObject$i(it) || (state = get$4(it)).type !== TYPE) {
	      throw new TypeError$7('Incompatible receiver, ' + TYPE + ' required');
	    } return state;
	  };
	};

	if (NATIVE_WEAK_MAP$1 || shared$4.state) {
	  var store = shared$4.state || (shared$4.state = new WeakMap$1());
	  /* eslint-disable no-self-assign -- prototype methods protection */
	  store.get = store.get;
	  store.has = store.has;
	  store.set = store.set;
	  /* eslint-enable no-self-assign -- prototype methods protection */
	  set$5 = function (it, metadata) {
	    if (store.has(it)) throw new TypeError$7(OBJECT_ALREADY_INITIALIZED);
	    metadata.facade = it;
	    store.set(it, metadata);
	    return metadata;
	  };
	  get$4 = function (it) {
	    return store.get(it) || {};
	  };
	  has$c = function (it) {
	    return store.has(it);
	  };
	} else {
	  var STATE = sharedKey$3('state');
	  hiddenKeys$5[STATE] = true;
	  set$5 = function (it, metadata) {
	    if (hasOwn$o(it, STATE)) throw new TypeError$7(OBJECT_ALREADY_INITIALIZED);
	    metadata.facade = it;
	    createNonEnumerableProperty$a(it, STATE, metadata);
	    return metadata;
	  };
	  get$4 = function (it) {
	    return hasOwn$o(it, STATE) ? it[STATE] : {};
	  };
	  has$c = function (it) {
	    return hasOwn$o(it, STATE);
	  };
	}

	var internalState = {
	  set: set$5,
	  get: get$4,
	  has: has$c,
	  enforce: enforce,
	  getterFor: getterFor$1
	};

	var uncurryThis$R = functionUncurryThis;
	var fails$L = fails$S;
	var isCallable$o = isCallable$w;
	var hasOwn$n = hasOwnProperty_1;
	var DESCRIPTORS$n = descriptors;
	var CONFIGURABLE_FUNCTION_NAME$1 = functionName.CONFIGURABLE;
	var inspectSource$2 = inspectSource$3;
	var InternalStateModule$b = internalState;

	var enforceInternalState$1 = InternalStateModule$b.enforce;
	var getInternalState$6 = InternalStateModule$b.get;
	var $String$4 = String;
	// eslint-disable-next-line es/no-object-defineproperty -- safe
	var defineProperty$b = Object.defineProperty;
	var stringSlice$c = uncurryThis$R(''.slice);
	var replace$9 = uncurryThis$R(''.replace);
	var join$4 = uncurryThis$R([].join);

	var CONFIGURABLE_LENGTH = DESCRIPTORS$n && !fails$L(function () {
	  return defineProperty$b(function () { /* empty */ }, 'length', { value: 8 }).length !== 8;
	});

	var TEMPLATE = String(String).split('String');

	var makeBuiltIn$2 = makeBuiltIn$3.exports = function (value, name, options) {
	  if (stringSlice$c($String$4(name), 0, 7) === 'Symbol(') {
	    name = '[' + replace$9($String$4(name), /^Symbol\(([^)]*)\).*$/, '$1') + ']';
	  }
	  if (options && options.getter) name = 'get ' + name;
	  if (options && options.setter) name = 'set ' + name;
	  if (!hasOwn$n(value, 'name') || (CONFIGURABLE_FUNCTION_NAME$1 && value.name !== name)) {
	    if (DESCRIPTORS$n) defineProperty$b(value, 'name', { value: name, configurable: true });
	    else value.name = name;
	  }
	  if (CONFIGURABLE_LENGTH && options && hasOwn$n(options, 'arity') && value.length !== options.arity) {
	    defineProperty$b(value, 'length', { value: options.arity });
	  }
	  try {
	    if (options && hasOwn$n(options, 'constructor') && options.constructor) {
	      if (DESCRIPTORS$n) defineProperty$b(value, 'prototype', { writable: false });
	    // in V8 ~ Chrome 53, prototypes of some methods, like `Array.prototype.values`, are non-writable
	    } else if (value.prototype) value.prototype = undefined;
	  } catch (error) { /* empty */ }
	  var state = enforceInternalState$1(value);
	  if (!hasOwn$n(state, 'source')) {
	    state.source = join$4(TEMPLATE, typeof name == 'string' ? name : '');
	  } return value;
	};

	// add fake Function#toString for correct work wrapped methods / constructors with methods like LoDash isNative
	// eslint-disable-next-line no-extend-native -- required
	Function.prototype.toString = makeBuiltIn$2(function toString() {
	  return isCallable$o(this) && getInternalState$6(this).source || inspectSource$2(this);
	}, 'toString');

	var makeBuiltInExports = makeBuiltIn$3.exports;

	var isCallable$n = isCallable$w;
	var definePropertyModule$4 = objectDefineProperty;
	var makeBuiltIn$1 = makeBuiltInExports;
	var defineGlobalProperty$1 = defineGlobalProperty$3;

	var defineBuiltIn$k = function (O, key, value, options) {
	  if (!options) options = {};
	  var simple = options.enumerable;
	  var name = options.name !== undefined ? options.name : key;
	  if (isCallable$n(value)) makeBuiltIn$1(value, name, options);
	  if (options.global) {
	    if (simple) O[key] = value;
	    else defineGlobalProperty$1(key, value);
	  } else {
	    try {
	      if (!options.unsafe) delete O[key];
	      else if (O[key]) simple = true;
	    } catch (error) { /* empty */ }
	    if (simple) O[key] = value;
	    else definePropertyModule$4.f(O, key, {
	      value: value,
	      enumerable: false,
	      configurable: !options.nonConfigurable,
	      writable: !options.nonWritable
	    });
	  } return O;
	};

	var objectGetOwnPropertyNames = {};

	var ceil$1 = Math.ceil;
	var floor$6 = Math.floor;

	// `Math.trunc` method
	// https://tc39.es/ecma262/#sec-math.trunc
	// eslint-disable-next-line es/no-math-trunc -- safe
	var mathTrunc = Math.trunc || function trunc(x) {
	  var n = +x;
	  return (n > 0 ? floor$6 : ceil$1)(n);
	};

	var trunc$1 = mathTrunc;

	// `ToIntegerOrInfinity` abstract operation
	// https://tc39.es/ecma262/#sec-tointegerorinfinity
	var toIntegerOrInfinity$b = function (argument) {
	  var number = +argument;
	  // eslint-disable-next-line no-self-compare -- NaN check
	  return number !== number || number === 0 ? 0 : trunc$1(number);
	};

	var toIntegerOrInfinity$a = toIntegerOrInfinity$b;

	var max$4 = Math.max;
	var min$4 = Math.min;

	// Helper for a popular repeating case of the spec:
	// Let integer be ? ToInteger(index).
	// If integer < 0, let result be max((length + integer), 0); else let result be min(integer, length).
	var toAbsoluteIndex$4 = function (index, length) {
	  var integer = toIntegerOrInfinity$a(index);
	  return integer < 0 ? max$4(integer + length, 0) : min$4(integer, length);
	};

	var toIntegerOrInfinity$9 = toIntegerOrInfinity$b;

	var min$3 = Math.min;

	// `ToLength` abstract operation
	// https://tc39.es/ecma262/#sec-tolength
	var toLength$6 = function (argument) {
	  var len = toIntegerOrInfinity$9(argument);
	  return len > 0 ? min$3(len, 0x1FFFFFFFFFFFFF) : 0; // 2 ** 53 - 1 == 9007199254740991
	};

	var toLength$5 = toLength$6;

	// `LengthOfArrayLike` abstract operation
	// https://tc39.es/ecma262/#sec-lengthofarraylike
	var lengthOfArrayLike$h = function (obj) {
	  return toLength$5(obj.length);
	};

	var toIndexedObject$b = toIndexedObject$d;
	var toAbsoluteIndex$3 = toAbsoluteIndex$4;
	var lengthOfArrayLike$g = lengthOfArrayLike$h;

	// `Array.prototype.{ indexOf, includes }` methods implementation
	var createMethod$6 = function (IS_INCLUDES) {
	  return function ($this, el, fromIndex) {
	    var O = toIndexedObject$b($this);
	    var length = lengthOfArrayLike$g(O);
	    if (length === 0) return !IS_INCLUDES && -1;
	    var index = toAbsoluteIndex$3(fromIndex, length);
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
	  includes: createMethod$6(true),
	  // `Array.prototype.indexOf` method
	  // https://tc39.es/ecma262/#sec-array.prototype.indexof
	  indexOf: createMethod$6(false)
	};

	var uncurryThis$Q = functionUncurryThis;
	var hasOwn$m = hasOwnProperty_1;
	var toIndexedObject$a = toIndexedObject$d;
	var indexOf$2 = arrayIncludes.indexOf;
	var hiddenKeys$4 = hiddenKeys$6;

	var push$d = uncurryThis$Q([].push);

	var objectKeysInternal = function (object, names) {
	  var O = toIndexedObject$a(object);
	  var i = 0;
	  var result = [];
	  var key;
	  for (key in O) !hasOwn$m(hiddenKeys$4, key) && hasOwn$m(O, key) && push$d(result, key);
	  // Don't enum bug & hidden keys
	  while (names.length > i) if (hasOwn$m(O, key = names[i++])) {
	    ~indexOf$2(result, key) || push$d(result, key);
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

	var hiddenKeys$3 = enumBugKeys$2.concat('length', 'prototype');

	// `Object.getOwnPropertyNames` method
	// https://tc39.es/ecma262/#sec-object.getownpropertynames
	// eslint-disable-next-line es/no-object-getownpropertynames -- safe
	objectGetOwnPropertyNames.f = Object.getOwnPropertyNames || function getOwnPropertyNames(O) {
	  return internalObjectKeys$1(O, hiddenKeys$3);
	};

	var objectGetOwnPropertySymbols = {};

	// eslint-disable-next-line es/no-object-getownpropertysymbols -- safe
	objectGetOwnPropertySymbols.f = Object.getOwnPropertySymbols;

	var getBuiltIn$i = getBuiltIn$k;
	var uncurryThis$P = functionUncurryThis;
	var getOwnPropertyNamesModule$2 = objectGetOwnPropertyNames;
	var getOwnPropertySymbolsModule$3 = objectGetOwnPropertySymbols;
	var anObject$o = anObject$q;

	var concat$2 = uncurryThis$P([].concat);

	// all object keys, includes non-enumerable and symbols
	var ownKeys$2 = getBuiltIn$i('Reflect', 'ownKeys') || function ownKeys(it) {
	  var keys = getOwnPropertyNamesModule$2.f(anObject$o(it));
	  var getOwnPropertySymbols = getOwnPropertySymbolsModule$3.f;
	  return getOwnPropertySymbols ? concat$2(keys, getOwnPropertySymbols(it)) : keys;
	};

	var hasOwn$l = hasOwnProperty_1;
	var ownKeys$1 = ownKeys$2;
	var getOwnPropertyDescriptorModule$1 = objectGetOwnPropertyDescriptor;
	var definePropertyModule$3 = objectDefineProperty;

	var copyConstructorProperties$5 = function (target, source, exceptions) {
	  var keys = ownKeys$1(source);
	  var defineProperty = definePropertyModule$3.f;
	  var getOwnPropertyDescriptor = getOwnPropertyDescriptorModule$1.f;
	  for (var i = 0; i < keys.length; i++) {
	    var key = keys[i];
	    if (!hasOwn$l(target, key) && !(exceptions && hasOwn$l(exceptions, key))) {
	      defineProperty(target, key, getOwnPropertyDescriptor(source, key));
	    }
	  }
	};

	var fails$K = fails$S;
	var isCallable$m = isCallable$w;

	var replacement = /#|\.prototype\./;

	var isForced$4 = function (feature, detection) {
	  var value = data[normalize(feature)];
	  return value === POLYFILL ? true
	    : value === NATIVE ? false
	    : isCallable$m(detection) ? fails$K(detection)
	    : !!detection;
	};

	var normalize = isForced$4.normalize = function (string) {
	  return String(string).replace(replacement, '.').toLowerCase();
	};

	var data = isForced$4.data = {};
	var NATIVE = isForced$4.NATIVE = 'N';
	var POLYFILL = isForced$4.POLYFILL = 'P';

	var isForced_1 = isForced$4;

	var globalThis$v = globalThis_1;
	var getOwnPropertyDescriptor$5 = objectGetOwnPropertyDescriptor.f;
	var createNonEnumerableProperty$9 = createNonEnumerableProperty$b;
	var defineBuiltIn$j = defineBuiltIn$k;
	var defineGlobalProperty = defineGlobalProperty$3;
	var copyConstructorProperties$4 = copyConstructorProperties$5;
	var isForced$3 = isForced_1;

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
	    target = globalThis$v;
	  } else if (STATIC) {
	    target = globalThis$v[TARGET] || defineGlobalProperty(TARGET, {});
	  } else {
	    target = globalThis$v[TARGET] && globalThis$v[TARGET].prototype;
	  }
	  if (target) for (key in source) {
	    sourceProperty = source[key];
	    if (options.dontCallGetSet) {
	      descriptor = getOwnPropertyDescriptor$5(target, key);
	      targetProperty = descriptor && descriptor.value;
	    } else targetProperty = target[key];
	    FORCED = isForced$3(GLOBAL ? key : TARGET + (STATIC ? '.' : '#') + key, options.forced);
	    // contained in target
	    if (!FORCED && targetProperty !== undefined) {
	      if (typeof sourceProperty == typeof targetProperty) continue;
	      copyConstructorProperties$4(sourceProperty, targetProperty);
	    }
	    // add a flag to not completely full polyfills
	    if (options.sham || (targetProperty && targetProperty.sham)) {
	      createNonEnumerableProperty$9(sourceProperty, 'sham', true);
	    }
	    defineBuiltIn$j(target, key, sourceProperty, options);
	  }
	};

	var wellKnownSymbol$u = wellKnownSymbol$w;

	var TO_STRING_TAG$5 = wellKnownSymbol$u('toStringTag');
	var test$1 = {};

	test$1[TO_STRING_TAG$5] = 'z';

	var toStringTagSupport = String(test$1) === '[object z]';

	var TO_STRING_TAG_SUPPORT$2 = toStringTagSupport;
	var isCallable$l = isCallable$w;
	var classofRaw$1 = classofRaw$2;
	var wellKnownSymbol$t = wellKnownSymbol$w;

	var TO_STRING_TAG$4 = wellKnownSymbol$t('toStringTag');
	var $Object$2 = Object;

	// ES3 wrong here
	var CORRECT_ARGUMENTS = classofRaw$1(function () { return arguments; }()) === 'Arguments';

	// fallback for IE11 Script Access Denied error
	var tryGet = function (it, key) {
	  try {
	    return it[key];
	  } catch (error) { /* empty */ }
	};

	// getting tag from ES6+ `Object.prototype.toString`
	var classof$d = TO_STRING_TAG_SUPPORT$2 ? classofRaw$1 : function (it) {
	  var O, tag, result;
	  return it === undefined ? 'Undefined' : it === null ? 'Null'
	    // @@toStringTag case
	    : typeof (tag = tryGet(O = $Object$2(it), TO_STRING_TAG$4)) == 'string' ? tag
	    // builtinTag case
	    : CORRECT_ARGUMENTS ? classofRaw$1(O)
	    // ES3 arguments fallback
	    : (result = classofRaw$1(O)) === 'Object' && isCallable$l(O.callee) ? 'Arguments' : result;
	};

	var classof$c = classof$d;

	var $String$3 = String;

	var toString$o = function (argument) {
	  if (classof$c(argument) === 'Symbol') throw new TypeError('Cannot convert a Symbol value to a string');
	  return $String$3(argument);
	};

	var objectDefineProperties = {};

	var internalObjectKeys = objectKeysInternal;
	var enumBugKeys$1 = enumBugKeys$3;

	// `Object.keys` method
	// https://tc39.es/ecma262/#sec-object.keys
	// eslint-disable-next-line es/no-object-keys -- safe
	var objectKeys$5 = Object.keys || function keys(O) {
	  return internalObjectKeys(O, enumBugKeys$1);
	};

	var DESCRIPTORS$m = descriptors;
	var V8_PROTOTYPE_DEFINE_BUG = v8PrototypeDefineBug;
	var definePropertyModule$2 = objectDefineProperty;
	var anObject$n = anObject$q;
	var toIndexedObject$9 = toIndexedObject$d;
	var objectKeys$4 = objectKeys$5;

	// `Object.defineProperties` method
	// https://tc39.es/ecma262/#sec-object.defineproperties
	// eslint-disable-next-line es/no-object-defineproperties -- safe
	objectDefineProperties.f = DESCRIPTORS$m && !V8_PROTOTYPE_DEFINE_BUG ? Object.defineProperties : function defineProperties(O, Properties) {
	  anObject$n(O);
	  var props = toIndexedObject$9(Properties);
	  var keys = objectKeys$4(Properties);
	  var length = keys.length;
	  var index = 0;
	  var key;
	  while (length > index) definePropertyModule$2.f(O, key = keys[index++], props[key]);
	  return O;
	};

	var getBuiltIn$h = getBuiltIn$k;

	var html$2 = getBuiltIn$h('document', 'documentElement');

	/* global ActiveXObject -- old IE, WSH */
	var anObject$m = anObject$q;
	var definePropertiesModule$1 = objectDefineProperties;
	var enumBugKeys = enumBugKeys$3;
	var hiddenKeys$2 = hiddenKeys$6;
	var html$1 = html$2;
	var documentCreateElement$1 = documentCreateElement$2;
	var sharedKey$2 = sharedKey$4;

	var GT = '>';
	var LT = '<';
	var PROTOTYPE$1 = 'prototype';
	var SCRIPT = 'script';
	var IE_PROTO$1 = sharedKey$2('IE_PROTO');

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
	  var iframe = documentCreateElement$1('iframe');
	  var JS = 'java' + SCRIPT + ':';
	  var iframeDocument;
	  iframe.style.display = 'none';
	  html$1.appendChild(iframe);
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
	  while (length--) delete NullProtoObject[PROTOTYPE$1][enumBugKeys[length]];
	  return NullProtoObject();
	};

	hiddenKeys$2[IE_PROTO$1] = true;

	// `Object.create` method
	// https://tc39.es/ecma262/#sec-object.create
	// eslint-disable-next-line es/no-object-create -- safe
	var objectCreate = Object.create || function create(O, Properties) {
	  var result;
	  if (O !== null) {
	    EmptyConstructor[PROTOTYPE$1] = anObject$m(O);
	    result = new EmptyConstructor();
	    EmptyConstructor[PROTOTYPE$1] = null;
	    // add "__proto__" for Object.getPrototypeOf polyfill
	    result[IE_PROTO$1] = O;
	  } else result = NullProtoObject();
	  return Properties === undefined ? result : definePropertiesModule$1.f(result, Properties);
	};

	var objectGetOwnPropertyNamesExternal = {};

	var uncurryThis$O = functionUncurryThis;

	var arraySlice$5 = uncurryThis$O([].slice);

	/* eslint-disable es/no-object-getownpropertynames -- safe */
	var classof$b = classofRaw$2;
	var toIndexedObject$8 = toIndexedObject$d;
	var $getOwnPropertyNames$1 = objectGetOwnPropertyNames.f;
	var arraySlice$4 = arraySlice$5;

	var windowNames = typeof window == 'object' && window && Object.getOwnPropertyNames
	  ? Object.getOwnPropertyNames(window) : [];

	var getWindowNames = function (it) {
	  try {
	    return $getOwnPropertyNames$1(it);
	  } catch (error) {
	    return arraySlice$4(windowNames);
	  }
	};

	// fallback for IE11 buggy Object.getOwnPropertyNames with iframe and window
	objectGetOwnPropertyNamesExternal.f = function getOwnPropertyNames(it) {
	  return windowNames && classof$b(it) === 'Window'
	    ? getWindowNames(it)
	    : $getOwnPropertyNames$1(toIndexedObject$8(it));
	};

	var makeBuiltIn = makeBuiltInExports;
	var defineProperty$a = objectDefineProperty;

	var defineBuiltInAccessor$b = function (target, name, descriptor) {
	  if (descriptor.get) makeBuiltIn(descriptor.get, name, { getter: true });
	  if (descriptor.set) makeBuiltIn(descriptor.set, name, { setter: true });
	  return defineProperty$a.f(target, name, descriptor);
	};

	var wellKnownSymbolWrapped = {};

	var wellKnownSymbol$s = wellKnownSymbol$w;

	wellKnownSymbolWrapped.f = wellKnownSymbol$s;

	var globalThis$u = globalThis_1;

	var path$2 = globalThis$u;

	var path$1 = path$2;
	var hasOwn$k = hasOwnProperty_1;
	var wrappedWellKnownSymbolModule$1 = wellKnownSymbolWrapped;
	var defineProperty$9 = objectDefineProperty.f;

	var wellKnownSymbolDefine = function (NAME) {
	  var Symbol = path$1.Symbol || (path$1.Symbol = {});
	  if (!hasOwn$k(Symbol, NAME)) defineProperty$9(Symbol, NAME, {
	    value: wrappedWellKnownSymbolModule$1.f(NAME)
	  });
	};

	var call$s = functionCall;
	var getBuiltIn$g = getBuiltIn$k;
	var wellKnownSymbol$r = wellKnownSymbol$w;
	var defineBuiltIn$i = defineBuiltIn$k;

	var symbolDefineToPrimitive = function () {
	  var Symbol = getBuiltIn$g('Symbol');
	  var SymbolPrototype = Symbol && Symbol.prototype;
	  var valueOf = SymbolPrototype && SymbolPrototype.valueOf;
	  var TO_PRIMITIVE = wellKnownSymbol$r('toPrimitive');

	  if (SymbolPrototype && !SymbolPrototype[TO_PRIMITIVE]) {
	    // `Symbol.prototype[@@toPrimitive]` method
	    // https://tc39.es/ecma262/#sec-symbol.prototype-@@toprimitive
	    // eslint-disable-next-line no-unused-vars -- required for .length
	    defineBuiltIn$i(SymbolPrototype, TO_PRIMITIVE, function (hint) {
	      return call$s(valueOf, this);
	    }, { arity: 1 });
	  }
	};

	var defineProperty$8 = objectDefineProperty.f;
	var hasOwn$j = hasOwnProperty_1;
	var wellKnownSymbol$q = wellKnownSymbol$w;

	var TO_STRING_TAG$3 = wellKnownSymbol$q('toStringTag');

	var setToStringTag$9 = function (target, TAG, STATIC) {
	  if (target && !STATIC) target = target.prototype;
	  if (target && !hasOwn$j(target, TO_STRING_TAG$3)) {
	    defineProperty$8(target, TO_STRING_TAG$3, { configurable: true, value: TAG });
	  }
	};

	var classofRaw = classofRaw$2;
	var uncurryThis$N = functionUncurryThis;

	var functionUncurryThisClause = function (fn) {
	  // Nashorn bug:
	  //   https://github.com/zloirock/core-js/issues/1128
	  //   https://github.com/zloirock/core-js/issues/1130
	  if (classofRaw(fn) === 'Function') return uncurryThis$N(fn);
	};

	var uncurryThis$M = functionUncurryThisClause;
	var aCallable$g = aCallable$i;
	var NATIVE_BIND$1 = functionBindNative;

	var bind$b = uncurryThis$M(uncurryThis$M.bind);

	// optional / simple context binding
	var functionBindContext = function (fn, that) {
	  aCallable$g(fn);
	  return that === undefined ? fn : NATIVE_BIND$1 ? bind$b(fn, that) : function (/* ...args */) {
	    return fn.apply(that, arguments);
	  };
	};

	var classof$a = classofRaw$2;

	// `IsArray` abstract operation
	// https://tc39.es/ecma262/#sec-isarray
	// eslint-disable-next-line es/no-array-isarray -- safe
	var isArray$6 = Array.isArray || function isArray(argument) {
	  return classof$a(argument) === 'Array';
	};

	var uncurryThis$L = functionUncurryThis;
	var fails$J = fails$S;
	var isCallable$k = isCallable$w;
	var classof$9 = classof$d;
	var getBuiltIn$f = getBuiltIn$k;
	var inspectSource$1 = inspectSource$3;

	var noop = function () { /* empty */ };
	var construct = getBuiltIn$f('Reflect', 'construct');
	var constructorRegExp = /^\s*(?:class|function)\b/;
	var exec$5 = uncurryThis$L(constructorRegExp.exec);
	var INCORRECT_TO_STRING$2 = !constructorRegExp.test(noop);

	var isConstructorModern = function isConstructor(argument) {
	  if (!isCallable$k(argument)) return false;
	  try {
	    construct(noop, [], argument);
	    return true;
	  } catch (error) {
	    return false;
	  }
	};

	var isConstructorLegacy = function isConstructor(argument) {
	  if (!isCallable$k(argument)) return false;
	  switch (classof$9(argument)) {
	    case 'AsyncFunction':
	    case 'GeneratorFunction':
	    case 'AsyncGeneratorFunction': return false;
	  }
	  try {
	    // we can't check .prototype since constructors produced by .bind haven't it
	    // `Function#toString` throws on some built-it function in some legacy engines
	    // (for example, `DOMQuad` and similar in FF41-)
	    return INCORRECT_TO_STRING$2 || !!exec$5(constructorRegExp, inspectSource$1(argument));
	  } catch (error) {
	    return true;
	  }
	};

	isConstructorLegacy.sham = true;

	// `IsConstructor` abstract operation
	// https://tc39.es/ecma262/#sec-isconstructor
	var isConstructor$6 = !construct || fails$J(function () {
	  var called;
	  return isConstructorModern(isConstructorModern.call)
	    || !isConstructorModern(Object)
	    || !isConstructorModern(function () { called = true; })
	    || called;
	}) ? isConstructorLegacy : isConstructorModern;

	var isArray$5 = isArray$6;
	var isConstructor$5 = isConstructor$6;
	var isObject$h = isObject$n;
	var wellKnownSymbol$p = wellKnownSymbol$w;

	var SPECIES$6 = wellKnownSymbol$p('species');
	var $Array$6 = Array;

	// a part of `ArraySpeciesCreate` abstract operation
	// https://tc39.es/ecma262/#sec-arrayspeciescreate
	var arraySpeciesConstructor$1 = function (originalArray) {
	  var C;
	  if (isArray$5(originalArray)) {
	    C = originalArray.constructor;
	    // cross-realm fallback
	    if (isConstructor$5(C) && (C === $Array$6 || isArray$5(C.prototype))) C = undefined;
	    else if (isObject$h(C)) {
	      C = C[SPECIES$6];
	      if (C === null) C = undefined;
	    }
	  } return C === undefined ? $Array$6 : C;
	};

	var arraySpeciesConstructor = arraySpeciesConstructor$1;

	// `ArraySpeciesCreate` abstract operation
	// https://tc39.es/ecma262/#sec-arrayspeciescreate
	var arraySpeciesCreate$3 = function (originalArray, length) {
	  return new (arraySpeciesConstructor(originalArray))(length === 0 ? 0 : length);
	};

	var bind$a = functionBindContext;
	var uncurryThis$K = functionUncurryThis;
	var IndexedObject$3 = indexedObject;
	var toObject$e = toObject$g;
	var lengthOfArrayLike$f = lengthOfArrayLike$h;
	var arraySpeciesCreate$2 = arraySpeciesCreate$3;

	var push$c = uncurryThis$K([].push);

	// `Array.prototype.{ forEach, map, filter, some, every, find, findIndex, filterReject }` methods implementation
	var createMethod$5 = function (TYPE) {
	  var IS_MAP = TYPE === 1;
	  var IS_FILTER = TYPE === 2;
	  var IS_SOME = TYPE === 3;
	  var IS_EVERY = TYPE === 4;
	  var IS_FIND_INDEX = TYPE === 6;
	  var IS_FILTER_REJECT = TYPE === 7;
	  var NO_HOLES = TYPE === 5 || IS_FIND_INDEX;
	  return function ($this, callbackfn, that, specificCreate) {
	    var O = toObject$e($this);
	    var self = IndexedObject$3(O);
	    var length = lengthOfArrayLike$f(self);
	    var boundFunction = bind$a(callbackfn, that);
	    var index = 0;
	    var create = specificCreate || arraySpeciesCreate$2;
	    var target = IS_MAP ? create($this, length) : IS_FILTER || IS_FILTER_REJECT ? create($this, 0) : undefined;
	    var value, result;
	    for (;length > index; index++) if (NO_HOLES || index in self) {
	      value = self[index];
	      result = boundFunction(value, index, O);
	      if (TYPE) {
	        if (IS_MAP) target[index] = result; // map
	        else if (result) switch (TYPE) {
	          case 3: return true;              // some
	          case 5: return value;             // find
	          case 6: return index;             // findIndex
	          case 2: push$c(target, value);      // filter
	        } else switch (TYPE) {
	          case 4: return false;             // every
	          case 7: push$c(target, value);      // filterReject
	        }
	      }
	    }
	    return IS_FIND_INDEX ? -1 : IS_SOME || IS_EVERY ? IS_EVERY : target;
	  };
	};

	var arrayIteration = {
	  // `Array.prototype.forEach` method
	  // https://tc39.es/ecma262/#sec-array.prototype.foreach
	  forEach: createMethod$5(0),
	  // `Array.prototype.map` method
	  // https://tc39.es/ecma262/#sec-array.prototype.map
	  map: createMethod$5(1),
	  // `Array.prototype.find` method
	  // https://tc39.es/ecma262/#sec-array.prototype.find
	  find: createMethod$5(5),
	  // `Array.prototype.findIndex` method
	  // https://tc39.es/ecma262/#sec-array.prototype.findIndex
	  findIndex: createMethod$5(6)};

	var $$1v = _export;
	var globalThis$t = globalThis_1;
	var call$r = functionCall;
	var uncurryThis$J = functionUncurryThis;
	var DESCRIPTORS$l = descriptors;
	var NATIVE_SYMBOL$4 = symbolConstructorDetection;
	var fails$I = fails$S;
	var hasOwn$i = hasOwnProperty_1;
	var isPrototypeOf$7 = objectIsPrototypeOf;
	var anObject$l = anObject$q;
	var toIndexedObject$7 = toIndexedObject$d;
	var toPropertyKey$1 = toPropertyKey$4;
	var $toString$3 = toString$o;
	var createPropertyDescriptor$7 = createPropertyDescriptor$a;
	var nativeObjectCreate = objectCreate;
	var objectKeys$3 = objectKeys$5;
	var getOwnPropertyNamesModule$1 = objectGetOwnPropertyNames;
	var getOwnPropertyNamesExternal = objectGetOwnPropertyNamesExternal;
	var getOwnPropertySymbolsModule$2 = objectGetOwnPropertySymbols;
	var getOwnPropertyDescriptorModule = objectGetOwnPropertyDescriptor;
	var definePropertyModule$1 = objectDefineProperty;
	var definePropertiesModule = objectDefineProperties;
	var propertyIsEnumerableModule$1 = objectPropertyIsEnumerable;
	var defineBuiltIn$h = defineBuiltIn$k;
	var defineBuiltInAccessor$a = defineBuiltInAccessor$b;
	var shared$3 = shared$7;
	var sharedKey$1 = sharedKey$4;
	var hiddenKeys$1 = hiddenKeys$6;
	var uid$2 = uid$5;
	var wellKnownSymbol$o = wellKnownSymbol$w;
	var wrappedWellKnownSymbolModule = wellKnownSymbolWrapped;
	var defineWellKnownSymbol$3 = wellKnownSymbolDefine;
	var defineSymbolToPrimitive$1 = symbolDefineToPrimitive;
	var setToStringTag$8 = setToStringTag$9;
	var InternalStateModule$a = internalState;
	var $forEach = arrayIteration.forEach;

	var HIDDEN = sharedKey$1('hidden');
	var SYMBOL = 'Symbol';
	var PROTOTYPE = 'prototype';

	var setInternalState$a = InternalStateModule$a.set;
	var getInternalState$5 = InternalStateModule$a.getterFor(SYMBOL);

	var ObjectPrototype$1 = Object[PROTOTYPE];
	var $Symbol = globalThis$t.Symbol;
	var SymbolPrototype$1 = $Symbol && $Symbol[PROTOTYPE];
	var RangeError$1 = globalThis$t.RangeError;
	var TypeError$6 = globalThis$t.TypeError;
	var QObject = globalThis$t.QObject;
	var nativeGetOwnPropertyDescriptor = getOwnPropertyDescriptorModule.f;
	var nativeDefineProperty = definePropertyModule$1.f;
	var nativeGetOwnPropertyNames = getOwnPropertyNamesExternal.f;
	var nativePropertyIsEnumerable = propertyIsEnumerableModule$1.f;
	var push$b = uncurryThis$J([].push);

	var AllSymbols = shared$3('symbols');
	var ObjectPrototypeSymbols = shared$3('op-symbols');
	var WellKnownSymbolsStore = shared$3('wks');

	// Don't use setters in Qt Script, https://github.com/zloirock/core-js/issues/173
	var USE_SETTER = !QObject || !QObject[PROTOTYPE] || !QObject[PROTOTYPE].findChild;

	// fallback for old Android, https://code.google.com/p/v8/issues/detail?id=687
	var fallbackDefineProperty = function (O, P, Attributes) {
	  var ObjectPrototypeDescriptor = nativeGetOwnPropertyDescriptor(ObjectPrototype$1, P);
	  if (ObjectPrototypeDescriptor) delete ObjectPrototype$1[P];
	  nativeDefineProperty(O, P, Attributes);
	  if (ObjectPrototypeDescriptor && O !== ObjectPrototype$1) {
	    nativeDefineProperty(ObjectPrototype$1, P, ObjectPrototypeDescriptor);
	  }
	};

	var setSymbolDescriptor = DESCRIPTORS$l && fails$I(function () {
	  return nativeObjectCreate(nativeDefineProperty({}, 'a', {
	    get: function () { return nativeDefineProperty(this, 'a', { value: 7 }).a; }
	  })).a !== 7;
	}) ? fallbackDefineProperty : nativeDefineProperty;

	var wrap = function (tag, description) {
	  var symbol = AllSymbols[tag] = nativeObjectCreate(SymbolPrototype$1);
	  setInternalState$a(symbol, {
	    type: SYMBOL,
	    tag: tag,
	    description: description
	  });
	  if (!DESCRIPTORS$l) symbol.description = description;
	  return symbol;
	};

	var $defineProperty = function defineProperty(O, P, Attributes) {
	  if (O === ObjectPrototype$1) $defineProperty(ObjectPrototypeSymbols, P, Attributes);
	  anObject$l(O);
	  var key = toPropertyKey$1(P);
	  anObject$l(Attributes);
	  if (hasOwn$i(AllSymbols, key)) {
	    if (!Attributes.enumerable) {
	      if (!hasOwn$i(O, HIDDEN)) nativeDefineProperty(O, HIDDEN, createPropertyDescriptor$7(1, nativeObjectCreate(null)));
	      O[HIDDEN][key] = true;
	    } else {
	      if (hasOwn$i(O, HIDDEN) && O[HIDDEN][key]) O[HIDDEN][key] = false;
	      Attributes = nativeObjectCreate(Attributes, { enumerable: createPropertyDescriptor$7(0, false) });
	    } return setSymbolDescriptor(O, key, Attributes);
	  } return nativeDefineProperty(O, key, Attributes);
	};

	var $defineProperties = function defineProperties(O, Properties) {
	  anObject$l(O);
	  var properties = toIndexedObject$7(Properties);
	  var keys = objectKeys$3(properties).concat($getOwnPropertySymbols(properties));
	  $forEach(keys, function (key) {
	    if (!DESCRIPTORS$l || call$r($propertyIsEnumerable$1, properties, key)) $defineProperty(O, key, properties[key]);
	  });
	  return O;
	};

	var $create = function create(O, Properties) {
	  return Properties === undefined ? nativeObjectCreate(O) : $defineProperties(nativeObjectCreate(O), Properties);
	};

	var $propertyIsEnumerable$1 = function propertyIsEnumerable(V) {
	  var P = toPropertyKey$1(V);
	  var enumerable = call$r(nativePropertyIsEnumerable, this, P);
	  if (this === ObjectPrototype$1 && hasOwn$i(AllSymbols, P) && !hasOwn$i(ObjectPrototypeSymbols, P)) return false;
	  return enumerable || !hasOwn$i(this, P) || !hasOwn$i(AllSymbols, P) || hasOwn$i(this, HIDDEN) && this[HIDDEN][P]
	    ? enumerable : true;
	};

	var $getOwnPropertyDescriptor = function getOwnPropertyDescriptor(O, P) {
	  var it = toIndexedObject$7(O);
	  var key = toPropertyKey$1(P);
	  if (it === ObjectPrototype$1 && hasOwn$i(AllSymbols, key) && !hasOwn$i(ObjectPrototypeSymbols, key)) return;
	  var descriptor = nativeGetOwnPropertyDescriptor(it, key);
	  if (descriptor && hasOwn$i(AllSymbols, key) && !(hasOwn$i(it, HIDDEN) && it[HIDDEN][key])) {
	    descriptor.enumerable = true;
	  }
	  return descriptor;
	};

	var $getOwnPropertyNames = function getOwnPropertyNames(O) {
	  var names = nativeGetOwnPropertyNames(toIndexedObject$7(O));
	  var result = [];
	  $forEach(names, function (key) {
	    if (!hasOwn$i(AllSymbols, key) && !hasOwn$i(hiddenKeys$1, key)) push$b(result, key);
	  });
	  return result;
	};

	var $getOwnPropertySymbols = function (O) {
	  var IS_OBJECT_PROTOTYPE = O === ObjectPrototype$1;
	  var names = nativeGetOwnPropertyNames(IS_OBJECT_PROTOTYPE ? ObjectPrototypeSymbols : toIndexedObject$7(O));
	  var result = [];
	  $forEach(names, function (key) {
	    if (hasOwn$i(AllSymbols, key) && (!IS_OBJECT_PROTOTYPE || hasOwn$i(ObjectPrototype$1, key))) {
	      push$b(result, AllSymbols[key]);
	    }
	  });
	  return result;
	};

	// `Symbol` constructor
	// https://tc39.es/ecma262/#sec-symbol-constructor
	if (!NATIVE_SYMBOL$4) {
	  $Symbol = function Symbol() {
	    if (isPrototypeOf$7(SymbolPrototype$1, this)) throw new TypeError$6('Symbol is not a constructor');
	    var description = !arguments.length || arguments[0] === undefined ? undefined : $toString$3(arguments[0]);
	    var tag = uid$2(description);
	    var setter = function (value) {
	      var $this = this === undefined ? globalThis$t : this;
	      if ($this === ObjectPrototype$1) call$r(setter, ObjectPrototypeSymbols, value);
	      if (hasOwn$i($this, HIDDEN) && hasOwn$i($this[HIDDEN], tag)) $this[HIDDEN][tag] = false;
	      var descriptor = createPropertyDescriptor$7(1, value);
	      try {
	        setSymbolDescriptor($this, tag, descriptor);
	      } catch (error) {
	        if (!(error instanceof RangeError$1)) throw error;
	        fallbackDefineProperty($this, tag, descriptor);
	      }
	    };
	    if (DESCRIPTORS$l && USE_SETTER) setSymbolDescriptor(ObjectPrototype$1, tag, { configurable: true, set: setter });
	    return wrap(tag, description);
	  };

	  SymbolPrototype$1 = $Symbol[PROTOTYPE];

	  defineBuiltIn$h(SymbolPrototype$1, 'toString', function toString() {
	    return getInternalState$5(this).tag;
	  });

	  defineBuiltIn$h($Symbol, 'withoutSetter', function (description) {
	    return wrap(uid$2(description), description);
	  });

	  propertyIsEnumerableModule$1.f = $propertyIsEnumerable$1;
	  definePropertyModule$1.f = $defineProperty;
	  definePropertiesModule.f = $defineProperties;
	  getOwnPropertyDescriptorModule.f = $getOwnPropertyDescriptor;
	  getOwnPropertyNamesModule$1.f = getOwnPropertyNamesExternal.f = $getOwnPropertyNames;
	  getOwnPropertySymbolsModule$2.f = $getOwnPropertySymbols;

	  wrappedWellKnownSymbolModule.f = function (name) {
	    return wrap(wellKnownSymbol$o(name), name);
	  };

	  if (DESCRIPTORS$l) {
	    // https://github.com/tc39/proposal-Symbol-description
	    defineBuiltInAccessor$a(SymbolPrototype$1, 'description', {
	      configurable: true,
	      get: function description() {
	        return getInternalState$5(this).description;
	      }
	    });
	    {
	      defineBuiltIn$h(ObjectPrototype$1, 'propertyIsEnumerable', $propertyIsEnumerable$1, { unsafe: true });
	    }
	  }
	}

	$$1v({ global: true, constructor: true, wrap: true, forced: !NATIVE_SYMBOL$4, sham: !NATIVE_SYMBOL$4 }, {
	  Symbol: $Symbol
	});

	$forEach(objectKeys$3(WellKnownSymbolsStore), function (name) {
	  defineWellKnownSymbol$3(name);
	});

	$$1v({ target: SYMBOL, stat: true, forced: !NATIVE_SYMBOL$4 }, {
	  useSetter: function () { USE_SETTER = true; },
	  useSimple: function () { USE_SETTER = false; }
	});

	$$1v({ target: 'Object', stat: true, forced: !NATIVE_SYMBOL$4, sham: !DESCRIPTORS$l }, {
	  // `Object.create` method
	  // https://tc39.es/ecma262/#sec-object.create
	  create: $create,
	  // `Object.defineProperty` method
	  // https://tc39.es/ecma262/#sec-object.defineproperty
	  defineProperty: $defineProperty,
	  // `Object.defineProperties` method
	  // https://tc39.es/ecma262/#sec-object.defineproperties
	  defineProperties: $defineProperties,
	  // `Object.getOwnPropertyDescriptor` method
	  // https://tc39.es/ecma262/#sec-object.getownpropertydescriptors
	  getOwnPropertyDescriptor: $getOwnPropertyDescriptor
	});

	$$1v({ target: 'Object', stat: true, forced: !NATIVE_SYMBOL$4 }, {
	  // `Object.getOwnPropertyNames` method
	  // https://tc39.es/ecma262/#sec-object.getownpropertynames
	  getOwnPropertyNames: $getOwnPropertyNames
	});

	// `Symbol.prototype[@@toPrimitive]` method
	// https://tc39.es/ecma262/#sec-symbol.prototype-@@toprimitive
	defineSymbolToPrimitive$1();

	// `Symbol.prototype[@@toStringTag]` property
	// https://tc39.es/ecma262/#sec-symbol.prototype-@@tostringtag
	setToStringTag$8($Symbol, SYMBOL);

	hiddenKeys$1[HIDDEN] = true;

	var NATIVE_SYMBOL$3 = symbolConstructorDetection;

	/* eslint-disable es/no-symbol -- safe */
	var symbolRegistryDetection = NATIVE_SYMBOL$3 && !!Symbol['for'] && !!Symbol.keyFor;

	var $$1u = _export;
	var getBuiltIn$e = getBuiltIn$k;
	var hasOwn$h = hasOwnProperty_1;
	var toString$n = toString$o;
	var shared$2 = shared$7;
	var NATIVE_SYMBOL_REGISTRY$1 = symbolRegistryDetection;

	var StringToSymbolRegistry = shared$2('string-to-symbol-registry');
	var SymbolToStringRegistry$1 = shared$2('symbol-to-string-registry');

	// `Symbol.for` method
	// https://tc39.es/ecma262/#sec-symbol.for
	$$1u({ target: 'Symbol', stat: true, forced: !NATIVE_SYMBOL_REGISTRY$1 }, {
	  'for': function (key) {
	    var string = toString$n(key);
	    if (hasOwn$h(StringToSymbolRegistry, string)) return StringToSymbolRegistry[string];
	    var symbol = getBuiltIn$e('Symbol')(string);
	    StringToSymbolRegistry[string] = symbol;
	    SymbolToStringRegistry$1[symbol] = string;
	    return symbol;
	  }
	});

	var $$1t = _export;
	var hasOwn$g = hasOwnProperty_1;
	var isSymbol$3 = isSymbol$6;
	var tryToString$4 = tryToString$6;
	var shared$1 = shared$7;
	var NATIVE_SYMBOL_REGISTRY = symbolRegistryDetection;

	var SymbolToStringRegistry = shared$1('symbol-to-string-registry');

	// `Symbol.keyFor` method
	// https://tc39.es/ecma262/#sec-symbol.keyfor
	$$1t({ target: 'Symbol', stat: true, forced: !NATIVE_SYMBOL_REGISTRY }, {
	  keyFor: function keyFor(sym) {
	    if (!isSymbol$3(sym)) throw new TypeError(tryToString$4(sym) + ' is not a symbol');
	    if (hasOwn$g(SymbolToStringRegistry, sym)) return SymbolToStringRegistry[sym];
	  }
	});

	var NATIVE_BIND = functionBindNative;

	var FunctionPrototype$1 = Function.prototype;
	var apply$4 = FunctionPrototype$1.apply;
	var call$q = FunctionPrototype$1.call;

	// eslint-disable-next-line es/no-reflect -- safe
	var functionApply = typeof Reflect == 'object' && Reflect.apply || (NATIVE_BIND ? call$q.bind(apply$4) : function () {
	  return call$q.apply(apply$4, arguments);
	});

	var uncurryThis$I = functionUncurryThis;
	var isArray$4 = isArray$6;
	var isCallable$j = isCallable$w;
	var classof$8 = classofRaw$2;
	var toString$m = toString$o;

	var push$a = uncurryThis$I([].push);

	var getJsonReplacerFunction = function (replacer) {
	  if (isCallable$j(replacer)) return replacer;
	  if (!isArray$4(replacer)) return;
	  var rawLength = replacer.length;
	  var keys = [];
	  for (var i = 0; i < rawLength; i++) {
	    var element = replacer[i];
	    if (typeof element == 'string') push$a(keys, element);
	    else if (typeof element == 'number' || classof$8(element) === 'Number' || classof$8(element) === 'String') push$a(keys, toString$m(element));
	  }
	  var keysLength = keys.length;
	  var root = true;
	  return function (key, value) {
	    if (root) {
	      root = false;
	      return value;
	    }
	    if (isArray$4(this)) return value;
	    for (var j = 0; j < keysLength; j++) if (keys[j] === key) return value;
	  };
	};

	var $$1s = _export;
	var getBuiltIn$d = getBuiltIn$k;
	var apply$3 = functionApply;
	var call$p = functionCall;
	var uncurryThis$H = functionUncurryThis;
	var fails$H = fails$S;
	var isCallable$i = isCallable$w;
	var isSymbol$2 = isSymbol$6;
	var arraySlice$3 = arraySlice$5;
	var getReplacerFunction = getJsonReplacerFunction;
	var NATIVE_SYMBOL$2 = symbolConstructorDetection;

	var $String$2 = String;
	var $stringify = getBuiltIn$d('JSON', 'stringify');
	var exec$4 = uncurryThis$H(/./.exec);
	var charAt$9 = uncurryThis$H(''.charAt);
	var charCodeAt$4 = uncurryThis$H(''.charCodeAt);
	var replace$8 = uncurryThis$H(''.replace);
	var numberToString$1 = uncurryThis$H(1.0.toString);

	var tester = /[\uD800-\uDFFF]/g;
	var low = /^[\uD800-\uDBFF]$/;
	var hi = /^[\uDC00-\uDFFF]$/;

	var WRONG_SYMBOLS_CONVERSION = !NATIVE_SYMBOL$2 || fails$H(function () {
	  var symbol = getBuiltIn$d('Symbol')('stringify detection');
	  // MS Edge converts symbol values to JSON as {}
	  return $stringify([symbol]) !== '[null]'
	    // WebKit converts symbol values to JSON as null
	    || $stringify({ a: symbol }) !== '{}'
	    // V8 throws on boxed symbols
	    || $stringify(Object(symbol)) !== '{}';
	});

	// https://github.com/tc39/proposal-well-formed-stringify
	var ILL_FORMED_UNICODE = fails$H(function () {
	  return $stringify('\uDF06\uD834') !== '"\\udf06\\ud834"'
	    || $stringify('\uDEAD') !== '"\\udead"';
	});

	var stringifyWithSymbolsFix = function (it, replacer) {
	  var args = arraySlice$3(arguments);
	  var $replacer = getReplacerFunction(replacer);
	  if (!isCallable$i($replacer) && (it === undefined || isSymbol$2(it))) return; // IE8 returns string on undefined
	  args[1] = function (key, value) {
	    // some old implementations (like WebKit) could pass numbers as keys
	    if (isCallable$i($replacer)) value = call$p($replacer, this, $String$2(key), value);
	    if (!isSymbol$2(value)) return value;
	  };
	  return apply$3($stringify, null, args);
	};

	var fixIllFormed = function (match, offset, string) {
	  var prev = charAt$9(string, offset - 1);
	  var next = charAt$9(string, offset + 1);
	  if ((exec$4(low, match) && !exec$4(hi, next)) || (exec$4(hi, match) && !exec$4(low, prev))) {
	    return '\\u' + numberToString$1(charCodeAt$4(match, 0), 16);
	  } return match;
	};

	if ($stringify) {
	  // `JSON.stringify` method
	  // https://tc39.es/ecma262/#sec-json.stringify
	  $$1s({ target: 'JSON', stat: true, arity: 3, forced: WRONG_SYMBOLS_CONVERSION || ILL_FORMED_UNICODE }, {
	    // eslint-disable-next-line no-unused-vars -- required for `.length`
	    stringify: function stringify(it, replacer, space) {
	      var args = arraySlice$3(arguments);
	      var result = apply$3(WRONG_SYMBOLS_CONVERSION ? stringifyWithSymbolsFix : $stringify, null, args);
	      return ILL_FORMED_UNICODE && typeof result == 'string' ? replace$8(result, tester, fixIllFormed) : result;
	    }
	  });
	}

	var $$1r = _export;
	var NATIVE_SYMBOL$1 = symbolConstructorDetection;
	var fails$G = fails$S;
	var getOwnPropertySymbolsModule$1 = objectGetOwnPropertySymbols;
	var toObject$d = toObject$g;

	// V8 ~ Chrome 38 and 39 `Object.getOwnPropertySymbols` fails on primitives
	// https://bugs.chromium.org/p/v8/issues/detail?id=3443
	var FORCED$b = !NATIVE_SYMBOL$1 || fails$G(function () { getOwnPropertySymbolsModule$1.f(1); });

	// `Object.getOwnPropertySymbols` method
	// https://tc39.es/ecma262/#sec-object.getownpropertysymbols
	$$1r({ target: 'Object', stat: true, forced: FORCED$b }, {
	  getOwnPropertySymbols: function getOwnPropertySymbols(it) {
	    var $getOwnPropertySymbols = getOwnPropertySymbolsModule$1.f;
	    return $getOwnPropertySymbols ? $getOwnPropertySymbols(toObject$d(it)) : [];
	  }
	});

	var $$1q = _export;
	var DESCRIPTORS$k = descriptors;
	var globalThis$s = globalThis_1;
	var uncurryThis$G = functionUncurryThis;
	var hasOwn$f = hasOwnProperty_1;
	var isCallable$h = isCallable$w;
	var isPrototypeOf$6 = objectIsPrototypeOf;
	var toString$l = toString$o;
	var defineBuiltInAccessor$9 = defineBuiltInAccessor$b;
	var copyConstructorProperties$3 = copyConstructorProperties$5;

	var NativeSymbol = globalThis$s.Symbol;
	var SymbolPrototype = NativeSymbol && NativeSymbol.prototype;

	if (DESCRIPTORS$k && isCallable$h(NativeSymbol) && (!('description' in SymbolPrototype) ||
	  // Safari 12 bug
	  NativeSymbol().description !== undefined
	)) {
	  var EmptyStringDescriptionStore = {};
	  // wrap Symbol constructor for correct work with undefined description
	  var SymbolWrapper = function Symbol() {
	    var description = arguments.length < 1 || arguments[0] === undefined ? undefined : toString$l(arguments[0]);
	    var result = isPrototypeOf$6(SymbolPrototype, this)
	      // eslint-disable-next-line sonarjs/inconsistent-function-call -- ok
	      ? new NativeSymbol(description)
	      // in Edge 13, String(Symbol(undefined)) === 'Symbol(undefined)'
	      : description === undefined ? NativeSymbol() : NativeSymbol(description);
	    if (description === '') EmptyStringDescriptionStore[result] = true;
	    return result;
	  };

	  copyConstructorProperties$3(SymbolWrapper, NativeSymbol);
	  SymbolWrapper.prototype = SymbolPrototype;
	  SymbolPrototype.constructor = SymbolWrapper;

	  var NATIVE_SYMBOL = String(NativeSymbol('description detection')) === 'Symbol(description detection)';
	  var thisSymbolValue = uncurryThis$G(SymbolPrototype.valueOf);
	  var symbolDescriptiveString = uncurryThis$G(SymbolPrototype.toString);
	  var regexp = /^Symbol\((.*)\)[^)]+$/;
	  var replace$7 = uncurryThis$G(''.replace);
	  var stringSlice$b = uncurryThis$G(''.slice);

	  defineBuiltInAccessor$9(SymbolPrototype, 'description', {
	    configurable: true,
	    get: function description() {
	      var symbol = thisSymbolValue(this);
	      if (hasOwn$f(EmptyStringDescriptionStore, symbol)) return '';
	      var string = symbolDescriptiveString(symbol);
	      var desc = NATIVE_SYMBOL ? stringSlice$b(string, 7, -1) : replace$7(string, regexp, '$1');
	      return desc === '' ? undefined : desc;
	    }
	  });

	  $$1q({ global: true, constructor: true, forced: true }, {
	    Symbol: SymbolWrapper
	  });
	}

	var defineWellKnownSymbol$2 = wellKnownSymbolDefine;

	// `Symbol.asyncIterator` well-known symbol
	// https://tc39.es/ecma262/#sec-symbol.asynciterator
	defineWellKnownSymbol$2('asyncIterator');

	var defineWellKnownSymbol$1 = wellKnownSymbolDefine;

	// `Symbol.iterator` well-known symbol
	// https://tc39.es/ecma262/#sec-symbol.iterator
	defineWellKnownSymbol$1('iterator');

	var defineWellKnownSymbol = wellKnownSymbolDefine;
	var defineSymbolToPrimitive = symbolDefineToPrimitive;

	// `Symbol.toPrimitive` well-known symbol
	// https://tc39.es/ecma262/#sec-symbol.toprimitive
	defineWellKnownSymbol('toPrimitive');

	// `Symbol.prototype[@@toPrimitive]` method
	// https://tc39.es/ecma262/#sec-symbol.prototype-@@toprimitive
	defineSymbolToPrimitive();

	var uncurryThis$F = functionUncurryThis;
	var aCallable$f = aCallable$i;

	var functionUncurryThisAccessor = function (object, key, method) {
	  try {
	    // eslint-disable-next-line es/no-object-getownpropertydescriptor -- safe
	    return uncurryThis$F(aCallable$f(Object.getOwnPropertyDescriptor(object, key)[method]));
	  } catch (error) { /* empty */ }
	};

	var isObject$g = isObject$n;

	var isPossiblePrototype$1 = function (argument) {
	  return isObject$g(argument) || argument === null;
	};

	var isPossiblePrototype = isPossiblePrototype$1;

	var $String$1 = String;
	var $TypeError$g = TypeError;

	var aPossiblePrototype$1 = function (argument) {
	  if (isPossiblePrototype(argument)) return argument;
	  throw new $TypeError$g("Can't set " + $String$1(argument) + ' as a prototype');
	};

	/* eslint-disable no-proto -- safe */
	var uncurryThisAccessor$1 = functionUncurryThisAccessor;
	var isObject$f = isObject$n;
	var requireObjectCoercible$d = requireObjectCoercible$g;
	var aPossiblePrototype = aPossiblePrototype$1;

	// `Object.setPrototypeOf` method
	// https://tc39.es/ecma262/#sec-object.setprototypeof
	// Works with __proto__ only. Old v8 can't work with null proto objects.
	// eslint-disable-next-line es/no-object-setprototypeof -- safe
	var objectSetPrototypeOf = Object.setPrototypeOf || ('__proto__' in {} ? function () {
	  var CORRECT_SETTER = false;
	  var test = {};
	  var setter;
	  try {
	    setter = uncurryThisAccessor$1(Object.prototype, '__proto__', 'set');
	    setter(test, []);
	    CORRECT_SETTER = test instanceof Array;
	  } catch (error) { /* empty */ }
	  return function setPrototypeOf(O, proto) {
	    requireObjectCoercible$d(O);
	    aPossiblePrototype(proto);
	    if (!isObject$f(O)) return O;
	    if (CORRECT_SETTER) setter(O, proto);
	    else O.__proto__ = proto;
	    return O;
	  };
	}() : undefined);

	var defineProperty$7 = objectDefineProperty.f;

	var proxyAccessor$1 = function (Target, Source, key) {
	  key in Target || defineProperty$7(Target, key, {
	    configurable: true,
	    get: function () { return Source[key]; },
	    set: function (it) { Source[key] = it; }
	  });
	};

	var isCallable$g = isCallable$w;
	var isObject$e = isObject$n;
	var setPrototypeOf$4 = objectSetPrototypeOf;

	// makes subclassing work correct for wrapped built-ins
	var inheritIfRequired$4 = function ($this, dummy, Wrapper) {
	  var NewTarget, NewTargetPrototype;
	  if (
	    // it can work only with native `setPrototypeOf`
	    setPrototypeOf$4 &&
	    // we haven't completely correct pre-ES6 way for getting `new.target`, so use this
	    isCallable$g(NewTarget = dummy.constructor) &&
	    NewTarget !== Wrapper &&
	    isObject$e(NewTargetPrototype = NewTarget.prototype) &&
	    NewTargetPrototype !== Wrapper.prototype
	  ) setPrototypeOf$4($this, NewTargetPrototype);
	  return $this;
	};

	var toString$k = toString$o;

	var normalizeStringArgument$5 = function (argument, $default) {
	  return argument === undefined ? arguments.length < 2 ? '' : $default : toString$k(argument);
	};

	var isObject$d = isObject$n;
	var createNonEnumerableProperty$8 = createNonEnumerableProperty$b;

	// `InstallErrorCause` abstract operation
	// https://tc39.es/proposal-error-cause/#sec-errorobjects-install-error-cause
	var installErrorCause$2 = function (O, options) {
	  if (isObject$d(options) && 'cause' in options) {
	    createNonEnumerableProperty$8(O, 'cause', options.cause);
	  }
	};

	var uncurryThis$E = functionUncurryThis;

	var $Error$1 = Error;
	var replace$6 = uncurryThis$E(''.replace);

	var TEST = (function (arg) { return String(new $Error$1(arg).stack); })('zxcasd');
	// eslint-disable-next-line redos/no-vulnerable, sonarjs/slow-regex -- safe
	var V8_OR_CHAKRA_STACK_ENTRY = /\n\s*at [^:]*:[^\n]*/;
	var IS_V8_OR_CHAKRA_STACK = V8_OR_CHAKRA_STACK_ENTRY.test(TEST);

	var errorStackClear = function (stack, dropEntries) {
	  if (IS_V8_OR_CHAKRA_STACK && typeof stack == 'string' && !$Error$1.prepareStackTrace) {
	    while (dropEntries--) stack = replace$6(stack, V8_OR_CHAKRA_STACK_ENTRY, '');
	  } return stack;
	};

	var fails$F = fails$S;
	var createPropertyDescriptor$6 = createPropertyDescriptor$a;

	var errorStackInstallable = !fails$F(function () {
	  var error = new Error('a');
	  if (!('stack' in error)) return true;
	  // eslint-disable-next-line es/no-object-defineproperty -- safe
	  Object.defineProperty(error, 'stack', createPropertyDescriptor$6(1, 7));
	  return error.stack !== 7;
	});

	var createNonEnumerableProperty$7 = createNonEnumerableProperty$b;
	var clearErrorStack$2 = errorStackClear;
	var ERROR_STACK_INSTALLABLE$1 = errorStackInstallable;

	// non-standard V8
	var captureStackTrace = Error.captureStackTrace;

	var errorStackInstall = function (error, C, stack, dropEntries) {
	  if (ERROR_STACK_INSTALLABLE$1) {
	    if (captureStackTrace) captureStackTrace(error, C);
	    else createNonEnumerableProperty$7(error, 'stack', clearErrorStack$2(stack, dropEntries));
	  }
	};

	var getBuiltIn$c = getBuiltIn$k;
	var hasOwn$e = hasOwnProperty_1;
	var createNonEnumerableProperty$6 = createNonEnumerableProperty$b;
	var isPrototypeOf$5 = objectIsPrototypeOf;
	var setPrototypeOf$3 = objectSetPrototypeOf;
	var copyConstructorProperties$2 = copyConstructorProperties$5;
	var proxyAccessor = proxyAccessor$1;
	var inheritIfRequired$3 = inheritIfRequired$4;
	var normalizeStringArgument$4 = normalizeStringArgument$5;
	var installErrorCause$1 = installErrorCause$2;
	var installErrorStack$1 = errorStackInstall;
	var DESCRIPTORS$j = descriptors;

	var wrapErrorConstructorWithCause$1 = function (FULL_NAME, wrapper, FORCED, IS_AGGREGATE_ERROR) {
	  var STACK_TRACE_LIMIT = 'stackTraceLimit';
	  var OPTIONS_POSITION = IS_AGGREGATE_ERROR ? 2 : 1;
	  var path = FULL_NAME.split('.');
	  var ERROR_NAME = path[path.length - 1];
	  var OriginalError = getBuiltIn$c.apply(null, path);

	  if (!OriginalError) return;

	  var OriginalErrorPrototype = OriginalError.prototype;

	  // V8 9.3- bug https://bugs.chromium.org/p/v8/issues/detail?id=12006
	  if (hasOwn$e(OriginalErrorPrototype, 'cause')) delete OriginalErrorPrototype.cause;

	  if (!FORCED) return OriginalError;

	  var BaseError = getBuiltIn$c('Error');

	  var WrappedError = wrapper(function (a, b) {
	    var message = normalizeStringArgument$4(IS_AGGREGATE_ERROR ? b : a, undefined);
	    var result = IS_AGGREGATE_ERROR ? new OriginalError(a) : new OriginalError();
	    if (message !== undefined) createNonEnumerableProperty$6(result, 'message', message);
	    installErrorStack$1(result, WrappedError, result.stack, 2);
	    if (this && isPrototypeOf$5(OriginalErrorPrototype, this)) inheritIfRequired$3(result, this, WrappedError);
	    if (arguments.length > OPTIONS_POSITION) installErrorCause$1(result, arguments[OPTIONS_POSITION]);
	    return result;
	  });

	  WrappedError.prototype = OriginalErrorPrototype;

	  if (ERROR_NAME !== 'Error') {
	    if (setPrototypeOf$3) setPrototypeOf$3(WrappedError, BaseError);
	    else copyConstructorProperties$2(WrappedError, BaseError, { name: true });
	  } else if (DESCRIPTORS$j && STACK_TRACE_LIMIT in OriginalError) {
	    proxyAccessor(WrappedError, OriginalError, STACK_TRACE_LIMIT);
	    proxyAccessor(WrappedError, OriginalError, 'prepareStackTrace');
	  }

	  copyConstructorProperties$2(WrappedError, OriginalError);

	  try {
	    // Safari 13- bug: WebAssembly errors does not have a proper `.name`
	    if (OriginalErrorPrototype.name !== ERROR_NAME) {
	      createNonEnumerableProperty$6(OriginalErrorPrototype, 'name', ERROR_NAME);
	    }
	    OriginalErrorPrototype.constructor = WrappedError;
	  } catch (error) { /* empty */ }

	  return WrappedError;
	};

	/* eslint-disable no-unused-vars -- required for functions `.length` */
	var $$1p = _export;
	var globalThis$r = globalThis_1;
	var apply$2 = functionApply;
	var wrapErrorConstructorWithCause = wrapErrorConstructorWithCause$1;

	var WEB_ASSEMBLY = 'WebAssembly';
	var WebAssembly = globalThis$r[WEB_ASSEMBLY];

	// eslint-disable-next-line es/no-error-cause -- feature detection
	var FORCED$a = new Error('e', { cause: 7 }).cause !== 7;

	var exportGlobalErrorCauseWrapper = function (ERROR_NAME, wrapper) {
	  var O = {};
	  O[ERROR_NAME] = wrapErrorConstructorWithCause(ERROR_NAME, wrapper, FORCED$a);
	  $$1p({ global: true, constructor: true, arity: 1, forced: FORCED$a }, O);
	};

	var exportWebAssemblyErrorCauseWrapper = function (ERROR_NAME, wrapper) {
	  if (WebAssembly && WebAssembly[ERROR_NAME]) {
	    var O = {};
	    O[ERROR_NAME] = wrapErrorConstructorWithCause(WEB_ASSEMBLY + '.' + ERROR_NAME, wrapper, FORCED$a);
	    $$1p({ target: WEB_ASSEMBLY, stat: true, constructor: true, arity: 1, forced: FORCED$a }, O);
	  }
	};

	// https://tc39.es/ecma262/#sec-nativeerror
	exportGlobalErrorCauseWrapper('Error', function (init) {
	  return function Error(message) { return apply$2(init, this, arguments); };
	});
	exportGlobalErrorCauseWrapper('EvalError', function (init) {
	  return function EvalError(message) { return apply$2(init, this, arguments); };
	});
	exportGlobalErrorCauseWrapper('RangeError', function (init) {
	  return function RangeError(message) { return apply$2(init, this, arguments); };
	});
	exportGlobalErrorCauseWrapper('ReferenceError', function (init) {
	  return function ReferenceError(message) { return apply$2(init, this, arguments); };
	});
	exportGlobalErrorCauseWrapper('SyntaxError', function (init) {
	  return function SyntaxError(message) { return apply$2(init, this, arguments); };
	});
	exportGlobalErrorCauseWrapper('TypeError', function (init) {
	  return function TypeError(message) { return apply$2(init, this, arguments); };
	});
	exportGlobalErrorCauseWrapper('URIError', function (init) {
	  return function URIError(message) { return apply$2(init, this, arguments); };
	});
	exportWebAssemblyErrorCauseWrapper('CompileError', function (init) {
	  return function CompileError(message) { return apply$2(init, this, arguments); };
	});
	exportWebAssemblyErrorCauseWrapper('LinkError', function (init) {
	  return function LinkError(message) { return apply$2(init, this, arguments); };
	});
	exportWebAssemblyErrorCauseWrapper('RuntimeError', function (init) {
	  return function RuntimeError(message) { return apply$2(init, this, arguments); };
	});

	var fails$E = fails$S;

	var correctPrototypeGetter = !fails$E(function () {
	  function F() { /* empty */ }
	  F.prototype.constructor = null;
	  // eslint-disable-next-line es/no-object-getprototypeof -- required for testing
	  return Object.getPrototypeOf(new F()) !== F.prototype;
	});

	var hasOwn$d = hasOwnProperty_1;
	var isCallable$f = isCallable$w;
	var toObject$c = toObject$g;
	var sharedKey = sharedKey$4;
	var CORRECT_PROTOTYPE_GETTER$1 = correctPrototypeGetter;

	var IE_PROTO = sharedKey('IE_PROTO');
	var $Object$1 = Object;
	var ObjectPrototype = $Object$1.prototype;

	// `Object.getPrototypeOf` method
	// https://tc39.es/ecma262/#sec-object.getprototypeof
	// eslint-disable-next-line es/no-object-getprototypeof -- safe
	var objectGetPrototypeOf$1 = CORRECT_PROTOTYPE_GETTER$1 ? $Object$1.getPrototypeOf : function (O) {
	  var object = toObject$c(O);
	  if (hasOwn$d(object, IE_PROTO)) return object[IE_PROTO];
	  var constructor = object.constructor;
	  if (isCallable$f(constructor) && object instanceof constructor) {
	    return constructor.prototype;
	  } return object instanceof $Object$1 ? ObjectPrototype : null;
	};

	var iterators = {};

	var wellKnownSymbol$n = wellKnownSymbol$w;
	var Iterators$4 = iterators;

	var ITERATOR$9 = wellKnownSymbol$n('iterator');
	var ArrayPrototype$1 = Array.prototype;

	// check on default Array iterator
	var isArrayIteratorMethod$2 = function (it) {
	  return it !== undefined && (Iterators$4.Array === it || ArrayPrototype$1[ITERATOR$9] === it);
	};

	var classof$7 = classof$d;
	var getMethod$5 = getMethod$7;
	var isNullOrUndefined$8 = isNullOrUndefined$b;
	var Iterators$3 = iterators;
	var wellKnownSymbol$m = wellKnownSymbol$w;

	var ITERATOR$8 = wellKnownSymbol$m('iterator');

	var getIteratorMethod$4 = function (it) {
	  if (!isNullOrUndefined$8(it)) return getMethod$5(it, ITERATOR$8)
	    || getMethod$5(it, '@@iterator')
	    || Iterators$3[classof$7(it)];
	};

	var call$o = functionCall;
	var aCallable$e = aCallable$i;
	var anObject$k = anObject$q;
	var tryToString$3 = tryToString$6;
	var getIteratorMethod$3 = getIteratorMethod$4;

	var $TypeError$f = TypeError;

	var getIterator$3 = function (argument, usingIterator) {
	  var iteratorMethod = arguments.length < 2 ? getIteratorMethod$3(argument) : usingIterator;
	  if (aCallable$e(iteratorMethod)) return anObject$k(call$o(iteratorMethod, argument));
	  throw new $TypeError$f(tryToString$3(argument) + ' is not iterable');
	};

	var call$n = functionCall;
	var anObject$j = anObject$q;
	var getMethod$4 = getMethod$7;

	var iteratorClose$5 = function (iterator, kind, value) {
	  var innerResult, innerError;
	  anObject$j(iterator);
	  try {
	    innerResult = getMethod$4(iterator, 'return');
	    if (!innerResult) {
	      if (kind === 'throw') throw value;
	      return value;
	    }
	    innerResult = call$n(innerResult, iterator);
	  } catch (error) {
	    innerError = true;
	    innerResult = error;
	  }
	  if (kind === 'throw') throw value;
	  if (innerError) throw innerResult;
	  anObject$j(innerResult);
	  return value;
	};

	var bind$9 = functionBindContext;
	var call$m = functionCall;
	var anObject$i = anObject$q;
	var tryToString$2 = tryToString$6;
	var isArrayIteratorMethod$1 = isArrayIteratorMethod$2;
	var lengthOfArrayLike$e = lengthOfArrayLike$h;
	var isPrototypeOf$4 = objectIsPrototypeOf;
	var getIterator$2 = getIterator$3;
	var getIteratorMethod$2 = getIteratorMethod$4;
	var iteratorClose$4 = iteratorClose$5;

	var $TypeError$e = TypeError;

	var Result = function (stopped, result) {
	  this.stopped = stopped;
	  this.result = result;
	};

	var ResultPrototype = Result.prototype;

	var iterate$d = function (iterable, unboundFunction, options) {
	  var that = options && options.that;
	  var AS_ENTRIES = !!(options && options.AS_ENTRIES);
	  var IS_RECORD = !!(options && options.IS_RECORD);
	  var IS_ITERATOR = !!(options && options.IS_ITERATOR);
	  var INTERRUPTED = !!(options && options.INTERRUPTED);
	  var fn = bind$9(unboundFunction, that);
	  var iterator, iterFn, index, length, result, next, step;

	  var stop = function (condition) {
	    if (iterator) iteratorClose$4(iterator, 'normal', condition);
	    return new Result(true, condition);
	  };

	  var callFn = function (value) {
	    if (AS_ENTRIES) {
	      anObject$i(value);
	      return INTERRUPTED ? fn(value[0], value[1], stop) : fn(value[0], value[1]);
	    } return INTERRUPTED ? fn(value, stop) : fn(value);
	  };

	  if (IS_RECORD) {
	    iterator = iterable.iterator;
	  } else if (IS_ITERATOR) {
	    iterator = iterable;
	  } else {
	    iterFn = getIteratorMethod$2(iterable);
	    if (!iterFn) throw new $TypeError$e(tryToString$2(iterable) + ' is not iterable');
	    // optimisation for array iterators
	    if (isArrayIteratorMethod$1(iterFn)) {
	      for (index = 0, length = lengthOfArrayLike$e(iterable); length > index; index++) {
	        result = callFn(iterable[index]);
	        if (result && isPrototypeOf$4(ResultPrototype, result)) return result;
	      } return new Result(false);
	    }
	    iterator = getIterator$2(iterable, iterFn);
	  }

	  next = IS_RECORD ? iterable.next : iterator.next;
	  while (!(step = call$m(next, iterator)).done) {
	    try {
	      result = callFn(step.value);
	    } catch (error) {
	      iteratorClose$4(iterator, 'throw', error);
	    }
	    if (typeof result == 'object' && result && isPrototypeOf$4(ResultPrototype, result)) return result;
	  } return new Result(false);
	};

	var $$1o = _export;
	var isPrototypeOf$3 = objectIsPrototypeOf;
	var getPrototypeOf$3 = objectGetPrototypeOf$1;
	var setPrototypeOf$2 = objectSetPrototypeOf;
	var copyConstructorProperties$1 = copyConstructorProperties$5;
	var create$8 = objectCreate;
	var createNonEnumerableProperty$5 = createNonEnumerableProperty$b;
	var createPropertyDescriptor$5 = createPropertyDescriptor$a;
	var installErrorCause = installErrorCause$2;
	var installErrorStack = errorStackInstall;
	var iterate$c = iterate$d;
	var normalizeStringArgument$3 = normalizeStringArgument$5;
	var wellKnownSymbol$l = wellKnownSymbol$w;

	var TO_STRING_TAG$2 = wellKnownSymbol$l('toStringTag');
	var $Error = Error;
	var push$9 = [].push;

	var $AggregateError = function AggregateError(errors, message /* , options */) {
	  var isInstance = isPrototypeOf$3(AggregateErrorPrototype, this);
	  var that;
	  if (setPrototypeOf$2) {
	    that = setPrototypeOf$2(new $Error(), isInstance ? getPrototypeOf$3(this) : AggregateErrorPrototype);
	  } else {
	    that = isInstance ? this : create$8(AggregateErrorPrototype);
	    createNonEnumerableProperty$5(that, TO_STRING_TAG$2, 'Error');
	  }
	  if (message !== undefined) createNonEnumerableProperty$5(that, 'message', normalizeStringArgument$3(message));
	  installErrorStack(that, $AggregateError, that.stack, 1);
	  if (arguments.length > 2) installErrorCause(that, arguments[2]);
	  var errorsArray = [];
	  iterate$c(errors, push$9, { that: errorsArray });
	  createNonEnumerableProperty$5(that, 'errors', errorsArray);
	  return that;
	};

	if (setPrototypeOf$2) setPrototypeOf$2($AggregateError, $Error);
	else copyConstructorProperties$1($AggregateError, $Error, { name: true });

	var AggregateErrorPrototype = $AggregateError.prototype = create$8($Error.prototype, {
	  constructor: createPropertyDescriptor$5(1, $AggregateError),
	  message: createPropertyDescriptor$5(1, ''),
	  name: createPropertyDescriptor$5(1, 'AggregateError')
	});

	// `AggregateError` constructor
	// https://tc39.es/ecma262/#sec-aggregate-error-constructor
	$$1o({ global: true, constructor: true, arity: 2 }, {
	  AggregateError: $AggregateError
	});

	var wellKnownSymbol$k = wellKnownSymbol$w;
	var create$7 = objectCreate;
	var defineProperty$6 = objectDefineProperty.f;

	var UNSCOPABLES = wellKnownSymbol$k('unscopables');
	var ArrayPrototype = Array.prototype;

	// Array.prototype[@@unscopables]
	// https://tc39.es/ecma262/#sec-array.prototype-@@unscopables
	if (ArrayPrototype[UNSCOPABLES] === undefined) {
	  defineProperty$6(ArrayPrototype, UNSCOPABLES, {
	    configurable: true,
	    value: create$7(null)
	  });
	}

	// add a key to Array.prototype[@@unscopables]
	var addToUnscopables$c = function (key) {
	  ArrayPrototype[UNSCOPABLES][key] = true;
	};

	var $$1n = _export;
	var toObject$b = toObject$g;
	var lengthOfArrayLike$d = lengthOfArrayLike$h;
	var toIntegerOrInfinity$8 = toIntegerOrInfinity$b;
	var addToUnscopables$b = addToUnscopables$c;

	// `Array.prototype.at` method
	// https://tc39.es/ecma262/#sec-array.prototype.at
	$$1n({ target: 'Array', proto: true }, {
	  at: function at(index) {
	    var O = toObject$b(this);
	    var len = lengthOfArrayLike$d(O);
	    var relativeIndex = toIntegerOrInfinity$8(index);
	    var k = relativeIndex >= 0 ? relativeIndex : len + relativeIndex;
	    return (k < 0 || k >= len) ? undefined : O[k];
	  }
	});

	addToUnscopables$b('at');

	var toObject$a = toObject$g;
	var toAbsoluteIndex$2 = toAbsoluteIndex$4;
	var lengthOfArrayLike$c = lengthOfArrayLike$h;

	// `Array.prototype.fill` method implementation
	// https://tc39.es/ecma262/#sec-array.prototype.fill
	var arrayFill = function fill(value /* , start = 0, end = @length */) {
	  var O = toObject$a(this);
	  var length = lengthOfArrayLike$c(O);
	  var argumentsLength = arguments.length;
	  var index = toAbsoluteIndex$2(argumentsLength > 1 ? arguments[1] : undefined, length);
	  var end = argumentsLength > 2 ? arguments[2] : undefined;
	  var endPos = end === undefined ? length : toAbsoluteIndex$2(end, length);
	  while (endPos > index) O[index++] = value;
	  return O;
	};

	var $$1m = _export;
	var fill = arrayFill;
	var addToUnscopables$a = addToUnscopables$c;

	// `Array.prototype.fill` method
	// https://tc39.es/ecma262/#sec-array.prototype.fill
	$$1m({ target: 'Array', proto: true }, {
	  fill: fill
	});

	// https://tc39.es/ecma262/#sec-array.prototype-@@unscopables
	addToUnscopables$a('fill');

	var $$1l = _export;
	var $find = arrayIteration.find;
	var addToUnscopables$9 = addToUnscopables$c;

	var FIND = 'find';
	var SKIPS_HOLES$1 = true;

	// Shouldn't skip holes
	// eslint-disable-next-line es/no-array-prototype-find -- testing
	if (FIND in []) Array(1)[FIND](function () { SKIPS_HOLES$1 = false; });

	// `Array.prototype.find` method
	// https://tc39.es/ecma262/#sec-array.prototype.find
	$$1l({ target: 'Array', proto: true, forced: SKIPS_HOLES$1 }, {
	  find: function find(callbackfn /* , that = undefined */) {
	    return $find(this, callbackfn, arguments.length > 1 ? arguments[1] : undefined);
	  }
	});

	// https://tc39.es/ecma262/#sec-array.prototype-@@unscopables
	addToUnscopables$9(FIND);

	var $$1k = _export;
	var $findIndex = arrayIteration.findIndex;
	var addToUnscopables$8 = addToUnscopables$c;

	var FIND_INDEX = 'findIndex';
	var SKIPS_HOLES = true;

	// Shouldn't skip holes
	// eslint-disable-next-line es/no-array-prototype-findindex -- testing
	if (FIND_INDEX in []) Array(1)[FIND_INDEX](function () { SKIPS_HOLES = false; });

	// `Array.prototype.findIndex` method
	// https://tc39.es/ecma262/#sec-array.prototype.findindex
	$$1k({ target: 'Array', proto: true, forced: SKIPS_HOLES }, {
	  findIndex: function findIndex(callbackfn /* , that = undefined */) {
	    return $findIndex(this, callbackfn, arguments.length > 1 ? arguments[1] : undefined);
	  }
	});

	// https://tc39.es/ecma262/#sec-array.prototype-@@unscopables
	addToUnscopables$8(FIND_INDEX);

	var bind$8 = functionBindContext;
	var IndexedObject$2 = indexedObject;
	var toObject$9 = toObject$g;
	var lengthOfArrayLike$b = lengthOfArrayLike$h;

	// `Array.prototype.{ findLast, findLastIndex }` methods implementation
	var createMethod$4 = function (TYPE) {
	  var IS_FIND_LAST_INDEX = TYPE === 1;
	  return function ($this, callbackfn, that) {
	    var O = toObject$9($this);
	    var self = IndexedObject$2(O);
	    var index = lengthOfArrayLike$b(self);
	    var boundFunction = bind$8(callbackfn, that);
	    var value, result;
	    while (index-- > 0) {
	      value = self[index];
	      result = boundFunction(value, index, O);
	      if (result) switch (TYPE) {
	        case 0: return value; // findLast
	        case 1: return index; // findLastIndex
	      }
	    }
	    return IS_FIND_LAST_INDEX ? -1 : undefined;
	  };
	};

	var arrayIterationFromLast = {
	  // `Array.prototype.findLast` method
	  // https://github.com/tc39/proposal-array-find-from-last
	  findLast: createMethod$4(0),
	  // `Array.prototype.findLastIndex` method
	  // https://github.com/tc39/proposal-array-find-from-last
	  findLastIndex: createMethod$4(1)
	};

	var $$1j = _export;
	var $findLast = arrayIterationFromLast.findLast;
	var addToUnscopables$7 = addToUnscopables$c;

	// `Array.prototype.findLast` method
	// https://tc39.es/ecma262/#sec-array.prototype.findlast
	$$1j({ target: 'Array', proto: true }, {
	  findLast: function findLast(callbackfn /* , that = undefined */) {
	    return $findLast(this, callbackfn, arguments.length > 1 ? arguments[1] : undefined);
	  }
	});

	addToUnscopables$7('findLast');

	var $$1i = _export;
	var $findLastIndex = arrayIterationFromLast.findLastIndex;
	var addToUnscopables$6 = addToUnscopables$c;

	// `Array.prototype.findLastIndex` method
	// https://tc39.es/ecma262/#sec-array.prototype.findlastindex
	$$1i({ target: 'Array', proto: true }, {
	  findLastIndex: function findLastIndex(callbackfn /* , that = undefined */) {
	    return $findLastIndex(this, callbackfn, arguments.length > 1 ? arguments[1] : undefined);
	  }
	});

	addToUnscopables$6('findLastIndex');

	var $TypeError$d = TypeError;
	var MAX_SAFE_INTEGER = 0x1FFFFFFFFFFFFF; // 2 ** 53 - 1 == 9007199254740991

	var doesNotExceedSafeInteger$2 = function (it) {
	  if (it > MAX_SAFE_INTEGER) throw $TypeError$d('Maximum allowed index exceeded');
	  return it;
	};

	var isArray$3 = isArray$6;
	var lengthOfArrayLike$a = lengthOfArrayLike$h;
	var doesNotExceedSafeInteger$1 = doesNotExceedSafeInteger$2;
	var bind$7 = functionBindContext;

	// `FlattenIntoArray` abstract operation
	// https://tc39.github.io/proposal-flatMap/#sec-FlattenIntoArray
	var flattenIntoArray$2 = function (target, original, source, sourceLen, start, depth, mapper, thisArg) {
	  var targetIndex = start;
	  var sourceIndex = 0;
	  var mapFn = mapper ? bind$7(mapper, thisArg) : false;
	  var element, elementLen;

	  while (sourceIndex < sourceLen) {
	    if (sourceIndex in source) {
	      element = mapFn ? mapFn(source[sourceIndex], sourceIndex, original) : source[sourceIndex];

	      if (depth > 0 && isArray$3(element)) {
	        elementLen = lengthOfArrayLike$a(element);
	        targetIndex = flattenIntoArray$2(target, original, element, elementLen, targetIndex, depth - 1) - 1;
	      } else {
	        doesNotExceedSafeInteger$1(targetIndex + 1);
	        target[targetIndex] = element;
	      }

	      targetIndex++;
	    }
	    sourceIndex++;
	  }
	  return targetIndex;
	};

	var flattenIntoArray_1 = flattenIntoArray$2;

	var $$1h = _export;
	var flattenIntoArray$1 = flattenIntoArray_1;
	var toObject$8 = toObject$g;
	var lengthOfArrayLike$9 = lengthOfArrayLike$h;
	var toIntegerOrInfinity$7 = toIntegerOrInfinity$b;
	var arraySpeciesCreate$1 = arraySpeciesCreate$3;

	// `Array.prototype.flat` method
	// https://tc39.es/ecma262/#sec-array.prototype.flat
	$$1h({ target: 'Array', proto: true }, {
	  flat: function flat(/* depthArg = 1 */) {
	    var depthArg = arguments.length ? arguments[0] : undefined;
	    var O = toObject$8(this);
	    var sourceLen = lengthOfArrayLike$9(O);
	    var A = arraySpeciesCreate$1(O, 0);
	    A.length = flattenIntoArray$1(A, O, O, sourceLen, 0, depthArg === undefined ? 1 : toIntegerOrInfinity$7(depthArg));
	    return A;
	  }
	});

	var $$1g = _export;
	var flattenIntoArray = flattenIntoArray_1;
	var aCallable$d = aCallable$i;
	var toObject$7 = toObject$g;
	var lengthOfArrayLike$8 = lengthOfArrayLike$h;
	var arraySpeciesCreate = arraySpeciesCreate$3;

	// `Array.prototype.flatMap` method
	// https://tc39.es/ecma262/#sec-array.prototype.flatmap
	$$1g({ target: 'Array', proto: true }, {
	  flatMap: function flatMap(callbackfn /* , thisArg */) {
	    var O = toObject$7(this);
	    var sourceLen = lengthOfArrayLike$8(O);
	    var A;
	    aCallable$d(callbackfn);
	    A = arraySpeciesCreate(O, 0);
	    A.length = flattenIntoArray(A, O, O, sourceLen, 0, 1, callbackfn, arguments.length > 1 ? arguments[1] : undefined);
	    return A;
	  }
	});

	var anObject$h = anObject$q;
	var iteratorClose$3 = iteratorClose$5;

	// call something on iterator step with safe closing on error
	var callWithSafeIterationClosing$2 = function (iterator, fn, value, ENTRIES) {
	  try {
	    return ENTRIES ? fn(anObject$h(value)[0], value[1]) : fn(value);
	  } catch (error) {
	    iteratorClose$3(iterator, 'throw', error);
	  }
	};

	var DESCRIPTORS$i = descriptors;
	var definePropertyModule = objectDefineProperty;
	var createPropertyDescriptor$4 = createPropertyDescriptor$a;

	var createProperty$6 = function (object, key, value) {
	  if (DESCRIPTORS$i) definePropertyModule.f(object, key, createPropertyDescriptor$4(0, value));
	  else object[key] = value;
	};

	var bind$6 = functionBindContext;
	var call$l = functionCall;
	var toObject$6 = toObject$g;
	var callWithSafeIterationClosing$1 = callWithSafeIterationClosing$2;
	var isArrayIteratorMethod = isArrayIteratorMethod$2;
	var isConstructor$4 = isConstructor$6;
	var lengthOfArrayLike$7 = lengthOfArrayLike$h;
	var createProperty$5 = createProperty$6;
	var getIterator$1 = getIterator$3;
	var getIteratorMethod$1 = getIteratorMethod$4;

	var $Array$5 = Array;

	// `Array.from` method implementation
	// https://tc39.es/ecma262/#sec-array.from
	var arrayFrom$1 = function from(arrayLike /* , mapfn = undefined, thisArg = undefined */) {
	  var O = toObject$6(arrayLike);
	  var IS_CONSTRUCTOR = isConstructor$4(this);
	  var argumentsLength = arguments.length;
	  var mapfn = argumentsLength > 1 ? arguments[1] : undefined;
	  var mapping = mapfn !== undefined;
	  if (mapping) mapfn = bind$6(mapfn, argumentsLength > 2 ? arguments[2] : undefined);
	  var iteratorMethod = getIteratorMethod$1(O);
	  var index = 0;
	  var length, result, step, iterator, next, value;
	  // if the target is not iterable or it's an array with the default iterator - use a simple case
	  if (iteratorMethod && !(this === $Array$5 && isArrayIteratorMethod(iteratorMethod))) {
	    result = IS_CONSTRUCTOR ? new this() : [];
	    iterator = getIterator$1(O, iteratorMethod);
	    next = iterator.next;
	    for (;!(step = call$l(next, iterator)).done; index++) {
	      value = mapping ? callWithSafeIterationClosing$1(iterator, mapfn, [step.value, index], true) : step.value;
	      createProperty$5(result, index, value);
	    }
	  } else {
	    length = lengthOfArrayLike$7(O);
	    result = IS_CONSTRUCTOR ? new this(length) : $Array$5(length);
	    for (;length > index; index++) {
	      value = mapping ? mapfn(O[index], index) : O[index];
	      createProperty$5(result, index, value);
	    }
	  }
	  result.length = index;
	  return result;
	};

	var wellKnownSymbol$j = wellKnownSymbol$w;

	var ITERATOR$7 = wellKnownSymbol$j('iterator');
	var SAFE_CLOSING = false;

	try {
	  var called = 0;
	  var iteratorWithReturn = {
	    next: function () {
	      return { done: !!called++ };
	    },
	    'return': function () {
	      SAFE_CLOSING = true;
	    }
	  };
	  iteratorWithReturn[ITERATOR$7] = function () {
	    return this;
	  };
	  // eslint-disable-next-line es/no-array-from, no-throw-literal -- required for testing
	  Array.from(iteratorWithReturn, function () { throw 2; });
	} catch (error) { /* empty */ }

	var checkCorrectnessOfIteration$3 = function (exec, SKIP_CLOSING) {
	  try {
	    if (!SKIP_CLOSING && !SAFE_CLOSING) return false;
	  } catch (error) { return false; } // workaround of old WebKit + `eval` bug
	  var ITERATION_SUPPORT = false;
	  try {
	    var object = {};
	    object[ITERATOR$7] = function () {
	      return {
	        next: function () {
	          return { done: ITERATION_SUPPORT = true };
	        }
	      };
	    };
	    exec(object);
	  } catch (error) { /* empty */ }
	  return ITERATION_SUPPORT;
	};

	var $$1f = _export;
	var from = arrayFrom$1;
	var checkCorrectnessOfIteration$2 = checkCorrectnessOfIteration$3;

	var INCORRECT_ITERATION = !checkCorrectnessOfIteration$2(function (iterable) {
	  // eslint-disable-next-line es/no-array-from -- required for testing
	  Array.from(iterable);
	});

	// `Array.from` method
	// https://tc39.es/ecma262/#sec-array.from
	$$1f({ target: 'Array', stat: true, forced: INCORRECT_ITERATION }, {
	  from: from
	});

	var $$1e = _export;
	var $includes = arrayIncludes.includes;
	var fails$D = fails$S;
	var addToUnscopables$5 = addToUnscopables$c;

	// FF99+ bug
	var BROKEN_ON_SPARSE = fails$D(function () {
	  // eslint-disable-next-line es/no-array-prototype-includes -- detection
	  return !Array(1).includes();
	});

	// `Array.prototype.includes` method
	// https://tc39.es/ecma262/#sec-array.prototype.includes
	$$1e({ target: 'Array', proto: true, forced: BROKEN_ON_SPARSE }, {
	  includes: function includes(el /* , fromIndex = 0 */) {
	    return $includes(this, el, arguments.length > 1 ? arguments[1] : undefined);
	  }
	});

	// https://tc39.es/ecma262/#sec-array.prototype-@@unscopables
	addToUnscopables$5('includes');

	var fails$C = fails$S;
	var isCallable$e = isCallable$w;
	var isObject$c = isObject$n;
	var getPrototypeOf$2 = objectGetPrototypeOf$1;
	var defineBuiltIn$g = defineBuiltIn$k;
	var wellKnownSymbol$i = wellKnownSymbol$w;

	var ITERATOR$6 = wellKnownSymbol$i('iterator');
	var BUGGY_SAFARI_ITERATORS$1 = false;

	// `%IteratorPrototype%` object
	// https://tc39.es/ecma262/#sec-%iteratorprototype%-object
	var IteratorPrototype$4, PrototypeOfArrayIteratorPrototype, arrayIterator;

	/* eslint-disable es/no-array-prototype-keys -- safe */
	if ([].keys) {
	  arrayIterator = [].keys();
	  // Safari 8 has buggy iterators w/o `next`
	  if (!('next' in arrayIterator)) BUGGY_SAFARI_ITERATORS$1 = true;
	  else {
	    PrototypeOfArrayIteratorPrototype = getPrototypeOf$2(getPrototypeOf$2(arrayIterator));
	    if (PrototypeOfArrayIteratorPrototype !== Object.prototype) IteratorPrototype$4 = PrototypeOfArrayIteratorPrototype;
	  }
	}

	var NEW_ITERATOR_PROTOTYPE = !isObject$c(IteratorPrototype$4) || fails$C(function () {
	  var test = {};
	  // FF44- legacy iterators case
	  return IteratorPrototype$4[ITERATOR$6].call(test) !== test;
	});

	if (NEW_ITERATOR_PROTOTYPE) IteratorPrototype$4 = {};

	// `%IteratorPrototype%[@@iterator]()` method
	// https://tc39.es/ecma262/#sec-%iteratorprototype%-@@iterator
	if (!isCallable$e(IteratorPrototype$4[ITERATOR$6])) {
	  defineBuiltIn$g(IteratorPrototype$4, ITERATOR$6, function () {
	    return this;
	  });
	}

	var iteratorsCore = {
	  IteratorPrototype: IteratorPrototype$4,
	  BUGGY_SAFARI_ITERATORS: BUGGY_SAFARI_ITERATORS$1
	};

	var IteratorPrototype$3 = iteratorsCore.IteratorPrototype;
	var create$6 = objectCreate;
	var createPropertyDescriptor$3 = createPropertyDescriptor$a;
	var setToStringTag$7 = setToStringTag$9;
	var Iterators$2 = iterators;

	var returnThis$1 = function () { return this; };

	var iteratorCreateConstructor = function (IteratorConstructor, NAME, next, ENUMERABLE_NEXT) {
	  var TO_STRING_TAG = NAME + ' Iterator';
	  IteratorConstructor.prototype = create$6(IteratorPrototype$3, { next: createPropertyDescriptor$3(+!ENUMERABLE_NEXT, next) });
	  setToStringTag$7(IteratorConstructor, TO_STRING_TAG, false);
	  Iterators$2[TO_STRING_TAG] = returnThis$1;
	  return IteratorConstructor;
	};

	var $$1d = _export;
	var call$k = functionCall;
	var FunctionName = functionName;
	var isCallable$d = isCallable$w;
	var createIteratorConstructor$2 = iteratorCreateConstructor;
	var getPrototypeOf$1 = objectGetPrototypeOf$1;
	var setPrototypeOf$1 = objectSetPrototypeOf;
	var setToStringTag$6 = setToStringTag$9;
	var createNonEnumerableProperty$4 = createNonEnumerableProperty$b;
	var defineBuiltIn$f = defineBuiltIn$k;
	var wellKnownSymbol$h = wellKnownSymbol$w;
	var Iterators$1 = iterators;
	var IteratorsCore = iteratorsCore;

	var PROPER_FUNCTION_NAME$2 = FunctionName.PROPER;
	var CONFIGURABLE_FUNCTION_NAME = FunctionName.CONFIGURABLE;
	var IteratorPrototype$2 = IteratorsCore.IteratorPrototype;
	var BUGGY_SAFARI_ITERATORS = IteratorsCore.BUGGY_SAFARI_ITERATORS;
	var ITERATOR$5 = wellKnownSymbol$h('iterator');
	var KEYS = 'keys';
	var VALUES = 'values';
	var ENTRIES = 'entries';

	var returnThis = function () { return this; };

	var iteratorDefine = function (Iterable, NAME, IteratorConstructor, next, DEFAULT, IS_SET, FORCED) {
	  createIteratorConstructor$2(IteratorConstructor, NAME, next);

	  var getIterationMethod = function (KIND) {
	    if (KIND === DEFAULT && defaultIterator) return defaultIterator;
	    if (!BUGGY_SAFARI_ITERATORS && KIND && KIND in IterablePrototype) return IterablePrototype[KIND];

	    switch (KIND) {
	      case KEYS: return function keys() { return new IteratorConstructor(this, KIND); };
	      case VALUES: return function values() { return new IteratorConstructor(this, KIND); };
	      case ENTRIES: return function entries() { return new IteratorConstructor(this, KIND); };
	    }

	    return function () { return new IteratorConstructor(this); };
	  };

	  var TO_STRING_TAG = NAME + ' Iterator';
	  var INCORRECT_VALUES_NAME = false;
	  var IterablePrototype = Iterable.prototype;
	  var nativeIterator = IterablePrototype[ITERATOR$5]
	    || IterablePrototype['@@iterator']
	    || DEFAULT && IterablePrototype[DEFAULT];
	  var defaultIterator = !BUGGY_SAFARI_ITERATORS && nativeIterator || getIterationMethod(DEFAULT);
	  var anyNativeIterator = NAME === 'Array' ? IterablePrototype.entries || nativeIterator : nativeIterator;
	  var CurrentIteratorPrototype, methods, KEY;

	  // fix native
	  if (anyNativeIterator) {
	    CurrentIteratorPrototype = getPrototypeOf$1(anyNativeIterator.call(new Iterable()));
	    if (CurrentIteratorPrototype !== Object.prototype && CurrentIteratorPrototype.next) {
	      if (getPrototypeOf$1(CurrentIteratorPrototype) !== IteratorPrototype$2) {
	        if (setPrototypeOf$1) {
	          setPrototypeOf$1(CurrentIteratorPrototype, IteratorPrototype$2);
	        } else if (!isCallable$d(CurrentIteratorPrototype[ITERATOR$5])) {
	          defineBuiltIn$f(CurrentIteratorPrototype, ITERATOR$5, returnThis);
	        }
	      }
	      // Set @@toStringTag to native iterators
	      setToStringTag$6(CurrentIteratorPrototype, TO_STRING_TAG, true);
	    }
	  }

	  // fix Array.prototype.{ values, @@iterator }.name in V8 / FF
	  if (PROPER_FUNCTION_NAME$2 && DEFAULT === VALUES && nativeIterator && nativeIterator.name !== VALUES) {
	    if (CONFIGURABLE_FUNCTION_NAME) {
	      createNonEnumerableProperty$4(IterablePrototype, 'name', VALUES);
	    } else {
	      INCORRECT_VALUES_NAME = true;
	      defaultIterator = function values() { return call$k(nativeIterator, this); };
	    }
	  }

	  // export additional methods
	  if (DEFAULT) {
	    methods = {
	      values: getIterationMethod(VALUES),
	      keys: IS_SET ? defaultIterator : getIterationMethod(KEYS),
	      entries: getIterationMethod(ENTRIES)
	    };
	    if (FORCED) for (KEY in methods) {
	      if (BUGGY_SAFARI_ITERATORS || INCORRECT_VALUES_NAME || !(KEY in IterablePrototype)) {
	        defineBuiltIn$f(IterablePrototype, KEY, methods[KEY]);
	      }
	    } else $$1d({ target: NAME, proto: true, forced: BUGGY_SAFARI_ITERATORS || INCORRECT_VALUES_NAME }, methods);
	  }

	  // define iterator
	  if (IterablePrototype[ITERATOR$5] !== defaultIterator) {
	    defineBuiltIn$f(IterablePrototype, ITERATOR$5, defaultIterator, { name: DEFAULT });
	  }
	  Iterators$1[NAME] = defaultIterator;

	  return methods;
	};

	// `CreateIterResultObject` abstract operation
	// https://tc39.es/ecma262/#sec-createiterresultobject
	var createIterResultObject$6 = function (value, done) {
	  return { value: value, done: done };
	};

	var toIndexedObject$6 = toIndexedObject$d;
	var addToUnscopables$4 = addToUnscopables$c;
	var Iterators = iterators;
	var InternalStateModule$9 = internalState;
	var defineProperty$5 = objectDefineProperty.f;
	var defineIterator$2 = iteratorDefine;
	var createIterResultObject$5 = createIterResultObject$6;
	var DESCRIPTORS$h = descriptors;

	var ARRAY_ITERATOR = 'Array Iterator';
	var setInternalState$9 = InternalStateModule$9.set;
	var getInternalState$4 = InternalStateModule$9.getterFor(ARRAY_ITERATOR);

	// `Array.prototype.entries` method
	// https://tc39.es/ecma262/#sec-array.prototype.entries
	// `Array.prototype.keys` method
	// https://tc39.es/ecma262/#sec-array.prototype.keys
	// `Array.prototype.values` method
	// https://tc39.es/ecma262/#sec-array.prototype.values
	// `Array.prototype[@@iterator]` method
	// https://tc39.es/ecma262/#sec-array.prototype-@@iterator
	// `CreateArrayIterator` internal method
	// https://tc39.es/ecma262/#sec-createarrayiterator
	var es_array_iterator = defineIterator$2(Array, 'Array', function (iterated, kind) {
	  setInternalState$9(this, {
	    type: ARRAY_ITERATOR,
	    target: toIndexedObject$6(iterated), // target
	    index: 0,                          // next index
	    kind: kind                         // kind
	  });
	// `%ArrayIteratorPrototype%.next` method
	// https://tc39.es/ecma262/#sec-%arrayiteratorprototype%.next
	}, function () {
	  var state = getInternalState$4(this);
	  var target = state.target;
	  var index = state.index++;
	  if (!target || index >= target.length) {
	    state.target = null;
	    return createIterResultObject$5(undefined, true);
	  }
	  switch (state.kind) {
	    case 'keys': return createIterResultObject$5(index, false);
	    case 'values': return createIterResultObject$5(target[index], false);
	  } return createIterResultObject$5([index, target[index]], false);
	}, 'values');

	// argumentsList[@@iterator] is %ArrayProto_values%
	// https://tc39.es/ecma262/#sec-createunmappedargumentsobject
	// https://tc39.es/ecma262/#sec-createmappedargumentsobject
	var values = Iterators.Arguments = Iterators.Array;

	// https://tc39.es/ecma262/#sec-array.prototype-@@unscopables
	addToUnscopables$4('keys');
	addToUnscopables$4('values');
	addToUnscopables$4('entries');

	// V8 ~ Chrome 45- bug
	if (DESCRIPTORS$h && values.name !== 'values') try {
	  defineProperty$5(values, 'name', { value: 'values' });
	} catch (error) { /* empty */ }

	var fails$B = fails$S;

	var arrayMethodIsStrict$2 = function (METHOD_NAME, argument) {
	  var method = [][METHOD_NAME];
	  return !!method && fails$B(function () {
	    // eslint-disable-next-line no-useless-call -- required for testing
	    method.call(null, argument || function () { return 1; }, 1);
	  });
	};

	var $$1c = _export;
	var uncurryThis$D = functionUncurryThis;
	var IndexedObject$1 = indexedObject;
	var toIndexedObject$5 = toIndexedObject$d;
	var arrayMethodIsStrict$1 = arrayMethodIsStrict$2;

	var nativeJoin = uncurryThis$D([].join);

	var ES3_STRINGS = IndexedObject$1 !== Object;
	var FORCED$9 = ES3_STRINGS || !arrayMethodIsStrict$1('join', ',');

	// `Array.prototype.join` method
	// https://tc39.es/ecma262/#sec-array.prototype.join
	$$1c({ target: 'Array', proto: true, forced: FORCED$9 }, {
	  join: function join(separator) {
	    return nativeJoin(toIndexedObject$5(this), separator === undefined ? ',' : separator);
	  }
	});

	var fails$A = fails$S;
	var wellKnownSymbol$g = wellKnownSymbol$w;
	var V8_VERSION$1 = environmentV8Version;

	var SPECIES$5 = wellKnownSymbol$g('species');

	var arrayMethodHasSpeciesSupport$2 = function (METHOD_NAME) {
	  // We can't use this feature detection in V8 since it causes
	  // deoptimization and serious performance degradation
	  // https://github.com/zloirock/core-js/issues/677
	  return V8_VERSION$1 >= 51 || !fails$A(function () {
	    var array = [];
	    var constructor = array.constructor = {};
	    constructor[SPECIES$5] = function () {
	      return { foo: 1 };
	    };
	    return array[METHOD_NAME](Boolean).foo !== 1;
	  });
	};

	var $$1b = _export;
	var $map = arrayIteration.map;
	var arrayMethodHasSpeciesSupport$1 = arrayMethodHasSpeciesSupport$2;

	var HAS_SPECIES_SUPPORT$1 = arrayMethodHasSpeciesSupport$1('map');

	// `Array.prototype.map` method
	// https://tc39.es/ecma262/#sec-array.prototype.map
	// with adding support of @@species
	$$1b({ target: 'Array', proto: true, forced: !HAS_SPECIES_SUPPORT$1 }, {
	  map: function map(callbackfn /* , thisArg */) {
	    return $map(this, callbackfn, arguments.length > 1 ? arguments[1] : undefined);
	  }
	});

	var $$1a = _export;
	var fails$z = fails$S;
	var isConstructor$3 = isConstructor$6;
	var createProperty$4 = createProperty$6;

	var $Array$4 = Array;

	var ISNT_GENERIC = fails$z(function () {
	  function F() { /* empty */ }
	  // eslint-disable-next-line es/no-array-of -- safe
	  return !($Array$4.of.call(F) instanceof F);
	});

	// `Array.of` method
	// https://tc39.es/ecma262/#sec-array.of
	// WebKit Array.of isn't generic
	$$1a({ target: 'Array', stat: true, forced: ISNT_GENERIC }, {
	  of: function of(/* ...args */) {
	    var index = 0;
	    var argumentsLength = arguments.length;
	    var result = new (isConstructor$3(this) ? this : $Array$4)(argumentsLength);
	    while (argumentsLength > index) createProperty$4(result, index, arguments[index++]);
	    result.length = argumentsLength;
	    return result;
	  }
	});

	var DESCRIPTORS$g = descriptors;
	var isArray$2 = isArray$6;

	var $TypeError$c = TypeError;
	// eslint-disable-next-line es/no-object-getownpropertydescriptor -- safe
	var getOwnPropertyDescriptor$4 = Object.getOwnPropertyDescriptor;

	// Safari < 13 does not throw an error in this case
	var SILENT_ON_NON_WRITABLE_LENGTH_SET = DESCRIPTORS$g && !function () {
	  // makes no sense without proper strict mode support
	  if (this !== undefined) return true;
	  try {
	    // eslint-disable-next-line es/no-object-defineproperty -- safe
	    Object.defineProperty([], 'length', { writable: false }).length = 1;
	  } catch (error) {
	    return error instanceof TypeError;
	  }
	}();

	var arraySetLength = SILENT_ON_NON_WRITABLE_LENGTH_SET ? function (O, length) {
	  if (isArray$2(O) && !getOwnPropertyDescriptor$4(O, 'length').writable) {
	    throw new $TypeError$c('Cannot set read only .length');
	  } return O.length = length;
	} : function (O, length) {
	  return O.length = length;
	};

	var $$19 = _export;
	var toObject$5 = toObject$g;
	var lengthOfArrayLike$6 = lengthOfArrayLike$h;
	var setArrayLength = arraySetLength;
	var doesNotExceedSafeInteger = doesNotExceedSafeInteger$2;
	var fails$y = fails$S;

	var INCORRECT_TO_LENGTH = fails$y(function () {
	  return [].push.call({ length: 0x100000000 }, 1) !== 4294967297;
	});

	// V8 <= 121 and Safari <= 15.4; FF < 23 throws InternalError
	// https://bugs.chromium.org/p/v8/issues/detail?id=12681
	var properErrorOnNonWritableLength = function () {
	  try {
	    // eslint-disable-next-line es/no-object-defineproperty -- safe
	    Object.defineProperty([], 'length', { writable: false }).push();
	  } catch (error) {
	    return error instanceof TypeError;
	  }
	};

	var FORCED$8 = INCORRECT_TO_LENGTH || !properErrorOnNonWritableLength();

	// `Array.prototype.push` method
	// https://tc39.es/ecma262/#sec-array.prototype.push
	$$19({ target: 'Array', proto: true, arity: 1, forced: FORCED$8 }, {
	  // eslint-disable-next-line no-unused-vars -- required for `.length`
	  push: function push(item) {
	    var O = toObject$5(this);
	    var len = lengthOfArrayLike$6(O);
	    var argCount = arguments.length;
	    doesNotExceedSafeInteger(len + argCount);
	    for (var i = 0; i < argCount; i++) {
	      O[len] = arguments[i];
	      len++;
	    }
	    setArrayLength(O, len);
	    return len;
	  }
	});

	var $$18 = _export;
	var isArray$1 = isArray$6;
	var isConstructor$2 = isConstructor$6;
	var isObject$b = isObject$n;
	var toAbsoluteIndex$1 = toAbsoluteIndex$4;
	var lengthOfArrayLike$5 = lengthOfArrayLike$h;
	var toIndexedObject$4 = toIndexedObject$d;
	var createProperty$3 = createProperty$6;
	var wellKnownSymbol$f = wellKnownSymbol$w;
	var arrayMethodHasSpeciesSupport = arrayMethodHasSpeciesSupport$2;
	var nativeSlice = arraySlice$5;

	var HAS_SPECIES_SUPPORT = arrayMethodHasSpeciesSupport('slice');

	var SPECIES$4 = wellKnownSymbol$f('species');
	var $Array$3 = Array;
	var max$3 = Math.max;

	// `Array.prototype.slice` method
	// https://tc39.es/ecma262/#sec-array.prototype.slice
	// fallback for not array-like ES3 strings and DOM objects
	$$18({ target: 'Array', proto: true, forced: !HAS_SPECIES_SUPPORT }, {
	  slice: function slice(start, end) {
	    var O = toIndexedObject$4(this);
	    var length = lengthOfArrayLike$5(O);
	    var k = toAbsoluteIndex$1(start, length);
	    var fin = toAbsoluteIndex$1(end === undefined ? length : end, length);
	    // inline `ArraySpeciesCreate` for usage native `Array#slice` where it's possible
	    var Constructor, result, n;
	    if (isArray$1(O)) {
	      Constructor = O.constructor;
	      // cross-realm fallback
	      if (isConstructor$2(Constructor) && (Constructor === $Array$3 || isArray$1(Constructor.prototype))) {
	        Constructor = undefined;
	      } else if (isObject$b(Constructor)) {
	        Constructor = Constructor[SPECIES$4];
	        if (Constructor === null) Constructor = undefined;
	      }
	      if (Constructor === $Array$3 || Constructor === undefined) {
	        return nativeSlice(O, k, fin);
	      }
	    }
	    result = new (Constructor === undefined ? $Array$3 : Constructor)(max$3(fin - k, 0));
	    for (n = 0; k < fin; k++, n++) if (k in O) createProperty$3(result, n, O[k]);
	    result.length = n;
	    return result;
	  }
	});

	var tryToString$1 = tryToString$6;

	var $TypeError$b = TypeError;

	var deletePropertyOrThrow$1 = function (O, P) {
	  if (!delete O[P]) throw new $TypeError$b('Cannot delete property ' + tryToString$1(P) + ' of ' + tryToString$1(O));
	};

	var arraySlice$2 = arraySlice$5;

	var floor$5 = Math.floor;

	var sort$1 = function (array, comparefn) {
	  var length = array.length;

	  if (length < 8) {
	    // insertion sort
	    var i = 1;
	    var element, j;

	    while (i < length) {
	      j = i;
	      element = array[i];
	      while (j && comparefn(array[j - 1], element) > 0) {
	        array[j] = array[--j];
	      }
	      if (j !== i++) array[j] = element;
	    }
	  } else {
	    // merge sort
	    var middle = floor$5(length / 2);
	    var left = sort$1(arraySlice$2(array, 0, middle), comparefn);
	    var right = sort$1(arraySlice$2(array, middle), comparefn);
	    var llength = left.length;
	    var rlength = right.length;
	    var lindex = 0;
	    var rindex = 0;

	    while (lindex < llength || rindex < rlength) {
	      array[lindex + rindex] = (lindex < llength && rindex < rlength)
	        ? comparefn(left[lindex], right[rindex]) <= 0 ? left[lindex++] : right[rindex++]
	        : lindex < llength ? left[lindex++] : right[rindex++];
	    }
	  }

	  return array;
	};

	var arraySort$1 = sort$1;

	var userAgent$6 = environmentUserAgent;

	var firefox = userAgent$6.match(/firefox\/(\d+)/i);

	var environmentFfVersion = !!firefox && +firefox[1];

	var UA = environmentUserAgent;

	var environmentIsIeOrEdge = /MSIE|Trident/.test(UA);

	var userAgent$5 = environmentUserAgent;

	var webkit = userAgent$5.match(/AppleWebKit\/(\d+)\./);

	var environmentWebkitVersion = !!webkit && +webkit[1];

	var $$17 = _export;
	var uncurryThis$C = functionUncurryThis;
	var aCallable$c = aCallable$i;
	var toObject$4 = toObject$g;
	var lengthOfArrayLike$4 = lengthOfArrayLike$h;
	var deletePropertyOrThrow = deletePropertyOrThrow$1;
	var toString$j = toString$o;
	var fails$x = fails$S;
	var internalSort = arraySort$1;
	var arrayMethodIsStrict = arrayMethodIsStrict$2;
	var FF = environmentFfVersion;
	var IE_OR_EDGE = environmentIsIeOrEdge;
	var V8$1 = environmentV8Version;
	var WEBKIT = environmentWebkitVersion;

	var test = [];
	var nativeSort = uncurryThis$C(test.sort);
	var push$8 = uncurryThis$C(test.push);

	// IE8-
	var FAILS_ON_UNDEFINED = fails$x(function () {
	  test.sort(undefined);
	});
	// V8 bug
	var FAILS_ON_NULL = fails$x(function () {
	  test.sort(null);
	});
	// Old WebKit
	var STRICT_METHOD = arrayMethodIsStrict('sort');

	var STABLE_SORT = !fails$x(function () {
	  // feature detection can be too slow, so check engines versions
	  if (V8$1) return V8$1 < 70;
	  if (FF && FF > 3) return;
	  if (IE_OR_EDGE) return true;
	  if (WEBKIT) return WEBKIT < 603;

	  var result = '';
	  var code, chr, value, index;

	  // generate an array with more 512 elements (Chakra and old V8 fails only in this case)
	  for (code = 65; code < 76; code++) {
	    chr = String.fromCharCode(code);

	    switch (code) {
	      case 66: case 69: case 70: case 72: value = 3; break;
	      case 68: case 71: value = 4; break;
	      default: value = 2;
	    }

	    for (index = 0; index < 47; index++) {
	      test.push({ k: chr + index, v: value });
	    }
	  }

	  test.sort(function (a, b) { return b.v - a.v; });

	  for (index = 0; index < test.length; index++) {
	    chr = test[index].k.charAt(0);
	    if (result.charAt(result.length - 1) !== chr) result += chr;
	  }

	  return result !== 'DGBEFHACIJK';
	});

	var FORCED$7 = FAILS_ON_UNDEFINED || !FAILS_ON_NULL || !STRICT_METHOD || !STABLE_SORT;

	var getSortCompare = function (comparefn) {
	  return function (x, y) {
	    if (y === undefined) return -1;
	    if (x === undefined) return 1;
	    if (comparefn !== undefined) return +comparefn(x, y) || 0;
	    return toString$j(x) > toString$j(y) ? 1 : -1;
	  };
	};

	// `Array.prototype.sort` method
	// https://tc39.es/ecma262/#sec-array.prototype.sort
	$$17({ target: 'Array', proto: true, forced: FORCED$7 }, {
	  sort: function sort(comparefn) {
	    if (comparefn !== undefined) aCallable$c(comparefn);

	    var array = toObject$4(this);

	    if (STABLE_SORT) return comparefn === undefined ? nativeSort(array) : nativeSort(array, comparefn);

	    var items = [];
	    var arrayLength = lengthOfArrayLike$4(array);
	    var itemsLength, index;

	    for (index = 0; index < arrayLength; index++) {
	      if (index in array) push$8(items, array[index]);
	    }

	    internalSort(items, getSortCompare(comparefn));

	    itemsLength = lengthOfArrayLike$4(items);
	    index = 0;

	    while (index < itemsLength) array[index] = items[index++];
	    while (index < arrayLength) deletePropertyOrThrow(array, index++);

	    return array;
	  }
	});

	var lengthOfArrayLike$3 = lengthOfArrayLike$h;

	// https://tc39.es/proposal-change-array-by-copy/#sec-array.prototype.toReversed
	// https://tc39.es/proposal-change-array-by-copy/#sec-%typedarray%.prototype.toReversed
	var arrayToReversed$1 = function (O, C) {
	  var len = lengthOfArrayLike$3(O);
	  var A = new C(len);
	  var k = 0;
	  for (; k < len; k++) A[k] = O[len - k - 1];
	  return A;
	};

	var $$16 = _export;
	var arrayToReversed = arrayToReversed$1;
	var toIndexedObject$3 = toIndexedObject$d;
	var addToUnscopables$3 = addToUnscopables$c;

	var $Array$2 = Array;

	// `Array.prototype.toReversed` method
	// https://tc39.es/ecma262/#sec-array.prototype.toreversed
	$$16({ target: 'Array', proto: true }, {
	  toReversed: function toReversed() {
	    return arrayToReversed(toIndexedObject$3(this), $Array$2);
	  }
	});

	addToUnscopables$3('toReversed');

	var lengthOfArrayLike$2 = lengthOfArrayLike$h;

	var arrayFromConstructorAndList$1 = function (Constructor, list, $length) {
	  var index = 0;
	  var length = arguments.length > 2 ? $length : lengthOfArrayLike$2(list);
	  var result = new Constructor(length);
	  while (length > index) result[index] = list[index++];
	  return result;
	};

	var globalThis$q = globalThis_1;

	var getBuiltInPrototypeMethod$1 = function (CONSTRUCTOR, METHOD) {
	  var Constructor = globalThis$q[CONSTRUCTOR];
	  var Prototype = Constructor && Constructor.prototype;
	  return Prototype && Prototype[METHOD];
	};

	var $$15 = _export;
	var uncurryThis$B = functionUncurryThis;
	var aCallable$b = aCallable$i;
	var toIndexedObject$2 = toIndexedObject$d;
	var arrayFromConstructorAndList = arrayFromConstructorAndList$1;
	var getBuiltInPrototypeMethod = getBuiltInPrototypeMethod$1;
	var addToUnscopables$2 = addToUnscopables$c;

	var $Array$1 = Array;
	var sort = uncurryThis$B(getBuiltInPrototypeMethod('Array', 'sort'));

	// `Array.prototype.toSorted` method
	// https://tc39.es/ecma262/#sec-array.prototype.tosorted
	$$15({ target: 'Array', proto: true }, {
	  toSorted: function toSorted(compareFn) {
	    if (compareFn !== undefined) aCallable$b(compareFn);
	    var O = toIndexedObject$2(this);
	    var A = arrayFromConstructorAndList($Array$1, O);
	    return sort(A, compareFn);
	  }
	});

	addToUnscopables$2('toSorted');

	// this method was added to unscopables after implementation
	// in popular engines, so it's moved to a separate module
	var addToUnscopables$1 = addToUnscopables$c;

	// https://tc39.es/ecma262/#sec-array.prototype-@@unscopables
	addToUnscopables$1('flat');

	// this method was added to unscopables after implementation
	// in popular engines, so it's moved to a separate module
	var addToUnscopables = addToUnscopables$c;

	// https://tc39.es/ecma262/#sec-array.prototype-@@unscopables
	addToUnscopables('flatMap');

	var lengthOfArrayLike$1 = lengthOfArrayLike$h;
	var toIntegerOrInfinity$6 = toIntegerOrInfinity$b;

	var $RangeError$5 = RangeError;

	// https://tc39.es/proposal-change-array-by-copy/#sec-array.prototype.with
	// https://tc39.es/proposal-change-array-by-copy/#sec-%typedarray%.prototype.with
	var arrayWith$1 = function (O, C, index, value) {
	  var len = lengthOfArrayLike$1(O);
	  var relativeIndex = toIntegerOrInfinity$6(index);
	  var actualIndex = relativeIndex < 0 ? len + relativeIndex : relativeIndex;
	  if (actualIndex >= len || actualIndex < 0) throw new $RangeError$5('Incorrect index');
	  var A = new C(len);
	  var k = 0;
	  for (; k < len; k++) A[k] = k === actualIndex ? value : O[k];
	  return A;
	};

	var $$14 = _export;
	var arrayWith = arrayWith$1;
	var toIndexedObject$1 = toIndexedObject$d;

	var $Array = Array;

	// `Array.prototype.with` method
	// https://tc39.es/ecma262/#sec-array.prototype.with
	$$14({ target: 'Array', proto: true }, {
	  'with': function (index, value) {
	    return arrayWith(toIndexedObject$1(this), $Array, index, value);
	  }
	});

	var anObject$g = anObject$q;
	var ordinaryToPrimitive = ordinaryToPrimitive$2;

	var $TypeError$a = TypeError;

	// `Date.prototype[@@toPrimitive](hint)` method implementation
	// https://tc39.es/ecma262/#sec-date.prototype-@@toprimitive
	var dateToPrimitive$1 = function (hint) {
	  anObject$g(this);
	  if (hint === 'string' || hint === 'default') hint = 'string';
	  else if (hint !== 'number') throw new $TypeError$a('Incorrect hint');
	  return ordinaryToPrimitive(this, hint);
	};

	var hasOwn$c = hasOwnProperty_1;
	var defineBuiltIn$e = defineBuiltIn$k;
	var dateToPrimitive = dateToPrimitive$1;
	var wellKnownSymbol$e = wellKnownSymbol$w;

	var TO_PRIMITIVE = wellKnownSymbol$e('toPrimitive');
	var DatePrototype = Date.prototype;

	// `Date.prototype[@@toPrimitive]` method
	// https://tc39.es/ecma262/#sec-date.prototype-@@toprimitive
	if (!hasOwn$c(DatePrototype, TO_PRIMITIVE)) {
	  defineBuiltIn$e(DatePrototype, TO_PRIMITIVE, dateToPrimitive);
	}

	var DESCRIPTORS$f = descriptors;
	var FUNCTION_NAME_EXISTS = functionName.EXISTS;
	var uncurryThis$A = functionUncurryThis;
	var defineBuiltInAccessor$8 = defineBuiltInAccessor$b;

	var FunctionPrototype = Function.prototype;
	var functionToString = uncurryThis$A(FunctionPrototype.toString);
	var nameRE = /function\b(?:\s|\/\*[\S\s]*?\*\/|\/\/[^\n\r]*[\n\r]+)*([^\s(/]*)/;
	var regExpExec$2 = uncurryThis$A(nameRE.exec);
	var NAME = 'name';

	// Function instances `.name` property
	// https://tc39.es/ecma262/#sec-function-instances-name
	if (DESCRIPTORS$f && !FUNCTION_NAME_EXISTS) {
	  defineBuiltInAccessor$8(FunctionPrototype, NAME, {
	    configurable: true,
	    get: function () {
	      try {
	        return regExpExec$2(nameRE, functionToString(this))[1];
	      } catch (error) {
	        return '';
	      }
	    }
	  });
	}

	var $$13 = _export;
	var globalThis$p = globalThis_1;

	// `globalThis` object
	// https://tc39.es/ecma262/#sec-globalthis
	$$13({ global: true, forced: globalThis$p.globalThis !== globalThis$p }, {
	  globalThis: globalThis$p
	});

	var isPrototypeOf$2 = objectIsPrototypeOf;

	var $TypeError$9 = TypeError;

	var anInstance$9 = function (it, Prototype) {
	  if (isPrototypeOf$2(Prototype, it)) return it;
	  throw new $TypeError$9('Incorrect invocation');
	};

	var $$12 = _export;
	var globalThis$o = globalThis_1;
	var anInstance$8 = anInstance$9;
	var anObject$f = anObject$q;
	var isCallable$c = isCallable$w;
	var getPrototypeOf = objectGetPrototypeOf$1;
	var defineBuiltInAccessor$7 = defineBuiltInAccessor$b;
	var createProperty$2 = createProperty$6;
	var fails$w = fails$S;
	var hasOwn$b = hasOwnProperty_1;
	var wellKnownSymbol$d = wellKnownSymbol$w;
	var IteratorPrototype$1 = iteratorsCore.IteratorPrototype;
	var DESCRIPTORS$e = descriptors;

	var CONSTRUCTOR = 'constructor';
	var ITERATOR$4 = 'Iterator';
	var TO_STRING_TAG$1 = wellKnownSymbol$d('toStringTag');

	var $TypeError$8 = TypeError;
	var NativeIterator = globalThis$o[ITERATOR$4];

	// FF56- have non-standard global helper `Iterator`
	var FORCED$6 = !isCallable$c(NativeIterator)
	  || NativeIterator.prototype !== IteratorPrototype$1
	  // FF44- non-standard `Iterator` passes previous tests
	  || !fails$w(function () { NativeIterator({}); });

	var IteratorConstructor = function Iterator() {
	  anInstance$8(this, IteratorPrototype$1);
	  if (getPrototypeOf(this) === IteratorPrototype$1) throw new $TypeError$8('Abstract class Iterator not directly constructable');
	};

	var defineIteratorPrototypeAccessor = function (key, value) {
	  if (DESCRIPTORS$e) {
	    defineBuiltInAccessor$7(IteratorPrototype$1, key, {
	      configurable: true,
	      get: function () {
	        return value;
	      },
	      set: function (replacement) {
	        anObject$f(this);
	        if (this === IteratorPrototype$1) throw new $TypeError$8("You can't redefine this property");
	        if (hasOwn$b(this, key)) this[key] = replacement;
	        else createProperty$2(this, key, replacement);
	      }
	    });
	  } else IteratorPrototype$1[key] = value;
	};

	if (!hasOwn$b(IteratorPrototype$1, TO_STRING_TAG$1)) defineIteratorPrototypeAccessor(TO_STRING_TAG$1, ITERATOR$4);

	if (FORCED$6 || !hasOwn$b(IteratorPrototype$1, CONSTRUCTOR) || IteratorPrototype$1[CONSTRUCTOR] === Object) {
	  defineIteratorPrototypeAccessor(CONSTRUCTOR, IteratorConstructor);
	}

	IteratorConstructor.prototype = IteratorPrototype$1;

	// `Iterator` constructor
	// https://tc39.es/ecma262/#sec-iterator
	$$12({ global: true, constructor: true, forced: FORCED$6 }, {
	  Iterator: IteratorConstructor
	});

	// `GetIteratorDirect(obj)` abstract operation
	// https://tc39.es/proposal-iterator-helpers/#sec-getiteratordirect
	var getIteratorDirect$2 = function (obj) {
	  return {
	    iterator: obj,
	    next: obj.next,
	    done: false
	  };
	};

	var defineBuiltIn$d = defineBuiltIn$k;

	var defineBuiltIns$5 = function (target, src, options) {
	  for (var key in src) defineBuiltIn$d(target, key, src[key], options);
	  return target;
	};

	var call$j = functionCall;
	var create$5 = objectCreate;
	var createNonEnumerableProperty$3 = createNonEnumerableProperty$b;
	var defineBuiltIns$4 = defineBuiltIns$5;
	var wellKnownSymbol$c = wellKnownSymbol$w;
	var InternalStateModule$8 = internalState;
	var getMethod$3 = getMethod$7;
	var IteratorPrototype = iteratorsCore.IteratorPrototype;
	var createIterResultObject$4 = createIterResultObject$6;
	var iteratorClose$2 = iteratorClose$5;

	var TO_STRING_TAG = wellKnownSymbol$c('toStringTag');
	var ITERATOR_HELPER = 'IteratorHelper';
	var WRAP_FOR_VALID_ITERATOR = 'WrapForValidIterator';
	var setInternalState$8 = InternalStateModule$8.set;

	var createIteratorProxyPrototype = function (IS_ITERATOR) {
	  var getInternalState = InternalStateModule$8.getterFor(IS_ITERATOR ? WRAP_FOR_VALID_ITERATOR : ITERATOR_HELPER);

	  return defineBuiltIns$4(create$5(IteratorPrototype), {
	    next: function next() {
	      var state = getInternalState(this);
	      // for simplification:
	      //   for `%WrapForValidIteratorPrototype%.next` our `nextHandler` returns `IterResultObject`
	      //   for `%IteratorHelperPrototype%.next` - just a value
	      if (IS_ITERATOR) return state.nextHandler();
	      try {
	        var result = state.done ? undefined : state.nextHandler();
	        return createIterResultObject$4(result, state.done);
	      } catch (error) {
	        state.done = true;
	        throw error;
	      }
	    },
	    'return': function () {
	      var state = getInternalState(this);
	      var iterator = state.iterator;
	      state.done = true;
	      if (IS_ITERATOR) {
	        var returnMethod = getMethod$3(iterator, 'return');
	        return returnMethod ? call$j(returnMethod, iterator) : createIterResultObject$4(undefined, true);
	      }
	      if (state.inner) try {
	        iteratorClose$2(state.inner.iterator, 'normal');
	      } catch (error) {
	        return iteratorClose$2(iterator, 'throw', error);
	      }
	      if (iterator) iteratorClose$2(iterator, 'normal');
	      return createIterResultObject$4(undefined, true);
	    }
	  });
	};

	var WrapForValidIteratorPrototype = createIteratorProxyPrototype(true);
	var IteratorHelperPrototype = createIteratorProxyPrototype(false);

	createNonEnumerableProperty$3(IteratorHelperPrototype, TO_STRING_TAG, 'Iterator Helper');

	var iteratorCreateProxy = function (nextHandler, IS_ITERATOR) {
	  var IteratorProxy = function Iterator(record, state) {
	    if (state) {
	      state.iterator = record.iterator;
	      state.next = record.next;
	    } else state = record;
	    state.type = IS_ITERATOR ? WRAP_FOR_VALID_ITERATOR : ITERATOR_HELPER;
	    state.nextHandler = nextHandler;
	    state.counter = 0;
	    state.done = false;
	    setInternalState$8(this, state);
	  };

	  IteratorProxy.prototype = IS_ITERATOR ? WrapForValidIteratorPrototype : IteratorHelperPrototype;

	  return IteratorProxy;
	};

	var call$i = functionCall;
	var aCallable$a = aCallable$i;
	var anObject$e = anObject$q;
	var getIteratorDirect$1 = getIteratorDirect$2;
	var createIteratorProxy = iteratorCreateProxy;
	var callWithSafeIterationClosing = callWithSafeIterationClosing$2;

	var IteratorProxy = createIteratorProxy(function () {
	  var iterator = this.iterator;
	  var result = anObject$e(call$i(this.next, iterator));
	  var done = this.done = !!result.done;
	  if (!done) return callWithSafeIterationClosing(iterator, this.mapper, [result.value, this.counter++], true);
	});

	// `Iterator.prototype.map` method
	// https://github.com/tc39/proposal-iterator-helpers
	var iteratorMap = function map(mapper) {
	  anObject$e(this);
	  aCallable$a(mapper);
	  return new IteratorProxy(getIteratorDirect$1(this), {
	    mapper: mapper
	  });
	};

	var $$11 = _export;
	var map = iteratorMap;
	var IS_PURE$2 = isPure;

	// `Iterator.prototype.map` method
	// https://tc39.es/ecma262/#sec-iterator.prototype.map
	$$11({ target: 'Iterator', proto: true, real: true, forced: IS_PURE$2 }, {
	  map: map
	});

	var internalMetadata = {exports: {}};

	// FF26- bug: ArrayBuffers are non-extensible, but Object.isExtensible does not report it
	var fails$v = fails$S;

	var arrayBufferNonExtensible = fails$v(function () {
	  if (typeof ArrayBuffer == 'function') {
	    var buffer = new ArrayBuffer(8);
	    // eslint-disable-next-line es/no-object-isextensible, es/no-object-defineproperty -- safe
	    if (Object.isExtensible(buffer)) Object.defineProperty(buffer, 'a', { value: 8 });
	  }
	});

	var fails$u = fails$S;
	var isObject$a = isObject$n;
	var classof$6 = classofRaw$2;
	var ARRAY_BUFFER_NON_EXTENSIBLE = arrayBufferNonExtensible;

	// eslint-disable-next-line es/no-object-isextensible -- safe
	var $isExtensible = Object.isExtensible;
	var FAILS_ON_PRIMITIVES$3 = fails$u(function () { $isExtensible(1); });

	// `Object.isExtensible` method
	// https://tc39.es/ecma262/#sec-object.isextensible
	var objectIsExtensible = (FAILS_ON_PRIMITIVES$3 || ARRAY_BUFFER_NON_EXTENSIBLE) ? function isExtensible(it) {
	  if (!isObject$a(it)) return false;
	  if (ARRAY_BUFFER_NON_EXTENSIBLE && classof$6(it) === 'ArrayBuffer') return false;
	  return $isExtensible ? $isExtensible(it) : true;
	} : $isExtensible;

	var fails$t = fails$S;

	var freezing = !fails$t(function () {
	  // eslint-disable-next-line es/no-object-isextensible, es/no-object-preventextensions -- required for testing
	  return Object.isExtensible(Object.preventExtensions({}));
	});

	var $$10 = _export;
	var uncurryThis$z = functionUncurryThis;
	var hiddenKeys = hiddenKeys$6;
	var isObject$9 = isObject$n;
	var hasOwn$a = hasOwnProperty_1;
	var defineProperty$4 = objectDefineProperty.f;
	var getOwnPropertyNamesModule = objectGetOwnPropertyNames;
	var getOwnPropertyNamesExternalModule = objectGetOwnPropertyNamesExternal;
	var isExtensible$1 = objectIsExtensible;
	var uid$1 = uid$5;
	var FREEZING$1 = freezing;

	var REQUIRED = false;
	var METADATA = uid$1('meta');
	var id$1 = 0;

	var setMetadata = function (it) {
	  defineProperty$4(it, METADATA, { value: {
	    objectID: 'O' + id$1++, // object ID
	    weakData: {}          // weak collections IDs
	  } });
	};

	var fastKey$1 = function (it, create) {
	  // return a primitive with prefix
	  if (!isObject$9(it)) return typeof it == 'symbol' ? it : (typeof it == 'string' ? 'S' : 'P') + it;
	  if (!hasOwn$a(it, METADATA)) {
	    // can't set metadata to uncaught frozen object
	    if (!isExtensible$1(it)) return 'F';
	    // not necessary to add metadata
	    if (!create) return 'E';
	    // add missing metadata
	    setMetadata(it);
	  // return object ID
	  } return it[METADATA].objectID;
	};

	var getWeakData$1 = function (it, create) {
	  if (!hasOwn$a(it, METADATA)) {
	    // can't set metadata to uncaught frozen object
	    if (!isExtensible$1(it)) return true;
	    // not necessary to add metadata
	    if (!create) return false;
	    // add missing metadata
	    setMetadata(it);
	  // return the store of weak collections IDs
	  } return it[METADATA].weakData;
	};

	// add metadata on freeze-family methods calling
	var onFreeze = function (it) {
	  if (FREEZING$1 && REQUIRED && isExtensible$1(it) && !hasOwn$a(it, METADATA)) setMetadata(it);
	  return it;
	};

	var enable = function () {
	  meta.enable = function () { /* empty */ };
	  REQUIRED = true;
	  var getOwnPropertyNames = getOwnPropertyNamesModule.f;
	  var splice = uncurryThis$z([].splice);
	  var test = {};
	  test[METADATA] = 1;

	  // prevent exposing of metadata key
	  if (getOwnPropertyNames(test).length) {
	    getOwnPropertyNamesModule.f = function (it) {
	      var result = getOwnPropertyNames(it);
	      for (var i = 0, length = result.length; i < length; i++) {
	        if (result[i] === METADATA) {
	          splice(result, i, 1);
	          break;
	        }
	      } return result;
	    };

	    $$10({ target: 'Object', stat: true, forced: true }, {
	      getOwnPropertyNames: getOwnPropertyNamesExternalModule.f
	    });
	  }
	};

	var meta = internalMetadata.exports = {
	  enable: enable,
	  fastKey: fastKey$1,
	  getWeakData: getWeakData$1,
	  onFreeze: onFreeze
	};

	hiddenKeys[METADATA] = true;

	var internalMetadataExports = internalMetadata.exports;

	var $$$ = _export;
	var globalThis$n = globalThis_1;
	var uncurryThis$y = functionUncurryThis;
	var isForced$2 = isForced_1;
	var defineBuiltIn$c = defineBuiltIn$k;
	var InternalMetadataModule$1 = internalMetadataExports;
	var iterate$b = iterate$d;
	var anInstance$7 = anInstance$9;
	var isCallable$b = isCallable$w;
	var isNullOrUndefined$7 = isNullOrUndefined$b;
	var isObject$8 = isObject$n;
	var fails$s = fails$S;
	var checkCorrectnessOfIteration$1 = checkCorrectnessOfIteration$3;
	var setToStringTag$5 = setToStringTag$9;
	var inheritIfRequired$2 = inheritIfRequired$4;

	var collection$3 = function (CONSTRUCTOR_NAME, wrapper, common) {
	  var IS_MAP = CONSTRUCTOR_NAME.indexOf('Map') !== -1;
	  var IS_WEAK = CONSTRUCTOR_NAME.indexOf('Weak') !== -1;
	  var ADDER = IS_MAP ? 'set' : 'add';
	  var NativeConstructor = globalThis$n[CONSTRUCTOR_NAME];
	  var NativePrototype = NativeConstructor && NativeConstructor.prototype;
	  var Constructor = NativeConstructor;
	  var exported = {};

	  var fixMethod = function (KEY) {
	    var uncurriedNativeMethod = uncurryThis$y(NativePrototype[KEY]);
	    defineBuiltIn$c(NativePrototype, KEY,
	      KEY === 'add' ? function add(value) {
	        uncurriedNativeMethod(this, value === 0 ? 0 : value);
	        return this;
	      } : KEY === 'delete' ? function (key) {
	        return IS_WEAK && !isObject$8(key) ? false : uncurriedNativeMethod(this, key === 0 ? 0 : key);
	      } : KEY === 'get' ? function get(key) {
	        return IS_WEAK && !isObject$8(key) ? undefined : uncurriedNativeMethod(this, key === 0 ? 0 : key);
	      } : KEY === 'has' ? function has(key) {
	        return IS_WEAK && !isObject$8(key) ? false : uncurriedNativeMethod(this, key === 0 ? 0 : key);
	      } : function set(key, value) {
	        uncurriedNativeMethod(this, key === 0 ? 0 : key, value);
	        return this;
	      }
	    );
	  };

	  var REPLACE = isForced$2(
	    CONSTRUCTOR_NAME,
	    !isCallable$b(NativeConstructor) || !(IS_WEAK || NativePrototype.forEach && !fails$s(function () {
	      new NativeConstructor().entries().next();
	    }))
	  );

	  if (REPLACE) {
	    // create collection constructor
	    Constructor = common.getConstructor(wrapper, CONSTRUCTOR_NAME, IS_MAP, ADDER);
	    InternalMetadataModule$1.enable();
	  } else if (isForced$2(CONSTRUCTOR_NAME, true)) {
	    var instance = new Constructor();
	    // early implementations not supports chaining
	    var HASNT_CHAINING = instance[ADDER](IS_WEAK ? {} : -0, 1) !== instance;
	    // V8 ~ Chromium 40- weak-collections throws on primitives, but should return false
	    var THROWS_ON_PRIMITIVES = fails$s(function () { instance.has(1); });
	    // most early implementations doesn't supports iterables, most modern - not close it correctly
	    // eslint-disable-next-line no-new -- required for testing
	    var ACCEPT_ITERABLES = checkCorrectnessOfIteration$1(function (iterable) { new NativeConstructor(iterable); });
	    // for early implementations -0 and +0 not the same
	    var BUGGY_ZERO = !IS_WEAK && fails$s(function () {
	      // V8 ~ Chromium 42- fails only with 5+ elements
	      var $instance = new NativeConstructor();
	      var index = 5;
	      while (index--) $instance[ADDER](index, index);
	      return !$instance.has(-0);
	    });

	    if (!ACCEPT_ITERABLES) {
	      Constructor = wrapper(function (dummy, iterable) {
	        anInstance$7(dummy, NativePrototype);
	        var that = inheritIfRequired$2(new NativeConstructor(), dummy, Constructor);
	        if (!isNullOrUndefined$7(iterable)) iterate$b(iterable, that[ADDER], { that: that, AS_ENTRIES: IS_MAP });
	        return that;
	      });
	      Constructor.prototype = NativePrototype;
	      NativePrototype.constructor = Constructor;
	    }

	    if (THROWS_ON_PRIMITIVES || BUGGY_ZERO) {
	      fixMethod('delete');
	      fixMethod('has');
	      IS_MAP && fixMethod('get');
	    }

	    if (BUGGY_ZERO || HASNT_CHAINING) fixMethod(ADDER);

	    // weak collections should not contains .clear method
	    if (IS_WEAK && NativePrototype.clear) delete NativePrototype.clear;
	  }

	  exported[CONSTRUCTOR_NAME] = Constructor;
	  $$$({ global: true, constructor: true, forced: Constructor !== NativeConstructor }, exported);

	  setToStringTag$5(Constructor, CONSTRUCTOR_NAME);

	  if (!IS_WEAK) common.setStrong(Constructor, CONSTRUCTOR_NAME, IS_MAP);

	  return Constructor;
	};

	var getBuiltIn$b = getBuiltIn$k;
	var defineBuiltInAccessor$6 = defineBuiltInAccessor$b;
	var wellKnownSymbol$b = wellKnownSymbol$w;
	var DESCRIPTORS$d = descriptors;

	var SPECIES$3 = wellKnownSymbol$b('species');

	var setSpecies$2 = function (CONSTRUCTOR_NAME) {
	  var Constructor = getBuiltIn$b(CONSTRUCTOR_NAME);

	  if (DESCRIPTORS$d && Constructor && !Constructor[SPECIES$3]) {
	    defineBuiltInAccessor$6(Constructor, SPECIES$3, {
	      configurable: true,
	      get: function () { return this; }
	    });
	  }
	};

	var create$4 = objectCreate;
	var defineBuiltInAccessor$5 = defineBuiltInAccessor$b;
	var defineBuiltIns$3 = defineBuiltIns$5;
	var bind$5 = functionBindContext;
	var anInstance$6 = anInstance$9;
	var isNullOrUndefined$6 = isNullOrUndefined$b;
	var iterate$a = iterate$d;
	var defineIterator$1 = iteratorDefine;
	var createIterResultObject$3 = createIterResultObject$6;
	var setSpecies$1 = setSpecies$2;
	var DESCRIPTORS$c = descriptors;
	var fastKey = internalMetadataExports.fastKey;
	var InternalStateModule$7 = internalState;

	var setInternalState$7 = InternalStateModule$7.set;
	var internalStateGetterFor$1 = InternalStateModule$7.getterFor;

	var collectionStrong$2 = {
	  getConstructor: function (wrapper, CONSTRUCTOR_NAME, IS_MAP, ADDER) {
	    var Constructor = wrapper(function (that, iterable) {
	      anInstance$6(that, Prototype);
	      setInternalState$7(that, {
	        type: CONSTRUCTOR_NAME,
	        index: create$4(null),
	        first: null,
	        last: null,
	        size: 0
	      });
	      if (!DESCRIPTORS$c) that.size = 0;
	      if (!isNullOrUndefined$6(iterable)) iterate$a(iterable, that[ADDER], { that: that, AS_ENTRIES: IS_MAP });
	    });

	    var Prototype = Constructor.prototype;

	    var getInternalState = internalStateGetterFor$1(CONSTRUCTOR_NAME);

	    var define = function (that, key, value) {
	      var state = getInternalState(that);
	      var entry = getEntry(that, key);
	      var previous, index;
	      // change existing entry
	      if (entry) {
	        entry.value = value;
	      // create new entry
	      } else {
	        state.last = entry = {
	          index: index = fastKey(key, true),
	          key: key,
	          value: value,
	          previous: previous = state.last,
	          next: null,
	          removed: false
	        };
	        if (!state.first) state.first = entry;
	        if (previous) previous.next = entry;
	        if (DESCRIPTORS$c) state.size++;
	        else that.size++;
	        // add to index
	        if (index !== 'F') state.index[index] = entry;
	      } return that;
	    };

	    var getEntry = function (that, key) {
	      var state = getInternalState(that);
	      // fast case
	      var index = fastKey(key);
	      var entry;
	      if (index !== 'F') return state.index[index];
	      // frozen object case
	      for (entry = state.first; entry; entry = entry.next) {
	        if (entry.key === key) return entry;
	      }
	    };

	    defineBuiltIns$3(Prototype, {
	      // `{ Map, Set }.prototype.clear()` methods
	      // https://tc39.es/ecma262/#sec-map.prototype.clear
	      // https://tc39.es/ecma262/#sec-set.prototype.clear
	      clear: function clear() {
	        var that = this;
	        var state = getInternalState(that);
	        var entry = state.first;
	        while (entry) {
	          entry.removed = true;
	          if (entry.previous) entry.previous = entry.previous.next = null;
	          entry = entry.next;
	        }
	        state.first = state.last = null;
	        state.index = create$4(null);
	        if (DESCRIPTORS$c) state.size = 0;
	        else that.size = 0;
	      },
	      // `{ Map, Set }.prototype.delete(key)` methods
	      // https://tc39.es/ecma262/#sec-map.prototype.delete
	      // https://tc39.es/ecma262/#sec-set.prototype.delete
	      'delete': function (key) {
	        var that = this;
	        var state = getInternalState(that);
	        var entry = getEntry(that, key);
	        if (entry) {
	          var next = entry.next;
	          var prev = entry.previous;
	          delete state.index[entry.index];
	          entry.removed = true;
	          if (prev) prev.next = next;
	          if (next) next.previous = prev;
	          if (state.first === entry) state.first = next;
	          if (state.last === entry) state.last = prev;
	          if (DESCRIPTORS$c) state.size--;
	          else that.size--;
	        } return !!entry;
	      },
	      // `{ Map, Set }.prototype.forEach(callbackfn, thisArg = undefined)` methods
	      // https://tc39.es/ecma262/#sec-map.prototype.foreach
	      // https://tc39.es/ecma262/#sec-set.prototype.foreach
	      forEach: function forEach(callbackfn /* , that = undefined */) {
	        var state = getInternalState(this);
	        var boundFunction = bind$5(callbackfn, arguments.length > 1 ? arguments[1] : undefined);
	        var entry;
	        while (entry = entry ? entry.next : state.first) {
	          boundFunction(entry.value, entry.key, this);
	          // revert to the last existing entry
	          while (entry && entry.removed) entry = entry.previous;
	        }
	      },
	      // `{ Map, Set}.prototype.has(key)` methods
	      // https://tc39.es/ecma262/#sec-map.prototype.has
	      // https://tc39.es/ecma262/#sec-set.prototype.has
	      has: function has(key) {
	        return !!getEntry(this, key);
	      }
	    });

	    defineBuiltIns$3(Prototype, IS_MAP ? {
	      // `Map.prototype.get(key)` method
	      // https://tc39.es/ecma262/#sec-map.prototype.get
	      get: function get(key) {
	        var entry = getEntry(this, key);
	        return entry && entry.value;
	      },
	      // `Map.prototype.set(key, value)` method
	      // https://tc39.es/ecma262/#sec-map.prototype.set
	      set: function set(key, value) {
	        return define(this, key === 0 ? 0 : key, value);
	      }
	    } : {
	      // `Set.prototype.add(value)` method
	      // https://tc39.es/ecma262/#sec-set.prototype.add
	      add: function add(value) {
	        return define(this, value = value === 0 ? 0 : value, value);
	      }
	    });
	    if (DESCRIPTORS$c) defineBuiltInAccessor$5(Prototype, 'size', {
	      configurable: true,
	      get: function () {
	        return getInternalState(this).size;
	      }
	    });
	    return Constructor;
	  },
	  setStrong: function (Constructor, CONSTRUCTOR_NAME, IS_MAP) {
	    var ITERATOR_NAME = CONSTRUCTOR_NAME + ' Iterator';
	    var getInternalCollectionState = internalStateGetterFor$1(CONSTRUCTOR_NAME);
	    var getInternalIteratorState = internalStateGetterFor$1(ITERATOR_NAME);
	    // `{ Map, Set }.prototype.{ keys, values, entries, @@iterator }()` methods
	    // https://tc39.es/ecma262/#sec-map.prototype.entries
	    // https://tc39.es/ecma262/#sec-map.prototype.keys
	    // https://tc39.es/ecma262/#sec-map.prototype.values
	    // https://tc39.es/ecma262/#sec-map.prototype-@@iterator
	    // https://tc39.es/ecma262/#sec-set.prototype.entries
	    // https://tc39.es/ecma262/#sec-set.prototype.keys
	    // https://tc39.es/ecma262/#sec-set.prototype.values
	    // https://tc39.es/ecma262/#sec-set.prototype-@@iterator
	    defineIterator$1(Constructor, CONSTRUCTOR_NAME, function (iterated, kind) {
	      setInternalState$7(this, {
	        type: ITERATOR_NAME,
	        target: iterated,
	        state: getInternalCollectionState(iterated),
	        kind: kind,
	        last: null
	      });
	    }, function () {
	      var state = getInternalIteratorState(this);
	      var kind = state.kind;
	      var entry = state.last;
	      // revert to the last existing entry
	      while (entry && entry.removed) entry = entry.previous;
	      // get next entry
	      if (!state.target || !(state.last = entry = entry ? entry.next : state.state.first)) {
	        // or finish the iteration
	        state.target = null;
	        return createIterResultObject$3(undefined, true);
	      }
	      // return step by kind
	      if (kind === 'keys') return createIterResultObject$3(entry.key, false);
	      if (kind === 'values') return createIterResultObject$3(entry.value, false);
	      return createIterResultObject$3([entry.key, entry.value], false);
	    }, IS_MAP ? 'entries' : 'values', !IS_MAP, true);

	    // `{ Map, Set }.prototype[@@species]` accessors
	    // https://tc39.es/ecma262/#sec-get-map-@@species
	    // https://tc39.es/ecma262/#sec-get-set-@@species
	    setSpecies$1(CONSTRUCTOR_NAME);
	  }
	};

	var collection$2 = collection$3;
	var collectionStrong$1 = collectionStrong$2;

	// `Map` constructor
	// https://tc39.es/ecma262/#sec-map-objects
	collection$2('Map', function (init) {
	  return function Map() { return init(this, arguments.length ? arguments[0] : undefined); };
	}, collectionStrong$1);

	var $$_ = _export;

	// eslint-disable-next-line es/no-math-hypot -- required for testing
	var $hypot = Math.hypot;
	var abs = Math.abs;
	var sqrt = Math.sqrt;

	// Chrome 77 bug
	// https://bugs.chromium.org/p/v8/issues/detail?id=9546
	var FORCED$5 = !!$hypot && $hypot(Infinity, NaN) !== Infinity;

	// `Math.hypot` method
	// https://tc39.es/ecma262/#sec-math.hypot
	$$_({ target: 'Math', stat: true, arity: 2, forced: FORCED$5 }, {
	  // eslint-disable-next-line no-unused-vars -- required for `.length`
	  hypot: function hypot(value1, value2) {
	    var sum = 0;
	    var i = 0;
	    var aLen = arguments.length;
	    var larg = 0;
	    var arg, div;
	    while (i < aLen) {
	      arg = abs(arguments[i++]);
	      if (larg < arg) {
	        div = larg / arg;
	        sum = sum * div * div + 1;
	        larg = arg;
	      } else if (arg > 0) {
	        div = arg / larg;
	        sum += div * div;
	      } else sum += arg;
	    }
	    return larg === Infinity ? Infinity : larg * sqrt(sum);
	  }
	});

	// `Math.sign` method implementation
	// https://tc39.es/ecma262/#sec-math.sign
	// eslint-disable-next-line es/no-math-sign -- safe
	var mathSign = Math.sign || function sign(x) {
	  var n = +x;
	  // eslint-disable-next-line no-self-compare -- NaN check
	  return n === 0 || n !== n ? n : n < 0 ? -1 : 1;
	};

	var $$Z = _export;
	var sign = mathSign;

	// `Math.sign` method
	// https://tc39.es/ecma262/#sec-math.sign
	$$Z({ target: 'Math', stat: true }, {
	  sign: sign
	});

	var $$Y = _export;
	var trunc = mathTrunc;

	// `Math.trunc` method
	// https://tc39.es/ecma262/#sec-math.trunc
	$$Y({ target: 'Math', stat: true }, {
	  trunc: trunc
	});

	var uncurryThis$x = functionUncurryThis;

	// `thisNumberValue` abstract operation
	// https://tc39.es/ecma262/#sec-thisnumbervalue
	var thisNumberValue$3 = uncurryThis$x(1.0.valueOf);

	// a string of all valid unicode whitespaces
	var whitespaces$3 = '\u0009\u000A\u000B\u000C\u000D\u0020\u00A0\u1680\u2000\u2001\u2002' +
	  '\u2003\u2004\u2005\u2006\u2007\u2008\u2009\u200A\u202F\u205F\u3000\u2028\u2029\uFEFF';

	var uncurryThis$w = functionUncurryThis;
	var requireObjectCoercible$c = requireObjectCoercible$g;
	var toString$i = toString$o;
	var whitespaces$2 = whitespaces$3;

	var replace$5 = uncurryThis$w(''.replace);
	var ltrim = RegExp('^[' + whitespaces$2 + ']+');
	var rtrim = RegExp('(^|[^' + whitespaces$2 + '])[' + whitespaces$2 + ']+$');

	// `String.prototype.{ trim, trimStart, trimEnd, trimLeft, trimRight }` methods implementation
	var createMethod$3 = function (TYPE) {
	  return function ($this) {
	    var string = toString$i(requireObjectCoercible$c($this));
	    if (TYPE & 1) string = replace$5(string, ltrim, '');
	    if (TYPE & 2) string = replace$5(string, rtrim, '$1');
	    return string;
	  };
	};

	var stringTrim = {
	  // `String.prototype.{ trimLeft, trimStart }` methods
	  // https://tc39.es/ecma262/#sec-string.prototype.trimstart
	  start: createMethod$3(1),
	  // `String.prototype.{ trimRight, trimEnd }` methods
	  // https://tc39.es/ecma262/#sec-string.prototype.trimend
	  end: createMethod$3(2),
	  // `String.prototype.trim` method
	  // https://tc39.es/ecma262/#sec-string.prototype.trim
	  trim: createMethod$3(3)
	};

	var $$X = _export;
	var IS_PURE$1 = isPure;
	var DESCRIPTORS$b = descriptors;
	var globalThis$m = globalThis_1;
	var path = path$2;
	var uncurryThis$v = functionUncurryThis;
	var isForced$1 = isForced_1;
	var hasOwn$9 = hasOwnProperty_1;
	var inheritIfRequired$1 = inheritIfRequired$4;
	var isPrototypeOf$1 = objectIsPrototypeOf;
	var isSymbol$1 = isSymbol$6;
	var toPrimitive = toPrimitive$2;
	var fails$r = fails$S;
	var getOwnPropertyNames$1 = objectGetOwnPropertyNames.f;
	var getOwnPropertyDescriptor$3 = objectGetOwnPropertyDescriptor.f;
	var defineProperty$3 = objectDefineProperty.f;
	var thisNumberValue$2 = thisNumberValue$3;
	var trim$1 = stringTrim.trim;

	var NUMBER = 'Number';
	var NativeNumber = globalThis$m[NUMBER];
	path[NUMBER];
	var NumberPrototype = NativeNumber.prototype;
	var TypeError$5 = globalThis$m.TypeError;
	var stringSlice$a = uncurryThis$v(''.slice);
	var charCodeAt$3 = uncurryThis$v(''.charCodeAt);

	// `ToNumeric` abstract operation
	// https://tc39.es/ecma262/#sec-tonumeric
	var toNumeric = function (value) {
	  var primValue = toPrimitive(value, 'number');
	  return typeof primValue == 'bigint' ? primValue : toNumber(primValue);
	};

	// `ToNumber` abstract operation
	// https://tc39.es/ecma262/#sec-tonumber
	var toNumber = function (argument) {
	  var it = toPrimitive(argument, 'number');
	  var first, third, radix, maxCode, digits, length, index, code;
	  if (isSymbol$1(it)) throw new TypeError$5('Cannot convert a Symbol value to a number');
	  if (typeof it == 'string' && it.length > 2) {
	    it = trim$1(it);
	    first = charCodeAt$3(it, 0);
	    if (first === 43 || first === 45) {
	      third = charCodeAt$3(it, 2);
	      if (third === 88 || third === 120) return NaN; // Number('+0x1') should be NaN, old V8 fix
	    } else if (first === 48) {
	      switch (charCodeAt$3(it, 1)) {
	        // fast equal of /^0b[01]+$/i
	        case 66:
	        case 98:
	          radix = 2;
	          maxCode = 49;
	          break;
	        // fast equal of /^0o[0-7]+$/i
	        case 79:
	        case 111:
	          radix = 8;
	          maxCode = 55;
	          break;
	        default:
	          return +it;
	      }
	      digits = stringSlice$a(it, 2);
	      length = digits.length;
	      for (index = 0; index < length; index++) {
	        code = charCodeAt$3(digits, index);
	        // parseInt parses a string to a first unavailable symbol
	        // but ToNumber should return NaN if a string contains unavailable symbols
	        if (code < 48 || code > maxCode) return NaN;
	      } return parseInt(digits, radix);
	    }
	  } return +it;
	};

	var FORCED$4 = isForced$1(NUMBER, !NativeNumber(' 0o1') || !NativeNumber('0b1') || NativeNumber('+0x1'));

	var calledWithNew = function (dummy) {
	  // includes check on 1..constructor(foo) case
	  return isPrototypeOf$1(NumberPrototype, dummy) && fails$r(function () { thisNumberValue$2(dummy); });
	};

	// `Number` constructor
	// https://tc39.es/ecma262/#sec-number-constructor
	var NumberWrapper = function Number(value) {
	  var n = arguments.length < 1 ? 0 : NativeNumber(toNumeric(value));
	  return calledWithNew(this) ? inheritIfRequired$1(Object(n), this, NumberWrapper) : n;
	};

	NumberWrapper.prototype = NumberPrototype;
	if (FORCED$4 && !IS_PURE$1) NumberPrototype.constructor = NumberWrapper;

	$$X({ global: true, constructor: true, wrap: true, forced: FORCED$4 }, {
	  Number: NumberWrapper
	});

	// Use `internal/copy-constructor-properties` helper in `core-js@4`
	var copyConstructorProperties = function (target, source) {
	  for (var keys = DESCRIPTORS$b ? getOwnPropertyNames$1(source) : (
	    // ES3:
	    'MAX_VALUE,MIN_VALUE,NaN,NEGATIVE_INFINITY,POSITIVE_INFINITY,' +
	    // ES2015 (in case, if modules with ES2015 Number statics required before):
	    'EPSILON,MAX_SAFE_INTEGER,MIN_SAFE_INTEGER,isFinite,isInteger,isNaN,isSafeInteger,parseFloat,parseInt,' +
	    // ESNext
	    'fromString,range'
	  ).split(','), j = 0, key; keys.length > j; j++) {
	    if (hasOwn$9(source, key = keys[j]) && !hasOwn$9(target, key)) {
	      defineProperty$3(target, key, getOwnPropertyDescriptor$3(source, key));
	    }
	  }
	};
	if (FORCED$4 || IS_PURE$1) copyConstructorProperties(path[NUMBER], NativeNumber);

	var $$W = _export;

	// `Number.EPSILON` constant
	// https://tc39.es/ecma262/#sec-number.epsilon
	$$W({ target: 'Number', stat: true, nonConfigurable: true, nonWritable: true }, {
	  EPSILON: Math.pow(2, -52)
	});

	var isObject$7 = isObject$n;

	var floor$4 = Math.floor;

	// `IsIntegralNumber` abstract operation
	// https://tc39.es/ecma262/#sec-isintegralnumber
	// eslint-disable-next-line es/no-number-isinteger -- safe
	var isIntegralNumber$1 = Number.isInteger || function isInteger(it) {
	  return !isObject$7(it) && isFinite(it) && floor$4(it) === it;
	};

	var $$V = _export;
	var isIntegralNumber = isIntegralNumber$1;

	// `Number.isInteger` method
	// https://tc39.es/ecma262/#sec-number.isinteger
	$$V({ target: 'Number', stat: true }, {
	  isInteger: isIntegralNumber
	});

	var globalThis$l = globalThis_1;
	var fails$q = fails$S;
	var uncurryThis$u = functionUncurryThis;
	var toString$h = toString$o;
	var trim = stringTrim.trim;
	var whitespaces$1 = whitespaces$3;

	var charAt$8 = uncurryThis$u(''.charAt);
	var $parseFloat = globalThis$l.parseFloat;
	var Symbol$1 = globalThis$l.Symbol;
	var ITERATOR$3 = Symbol$1 && Symbol$1.iterator;
	var FORCED$3 = 1 / $parseFloat(whitespaces$1 + '-0') !== -Infinity
	  // MS Edge 18- broken with boxed symbols
	  || (ITERATOR$3 && !fails$q(function () { $parseFloat(Object(ITERATOR$3)); }));

	// `parseFloat` method
	// https://tc39.es/ecma262/#sec-parsefloat-string
	var numberParseFloat = FORCED$3 ? function parseFloat(string) {
	  var trimmedString = trim(toString$h(string));
	  var result = $parseFloat(trimmedString);
	  return result === 0 && charAt$8(trimmedString, 0) === '-' ? -0 : result;
	} : $parseFloat;

	var $$U = _export;
	var parseFloat = numberParseFloat;

	// `Number.parseFloat` method
	// https://tc39.es/ecma262/#sec-number.parseFloat
	// eslint-disable-next-line es/no-number-parsefloat -- required for testing
	$$U({ target: 'Number', stat: true, forced: Number.parseFloat !== parseFloat }, {
	  parseFloat: parseFloat
	});

	var toIntegerOrInfinity$5 = toIntegerOrInfinity$b;
	var toString$g = toString$o;
	var requireObjectCoercible$b = requireObjectCoercible$g;

	var $RangeError$4 = RangeError;

	// `String.prototype.repeat` method implementation
	// https://tc39.es/ecma262/#sec-string.prototype.repeat
	var stringRepeat = function repeat(count) {
	  var str = toString$g(requireObjectCoercible$b(this));
	  var result = '';
	  var n = toIntegerOrInfinity$5(count);
	  if (n < 0 || n === Infinity) throw new $RangeError$4('Wrong number of repetitions');
	  for (;n > 0; (n >>>= 1) && (str += str)) if (n & 1) result += str;
	  return result;
	};

	var $$T = _export;
	var uncurryThis$t = functionUncurryThis;
	var toIntegerOrInfinity$4 = toIntegerOrInfinity$b;
	var thisNumberValue$1 = thisNumberValue$3;
	var $repeat$1 = stringRepeat;
	var fails$p = fails$S;

	var $RangeError$3 = RangeError;
	var $String = String;
	var floor$3 = Math.floor;
	var repeat$2 = uncurryThis$t($repeat$1);
	var stringSlice$9 = uncurryThis$t(''.slice);
	var nativeToFixed = uncurryThis$t(1.0.toFixed);

	var pow$1 = function (x, n, acc) {
	  return n === 0 ? acc : n % 2 === 1 ? pow$1(x, n - 1, acc * x) : pow$1(x * x, n / 2, acc);
	};

	var log = function (x) {
	  var n = 0;
	  var x2 = x;
	  while (x2 >= 4096) {
	    n += 12;
	    x2 /= 4096;
	  }
	  while (x2 >= 2) {
	    n += 1;
	    x2 /= 2;
	  } return n;
	};

	var multiply = function (data, n, c) {
	  var index = -1;
	  var c2 = c;
	  while (++index < 6) {
	    c2 += n * data[index];
	    data[index] = c2 % 1e7;
	    c2 = floor$3(c2 / 1e7);
	  }
	};

	var divide = function (data, n) {
	  var index = 6;
	  var c = 0;
	  while (--index >= 0) {
	    c += data[index];
	    data[index] = floor$3(c / n);
	    c = (c % n) * 1e7;
	  }
	};

	var dataToString = function (data) {
	  var index = 6;
	  var s = '';
	  while (--index >= 0) {
	    if (s !== '' || index === 0 || data[index] !== 0) {
	      var t = $String(data[index]);
	      s = s === '' ? t : s + repeat$2('0', 7 - t.length) + t;
	    }
	  } return s;
	};

	var FORCED$2 = fails$p(function () {
	  return nativeToFixed(0.00008, 3) !== '0.000' ||
	    nativeToFixed(0.9, 0) !== '1' ||
	    nativeToFixed(1.255, 2) !== '1.25' ||
	    nativeToFixed(1000000000000000128.0, 0) !== '1000000000000000128';
	}) || !fails$p(function () {
	  // V8 ~ Android 4.3-
	  nativeToFixed({});
	});

	// `Number.prototype.toFixed` method
	// https://tc39.es/ecma262/#sec-number.prototype.tofixed
	$$T({ target: 'Number', proto: true, forced: FORCED$2 }, {
	  toFixed: function toFixed(fractionDigits) {
	    var number = thisNumberValue$1(this);
	    var fractDigits = toIntegerOrInfinity$4(fractionDigits);
	    var data = [0, 0, 0, 0, 0, 0];
	    var sign = '';
	    var result = '0';
	    var e, z, j, k;

	    // TODO: ES2018 increased the maximum number of fraction digits to 100, need to improve the implementation
	    if (fractDigits < 0 || fractDigits > 20) throw new $RangeError$3('Incorrect fraction digits');
	    // eslint-disable-next-line no-self-compare -- NaN check
	    if (number !== number) return 'NaN';
	    if (number <= -1e21 || number >= 1e21) return $String(number);
	    if (number < 0) {
	      sign = '-';
	      number = -number;
	    }
	    if (number > 1e-21) {
	      e = log(number * pow$1(2, 69, 1)) - 69;
	      z = e < 0 ? number * pow$1(2, -e, 1) : number / pow$1(2, e, 1);
	      z *= 0x10000000000000;
	      e = 52 - e;
	      if (e > 0) {
	        multiply(data, 0, z);
	        j = fractDigits;
	        while (j >= 7) {
	          multiply(data, 1e7, 0);
	          j -= 7;
	        }
	        multiply(data, pow$1(10, j, 1), 0);
	        j = e - 1;
	        while (j >= 23) {
	          divide(data, 1 << 23);
	          j -= 23;
	        }
	        divide(data, 1 << j);
	        multiply(data, 1, 1);
	        divide(data, 2);
	        result = dataToString(data);
	      } else {
	        multiply(data, 0, z);
	        multiply(data, 1 << -e, 0);
	        result = dataToString(data) + repeat$2('0', fractDigits);
	      }
	    }
	    if (fractDigits > 0) {
	      k = result.length;
	      result = sign + (k <= fractDigits
	        ? '0.' + repeat$2('0', fractDigits - k) + result
	        : stringSlice$9(result, 0, k - fractDigits) + '.' + stringSlice$9(result, k - fractDigits));
	    } else {
	      result = sign + result;
	    } return result;
	  }
	});

	var DESCRIPTORS$a = descriptors;
	var uncurryThis$s = functionUncurryThis;
	var call$h = functionCall;
	var fails$o = fails$S;
	var objectKeys$2 = objectKeys$5;
	var getOwnPropertySymbolsModule = objectGetOwnPropertySymbols;
	var propertyIsEnumerableModule = objectPropertyIsEnumerable;
	var toObject$3 = toObject$g;
	var IndexedObject = indexedObject;

	// eslint-disable-next-line es/no-object-assign -- safe
	var $assign = Object.assign;
	// eslint-disable-next-line es/no-object-defineproperty -- required for testing
	var defineProperty$2 = Object.defineProperty;
	var concat$1 = uncurryThis$s([].concat);

	// `Object.assign` method
	// https://tc39.es/ecma262/#sec-object.assign
	var objectAssign = !$assign || fails$o(function () {
	  // should have correct order of operations (Edge bug)
	  if (DESCRIPTORS$a && $assign({ b: 1 }, $assign(defineProperty$2({}, 'a', {
	    enumerable: true,
	    get: function () {
	      defineProperty$2(this, 'b', {
	        value: 3,
	        enumerable: false
	      });
	    }
	  }), { b: 2 })).b !== 1) return true;
	  // should work with symbols and should have deterministic property order (V8 bug)
	  var A = {};
	  var B = {};
	  // eslint-disable-next-line es/no-symbol -- safe
	  var symbol = Symbol('assign detection');
	  var alphabet = 'abcdefghijklmnopqrst';
	  A[symbol] = 7;
	  alphabet.split('').forEach(function (chr) { B[chr] = chr; });
	  return $assign({}, A)[symbol] !== 7 || objectKeys$2($assign({}, B)).join('') !== alphabet;
	}) ? function assign(target, source) { // eslint-disable-line no-unused-vars -- required for `.length`
	  var T = toObject$3(target);
	  var argumentsLength = arguments.length;
	  var index = 1;
	  var getOwnPropertySymbols = getOwnPropertySymbolsModule.f;
	  var propertyIsEnumerable = propertyIsEnumerableModule.f;
	  while (argumentsLength > index) {
	    var S = IndexedObject(arguments[index++]);
	    var keys = getOwnPropertySymbols ? concat$1(objectKeys$2(S), getOwnPropertySymbols(S)) : objectKeys$2(S);
	    var length = keys.length;
	    var j = 0;
	    var key;
	    while (length > j) {
	      key = keys[j++];
	      if (!DESCRIPTORS$a || call$h(propertyIsEnumerable, S, key)) T[key] = S[key];
	    }
	  } return T;
	} : $assign;

	var $$S = _export;
	var assign$1 = objectAssign;

	// `Object.assign` method
	// https://tc39.es/ecma262/#sec-object.assign
	// eslint-disable-next-line es/no-object-assign -- required for testing
	$$S({ target: 'Object', stat: true, arity: 2, forced: Object.assign !== assign$1 }, {
	  assign: assign$1
	});

	var DESCRIPTORS$9 = descriptors;
	var fails$n = fails$S;
	var uncurryThis$r = functionUncurryThis;
	var objectGetPrototypeOf = objectGetPrototypeOf$1;
	var objectKeys$1 = objectKeys$5;
	var toIndexedObject = toIndexedObject$d;
	var $propertyIsEnumerable = objectPropertyIsEnumerable.f;

	var propertyIsEnumerable = uncurryThis$r($propertyIsEnumerable);
	var push$7 = uncurryThis$r([].push);

	// in some IE versions, `propertyIsEnumerable` returns incorrect result on integer keys
	// of `null` prototype objects
	var IE_BUG = DESCRIPTORS$9 && fails$n(function () {
	  // eslint-disable-next-line es/no-object-create -- safe
	  var O = Object.create(null);
	  O[2] = 2;
	  return !propertyIsEnumerable(O, 2);
	});

	// `Object.{ entries, values }` methods implementation
	var createMethod$2 = function (TO_ENTRIES) {
	  return function (it) {
	    var O = toIndexedObject(it);
	    var keys = objectKeys$1(O);
	    var IE_WORKAROUND = IE_BUG && objectGetPrototypeOf(O) === null;
	    var length = keys.length;
	    var i = 0;
	    var result = [];
	    var key;
	    while (length > i) {
	      key = keys[i++];
	      if (!DESCRIPTORS$9 || (IE_WORKAROUND ? key in O : propertyIsEnumerable(O, key))) {
	        push$7(result, TO_ENTRIES ? [key, O[key]] : O[key]);
	      }
	    }
	    return result;
	  };
	};

	var objectToArray = {
	  // `Object.entries` method
	  // https://tc39.es/ecma262/#sec-object.entries
	  entries: createMethod$2(true),
	  // `Object.values` method
	  // https://tc39.es/ecma262/#sec-object.values
	  values: createMethod$2(false)
	};

	var $$R = _export;
	var $entries = objectToArray.entries;

	// `Object.entries` method
	// https://tc39.es/ecma262/#sec-object.entries
	$$R({ target: 'Object', stat: true }, {
	  entries: function entries(O) {
	    return $entries(O);
	  }
	});

	var $$Q = _export;
	var iterate$9 = iterate$d;
	var createProperty$1 = createProperty$6;

	// `Object.fromEntries` method
	// https://tc39.es/ecma262/#sec-object.fromentries
	$$Q({ target: 'Object', stat: true }, {
	  fromEntries: function fromEntries(iterable) {
	    var obj = {};
	    iterate$9(iterable, function (k, v) {
	      createProperty$1(obj, k, v);
	    }, { AS_ENTRIES: true });
	    return obj;
	  }
	});

	var $$P = _export;
	var fails$m = fails$S;
	var getOwnPropertyNames = objectGetOwnPropertyNamesExternal.f;

	// eslint-disable-next-line es/no-object-getownpropertynames -- required for testing
	var FAILS_ON_PRIMITIVES$2 = fails$m(function () { return !Object.getOwnPropertyNames(1); });

	// `Object.getOwnPropertyNames` method
	// https://tc39.es/ecma262/#sec-object.getownpropertynames
	$$P({ target: 'Object', stat: true, forced: FAILS_ON_PRIMITIVES$2 }, {
	  getOwnPropertyNames: getOwnPropertyNames
	});

	var $$O = _export;
	var fails$l = fails$S;
	var toObject$2 = toObject$g;
	var nativeGetPrototypeOf = objectGetPrototypeOf$1;
	var CORRECT_PROTOTYPE_GETTER = correctPrototypeGetter;

	var FAILS_ON_PRIMITIVES$1 = fails$l(function () { nativeGetPrototypeOf(1); });

	// `Object.getPrototypeOf` method
	// https://tc39.es/ecma262/#sec-object.getprototypeof
	$$O({ target: 'Object', stat: true, forced: FAILS_ON_PRIMITIVES$1, sham: !CORRECT_PROTOTYPE_GETTER }, {
	  getPrototypeOf: function getPrototypeOf(it) {
	    return nativeGetPrototypeOf(toObject$2(it));
	  }
	});

	var $$N = _export;
	var getBuiltIn$a = getBuiltIn$k;
	var uncurryThis$q = functionUncurryThis;
	var aCallable$9 = aCallable$i;
	var requireObjectCoercible$a = requireObjectCoercible$g;
	var toPropertyKey = toPropertyKey$4;
	var iterate$8 = iterate$d;
	var fails$k = fails$S;

	// eslint-disable-next-line es/no-object-groupby -- testing
	var nativeGroupBy = Object.groupBy;
	var create$3 = getBuiltIn$a('Object', 'create');
	var push$6 = uncurryThis$q([].push);

	var DOES_NOT_WORK_WITH_PRIMITIVES = !nativeGroupBy || fails$k(function () {
	  return nativeGroupBy('ab', function (it) {
	    return it;
	  }).a.length !== 1;
	});

	// `Object.groupBy` method
	// https://tc39.es/ecma262/#sec-object.groupby
	$$N({ target: 'Object', stat: true, forced: DOES_NOT_WORK_WITH_PRIMITIVES }, {
	  groupBy: function groupBy(items, callbackfn) {
	    requireObjectCoercible$a(items);
	    aCallable$9(callbackfn);
	    var obj = create$3(null);
	    var k = 0;
	    iterate$8(items, function (value) {
	      var key = toPropertyKey(callbackfn(value, k++));
	      // in some IE versions, `hasOwnProperty` returns incorrect result on integer keys
	      // but since it's a `null` prototype object, we can safely use `in`
	      if (key in obj) push$6(obj[key], value);
	      else obj[key] = [value];
	    });
	    return obj;
	  }
	});

	var $$M = _export;
	var hasOwn$8 = hasOwnProperty_1;

	// `Object.hasOwn` method
	// https://tc39.es/ecma262/#sec-object.hasown
	$$M({ target: 'Object', stat: true }, {
	  hasOwn: hasOwn$8
	});

	var $$L = _export;
	var toObject$1 = toObject$g;
	var nativeKeys = objectKeys$5;
	var fails$j = fails$S;

	var FAILS_ON_PRIMITIVES = fails$j(function () { nativeKeys(1); });

	// `Object.keys` method
	// https://tc39.es/ecma262/#sec-object.keys
	$$L({ target: 'Object', stat: true, forced: FAILS_ON_PRIMITIVES }, {
	  keys: function keys(it) {
	    return nativeKeys(toObject$1(it));
	  }
	});

	var TO_STRING_TAG_SUPPORT$1 = toStringTagSupport;
	var classof$5 = classof$d;

	// `Object.prototype.toString` method implementation
	// https://tc39.es/ecma262/#sec-object.prototype.tostring
	var objectToString = TO_STRING_TAG_SUPPORT$1 ? {}.toString : function toString() {
	  return '[object ' + classof$5(this) + ']';
	};

	var TO_STRING_TAG_SUPPORT = toStringTagSupport;
	var defineBuiltIn$b = defineBuiltIn$k;
	var toString$f = objectToString;

	// `Object.prototype.toString` method
	// https://tc39.es/ecma262/#sec-object.prototype.tostring
	if (!TO_STRING_TAG_SUPPORT) {
	  defineBuiltIn$b(Object.prototype, 'toString', toString$f, { unsafe: true });
	}

	var $$K = _export;
	var $values = objectToArray.values;

	// `Object.values` method
	// https://tc39.es/ecma262/#sec-object.values
	$$K({ target: 'Object', stat: true }, {
	  values: function values(O) {
	    return $values(O);
	  }
	});

	/* global Bun, Deno -- detection */
	var globalThis$k = globalThis_1;
	var userAgent$4 = environmentUserAgent;
	var classof$4 = classofRaw$2;

	var userAgentStartsWith = function (string) {
	  return userAgent$4.slice(0, string.length) === string;
	};

	var environment = (function () {
	  if (userAgentStartsWith('Bun/')) return 'BUN';
	  if (userAgentStartsWith('Cloudflare-Workers')) return 'CLOUDFLARE';
	  if (userAgentStartsWith('Deno/')) return 'DENO';
	  if (userAgentStartsWith('Node.js/')) return 'NODE';
	  if (globalThis$k.Bun && typeof Bun.version == 'string') return 'BUN';
	  if (globalThis$k.Deno && typeof Deno.version == 'object') return 'DENO';
	  if (classof$4(globalThis$k.process) === 'process') return 'NODE';
	  if (globalThis$k.window && globalThis$k.document) return 'BROWSER';
	  return 'REST';
	})();

	var ENVIRONMENT$2 = environment;

	var environmentIsNode = ENVIRONMENT$2 === 'NODE';

	var isConstructor$1 = isConstructor$6;
	var tryToString = tryToString$6;

	var $TypeError$7 = TypeError;

	// `Assert: IsConstructor(argument) is true`
	var aConstructor$1 = function (argument) {
	  if (isConstructor$1(argument)) return argument;
	  throw new $TypeError$7(tryToString(argument) + ' is not a constructor');
	};

	var anObject$d = anObject$q;
	var aConstructor = aConstructor$1;
	var isNullOrUndefined$5 = isNullOrUndefined$b;
	var wellKnownSymbol$a = wellKnownSymbol$w;

	var SPECIES$2 = wellKnownSymbol$a('species');

	// `SpeciesConstructor` abstract operation
	// https://tc39.es/ecma262/#sec-speciesconstructor
	var speciesConstructor$3 = function (O, defaultConstructor) {
	  var C = anObject$d(O).constructor;
	  var S;
	  return C === undefined || isNullOrUndefined$5(S = anObject$d(C)[SPECIES$2]) ? defaultConstructor : aConstructor(S);
	};

	var $TypeError$6 = TypeError;

	var validateArgumentsLength$6 = function (passed, required) {
	  if (passed < required) throw new $TypeError$6('Not enough arguments');
	  return passed;
	};

	var userAgent$3 = environmentUserAgent;

	// eslint-disable-next-line redos/no-vulnerable -- safe
	var environmentIsIos = /(?:ipad|iphone|ipod).*applewebkit/i.test(userAgent$3);

	var globalThis$j = globalThis_1;
	var apply$1 = functionApply;
	var bind$4 = functionBindContext;
	var isCallable$a = isCallable$w;
	var hasOwn$7 = hasOwnProperty_1;
	var fails$i = fails$S;
	var html = html$2;
	var arraySlice$1 = arraySlice$5;
	var createElement = documentCreateElement$2;
	var validateArgumentsLength$5 = validateArgumentsLength$6;
	var IS_IOS$1 = environmentIsIos;
	var IS_NODE$3 = environmentIsNode;

	var set$4 = globalThis$j.setImmediate;
	var clear = globalThis$j.clearImmediate;
	var process$2 = globalThis$j.process;
	var Dispatch = globalThis$j.Dispatch;
	var Function$1 = globalThis$j.Function;
	var MessageChannel = globalThis$j.MessageChannel;
	var String$1 = globalThis$j.String;
	var counter = 0;
	var queue$2 = {};
	var ONREADYSTATECHANGE = 'onreadystatechange';
	var $location, defer, channel$1, port;

	fails$i(function () {
	  // Deno throws a ReferenceError on `location` access without `--location` flag
	  $location = globalThis$j.location;
	});

	var run = function (id) {
	  if (hasOwn$7(queue$2, id)) {
	    var fn = queue$2[id];
	    delete queue$2[id];
	    fn();
	  }
	};

	var runner = function (id) {
	  return function () {
	    run(id);
	  };
	};

	var eventListener = function (event) {
	  run(event.data);
	};

	var globalPostMessageDefer = function (id) {
	  // old engines have not location.origin
	  globalThis$j.postMessage(String$1(id), $location.protocol + '//' + $location.host);
	};

	// Node.js 0.9+ & IE10+ has setImmediate, otherwise:
	if (!set$4 || !clear) {
	  set$4 = function setImmediate(handler) {
	    validateArgumentsLength$5(arguments.length, 1);
	    var fn = isCallable$a(handler) ? handler : Function$1(handler);
	    var args = arraySlice$1(arguments, 1);
	    queue$2[++counter] = function () {
	      apply$1(fn, undefined, args);
	    };
	    defer(counter);
	    return counter;
	  };
	  clear = function clearImmediate(id) {
	    delete queue$2[id];
	  };
	  // Node.js 0.8-
	  if (IS_NODE$3) {
	    defer = function (id) {
	      process$2.nextTick(runner(id));
	    };
	  // Sphere (JS game engine) Dispatch API
	  } else if (Dispatch && Dispatch.now) {
	    defer = function (id) {
	      Dispatch.now(runner(id));
	    };
	  // Browsers with MessageChannel, includes WebWorkers
	  // except iOS - https://github.com/zloirock/core-js/issues/624
	  } else if (MessageChannel && !IS_IOS$1) {
	    channel$1 = new MessageChannel();
	    port = channel$1.port2;
	    channel$1.port1.onmessage = eventListener;
	    defer = bind$4(port.postMessage, port);
	  // Browsers with postMessage, skip WebWorkers
	  // IE8 has postMessage, but it's sync & typeof its postMessage is 'object'
	  } else if (
	    globalThis$j.addEventListener &&
	    isCallable$a(globalThis$j.postMessage) &&
	    !globalThis$j.importScripts &&
	    $location && $location.protocol !== 'file:' &&
	    !fails$i(globalPostMessageDefer)
	  ) {
	    defer = globalPostMessageDefer;
	    globalThis$j.addEventListener('message', eventListener, false);
	  // IE8-
	  } else if (ONREADYSTATECHANGE in createElement('script')) {
	    defer = function (id) {
	      html.appendChild(createElement('script'))[ONREADYSTATECHANGE] = function () {
	        html.removeChild(this);
	        run(id);
	      };
	    };
	  // Rest old browsers
	  } else {
	    defer = function (id) {
	      setTimeout(runner(id), 0);
	    };
	  }
	}

	var task$1 = {
	  set: set$4};

	var globalThis$i = globalThis_1;
	var DESCRIPTORS$8 = descriptors;

	// eslint-disable-next-line es/no-object-getownpropertydescriptor -- safe
	var getOwnPropertyDescriptor$2 = Object.getOwnPropertyDescriptor;

	// Avoid NodeJS experimental warning
	var safeGetBuiltIn$2 = function (name) {
	  if (!DESCRIPTORS$8) return globalThis$i[name];
	  var descriptor = getOwnPropertyDescriptor$2(globalThis$i, name);
	  return descriptor && descriptor.value;
	};

	var Queue$2 = function () {
	  this.head = null;
	  this.tail = null;
	};

	Queue$2.prototype = {
	  add: function (item) {
	    var entry = { item: item, next: null };
	    var tail = this.tail;
	    if (tail) tail.next = entry;
	    else this.head = entry;
	    this.tail = entry;
	  },
	  get: function () {
	    var entry = this.head;
	    if (entry) {
	      var next = this.head = entry.next;
	      if (next === null) this.tail = null;
	      return entry.item;
	    }
	  }
	};

	var queue$1 = Queue$2;

	var userAgent$2 = environmentUserAgent;

	var environmentIsIosPebble = /ipad|iphone|ipod/i.test(userAgent$2) && typeof Pebble != 'undefined';

	var userAgent$1 = environmentUserAgent;

	var environmentIsWebosWebkit = /web0s(?!.*chrome)/i.test(userAgent$1);

	var globalThis$h = globalThis_1;
	var safeGetBuiltIn$1 = safeGetBuiltIn$2;
	var bind$3 = functionBindContext;
	var macrotask = task$1.set;
	var Queue$1 = queue$1;
	var IS_IOS = environmentIsIos;
	var IS_IOS_PEBBLE = environmentIsIosPebble;
	var IS_WEBOS_WEBKIT = environmentIsWebosWebkit;
	var IS_NODE$2 = environmentIsNode;

	var MutationObserver = globalThis$h.MutationObserver || globalThis$h.WebKitMutationObserver;
	var document$2 = globalThis$h.document;
	var process$1 = globalThis$h.process;
	var Promise$1 = globalThis$h.Promise;
	var microtask$1 = safeGetBuiltIn$1('queueMicrotask');
	var notify$1, toggle, node, promise, then;

	// modern engines have queueMicrotask method
	if (!microtask$1) {
	  var queue = new Queue$1();

	  var flush = function () {
	    var parent, fn;
	    if (IS_NODE$2 && (parent = process$1.domain)) parent.exit();
	    while (fn = queue.get()) try {
	      fn();
	    } catch (error) {
	      if (queue.head) notify$1();
	      throw error;
	    }
	    if (parent) parent.enter();
	  };

	  // browsers with MutationObserver, except iOS - https://github.com/zloirock/core-js/issues/339
	  // also except WebOS Webkit https://github.com/zloirock/core-js/issues/898
	  if (!IS_IOS && !IS_NODE$2 && !IS_WEBOS_WEBKIT && MutationObserver && document$2) {
	    toggle = true;
	    node = document$2.createTextNode('');
	    new MutationObserver(flush).observe(node, { characterData: true });
	    notify$1 = function () {
	      node.data = toggle = !toggle;
	    };
	  // environments with maybe non-completely correct, but existent Promise
	  } else if (!IS_IOS_PEBBLE && Promise$1 && Promise$1.resolve) {
	    // Promise.resolve without an argument throws an error in LG WebOS 2
	    promise = Promise$1.resolve(undefined);
	    // workaround of WebKit ~ iOS Safari 10.1 bug
	    promise.constructor = Promise$1;
	    then = bind$3(promise.then, promise);
	    notify$1 = function () {
	      then(flush);
	    };
	  // Node.js without promises
	  } else if (IS_NODE$2) {
	    notify$1 = function () {
	      process$1.nextTick(flush);
	    };
	  // for other environments - macrotask based on:
	  // - setImmediate
	  // - MessageChannel
	  // - window.postMessage
	  // - onreadystatechange
	  // - setTimeout
	  } else {
	    // `webpack` dev server bug on IE global methods - use bind(fn, global)
	    macrotask = bind$3(macrotask, globalThis$h);
	    notify$1 = function () {
	      macrotask(flush);
	    };
	  }

	  microtask$1 = function (fn) {
	    if (!queue.head) notify$1();
	    queue.add(fn);
	  };
	}

	var microtask_1 = microtask$1;

	var hostReportErrors$1 = function (a, b) {
	  try {
	    // eslint-disable-next-line no-console -- safe
	    arguments.length === 1 ? console.error(a) : console.error(a, b);
	  } catch (error) { /* empty */ }
	};

	var perform$5 = function (exec) {
	  try {
	    return { error: false, value: exec() };
	  } catch (error) {
	    return { error: true, value: error };
	  }
	};

	var globalThis$g = globalThis_1;

	var promiseNativeConstructor = globalThis$g.Promise;

	var globalThis$f = globalThis_1;
	var NativePromiseConstructor$4 = promiseNativeConstructor;
	var isCallable$9 = isCallable$w;
	var isForced = isForced_1;
	var inspectSource = inspectSource$3;
	var wellKnownSymbol$9 = wellKnownSymbol$w;
	var ENVIRONMENT$1 = environment;
	var V8_VERSION = environmentV8Version;

	NativePromiseConstructor$4 && NativePromiseConstructor$4.prototype;
	var SPECIES$1 = wellKnownSymbol$9('species');
	var SUBCLASSING = false;
	var NATIVE_PROMISE_REJECTION_EVENT$1 = isCallable$9(globalThis$f.PromiseRejectionEvent);

	var FORCED_PROMISE_CONSTRUCTOR$5 = isForced('Promise', function () {
	  var PROMISE_CONSTRUCTOR_SOURCE = inspectSource(NativePromiseConstructor$4);
	  var GLOBAL_CORE_JS_PROMISE = PROMISE_CONSTRUCTOR_SOURCE !== String(NativePromiseConstructor$4);
	  // V8 6.6 (Node 10 and Chrome 66) have a bug with resolving custom thenables
	  // https://bugs.chromium.org/p/chromium/issues/detail?id=830565
	  // We can't detect it synchronously, so just check versions
	  if (!GLOBAL_CORE_JS_PROMISE && V8_VERSION === 66) return true;
	  // We can't use @@species feature detection in V8 since it causes
	  // deoptimization and performance degradation
	  // https://github.com/zloirock/core-js/issues/679
	  if (!V8_VERSION || V8_VERSION < 51 || !/native code/.test(PROMISE_CONSTRUCTOR_SOURCE)) {
	    // Detect correctness of subclassing with @@species support
	    var promise = new NativePromiseConstructor$4(function (resolve) { resolve(1); });
	    var FakePromise = function (exec) {
	      exec(function () { /* empty */ }, function () { /* empty */ });
	    };
	    var constructor = promise.constructor = {};
	    constructor[SPECIES$1] = FakePromise;
	    SUBCLASSING = promise.then(function () { /* empty */ }) instanceof FakePromise;
	    if (!SUBCLASSING) return true;
	  // Unhandled rejections tracking support, NodeJS Promise without it fails @@species test
	  } return !GLOBAL_CORE_JS_PROMISE && (ENVIRONMENT$1 === 'BROWSER' || ENVIRONMENT$1 === 'DENO') && !NATIVE_PROMISE_REJECTION_EVENT$1;
	});

	var promiseConstructorDetection = {
	  CONSTRUCTOR: FORCED_PROMISE_CONSTRUCTOR$5,
	  REJECTION_EVENT: NATIVE_PROMISE_REJECTION_EVENT$1,
	  SUBCLASSING: SUBCLASSING
	};

	var newPromiseCapability$2 = {};

	var aCallable$8 = aCallable$i;

	var $TypeError$5 = TypeError;

	var PromiseCapability = function (C) {
	  var resolve, reject;
	  this.promise = new C(function ($$resolve, $$reject) {
	    if (resolve !== undefined || reject !== undefined) throw new $TypeError$5('Bad Promise constructor');
	    resolve = $$resolve;
	    reject = $$reject;
	  });
	  this.resolve = aCallable$8(resolve);
	  this.reject = aCallable$8(reject);
	};

	// `NewPromiseCapability` abstract operation
	// https://tc39.es/ecma262/#sec-newpromisecapability
	newPromiseCapability$2.f = function (C) {
	  return new PromiseCapability(C);
	};

	var $$J = _export;
	var IS_NODE$1 = environmentIsNode;
	var globalThis$e = globalThis_1;
	var call$g = functionCall;
	var defineBuiltIn$a = defineBuiltIn$k;
	var setPrototypeOf = objectSetPrototypeOf;
	var setToStringTag$4 = setToStringTag$9;
	var setSpecies = setSpecies$2;
	var aCallable$7 = aCallable$i;
	var isCallable$8 = isCallable$w;
	var isObject$6 = isObject$n;
	var anInstance$5 = anInstance$9;
	var speciesConstructor$2 = speciesConstructor$3;
	var task = task$1.set;
	var microtask = microtask_1;
	var hostReportErrors = hostReportErrors$1;
	var perform$4 = perform$5;
	var Queue = queue$1;
	var InternalStateModule$6 = internalState;
	var NativePromiseConstructor$3 = promiseNativeConstructor;
	var PromiseConstructorDetection = promiseConstructorDetection;
	var newPromiseCapabilityModule$6 = newPromiseCapability$2;

	var PROMISE = 'Promise';
	var FORCED_PROMISE_CONSTRUCTOR$4 = PromiseConstructorDetection.CONSTRUCTOR;
	var NATIVE_PROMISE_REJECTION_EVENT = PromiseConstructorDetection.REJECTION_EVENT;
	var NATIVE_PROMISE_SUBCLASSING = PromiseConstructorDetection.SUBCLASSING;
	var getInternalPromiseState = InternalStateModule$6.getterFor(PROMISE);
	var setInternalState$6 = InternalStateModule$6.set;
	var NativePromisePrototype$2 = NativePromiseConstructor$3 && NativePromiseConstructor$3.prototype;
	var PromiseConstructor = NativePromiseConstructor$3;
	var PromisePrototype = NativePromisePrototype$2;
	var TypeError$4 = globalThis$e.TypeError;
	var document$1 = globalThis$e.document;
	var process = globalThis$e.process;
	var newPromiseCapability$1 = newPromiseCapabilityModule$6.f;
	var newGenericPromiseCapability = newPromiseCapability$1;

	var DISPATCH_EVENT = !!(document$1 && document$1.createEvent && globalThis$e.dispatchEvent);
	var UNHANDLED_REJECTION = 'unhandledrejection';
	var REJECTION_HANDLED = 'rejectionhandled';
	var PENDING = 0;
	var FULFILLED = 1;
	var REJECTED = 2;
	var HANDLED = 1;
	var UNHANDLED = 2;

	var Internal, OwnPromiseCapability, PromiseWrapper, nativeThen;

	// helpers
	var isThenable = function (it) {
	  var then;
	  return isObject$6(it) && isCallable$8(then = it.then) ? then : false;
	};

	var callReaction = function (reaction, state) {
	  var value = state.value;
	  var ok = state.state === FULFILLED;
	  var handler = ok ? reaction.ok : reaction.fail;
	  var resolve = reaction.resolve;
	  var reject = reaction.reject;
	  var domain = reaction.domain;
	  var result, then, exited;
	  try {
	    if (handler) {
	      if (!ok) {
	        if (state.rejection === UNHANDLED) onHandleUnhandled(state);
	        state.rejection = HANDLED;
	      }
	      if (handler === true) result = value;
	      else {
	        if (domain) domain.enter();
	        result = handler(value); // can throw
	        if (domain) {
	          domain.exit();
	          exited = true;
	        }
	      }
	      if (result === reaction.promise) {
	        reject(new TypeError$4('Promise-chain cycle'));
	      } else if (then = isThenable(result)) {
	        call$g(then, result, resolve, reject);
	      } else resolve(result);
	    } else reject(value);
	  } catch (error) {
	    if (domain && !exited) domain.exit();
	    reject(error);
	  }
	};

	var notify = function (state, isReject) {
	  if (state.notified) return;
	  state.notified = true;
	  microtask(function () {
	    var reactions = state.reactions;
	    var reaction;
	    while (reaction = reactions.get()) {
	      callReaction(reaction, state);
	    }
	    state.notified = false;
	    if (isReject && !state.rejection) onUnhandled(state);
	  });
	};

	var dispatchEvent = function (name, promise, reason) {
	  var event, handler;
	  if (DISPATCH_EVENT) {
	    event = document$1.createEvent('Event');
	    event.promise = promise;
	    event.reason = reason;
	    event.initEvent(name, false, true);
	    globalThis$e.dispatchEvent(event);
	  } else event = { promise: promise, reason: reason };
	  if (!NATIVE_PROMISE_REJECTION_EVENT && (handler = globalThis$e['on' + name])) handler(event);
	  else if (name === UNHANDLED_REJECTION) hostReportErrors('Unhandled promise rejection', reason);
	};

	var onUnhandled = function (state) {
	  call$g(task, globalThis$e, function () {
	    var promise = state.facade;
	    var value = state.value;
	    var IS_UNHANDLED = isUnhandled(state);
	    var result;
	    if (IS_UNHANDLED) {
	      result = perform$4(function () {
	        if (IS_NODE$1) {
	          process.emit('unhandledRejection', value, promise);
	        } else dispatchEvent(UNHANDLED_REJECTION, promise, value);
	      });
	      // Browsers should not trigger `rejectionHandled` event if it was handled here, NodeJS - should
	      state.rejection = IS_NODE$1 || isUnhandled(state) ? UNHANDLED : HANDLED;
	      if (result.error) throw result.value;
	    }
	  });
	};

	var isUnhandled = function (state) {
	  return state.rejection !== HANDLED && !state.parent;
	};

	var onHandleUnhandled = function (state) {
	  call$g(task, globalThis$e, function () {
	    var promise = state.facade;
	    if (IS_NODE$1) {
	      process.emit('rejectionHandled', promise);
	    } else dispatchEvent(REJECTION_HANDLED, promise, state.value);
	  });
	};

	var bind$2 = function (fn, state, unwrap) {
	  return function (value) {
	    fn(state, value, unwrap);
	  };
	};

	var internalReject = function (state, value, unwrap) {
	  if (state.done) return;
	  state.done = true;
	  if (unwrap) state = unwrap;
	  state.value = value;
	  state.state = REJECTED;
	  notify(state, true);
	};

	var internalResolve = function (state, value, unwrap) {
	  if (state.done) return;
	  state.done = true;
	  if (unwrap) state = unwrap;
	  try {
	    if (state.facade === value) throw new TypeError$4("Promise can't be resolved itself");
	    var then = isThenable(value);
	    if (then) {
	      microtask(function () {
	        var wrapper = { done: false };
	        try {
	          call$g(then, value,
	            bind$2(internalResolve, wrapper, state),
	            bind$2(internalReject, wrapper, state)
	          );
	        } catch (error) {
	          internalReject(wrapper, error, state);
	        }
	      });
	    } else {
	      state.value = value;
	      state.state = FULFILLED;
	      notify(state, false);
	    }
	  } catch (error) {
	    internalReject({ done: false }, error, state);
	  }
	};

	// constructor polyfill
	if (FORCED_PROMISE_CONSTRUCTOR$4) {
	  // 25.4.3.1 Promise(executor)
	  PromiseConstructor = function Promise(executor) {
	    anInstance$5(this, PromisePrototype);
	    aCallable$7(executor);
	    call$g(Internal, this);
	    var state = getInternalPromiseState(this);
	    try {
	      executor(bind$2(internalResolve, state), bind$2(internalReject, state));
	    } catch (error) {
	      internalReject(state, error);
	    }
	  };

	  PromisePrototype = PromiseConstructor.prototype;

	  // eslint-disable-next-line no-unused-vars -- required for `.length`
	  Internal = function Promise(executor) {
	    setInternalState$6(this, {
	      type: PROMISE,
	      done: false,
	      notified: false,
	      parent: false,
	      reactions: new Queue(),
	      rejection: false,
	      state: PENDING,
	      value: null
	    });
	  };

	  // `Promise.prototype.then` method
	  // https://tc39.es/ecma262/#sec-promise.prototype.then
	  Internal.prototype = defineBuiltIn$a(PromisePrototype, 'then', function then(onFulfilled, onRejected) {
	    var state = getInternalPromiseState(this);
	    var reaction = newPromiseCapability$1(speciesConstructor$2(this, PromiseConstructor));
	    state.parent = true;
	    reaction.ok = isCallable$8(onFulfilled) ? onFulfilled : true;
	    reaction.fail = isCallable$8(onRejected) && onRejected;
	    reaction.domain = IS_NODE$1 ? process.domain : undefined;
	    if (state.state === PENDING) state.reactions.add(reaction);
	    else microtask(function () {
	      callReaction(reaction, state);
	    });
	    return reaction.promise;
	  });

	  OwnPromiseCapability = function () {
	    var promise = new Internal();
	    var state = getInternalPromiseState(promise);
	    this.promise = promise;
	    this.resolve = bind$2(internalResolve, state);
	    this.reject = bind$2(internalReject, state);
	  };

	  newPromiseCapabilityModule$6.f = newPromiseCapability$1 = function (C) {
	    return C === PromiseConstructor || C === PromiseWrapper
	      ? new OwnPromiseCapability(C)
	      : newGenericPromiseCapability(C);
	  };

	  if (isCallable$8(NativePromiseConstructor$3) && NativePromisePrototype$2 !== Object.prototype) {
	    nativeThen = NativePromisePrototype$2.then;

	    if (!NATIVE_PROMISE_SUBCLASSING) {
	      // make `Promise#then` return a polyfilled `Promise` for native promise-based APIs
	      defineBuiltIn$a(NativePromisePrototype$2, 'then', function then(onFulfilled, onRejected) {
	        var that = this;
	        return new PromiseConstructor(function (resolve, reject) {
	          call$g(nativeThen, that, resolve, reject);
	        }).then(onFulfilled, onRejected);
	      // https://github.com/zloirock/core-js/issues/640
	      }, { unsafe: true });
	    }

	    // make `.constructor === Promise` work for native promise-based APIs
	    try {
	      delete NativePromisePrototype$2.constructor;
	    } catch (error) { /* empty */ }

	    // make `instanceof Promise` work for native promise-based APIs
	    if (setPrototypeOf) {
	      setPrototypeOf(NativePromisePrototype$2, PromisePrototype);
	    }
	  }
	}

	// `Promise` constructor
	// https://tc39.es/ecma262/#sec-promise-executor
	$$J({ global: true, constructor: true, wrap: true, forced: FORCED_PROMISE_CONSTRUCTOR$4 }, {
	  Promise: PromiseConstructor
	});

	setToStringTag$4(PromiseConstructor, PROMISE, false);
	setSpecies(PROMISE);

	var NativePromiseConstructor$2 = promiseNativeConstructor;
	var checkCorrectnessOfIteration = checkCorrectnessOfIteration$3;
	var FORCED_PROMISE_CONSTRUCTOR$3 = promiseConstructorDetection.CONSTRUCTOR;

	var promiseStaticsIncorrectIteration = FORCED_PROMISE_CONSTRUCTOR$3 || !checkCorrectnessOfIteration(function (iterable) {
	  NativePromiseConstructor$2.all(iterable).then(undefined, function () { /* empty */ });
	});

	var $$I = _export;
	var call$f = functionCall;
	var aCallable$6 = aCallable$i;
	var newPromiseCapabilityModule$5 = newPromiseCapability$2;
	var perform$3 = perform$5;
	var iterate$7 = iterate$d;
	var PROMISE_STATICS_INCORRECT_ITERATION$3 = promiseStaticsIncorrectIteration;

	// `Promise.all` method
	// https://tc39.es/ecma262/#sec-promise.all
	$$I({ target: 'Promise', stat: true, forced: PROMISE_STATICS_INCORRECT_ITERATION$3 }, {
	  all: function all(iterable) {
	    var C = this;
	    var capability = newPromiseCapabilityModule$5.f(C);
	    var resolve = capability.resolve;
	    var reject = capability.reject;
	    var result = perform$3(function () {
	      var $promiseResolve = aCallable$6(C.resolve);
	      var values = [];
	      var counter = 0;
	      var remaining = 1;
	      iterate$7(iterable, function (promise) {
	        var index = counter++;
	        var alreadyCalled = false;
	        remaining++;
	        call$f($promiseResolve, C, promise).then(function (value) {
	          if (alreadyCalled) return;
	          alreadyCalled = true;
	          values[index] = value;
	          --remaining || resolve(values);
	        }, reject);
	      });
	      --remaining || resolve(values);
	    });
	    if (result.error) reject(result.value);
	    return capability.promise;
	  }
	});

	var $$H = _export;
	var FORCED_PROMISE_CONSTRUCTOR$2 = promiseConstructorDetection.CONSTRUCTOR;
	var NativePromiseConstructor$1 = promiseNativeConstructor;
	var getBuiltIn$9 = getBuiltIn$k;
	var isCallable$7 = isCallable$w;
	var defineBuiltIn$9 = defineBuiltIn$k;

	var NativePromisePrototype$1 = NativePromiseConstructor$1 && NativePromiseConstructor$1.prototype;

	// `Promise.prototype.catch` method
	// https://tc39.es/ecma262/#sec-promise.prototype.catch
	$$H({ target: 'Promise', proto: true, forced: FORCED_PROMISE_CONSTRUCTOR$2, real: true }, {
	  'catch': function (onRejected) {
	    return this.then(undefined, onRejected);
	  }
	});

	// makes sure that native promise-based APIs `Promise#catch` properly works with patched `Promise#then`
	if (isCallable$7(NativePromiseConstructor$1)) {
	  var method$1 = getBuiltIn$9('Promise').prototype['catch'];
	  if (NativePromisePrototype$1['catch'] !== method$1) {
	    defineBuiltIn$9(NativePromisePrototype$1, 'catch', method$1, { unsafe: true });
	  }
	}

	var $$G = _export;
	var call$e = functionCall;
	var aCallable$5 = aCallable$i;
	var newPromiseCapabilityModule$4 = newPromiseCapability$2;
	var perform$2 = perform$5;
	var iterate$6 = iterate$d;
	var PROMISE_STATICS_INCORRECT_ITERATION$2 = promiseStaticsIncorrectIteration;

	// `Promise.race` method
	// https://tc39.es/ecma262/#sec-promise.race
	$$G({ target: 'Promise', stat: true, forced: PROMISE_STATICS_INCORRECT_ITERATION$2 }, {
	  race: function race(iterable) {
	    var C = this;
	    var capability = newPromiseCapabilityModule$4.f(C);
	    var reject = capability.reject;
	    var result = perform$2(function () {
	      var $promiseResolve = aCallable$5(C.resolve);
	      iterate$6(iterable, function (promise) {
	        call$e($promiseResolve, C, promise).then(capability.resolve, reject);
	      });
	    });
	    if (result.error) reject(result.value);
	    return capability.promise;
	  }
	});

	var $$F = _export;
	var newPromiseCapabilityModule$3 = newPromiseCapability$2;
	var FORCED_PROMISE_CONSTRUCTOR$1 = promiseConstructorDetection.CONSTRUCTOR;

	// `Promise.reject` method
	// https://tc39.es/ecma262/#sec-promise.reject
	$$F({ target: 'Promise', stat: true, forced: FORCED_PROMISE_CONSTRUCTOR$1 }, {
	  reject: function reject(r) {
	    var capability = newPromiseCapabilityModule$3.f(this);
	    var capabilityReject = capability.reject;
	    capabilityReject(r);
	    return capability.promise;
	  }
	});

	var anObject$c = anObject$q;
	var isObject$5 = isObject$n;
	var newPromiseCapability = newPromiseCapability$2;

	var promiseResolve$2 = function (C, x) {
	  anObject$c(C);
	  if (isObject$5(x) && x.constructor === C) return x;
	  var promiseCapability = newPromiseCapability.f(C);
	  var resolve = promiseCapability.resolve;
	  resolve(x);
	  return promiseCapability.promise;
	};

	var $$E = _export;
	var getBuiltIn$8 = getBuiltIn$k;
	var FORCED_PROMISE_CONSTRUCTOR = promiseConstructorDetection.CONSTRUCTOR;
	var promiseResolve$1 = promiseResolve$2;

	getBuiltIn$8('Promise');

	// `Promise.resolve` method
	// https://tc39.es/ecma262/#sec-promise.resolve
	$$E({ target: 'Promise', stat: true, forced: FORCED_PROMISE_CONSTRUCTOR }, {
	  resolve: function resolve(x) {
	    return promiseResolve$1(this, x);
	  }
	});

	var $$D = _export;
	var call$d = functionCall;
	var aCallable$4 = aCallable$i;
	var newPromiseCapabilityModule$2 = newPromiseCapability$2;
	var perform$1 = perform$5;
	var iterate$5 = iterate$d;
	var PROMISE_STATICS_INCORRECT_ITERATION$1 = promiseStaticsIncorrectIteration;

	// `Promise.allSettled` method
	// https://tc39.es/ecma262/#sec-promise.allsettled
	$$D({ target: 'Promise', stat: true, forced: PROMISE_STATICS_INCORRECT_ITERATION$1 }, {
	  allSettled: function allSettled(iterable) {
	    var C = this;
	    var capability = newPromiseCapabilityModule$2.f(C);
	    var resolve = capability.resolve;
	    var reject = capability.reject;
	    var result = perform$1(function () {
	      var promiseResolve = aCallable$4(C.resolve);
	      var values = [];
	      var counter = 0;
	      var remaining = 1;
	      iterate$5(iterable, function (promise) {
	        var index = counter++;
	        var alreadyCalled = false;
	        remaining++;
	        call$d(promiseResolve, C, promise).then(function (value) {
	          if (alreadyCalled) return;
	          alreadyCalled = true;
	          values[index] = { status: 'fulfilled', value: value };
	          --remaining || resolve(values);
	        }, function (error) {
	          if (alreadyCalled) return;
	          alreadyCalled = true;
	          values[index] = { status: 'rejected', reason: error };
	          --remaining || resolve(values);
	        });
	      });
	      --remaining || resolve(values);
	    });
	    if (result.error) reject(result.value);
	    return capability.promise;
	  }
	});

	var $$C = _export;
	var call$c = functionCall;
	var aCallable$3 = aCallable$i;
	var getBuiltIn$7 = getBuiltIn$k;
	var newPromiseCapabilityModule$1 = newPromiseCapability$2;
	var perform = perform$5;
	var iterate$4 = iterate$d;
	var PROMISE_STATICS_INCORRECT_ITERATION = promiseStaticsIncorrectIteration;

	var PROMISE_ANY_ERROR = 'No one promise resolved';

	// `Promise.any` method
	// https://tc39.es/ecma262/#sec-promise.any
	$$C({ target: 'Promise', stat: true, forced: PROMISE_STATICS_INCORRECT_ITERATION }, {
	  any: function any(iterable) {
	    var C = this;
	    var AggregateError = getBuiltIn$7('AggregateError');
	    var capability = newPromiseCapabilityModule$1.f(C);
	    var resolve = capability.resolve;
	    var reject = capability.reject;
	    var result = perform(function () {
	      var promiseResolve = aCallable$3(C.resolve);
	      var errors = [];
	      var counter = 0;
	      var remaining = 1;
	      var alreadyResolved = false;
	      iterate$4(iterable, function (promise) {
	        var index = counter++;
	        var alreadyRejected = false;
	        remaining++;
	        call$c(promiseResolve, C, promise).then(function (value) {
	          if (alreadyRejected || alreadyResolved) return;
	          alreadyResolved = true;
	          resolve(value);
	        }, function (error) {
	          if (alreadyRejected || alreadyResolved) return;
	          alreadyRejected = true;
	          errors[index] = error;
	          --remaining || reject(new AggregateError(errors, PROMISE_ANY_ERROR));
	        });
	      });
	      --remaining || reject(new AggregateError(errors, PROMISE_ANY_ERROR));
	    });
	    if (result.error) reject(result.value);
	    return capability.promise;
	  }
	});

	var $$B = _export;
	var NativePromiseConstructor = promiseNativeConstructor;
	var fails$h = fails$S;
	var getBuiltIn$6 = getBuiltIn$k;
	var isCallable$6 = isCallable$w;
	var speciesConstructor$1 = speciesConstructor$3;
	var promiseResolve = promiseResolve$2;
	var defineBuiltIn$8 = defineBuiltIn$k;

	var NativePromisePrototype = NativePromiseConstructor && NativePromiseConstructor.prototype;

	// Safari bug https://bugs.webkit.org/show_bug.cgi?id=200829
	var NON_GENERIC = !!NativePromiseConstructor && fails$h(function () {
	  // eslint-disable-next-line unicorn/no-thenable -- required for testing
	  NativePromisePrototype['finally'].call({ then: function () { /* empty */ } }, function () { /* empty */ });
	});

	// `Promise.prototype.finally` method
	// https://tc39.es/ecma262/#sec-promise.prototype.finally
	$$B({ target: 'Promise', proto: true, real: true, forced: NON_GENERIC }, {
	  'finally': function (onFinally) {
	    var C = speciesConstructor$1(this, getBuiltIn$6('Promise'));
	    var isFunction = isCallable$6(onFinally);
	    return this.then(
	      isFunction ? function (x) {
	        return promiseResolve(C, onFinally()).then(function () { return x; });
	      } : onFinally,
	      isFunction ? function (e) {
	        return promiseResolve(C, onFinally()).then(function () { throw e; });
	      } : onFinally
	    );
	  }
	});

	// makes sure that native promise-based APIs `Promise#finally` properly works with patched `Promise#then`
	if (isCallable$6(NativePromiseConstructor)) {
	  var method = getBuiltIn$6('Promise').prototype['finally'];
	  if (NativePromisePrototype['finally'] !== method) {
	    defineBuiltIn$8(NativePromisePrototype, 'finally', method, { unsafe: true });
	  }
	}

	var $$A = _export;
	var newPromiseCapabilityModule = newPromiseCapability$2;

	// `Promise.withResolvers` method
	// https://tc39.es/ecma262/#sec-promise.withResolvers
	$$A({ target: 'Promise', stat: true }, {
	  withResolvers: function withResolvers() {
	    var promiseCapability = newPromiseCapabilityModule.f(this);
	    return {
	      promise: promiseCapability.promise,
	      resolve: promiseCapability.resolve,
	      reject: promiseCapability.reject
	    };
	  }
	});

	var $$z = _export;
	var ownKeys = ownKeys$2;

	// `Reflect.ownKeys` method
	// https://tc39.es/ecma262/#sec-reflect.ownkeys
	$$z({ target: 'Reflect', stat: true }, {
	  ownKeys: ownKeys
	});

	var anObject$b = anObject$q;

	// `RegExp.prototype.flags` getter implementation
	// https://tc39.es/ecma262/#sec-get-regexp.prototype.flags
	var regexpFlags$1 = function () {
	  var that = anObject$b(this);
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

	var fails$g = fails$S;
	var globalThis$d = globalThis_1;

	// babel-minify and Closure Compiler transpiles RegExp('a', 'y') -> /a/y and it causes SyntaxError
	var $RegExp$2 = globalThis$d.RegExp;

	var UNSUPPORTED_Y$1 = fails$g(function () {
	  var re = $RegExp$2('a', 'y');
	  re.lastIndex = 2;
	  return re.exec('abcd') !== null;
	});

	// UC Browser bug
	// https://github.com/zloirock/core-js/issues/1008
	UNSUPPORTED_Y$1 || fails$g(function () {
	  return !$RegExp$2('a', 'y').sticky;
	});

	var BROKEN_CARET = UNSUPPORTED_Y$1 || fails$g(function () {
	  // https://bugzilla.mozilla.org/show_bug.cgi?id=773687
	  var re = $RegExp$2('^r', 'gy');
	  re.lastIndex = 2;
	  return re.exec('str') !== null;
	});

	var regexpStickyHelpers = {
	  BROKEN_CARET: BROKEN_CARET};

	var fails$f = fails$S;
	var globalThis$c = globalThis_1;

	// babel-minify and Closure Compiler transpiles RegExp('.', 's') -> /./s and it causes SyntaxError
	var $RegExp$1 = globalThis$c.RegExp;

	var regexpUnsupportedDotAll = fails$f(function () {
	  var re = $RegExp$1('.', 's');
	  return !(re.dotAll && re.test('\n') && re.flags === 's');
	});

	var fails$e = fails$S;
	var globalThis$b = globalThis_1;

	// babel-minify and Closure Compiler transpiles RegExp('(?<a>b)', 'g') -> /(?<a>b)/g and it causes SyntaxError
	var $RegExp = globalThis$b.RegExp;

	var regexpUnsupportedNcg = fails$e(function () {
	  var re = $RegExp('(?<a>b)', 'g');
	  return re.exec('b').groups.a !== 'b' ||
	    'b'.replace(re, '$<a>c') !== 'bc';
	});

	/* eslint-disable regexp/no-empty-capturing-group, regexp/no-empty-group, regexp/no-lazy-ends -- testing */
	/* eslint-disable regexp/no-useless-quantifier -- testing */
	var call$b = functionCall;
	var uncurryThis$p = functionUncurryThis;
	var toString$e = toString$o;
	var regexpFlags = regexpFlags$1;
	var stickyHelpers = regexpStickyHelpers;
	var shared = shared$7;
	var create$2 = objectCreate;
	var getInternalState$3 = internalState.get;
	var UNSUPPORTED_DOT_ALL = regexpUnsupportedDotAll;
	var UNSUPPORTED_NCG = regexpUnsupportedNcg;

	var nativeReplace = shared('native-string-replace', String.prototype.replace);
	var nativeExec = RegExp.prototype.exec;
	var patchedExec = nativeExec;
	var charAt$7 = uncurryThis$p(''.charAt);
	var indexOf$1 = uncurryThis$p(''.indexOf);
	var replace$4 = uncurryThis$p(''.replace);
	var stringSlice$8 = uncurryThis$p(''.slice);

	var UPDATES_LAST_INDEX_WRONG = (function () {
	  var re1 = /a/;
	  var re2 = /b*/g;
	  call$b(nativeExec, re1, 'a');
	  call$b(nativeExec, re2, 'a');
	  return re1.lastIndex !== 0 || re2.lastIndex !== 0;
	})();

	var UNSUPPORTED_Y = stickyHelpers.BROKEN_CARET;

	// nonparticipating capturing group, copied from es5-shim's String#split patch.
	var NPCG_INCLUDED = /()??/.exec('')[1] !== undefined;

	var PATCH = UPDATES_LAST_INDEX_WRONG || NPCG_INCLUDED || UNSUPPORTED_Y || UNSUPPORTED_DOT_ALL || UNSUPPORTED_NCG;

	if (PATCH) {
	  patchedExec = function exec(string) {
	    var re = this;
	    var state = getInternalState$3(re);
	    var str = toString$e(string);
	    var raw = state.raw;
	    var result, reCopy, lastIndex, match, i, object, group;

	    if (raw) {
	      raw.lastIndex = re.lastIndex;
	      result = call$b(patchedExec, raw, str);
	      re.lastIndex = raw.lastIndex;
	      return result;
	    }

	    var groups = state.groups;
	    var sticky = UNSUPPORTED_Y && re.sticky;
	    var flags = call$b(regexpFlags, re);
	    var source = re.source;
	    var charsAdded = 0;
	    var strCopy = str;

	    if (sticky) {
	      flags = replace$4(flags, 'y', '');
	      if (indexOf$1(flags, 'g') === -1) {
	        flags += 'g';
	      }

	      strCopy = stringSlice$8(str, re.lastIndex);
	      // Support anchored sticky behavior.
	      if (re.lastIndex > 0 && (!re.multiline || re.multiline && charAt$7(str, re.lastIndex - 1) !== '\n')) {
	        source = '(?: ' + source + ')';
	        strCopy = ' ' + strCopy;
	        charsAdded++;
	      }
	      // ^(? + rx + ) is needed, in combination with some str slicing, to
	      // simulate the 'y' flag.
	      reCopy = new RegExp('^(?:' + source + ')', flags);
	    }

	    if (NPCG_INCLUDED) {
	      reCopy = new RegExp('^' + source + '$(?!\\s)', flags);
	    }
	    if (UPDATES_LAST_INDEX_WRONG) lastIndex = re.lastIndex;

	    match = call$b(nativeExec, sticky ? reCopy : re, strCopy);

	    if (sticky) {
	      if (match) {
	        match.input = stringSlice$8(match.input, charsAdded);
	        match[0] = stringSlice$8(match[0], charsAdded);
	        match.index = re.lastIndex;
	        re.lastIndex += match[0].length;
	      } else re.lastIndex = 0;
	    } else if (UPDATES_LAST_INDEX_WRONG && match) {
	      re.lastIndex = re.global ? match.index + match[0].length : lastIndex;
	    }
	    if (NPCG_INCLUDED && match && match.length > 1) {
	      // Fix browsers whose `exec` methods don't consistently return `undefined`
	      // for NPCG, like IE8. NOTE: This doesn't work for /(.?)?/
	      call$b(nativeReplace, match[0], reCopy, function () {
	        for (i = 1; i < arguments.length - 2; i++) {
	          if (arguments[i] === undefined) match[i] = undefined;
	        }
	      });
	    }

	    if (match && groups) {
	      match.groups = object = create$2(null);
	      for (i = 0; i < groups.length; i++) {
	        group = groups[i];
	        object[group[0]] = match[group[1]];
	      }
	    }

	    return match;
	  };
	}

	var regexpExec$2 = patchedExec;

	var $$y = _export;
	var exec$3 = regexpExec$2;

	// `RegExp.prototype.exec` method
	// https://tc39.es/ecma262/#sec-regexp.prototype.exec
	$$y({ target: 'RegExp', proto: true, forced: /./.exec !== exec$3 }, {
	  exec: exec$3
	});

	var globalThis$a = globalThis_1;
	var DESCRIPTORS$7 = descriptors;
	var defineBuiltInAccessor$4 = defineBuiltInAccessor$b;
	var regExpFlags$1 = regexpFlags$1;
	var fails$d = fails$S;

	// babel-minify and Closure Compiler transpiles RegExp('.', 'd') -> /./d and it causes SyntaxError
	var RegExp$1 = globalThis$a.RegExp;
	var RegExpPrototype$4 = RegExp$1.prototype;

	var FORCED$1 = DESCRIPTORS$7 && fails$d(function () {
	  var INDICES_SUPPORT = true;
	  try {
	    RegExp$1('.', 'd');
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
	  var result = Object.getOwnPropertyDescriptor(RegExpPrototype$4, 'flags').get.call(O);

	  return result !== expected || calls !== expected;
	});

	// `RegExp.prototype.flags` getter
	// https://tc39.es/ecma262/#sec-get-regexp.prototype.flags
	if (FORCED$1) defineBuiltInAccessor$4(RegExpPrototype$4, 'flags', {
	  configurable: true,
	  get: regExpFlags$1
	});

	// TODO: Remove from `core-js@4` since it's moved to entry points

	var $$x = _export;
	var call$a = functionCall;
	var isCallable$5 = isCallable$w;
	var anObject$a = anObject$q;
	var toString$d = toString$o;

	var DELEGATES_TO_EXEC = function () {
	  var execCalled = false;
	  var re = /[ac]/;
	  re.exec = function () {
	    execCalled = true;
	    return /./.exec.apply(this, arguments);
	  };
	  return re.test('abc') === true && execCalled;
	}();

	var nativeTest = /./.test;

	// `RegExp.prototype.test` method
	// https://tc39.es/ecma262/#sec-regexp.prototype.test
	$$x({ target: 'RegExp', proto: true, forced: !DELEGATES_TO_EXEC }, {
	  test: function (S) {
	    var R = anObject$a(this);
	    var string = toString$d(S);
	    var exec = R.exec;
	    if (!isCallable$5(exec)) return call$a(nativeTest, R, string);
	    var result = call$a(exec, R, string);
	    if (result === null) return false;
	    anObject$a(result);
	    return true;
	  }
	});

	var call$9 = functionCall;
	var hasOwn$6 = hasOwnProperty_1;
	var isPrototypeOf = objectIsPrototypeOf;
	var regExpFlags = regexpFlags$1;

	var RegExpPrototype$3 = RegExp.prototype;

	var regexpGetFlags = function (R) {
	  var flags = R.flags;
	  return flags === undefined && !('flags' in RegExpPrototype$3) && !hasOwn$6(R, 'flags') && isPrototypeOf(RegExpPrototype$3, R)
	    ? call$9(regExpFlags, R) : flags;
	};

	var PROPER_FUNCTION_NAME$1 = functionName.PROPER;
	var defineBuiltIn$7 = defineBuiltIn$k;
	var anObject$9 = anObject$q;
	var $toString$2 = toString$o;
	var fails$c = fails$S;
	var getRegExpFlags$3 = regexpGetFlags;

	var TO_STRING = 'toString';
	var RegExpPrototype$2 = RegExp.prototype;
	var nativeToString = RegExpPrototype$2[TO_STRING];

	var NOT_GENERIC = fails$c(function () { return nativeToString.call({ source: 'a', flags: 'b' }) !== '/a/b'; });
	// FF44- RegExp#toString has a wrong name
	var INCORRECT_NAME = PROPER_FUNCTION_NAME$1 && nativeToString.name !== TO_STRING;

	// `RegExp.prototype.toString` method
	// https://tc39.es/ecma262/#sec-regexp.prototype.tostring
	if (NOT_GENERIC || INCORRECT_NAME) {
	  defineBuiltIn$7(RegExpPrototype$2, TO_STRING, function toString() {
	    var R = anObject$9(this);
	    var pattern = $toString$2(R.source);
	    var flags = $toString$2(getRegExpFlags$3(R));
	    return '/' + pattern + '/' + flags;
	  }, { unsafe: true });
	}

	var collection$1 = collection$3;
	var collectionStrong = collectionStrong$2;

	// `Set` constructor
	// https://tc39.es/ecma262/#sec-set-objects
	collection$1('Set', function (init) {
	  return function Set() { return init(this, arguments.length ? arguments[0] : undefined); };
	}, collectionStrong);

	var uncurryThis$o = functionUncurryThis;

	// eslint-disable-next-line es/no-set -- safe
	var SetPrototype$1 = Set.prototype;

	var setHelpers = {
	  // eslint-disable-next-line es/no-set -- safe
	  Set: Set,
	  add: uncurryThis$o(SetPrototype$1.add),
	  has: uncurryThis$o(SetPrototype$1.has),
	  remove: uncurryThis$o(SetPrototype$1['delete']),
	  proto: SetPrototype$1
	};

	var has$b = setHelpers.has;

	// Perform ? RequireInternalSlot(M, [[SetData]])
	var aSet$7 = function (it) {
	  has$b(it);
	  return it;
	};

	var call$8 = functionCall;

	var iterateSimple$7 = function (record, fn, ITERATOR_INSTEAD_OF_RECORD) {
	  var iterator = ITERATOR_INSTEAD_OF_RECORD ? record : record.iterator;
	  var next = record.next;
	  var step, result;
	  while (!(step = call$8(next, iterator)).done) {
	    result = fn(step.value);
	    if (result !== undefined) return result;
	  }
	};

	var uncurryThis$n = functionUncurryThis;
	var iterateSimple$6 = iterateSimple$7;
	var SetHelpers$6 = setHelpers;

	var Set$4 = SetHelpers$6.Set;
	var SetPrototype = SetHelpers$6.proto;
	var forEach$2 = uncurryThis$n(SetPrototype.forEach);
	var keys = uncurryThis$n(SetPrototype.keys);
	var next = keys(new Set$4()).next;

	var setIterate$1 = function (set, fn, interruptible) {
	  return interruptible ? iterateSimple$6({ iterator: keys(set), next: next }, fn) : forEach$2(set, fn);
	};

	var SetHelpers$5 = setHelpers;
	var iterate$3 = setIterate$1;

	var Set$3 = SetHelpers$5.Set;
	var add$3 = SetHelpers$5.add;

	var setClone = function (set) {
	  var result = new Set$3();
	  iterate$3(set, function (it) {
	    add$3(result, it);
	  });
	  return result;
	};

	var uncurryThisAccessor = functionUncurryThisAccessor;
	var SetHelpers$4 = setHelpers;

	var setSize = uncurryThisAccessor(SetHelpers$4.proto, 'size', 'get') || function (set) {
	  return set.size;
	};

	var aCallable$2 = aCallable$i;
	var anObject$8 = anObject$q;
	var call$7 = functionCall;
	var toIntegerOrInfinity$3 = toIntegerOrInfinity$b;
	var getIteratorDirect = getIteratorDirect$2;

	var INVALID_SIZE = 'Invalid size';
	var $RangeError$2 = RangeError;
	var $TypeError$4 = TypeError;
	var max$2 = Math.max;

	var SetRecord = function (set, intSize) {
	  this.set = set;
	  this.size = max$2(intSize, 0);
	  this.has = aCallable$2(set.has);
	  this.keys = aCallable$2(set.keys);
	};

	SetRecord.prototype = {
	  getIterator: function () {
	    return getIteratorDirect(anObject$8(call$7(this.keys, this.set)));
	  },
	  includes: function (it) {
	    return call$7(this.has, this.set, it);
	  }
	};

	// `GetSetRecord` abstract operation
	// https://tc39.es/proposal-set-methods/#sec-getsetrecord
	var getSetRecord$7 = function (obj) {
	  anObject$8(obj);
	  var numSize = +obj.size;
	  // NOTE: If size is undefined, then numSize will be NaN
	  // eslint-disable-next-line no-self-compare -- NaN check
	  if (numSize !== numSize) throw new $TypeError$4(INVALID_SIZE);
	  var intSize = toIntegerOrInfinity$3(numSize);
	  if (intSize < 0) throw new $RangeError$2(INVALID_SIZE);
	  return new SetRecord(obj, intSize);
	};

	var aSet$6 = aSet$7;
	var SetHelpers$3 = setHelpers;
	var clone$2 = setClone;
	var size$4 = setSize;
	var getSetRecord$6 = getSetRecord$7;
	var iterateSet$2 = setIterate$1;
	var iterateSimple$5 = iterateSimple$7;

	var has$a = SetHelpers$3.has;
	var remove$1 = SetHelpers$3.remove;

	// `Set.prototype.difference` method
	// https://github.com/tc39/proposal-set-methods
	var setDifference = function difference(other) {
	  var O = aSet$6(this);
	  var otherRec = getSetRecord$6(other);
	  var result = clone$2(O);
	  if (size$4(O) <= otherRec.size) iterateSet$2(O, function (e) {
	    if (otherRec.includes(e)) remove$1(result, e);
	  });
	  else iterateSimple$5(otherRec.getIterator(), function (e) {
	    if (has$a(O, e)) remove$1(result, e);
	  });
	  return result;
	};

	var getBuiltIn$5 = getBuiltIn$k;

	var createSetLike = function (size) {
	  return {
	    size: size,
	    has: function () {
	      return false;
	    },
	    keys: function () {
	      return {
	        next: function () {
	          return { done: true };
	        }
	      };
	    }
	  };
	};

	var setMethodAcceptSetLike$7 = function (name) {
	  var Set = getBuiltIn$5('Set');
	  try {
	    new Set()[name](createSetLike(0));
	    try {
	      // late spec change, early WebKit ~ Safari 17.0 beta implementation does not pass it
	      // https://github.com/tc39/proposal-set-methods/pull/88
	      new Set()[name](createSetLike(-1));
	      return false;
	    } catch (error2) {
	      return true;
	    }
	  } catch (error) {
	    return false;
	  }
	};

	var $$w = _export;
	var difference = setDifference;
	var setMethodAcceptSetLike$6 = setMethodAcceptSetLike$7;

	// `Set.prototype.difference` method
	// https://tc39.es/ecma262/#sec-set.prototype.difference
	$$w({ target: 'Set', proto: true, real: true, forced: !setMethodAcceptSetLike$6('difference') }, {
	  difference: difference
	});

	var aSet$5 = aSet$7;
	var SetHelpers$2 = setHelpers;
	var size$3 = setSize;
	var getSetRecord$5 = getSetRecord$7;
	var iterateSet$1 = setIterate$1;
	var iterateSimple$4 = iterateSimple$7;

	var Set$2 = SetHelpers$2.Set;
	var add$2 = SetHelpers$2.add;
	var has$9 = SetHelpers$2.has;

	// `Set.prototype.intersection` method
	// https://github.com/tc39/proposal-set-methods
	var setIntersection = function intersection(other) {
	  var O = aSet$5(this);
	  var otherRec = getSetRecord$5(other);
	  var result = new Set$2();

	  if (size$3(O) > otherRec.size) {
	    iterateSimple$4(otherRec.getIterator(), function (e) {
	      if (has$9(O, e)) add$2(result, e);
	    });
	  } else {
	    iterateSet$1(O, function (e) {
	      if (otherRec.includes(e)) add$2(result, e);
	    });
	  }

	  return result;
	};

	var $$v = _export;
	var fails$b = fails$S;
	var intersection = setIntersection;
	var setMethodAcceptSetLike$5 = setMethodAcceptSetLike$7;

	var INCORRECT = !setMethodAcceptSetLike$5('intersection') || fails$b(function () {
	  // eslint-disable-next-line es/no-array-from, es/no-set -- testing
	  return String(Array.from(new Set([1, 2, 3]).intersection(new Set([3, 2])))) !== '3,2';
	});

	// `Set.prototype.intersection` method
	// https://tc39.es/ecma262/#sec-set.prototype.intersection
	$$v({ target: 'Set', proto: true, real: true, forced: INCORRECT }, {
	  intersection: intersection
	});

	var aSet$4 = aSet$7;
	var has$8 = setHelpers.has;
	var size$2 = setSize;
	var getSetRecord$4 = getSetRecord$7;
	var iterateSet = setIterate$1;
	var iterateSimple$3 = iterateSimple$7;
	var iteratorClose$1 = iteratorClose$5;

	// `Set.prototype.isDisjointFrom` method
	// https://tc39.github.io/proposal-set-methods/#Set.prototype.isDisjointFrom
	var setIsDisjointFrom = function isDisjointFrom(other) {
	  var O = aSet$4(this);
	  var otherRec = getSetRecord$4(other);
	  if (size$2(O) <= otherRec.size) return iterateSet(O, function (e) {
	    if (otherRec.includes(e)) return false;
	  }, true) !== false;
	  var iterator = otherRec.getIterator();
	  return iterateSimple$3(iterator, function (e) {
	    if (has$8(O, e)) return iteratorClose$1(iterator, 'normal', false);
	  }) !== false;
	};

	var $$u = _export;
	var isDisjointFrom = setIsDisjointFrom;
	var setMethodAcceptSetLike$4 = setMethodAcceptSetLike$7;

	// `Set.prototype.isDisjointFrom` method
	// https://tc39.es/ecma262/#sec-set.prototype.isdisjointfrom
	$$u({ target: 'Set', proto: true, real: true, forced: !setMethodAcceptSetLike$4('isDisjointFrom') }, {
	  isDisjointFrom: isDisjointFrom
	});

	var aSet$3 = aSet$7;
	var size$1 = setSize;
	var iterate$2 = setIterate$1;
	var getSetRecord$3 = getSetRecord$7;

	// `Set.prototype.isSubsetOf` method
	// https://tc39.github.io/proposal-set-methods/#Set.prototype.isSubsetOf
	var setIsSubsetOf = function isSubsetOf(other) {
	  var O = aSet$3(this);
	  var otherRec = getSetRecord$3(other);
	  if (size$1(O) > otherRec.size) return false;
	  return iterate$2(O, function (e) {
	    if (!otherRec.includes(e)) return false;
	  }, true) !== false;
	};

	var $$t = _export;
	var isSubsetOf = setIsSubsetOf;
	var setMethodAcceptSetLike$3 = setMethodAcceptSetLike$7;

	// `Set.prototype.isSubsetOf` method
	// https://tc39.es/ecma262/#sec-set.prototype.issubsetof
	$$t({ target: 'Set', proto: true, real: true, forced: !setMethodAcceptSetLike$3('isSubsetOf') }, {
	  isSubsetOf: isSubsetOf
	});

	var aSet$2 = aSet$7;
	var has$7 = setHelpers.has;
	var size = setSize;
	var getSetRecord$2 = getSetRecord$7;
	var iterateSimple$2 = iterateSimple$7;
	var iteratorClose = iteratorClose$5;

	// `Set.prototype.isSupersetOf` method
	// https://tc39.github.io/proposal-set-methods/#Set.prototype.isSupersetOf
	var setIsSupersetOf = function isSupersetOf(other) {
	  var O = aSet$2(this);
	  var otherRec = getSetRecord$2(other);
	  if (size(O) < otherRec.size) return false;
	  var iterator = otherRec.getIterator();
	  return iterateSimple$2(iterator, function (e) {
	    if (!has$7(O, e)) return iteratorClose(iterator, 'normal', false);
	  }) !== false;
	};

	var $$s = _export;
	var isSupersetOf = setIsSupersetOf;
	var setMethodAcceptSetLike$2 = setMethodAcceptSetLike$7;

	// `Set.prototype.isSupersetOf` method
	// https://tc39.es/ecma262/#sec-set.prototype.issupersetof
	$$s({ target: 'Set', proto: true, real: true, forced: !setMethodAcceptSetLike$2('isSupersetOf') }, {
	  isSupersetOf: isSupersetOf
	});

	var aSet$1 = aSet$7;
	var SetHelpers$1 = setHelpers;
	var clone$1 = setClone;
	var getSetRecord$1 = getSetRecord$7;
	var iterateSimple$1 = iterateSimple$7;

	var add$1 = SetHelpers$1.add;
	var has$6 = SetHelpers$1.has;
	var remove = SetHelpers$1.remove;

	// `Set.prototype.symmetricDifference` method
	// https://github.com/tc39/proposal-set-methods
	var setSymmetricDifference = function symmetricDifference(other) {
	  var O = aSet$1(this);
	  var keysIter = getSetRecord$1(other).getIterator();
	  var result = clone$1(O);
	  iterateSimple$1(keysIter, function (e) {
	    if (has$6(O, e)) remove(result, e);
	    else add$1(result, e);
	  });
	  return result;
	};

	var $$r = _export;
	var symmetricDifference = setSymmetricDifference;
	var setMethodAcceptSetLike$1 = setMethodAcceptSetLike$7;

	// `Set.prototype.symmetricDifference` method
	// https://tc39.es/ecma262/#sec-set.prototype.symmetricdifference
	$$r({ target: 'Set', proto: true, real: true, forced: !setMethodAcceptSetLike$1('symmetricDifference') }, {
	  symmetricDifference: symmetricDifference
	});

	var aSet = aSet$7;
	var add = setHelpers.add;
	var clone = setClone;
	var getSetRecord = getSetRecord$7;
	var iterateSimple = iterateSimple$7;

	// `Set.prototype.union` method
	// https://github.com/tc39/proposal-set-methods
	var setUnion = function union(other) {
	  var O = aSet(this);
	  var keysIter = getSetRecord(other).getIterator();
	  var result = clone(O);
	  iterateSimple(keysIter, function (it) {
	    add(result, it);
	  });
	  return result;
	};

	var $$q = _export;
	var union = setUnion;
	var setMethodAcceptSetLike = setMethodAcceptSetLike$7;

	// `Set.prototype.union` method
	// https://tc39.es/ecma262/#sec-set.prototype.union
	$$q({ target: 'Set', proto: true, real: true, forced: !setMethodAcceptSetLike('union') }, {
	  union: union
	});

	var $$p = _export;
	var uncurryThis$m = functionUncurryThis;
	var requireObjectCoercible$9 = requireObjectCoercible$g;
	var toIntegerOrInfinity$2 = toIntegerOrInfinity$b;
	var toString$c = toString$o;
	var fails$a = fails$S;

	var charAt$6 = uncurryThis$m(''.charAt);

	var FORCED = fails$a(function () {
	  // eslint-disable-next-line es/no-string-prototype-at -- safe
	  return '𠮷'.at(-2) !== '\uD842';
	});

	// `String.prototype.at` method
	// https://tc39.es/ecma262/#sec-string.prototype.at
	$$p({ target: 'String', proto: true, forced: FORCED }, {
	  at: function at(index) {
	    var S = toString$c(requireObjectCoercible$9(this));
	    var len = S.length;
	    var relativeIndex = toIntegerOrInfinity$2(index);
	    var k = relativeIndex >= 0 ? relativeIndex : len + relativeIndex;
	    return (k < 0 || k >= len) ? undefined : charAt$6(S, k);
	  }
	});

	var uncurryThis$l = functionUncurryThis;
	var toIntegerOrInfinity$1 = toIntegerOrInfinity$b;
	var toString$b = toString$o;
	var requireObjectCoercible$8 = requireObjectCoercible$g;

	var charAt$5 = uncurryThis$l(''.charAt);
	var charCodeAt$2 = uncurryThis$l(''.charCodeAt);
	var stringSlice$7 = uncurryThis$l(''.slice);

	var createMethod$1 = function (CONVERT_TO_STRING) {
	  return function ($this, pos) {
	    var S = toString$b(requireObjectCoercible$8($this));
	    var position = toIntegerOrInfinity$1(pos);
	    var size = S.length;
	    var first, second;
	    if (position < 0 || position >= size) return CONVERT_TO_STRING ? '' : undefined;
	    first = charCodeAt$2(S, position);
	    return first < 0xD800 || first > 0xDBFF || position + 1 === size
	      || (second = charCodeAt$2(S, position + 1)) < 0xDC00 || second > 0xDFFF
	        ? CONVERT_TO_STRING
	          ? charAt$5(S, position)
	          : first
	        : CONVERT_TO_STRING
	          ? stringSlice$7(S, position, position + 2)
	          : (first - 0xD800 << 10) + (second - 0xDC00) + 0x10000;
	  };
	};

	var stringMultibyte = {
	  // `String.prototype.codePointAt` method
	  // https://tc39.es/ecma262/#sec-string.prototype.codepointat
	  codeAt: createMethod$1(false),
	  // `String.prototype.at` method
	  // https://github.com/mathiasbynens/String.prototype.at
	  charAt: createMethod$1(true)
	};

	var $$o = _export;
	var codeAt$1 = stringMultibyte.codeAt;

	// `String.prototype.codePointAt` method
	// https://tc39.es/ecma262/#sec-string.prototype.codepointat
	$$o({ target: 'String', proto: true }, {
	  codePointAt: function codePointAt(pos) {
	    return codeAt$1(this, pos);
	  }
	});

	var isObject$4 = isObject$n;
	var classof$3 = classofRaw$2;
	var wellKnownSymbol$8 = wellKnownSymbol$w;

	var MATCH$1 = wellKnownSymbol$8('match');

	// `IsRegExp` abstract operation
	// https://tc39.es/ecma262/#sec-isregexp
	var isRegexp = function (it) {
	  var isRegExp;
	  return isObject$4(it) && ((isRegExp = it[MATCH$1]) !== undefined ? !!isRegExp : classof$3(it) === 'RegExp');
	};

	var isRegExp$2 = isRegexp;

	var $TypeError$3 = TypeError;

	var notARegexp = function (it) {
	  if (isRegExp$2(it)) {
	    throw new $TypeError$3("The method doesn't accept regular expressions");
	  } return it;
	};

	var wellKnownSymbol$7 = wellKnownSymbol$w;

	var MATCH = wellKnownSymbol$7('match');

	var correctIsRegexpLogic = function (METHOD_NAME) {
	  var regexp = /./;
	  try {
	    '/./'[METHOD_NAME](regexp);
	  } catch (error1) {
	    try {
	      regexp[MATCH] = false;
	      return '/./'[METHOD_NAME](regexp);
	    } catch (error2) { /* empty */ }
	  } return false;
	};

	var $$n = _export;
	var uncurryThis$k = functionUncurryThisClause;
	var getOwnPropertyDescriptor$1 = objectGetOwnPropertyDescriptor.f;
	var toLength$4 = toLength$6;
	var toString$a = toString$o;
	var notARegExp$2 = notARegexp;
	var requireObjectCoercible$7 = requireObjectCoercible$g;
	var correctIsRegExpLogic$2 = correctIsRegexpLogic;

	var slice = uncurryThis$k(''.slice);
	var min$2 = Math.min;

	var CORRECT_IS_REGEXP_LOGIC$1 = correctIsRegExpLogic$2('endsWith');
	// https://github.com/zloirock/core-js/pull/702
	var MDN_POLYFILL_BUG$1 = !CORRECT_IS_REGEXP_LOGIC$1 && !!function () {
	  var descriptor = getOwnPropertyDescriptor$1(String.prototype, 'endsWith');
	  return descriptor && !descriptor.writable;
	}();

	// `String.prototype.endsWith` method
	// https://tc39.es/ecma262/#sec-string.prototype.endswith
	$$n({ target: 'String', proto: true, forced: !MDN_POLYFILL_BUG$1 && !CORRECT_IS_REGEXP_LOGIC$1 }, {
	  endsWith: function endsWith(searchString /* , endPosition = @length */) {
	    var that = toString$a(requireObjectCoercible$7(this));
	    notARegExp$2(searchString);
	    var endPosition = arguments.length > 1 ? arguments[1] : undefined;
	    var len = that.length;
	    var end = endPosition === undefined ? len : min$2(toLength$4(endPosition), len);
	    var search = toString$a(searchString);
	    return slice(that, end - search.length, end) === search;
	  }
	});

	var $$m = _export;
	var uncurryThis$j = functionUncurryThis;
	var toAbsoluteIndex = toAbsoluteIndex$4;

	var $RangeError$1 = RangeError;
	var fromCharCode$2 = String.fromCharCode;
	// eslint-disable-next-line es/no-string-fromcodepoint -- required for testing
	var $fromCodePoint = String.fromCodePoint;
	var join$3 = uncurryThis$j([].join);

	// length should be 1, old FF problem
	var INCORRECT_LENGTH = !!$fromCodePoint && $fromCodePoint.length !== 1;

	// `String.fromCodePoint` method
	// https://tc39.es/ecma262/#sec-string.fromcodepoint
	$$m({ target: 'String', stat: true, arity: 1, forced: INCORRECT_LENGTH }, {
	  // eslint-disable-next-line no-unused-vars -- required for `.length`
	  fromCodePoint: function fromCodePoint(x) {
	    var elements = [];
	    var length = arguments.length;
	    var i = 0;
	    var code;
	    while (length > i) {
	      code = +arguments[i++];
	      if (toAbsoluteIndex(code, 0x10FFFF) !== code) throw new $RangeError$1(code + ' is not a valid code point');
	      elements[i] = code < 0x10000
	        ? fromCharCode$2(code)
	        : fromCharCode$2(((code -= 0x10000) >> 10) + 0xD800, code % 0x400 + 0xDC00);
	    } return join$3(elements, '');
	  }
	});

	var $$l = _export;
	var uncurryThis$i = functionUncurryThis;
	var notARegExp$1 = notARegexp;
	var requireObjectCoercible$6 = requireObjectCoercible$g;
	var toString$9 = toString$o;
	var correctIsRegExpLogic$1 = correctIsRegexpLogic;

	var stringIndexOf$2 = uncurryThis$i(''.indexOf);

	// `String.prototype.includes` method
	// https://tc39.es/ecma262/#sec-string.prototype.includes
	$$l({ target: 'String', proto: true, forced: !correctIsRegExpLogic$1('includes') }, {
	  includes: function includes(searchString /* , position = 0 */) {
	    return !!~stringIndexOf$2(
	      toString$9(requireObjectCoercible$6(this)),
	      toString$9(notARegExp$1(searchString)),
	      arguments.length > 1 ? arguments[1] : undefined
	    );
	  }
	});

	var $$k = _export;
	var uncurryThis$h = functionUncurryThis;
	var requireObjectCoercible$5 = requireObjectCoercible$g;
	var toString$8 = toString$o;

	var charCodeAt$1 = uncurryThis$h(''.charCodeAt);

	// `String.prototype.isWellFormed` method
	// https://tc39.es/ecma262/#sec-string.prototype.iswellformed
	$$k({ target: 'String', proto: true }, {
	  isWellFormed: function isWellFormed() {
	    var S = toString$8(requireObjectCoercible$5(this));
	    var length = S.length;
	    for (var i = 0; i < length; i++) {
	      var charCode = charCodeAt$1(S, i);
	      // single UTF-16 code unit
	      if ((charCode & 0xF800) !== 0xD800) continue;
	      // unpaired surrogate
	      if (charCode >= 0xDC00 || ++i >= length || (charCodeAt$1(S, i) & 0xFC00) !== 0xDC00) return false;
	    } return true;
	  }
	});

	var charAt$4 = stringMultibyte.charAt;
	var toString$7 = toString$o;
	var InternalStateModule$5 = internalState;
	var defineIterator = iteratorDefine;
	var createIterResultObject$2 = createIterResultObject$6;

	var STRING_ITERATOR = 'String Iterator';
	var setInternalState$5 = InternalStateModule$5.set;
	var getInternalState$2 = InternalStateModule$5.getterFor(STRING_ITERATOR);

	// `String.prototype[@@iterator]` method
	// https://tc39.es/ecma262/#sec-string.prototype-@@iterator
	defineIterator(String, 'String', function (iterated) {
	  setInternalState$5(this, {
	    type: STRING_ITERATOR,
	    string: toString$7(iterated),
	    index: 0
	  });
	// `%StringIteratorPrototype%.next` method
	// https://tc39.es/ecma262/#sec-%stringiteratorprototype%.next
	}, function next() {
	  var state = getInternalState$2(this);
	  var string = state.string;
	  var index = state.index;
	  var point;
	  if (index >= string.length) return createIterResultObject$2(undefined, true);
	  point = charAt$4(string, index);
	  state.index += point.length;
	  return createIterResultObject$2(point, false);
	});

	var charAt$3 = stringMultibyte.charAt;

	// `AdvanceStringIndex` abstract operation
	// https://tc39.es/ecma262/#sec-advancestringindex
	var advanceStringIndex$2 = function (S, index, unicode) {
	  return index + (unicode ? charAt$3(S, index).length : 1);
	};

	var call$6 = functionCall;
	var anObject$7 = anObject$q;
	var isCallable$4 = isCallable$w;
	var classof$2 = classofRaw$2;
	var regexpExec$1 = regexpExec$2;

	var $TypeError$2 = TypeError;

	// `RegExpExec` abstract operation
	// https://tc39.es/ecma262/#sec-regexpexec
	var regexpExecAbstract = function (R, S) {
	  var exec = R.exec;
	  if (isCallable$4(exec)) {
	    var result = call$6(exec, R, S);
	    if (result !== null) anObject$7(result);
	    return result;
	  }
	  if (classof$2(R) === 'RegExp') return call$6(regexpExec$1, R, S);
	  throw new $TypeError$2('RegExp#exec called on incompatible receiver');
	};

	/* eslint-disable es/no-string-prototype-matchall -- safe */
	var $$j = _export;
	var call$5 = functionCall;
	var uncurryThis$g = functionUncurryThisClause;
	var createIteratorConstructor$1 = iteratorCreateConstructor;
	var createIterResultObject$1 = createIterResultObject$6;
	var requireObjectCoercible$4 = requireObjectCoercible$g;
	var toLength$3 = toLength$6;
	var toString$6 = toString$o;
	var anObject$6 = anObject$q;
	var isNullOrUndefined$4 = isNullOrUndefined$b;
	var isRegExp$1 = isRegexp;
	var getRegExpFlags$2 = regexpGetFlags;
	var getMethod$2 = getMethod$7;
	var defineBuiltIn$6 = defineBuiltIn$k;
	var fails$9 = fails$S;
	var wellKnownSymbol$6 = wellKnownSymbol$w;
	var speciesConstructor = speciesConstructor$3;
	var advanceStringIndex$1 = advanceStringIndex$2;
	var regExpExec$1 = regexpExecAbstract;
	var InternalStateModule$4 = internalState;

	var MATCH_ALL = wellKnownSymbol$6('matchAll');
	var REGEXP_STRING = 'RegExp String';
	var REGEXP_STRING_ITERATOR = REGEXP_STRING + ' Iterator';
	var setInternalState$4 = InternalStateModule$4.set;
	var getInternalState$1 = InternalStateModule$4.getterFor(REGEXP_STRING_ITERATOR);
	var RegExpPrototype$1 = RegExp.prototype;
	var $TypeError$1 = TypeError;
	var stringIndexOf$1 = uncurryThis$g(''.indexOf);
	var nativeMatchAll = uncurryThis$g(''.matchAll);

	var WORKS_WITH_NON_GLOBAL_REGEX = !!nativeMatchAll && !fails$9(function () {
	  nativeMatchAll('a', /./);
	});

	var $RegExpStringIterator = createIteratorConstructor$1(function RegExpStringIterator(regexp, string, $global, fullUnicode) {
	  setInternalState$4(this, {
	    type: REGEXP_STRING_ITERATOR,
	    regexp: regexp,
	    string: string,
	    global: $global,
	    unicode: fullUnicode,
	    done: false
	  });
	}, REGEXP_STRING, function next() {
	  var state = getInternalState$1(this);
	  if (state.done) return createIterResultObject$1(undefined, true);
	  var R = state.regexp;
	  var S = state.string;
	  var match = regExpExec$1(R, S);
	  if (match === null) {
	    state.done = true;
	    return createIterResultObject$1(undefined, true);
	  }
	  if (state.global) {
	    if (toString$6(match[0]) === '') R.lastIndex = advanceStringIndex$1(S, toLength$3(R.lastIndex), state.unicode);
	    return createIterResultObject$1(match, false);
	  }
	  state.done = true;
	  return createIterResultObject$1(match, false);
	});

	var $matchAll = function (string) {
	  var R = anObject$6(this);
	  var S = toString$6(string);
	  var C = speciesConstructor(R, RegExp);
	  var flags = toString$6(getRegExpFlags$2(R));
	  var matcher, $global, fullUnicode;
	  matcher = new C(C === RegExp ? R.source : R, flags);
	  $global = !!~stringIndexOf$1(flags, 'g');
	  fullUnicode = !!~stringIndexOf$1(flags, 'u');
	  matcher.lastIndex = toLength$3(R.lastIndex);
	  return new $RegExpStringIterator(matcher, S, $global, fullUnicode);
	};

	// `String.prototype.matchAll` method
	// https://tc39.es/ecma262/#sec-string.prototype.matchall
	$$j({ target: 'String', proto: true, forced: WORKS_WITH_NON_GLOBAL_REGEX }, {
	  matchAll: function matchAll(regexp) {
	    var O = requireObjectCoercible$4(this);
	    var flags, S, matcher, rx;
	    if (!isNullOrUndefined$4(regexp)) {
	      if (isRegExp$1(regexp)) {
	        flags = toString$6(requireObjectCoercible$4(getRegExpFlags$2(regexp)));
	        if (!~stringIndexOf$1(flags, 'g')) throw new $TypeError$1('`.matchAll` does not allow non-global regexes');
	      }
	      if (WORKS_WITH_NON_GLOBAL_REGEX) return nativeMatchAll(O, regexp);
	      matcher = getMethod$2(regexp, MATCH_ALL);
	      if (matcher) return call$5(matcher, regexp, O);
	    } else if (WORKS_WITH_NON_GLOBAL_REGEX) return nativeMatchAll(O, regexp);
	    S = toString$6(O);
	    rx = new RegExp(regexp, 'g');
	    return rx[MATCH_ALL](S);
	  }
	});

	MATCH_ALL in RegExpPrototype$1 || defineBuiltIn$6(RegExpPrototype$1, MATCH_ALL, $matchAll);

	// https://github.com/tc39/proposal-string-pad-start-end
	var uncurryThis$f = functionUncurryThis;
	var toLength$2 = toLength$6;
	var toString$5 = toString$o;
	var $repeat = stringRepeat;
	var requireObjectCoercible$3 = requireObjectCoercible$g;

	var repeat$1 = uncurryThis$f($repeat);
	var stringSlice$6 = uncurryThis$f(''.slice);
	var ceil = Math.ceil;

	// `String.prototype.{ padStart, padEnd }` methods implementation
	var createMethod = function (IS_END) {
	  return function ($this, maxLength, fillString) {
	    var S = toString$5(requireObjectCoercible$3($this));
	    var intMaxLength = toLength$2(maxLength);
	    var stringLength = S.length;
	    var fillStr = fillString === undefined ? ' ' : toString$5(fillString);
	    var fillLen, stringFiller;
	    if (intMaxLength <= stringLength || fillStr === '') return S;
	    fillLen = intMaxLength - stringLength;
	    stringFiller = repeat$1(fillStr, ceil(fillLen / fillStr.length));
	    if (stringFiller.length > fillLen) stringFiller = stringSlice$6(stringFiller, 0, fillLen);
	    return IS_END ? S + stringFiller : stringFiller + S;
	  };
	};

	var stringPad = {
	  // `String.prototype.padStart` method
	  // https://tc39.es/ecma262/#sec-string.prototype.padstart
	  start: createMethod(false),
	  // `String.prototype.padEnd` method
	  // https://tc39.es/ecma262/#sec-string.prototype.padend
	  end: createMethod(true)
	};

	// https://github.com/zloirock/core-js/issues/280
	var userAgent = environmentUserAgent;

	var stringPadWebkitBug = /Version\/10(?:\.\d+){1,2}(?: [\w./]+)?(?: Mobile\/\w+)? Safari\//.test(userAgent);

	var $$i = _export;
	var $padEnd = stringPad.end;
	var WEBKIT_BUG$1 = stringPadWebkitBug;

	// `String.prototype.padEnd` method
	// https://tc39.es/ecma262/#sec-string.prototype.padend
	$$i({ target: 'String', proto: true, forced: WEBKIT_BUG$1 }, {
	  padEnd: function padEnd(maxLength /* , fillString = ' ' */) {
	    return $padEnd(this, maxLength, arguments.length > 1 ? arguments[1] : undefined);
	  }
	});

	var $$h = _export;
	var $padStart = stringPad.start;
	var WEBKIT_BUG = stringPadWebkitBug;

	// `String.prototype.padStart` method
	// https://tc39.es/ecma262/#sec-string.prototype.padstart
	$$h({ target: 'String', proto: true, forced: WEBKIT_BUG }, {
	  padStart: function padStart(maxLength /* , fillString = ' ' */) {
	    return $padStart(this, maxLength, arguments.length > 1 ? arguments[1] : undefined);
	  }
	});

	var $$g = _export;
	var repeat = stringRepeat;

	// `String.prototype.repeat` method
	// https://tc39.es/ecma262/#sec-string.prototype.repeat
	$$g({ target: 'String', proto: true }, {
	  repeat: repeat
	});

	// TODO: Remove from `core-js@4` since it's moved to entry points

	var call$4 = functionCall;
	var defineBuiltIn$5 = defineBuiltIn$k;
	var regexpExec = regexpExec$2;
	var fails$8 = fails$S;
	var wellKnownSymbol$5 = wellKnownSymbol$w;
	var createNonEnumerableProperty$2 = createNonEnumerableProperty$b;

	var SPECIES = wellKnownSymbol$5('species');
	var RegExpPrototype = RegExp.prototype;

	var fixRegexpWellKnownSymbolLogic = function (KEY, exec, FORCED, SHAM) {
	  var SYMBOL = wellKnownSymbol$5(KEY);

	  var DELEGATES_TO_SYMBOL = !fails$8(function () {
	    // String methods call symbol-named RegExp methods
	    var O = {};
	    O[SYMBOL] = function () { return 7; };
	    return ''[KEY](O) !== 7;
	  });

	  var DELEGATES_TO_EXEC = DELEGATES_TO_SYMBOL && !fails$8(function () {
	    // Symbol-named RegExp methods call .exec
	    var execCalled = false;
	    var re = /a/;

	    if (KEY === 'split') {
	      // We can't use real regex here since it causes deoptimization
	      // and serious performance degradation in V8
	      // https://github.com/zloirock/core-js/issues/306
	      re = {};
	      // RegExp[@@split] doesn't call the regex's exec method, but first creates
	      // a new one. We need to return the patched regex when creating the new one.
	      re.constructor = {};
	      re.constructor[SPECIES] = function () { return re; };
	      re.flags = '';
	      re[SYMBOL] = /./[SYMBOL];
	    }

	    re.exec = function () {
	      execCalled = true;
	      return null;
	    };

	    re[SYMBOL]('');
	    return !execCalled;
	  });

	  if (
	    !DELEGATES_TO_SYMBOL ||
	    !DELEGATES_TO_EXEC ||
	    FORCED
	  ) {
	    var nativeRegExpMethod = /./[SYMBOL];
	    var methods = exec(SYMBOL, ''[KEY], function (nativeMethod, regexp, str, arg2, forceStringMethod) {
	      var $exec = regexp.exec;
	      if ($exec === regexpExec || $exec === RegExpPrototype.exec) {
	        if (DELEGATES_TO_SYMBOL && !forceStringMethod) {
	          // The native String method already delegates to @@method (this
	          // polyfilled function), leasing to infinite recursion.
	          // We avoid it by directly calling the native @@method method.
	          return { done: true, value: call$4(nativeRegExpMethod, regexp, str, arg2) };
	        }
	        return { done: true, value: call$4(nativeMethod, str, regexp, arg2) };
	      }
	      return { done: false };
	    });

	    defineBuiltIn$5(String.prototype, KEY, methods[0]);
	    defineBuiltIn$5(RegExpPrototype, SYMBOL, methods[1]);
	  }

	  if (SHAM) createNonEnumerableProperty$2(RegExpPrototype[SYMBOL], 'sham', true);
	};

	var uncurryThis$e = functionUncurryThis;
	var toObject = toObject$g;

	var floor$2 = Math.floor;
	var charAt$2 = uncurryThis$e(''.charAt);
	var replace$3 = uncurryThis$e(''.replace);
	var stringSlice$5 = uncurryThis$e(''.slice);
	// eslint-disable-next-line redos/no-vulnerable -- safe
	var SUBSTITUTION_SYMBOLS = /\$([$&'`]|\d{1,2}|<[^>]*>)/g;
	var SUBSTITUTION_SYMBOLS_NO_NAMED = /\$([$&'`]|\d{1,2})/g;

	// `GetSubstitution` abstract operation
	// https://tc39.es/ecma262/#sec-getsubstitution
	var getSubstitution$2 = function (matched, str, position, captures, namedCaptures, replacement) {
	  var tailPos = position + matched.length;
	  var m = captures.length;
	  var symbols = SUBSTITUTION_SYMBOLS_NO_NAMED;
	  if (namedCaptures !== undefined) {
	    namedCaptures = toObject(namedCaptures);
	    symbols = SUBSTITUTION_SYMBOLS;
	  }
	  return replace$3(replacement, symbols, function (match, ch) {
	    var capture;
	    switch (charAt$2(ch, 0)) {
	      case '$': return '$';
	      case '&': return matched;
	      case '`': return stringSlice$5(str, 0, position);
	      case "'": return stringSlice$5(str, tailPos);
	      case '<':
	        capture = namedCaptures[stringSlice$5(ch, 1, -1)];
	        break;
	      default: // \d\d?
	        var n = +ch;
	        if (n === 0) return match;
	        if (n > m) {
	          var f = floor$2(n / 10);
	          if (f === 0) return match;
	          if (f <= m) return captures[f - 1] === undefined ? charAt$2(ch, 1) : captures[f - 1] + charAt$2(ch, 1);
	          return match;
	        }
	        capture = captures[n - 1];
	    }
	    return capture === undefined ? '' : capture;
	  });
	};

	var apply = functionApply;
	var call$3 = functionCall;
	var uncurryThis$d = functionUncurryThis;
	var fixRegExpWellKnownSymbolLogic = fixRegexpWellKnownSymbolLogic;
	var fails$7 = fails$S;
	var anObject$5 = anObject$q;
	var isCallable$3 = isCallable$w;
	var isNullOrUndefined$3 = isNullOrUndefined$b;
	var toIntegerOrInfinity = toIntegerOrInfinity$b;
	var toLength$1 = toLength$6;
	var toString$4 = toString$o;
	var requireObjectCoercible$2 = requireObjectCoercible$g;
	var advanceStringIndex = advanceStringIndex$2;
	var getMethod$1 = getMethod$7;
	var getSubstitution$1 = getSubstitution$2;
	var regExpExec = regexpExecAbstract;
	var wellKnownSymbol$4 = wellKnownSymbol$w;

	var REPLACE$1 = wellKnownSymbol$4('replace');
	var max$1 = Math.max;
	var min$1 = Math.min;
	var concat = uncurryThis$d([].concat);
	var push$5 = uncurryThis$d([].push);
	var stringIndexOf = uncurryThis$d(''.indexOf);
	var stringSlice$4 = uncurryThis$d(''.slice);

	var maybeToString = function (it) {
	  return it === undefined ? it : String(it);
	};

	// IE <= 11 replaces $0 with the whole match, as if it was $&
	// https://stackoverflow.com/questions/6024666/getting-ie-to-replace-a-regex-with-the-literal-string-0
	var REPLACE_KEEPS_$0 = (function () {
	  // eslint-disable-next-line regexp/prefer-escape-replacement-dollar-char -- required for testing
	  return 'a'.replace(/./, '$0') === '$0';
	})();

	// Safari <= 13.0.3(?) substitutes nth capture where n>m with an empty string
	var REGEXP_REPLACE_SUBSTITUTES_UNDEFINED_CAPTURE = (function () {
	  if (/./[REPLACE$1]) {
	    return /./[REPLACE$1]('a', '$0') === '';
	  }
	  return false;
	})();

	var REPLACE_SUPPORTS_NAMED_GROUPS = !fails$7(function () {
	  var re = /./;
	  re.exec = function () {
	    var result = [];
	    result.groups = { a: '7' };
	    return result;
	  };
	  // eslint-disable-next-line regexp/no-useless-dollar-replacements -- false positive
	  return ''.replace(re, '$<a>') !== '7';
	});

	// @@replace logic
	fixRegExpWellKnownSymbolLogic('replace', function (_, nativeReplace, maybeCallNative) {
	  var UNSAFE_SUBSTITUTE = REGEXP_REPLACE_SUBSTITUTES_UNDEFINED_CAPTURE ? '$' : '$0';

	  return [
	    // `String.prototype.replace` method
	    // https://tc39.es/ecma262/#sec-string.prototype.replace
	    function replace(searchValue, replaceValue) {
	      var O = requireObjectCoercible$2(this);
	      var replacer = isNullOrUndefined$3(searchValue) ? undefined : getMethod$1(searchValue, REPLACE$1);
	      return replacer
	        ? call$3(replacer, searchValue, O, replaceValue)
	        : call$3(nativeReplace, toString$4(O), searchValue, replaceValue);
	    },
	    // `RegExp.prototype[@@replace]` method
	    // https://tc39.es/ecma262/#sec-regexp.prototype-@@replace
	    function (string, replaceValue) {
	      var rx = anObject$5(this);
	      var S = toString$4(string);

	      if (
	        typeof replaceValue == 'string' &&
	        stringIndexOf(replaceValue, UNSAFE_SUBSTITUTE) === -1 &&
	        stringIndexOf(replaceValue, '$<') === -1
	      ) {
	        var res = maybeCallNative(nativeReplace, rx, S, replaceValue);
	        if (res.done) return res.value;
	      }

	      var functionalReplace = isCallable$3(replaceValue);
	      if (!functionalReplace) replaceValue = toString$4(replaceValue);

	      var global = rx.global;
	      var fullUnicode;
	      if (global) {
	        fullUnicode = rx.unicode;
	        rx.lastIndex = 0;
	      }

	      var results = [];
	      var result;
	      while (true) {
	        result = regExpExec(rx, S);
	        if (result === null) break;

	        push$5(results, result);
	        if (!global) break;

	        var matchStr = toString$4(result[0]);
	        if (matchStr === '') rx.lastIndex = advanceStringIndex(S, toLength$1(rx.lastIndex), fullUnicode);
	      }

	      var accumulatedResult = '';
	      var nextSourcePosition = 0;
	      for (var i = 0; i < results.length; i++) {
	        result = results[i];

	        var matched = toString$4(result[0]);
	        var position = max$1(min$1(toIntegerOrInfinity(result.index), S.length), 0);
	        var captures = [];
	        var replacement;
	        // NOTE: This is equivalent to
	        //   captures = result.slice(1).map(maybeToString)
	        // but for some reason `nativeSlice.call(result, 1, result.length)` (called in
	        // the slice polyfill when slicing native arrays) "doesn't work" in safari 9 and
	        // causes a crash (https://pastebin.com/N21QzeQA) when trying to debug it.
	        for (var j = 1; j < result.length; j++) push$5(captures, maybeToString(result[j]));
	        var namedCaptures = result.groups;
	        if (functionalReplace) {
	          var replacerArgs = concat([matched], captures, position, S);
	          if (namedCaptures !== undefined) push$5(replacerArgs, namedCaptures);
	          replacement = toString$4(apply(replaceValue, undefined, replacerArgs));
	        } else {
	          replacement = getSubstitution$1(matched, S, position, captures, namedCaptures, replaceValue);
	        }
	        if (position >= nextSourcePosition) {
	          accumulatedResult += stringSlice$4(S, nextSourcePosition, position) + replacement;
	          nextSourcePosition = position + matched.length;
	        }
	      }

	      return accumulatedResult + stringSlice$4(S, nextSourcePosition);
	    }
	  ];
	}, !REPLACE_SUPPORTS_NAMED_GROUPS || !REPLACE_KEEPS_$0 || REGEXP_REPLACE_SUBSTITUTES_UNDEFINED_CAPTURE);

	var $$f = _export;
	var call$2 = functionCall;
	var uncurryThis$c = functionUncurryThis;
	var requireObjectCoercible$1 = requireObjectCoercible$g;
	var isCallable$2 = isCallable$w;
	var isNullOrUndefined$2 = isNullOrUndefined$b;
	var isRegExp = isRegexp;
	var toString$3 = toString$o;
	var getMethod = getMethod$7;
	var getRegExpFlags$1 = regexpGetFlags;
	var getSubstitution = getSubstitution$2;
	var wellKnownSymbol$3 = wellKnownSymbol$w;

	var REPLACE = wellKnownSymbol$3('replace');
	var $TypeError = TypeError;
	var indexOf = uncurryThis$c(''.indexOf);
	uncurryThis$c(''.replace);
	var stringSlice$3 = uncurryThis$c(''.slice);
	var max = Math.max;

	// `String.prototype.replaceAll` method
	// https://tc39.es/ecma262/#sec-string.prototype.replaceall
	$$f({ target: 'String', proto: true }, {
	  replaceAll: function replaceAll(searchValue, replaceValue) {
	    var O = requireObjectCoercible$1(this);
	    var IS_REG_EXP, flags, replacer, string, searchString, functionalReplace, searchLength, advanceBy, position, replacement;
	    var endOfLastMatch = 0;
	    var result = '';
	    if (!isNullOrUndefined$2(searchValue)) {
	      IS_REG_EXP = isRegExp(searchValue);
	      if (IS_REG_EXP) {
	        flags = toString$3(requireObjectCoercible$1(getRegExpFlags$1(searchValue)));
	        if (!~indexOf(flags, 'g')) throw new $TypeError('`.replaceAll` does not allow non-global regexes');
	      }
	      replacer = getMethod(searchValue, REPLACE);
	      if (replacer) return call$2(replacer, searchValue, O, replaceValue);
	    }
	    string = toString$3(O);
	    searchString = toString$3(searchValue);
	    functionalReplace = isCallable$2(replaceValue);
	    if (!functionalReplace) replaceValue = toString$3(replaceValue);
	    searchLength = searchString.length;
	    advanceBy = max(1, searchLength);
	    position = indexOf(string, searchString);
	    while (position !== -1) {
	      replacement = functionalReplace
	        ? toString$3(replaceValue(searchString, position, string))
	        : getSubstitution(searchString, string, position, [], undefined, replaceValue);
	      result += stringSlice$3(string, endOfLastMatch, position) + replacement;
	      endOfLastMatch = position + searchLength;
	      position = position + advanceBy > string.length ? -1 : indexOf(string, searchString, position + advanceBy);
	    }
	    if (endOfLastMatch < string.length) {
	      result += stringSlice$3(string, endOfLastMatch);
	    }
	    return result;
	  }
	});

	var $$e = _export;
	var uncurryThis$b = functionUncurryThisClause;
	var getOwnPropertyDescriptor = objectGetOwnPropertyDescriptor.f;
	var toLength = toLength$6;
	var toString$2 = toString$o;
	var notARegExp = notARegexp;
	var requireObjectCoercible = requireObjectCoercible$g;
	var correctIsRegExpLogic = correctIsRegexpLogic;

	var stringSlice$2 = uncurryThis$b(''.slice);
	var min = Math.min;

	var CORRECT_IS_REGEXP_LOGIC = correctIsRegExpLogic('startsWith');
	// https://github.com/zloirock/core-js/pull/702
	var MDN_POLYFILL_BUG = !CORRECT_IS_REGEXP_LOGIC && !!function () {
	  var descriptor = getOwnPropertyDescriptor(String.prototype, 'startsWith');
	  return descriptor && !descriptor.writable;
	}();

	// `String.prototype.startsWith` method
	// https://tc39.es/ecma262/#sec-string.prototype.startswith
	$$e({ target: 'String', proto: true, forced: !MDN_POLYFILL_BUG && !CORRECT_IS_REGEXP_LOGIC }, {
	  startsWith: function startsWith(searchString /* , position = 0 */) {
	    var that = toString$2(requireObjectCoercible(this));
	    notARegExp(searchString);
	    var index = toLength(min(arguments.length > 1 ? arguments[1] : undefined, that.length));
	    var search = toString$2(searchString);
	    return stringSlice$2(that, index, index + search.length) === search;
	  }
	});

	var PROPER_FUNCTION_NAME = functionName.PROPER;
	var fails$6 = fails$S;
	var whitespaces = whitespaces$3;

	var non = '\u200B\u0085\u180E';

	// check that a method works with the correct list
	// of whitespaces and has a correct name
	var stringTrimForced = function (METHOD_NAME) {
	  return fails$6(function () {
	    return !!whitespaces[METHOD_NAME]()
	      || non[METHOD_NAME]() !== non
	      || (PROPER_FUNCTION_NAME && whitespaces[METHOD_NAME].name !== METHOD_NAME);
	  });
	};

	var $trimEnd = stringTrim.end;
	var forcedStringTrimMethod$1 = stringTrimForced;

	// `String.prototype.{ trimEnd, trimRight }` method
	// https://tc39.es/ecma262/#sec-string.prototype.trimend
	// https://tc39.es/ecma262/#String.prototype.trimright
	var stringTrimEnd = forcedStringTrimMethod$1('trimEnd') ? function trimEnd() {
	  return $trimEnd(this);
	// eslint-disable-next-line es/no-string-prototype-trimstart-trimend -- safe
	} : ''.trimEnd;

	var $$d = _export;
	var trimEnd$1 = stringTrimEnd;

	// `String.prototype.trimRight` method
	// https://tc39.es/ecma262/#sec-string.prototype.trimend
	// eslint-disable-next-line es/no-string-prototype-trimleft-trimright -- safe
	$$d({ target: 'String', proto: true, name: 'trimEnd', forced: ''.trimRight !== trimEnd$1 }, {
	  trimRight: trimEnd$1
	});

	// TODO: Remove this line from `core-js@4`

	var $$c = _export;
	var trimEnd = stringTrimEnd;

	// `String.prototype.trimEnd` method
	// https://tc39.es/ecma262/#sec-string.prototype.trimend
	// eslint-disable-next-line es/no-string-prototype-trimstart-trimend -- safe
	$$c({ target: 'String', proto: true, name: 'trimEnd', forced: ''.trimEnd !== trimEnd }, {
	  trimEnd: trimEnd
	});

	var $trimStart = stringTrim.start;
	var forcedStringTrimMethod = stringTrimForced;

	// `String.prototype.{ trimStart, trimLeft }` method
	// https://tc39.es/ecma262/#sec-string.prototype.trimstart
	// https://tc39.es/ecma262/#String.prototype.trimleft
	var stringTrimStart = forcedStringTrimMethod('trimStart') ? function trimStart() {
	  return $trimStart(this);
	// eslint-disable-next-line es/no-string-prototype-trimstart-trimend -- safe
	} : ''.trimStart;

	var $$b = _export;
	var trimStart$1 = stringTrimStart;

	// `String.prototype.trimLeft` method
	// https://tc39.es/ecma262/#sec-string.prototype.trimleft
	// eslint-disable-next-line es/no-string-prototype-trimleft-trimright -- safe
	$$b({ target: 'String', proto: true, name: 'trimStart', forced: ''.trimLeft !== trimStart$1 }, {
	  trimLeft: trimStart$1
	});

	// TODO: Remove this line from `core-js@4`

	var $$a = _export;
	var trimStart = stringTrimStart;

	// `String.prototype.trimStart` method
	// https://tc39.es/ecma262/#sec-string.prototype.trimstart
	// eslint-disable-next-line es/no-string-prototype-trimstart-trimend -- safe
	$$a({ target: 'String', proto: true, name: 'trimStart', forced: ''.trimStart !== trimStart }, {
	  trimStart: trimStart
	});

	var uncurryThis$a = functionUncurryThis;
	var defineBuiltIns$2 = defineBuiltIns$5;
	var getWeakData = internalMetadataExports.getWeakData;
	var anInstance$4 = anInstance$9;
	var anObject$4 = anObject$q;
	var isNullOrUndefined$1 = isNullOrUndefined$b;
	var isObject$3 = isObject$n;
	var iterate$1 = iterate$d;
	var ArrayIterationModule = arrayIteration;
	var hasOwn$5 = hasOwnProperty_1;
	var InternalStateModule$3 = internalState;

	var setInternalState$3 = InternalStateModule$3.set;
	var internalStateGetterFor = InternalStateModule$3.getterFor;
	var find$1 = ArrayIterationModule.find;
	var findIndex = ArrayIterationModule.findIndex;
	var splice$1 = uncurryThis$a([].splice);
	var id = 0;

	// fallback for uncaught frozen keys
	var uncaughtFrozenStore = function (state) {
	  return state.frozen || (state.frozen = new UncaughtFrozenStore());
	};

	var UncaughtFrozenStore = function () {
	  this.entries = [];
	};

	var findUncaughtFrozen = function (store, key) {
	  return find$1(store.entries, function (it) {
	    return it[0] === key;
	  });
	};

	UncaughtFrozenStore.prototype = {
	  get: function (key) {
	    var entry = findUncaughtFrozen(this, key);
	    if (entry) return entry[1];
	  },
	  has: function (key) {
	    return !!findUncaughtFrozen(this, key);
	  },
	  set: function (key, value) {
	    var entry = findUncaughtFrozen(this, key);
	    if (entry) entry[1] = value;
	    else this.entries.push([key, value]);
	  },
	  'delete': function (key) {
	    var index = findIndex(this.entries, function (it) {
	      return it[0] === key;
	    });
	    if (~index) splice$1(this.entries, index, 1);
	    return !!~index;
	  }
	};

	var collectionWeak$1 = {
	  getConstructor: function (wrapper, CONSTRUCTOR_NAME, IS_MAP, ADDER) {
	    var Constructor = wrapper(function (that, iterable) {
	      anInstance$4(that, Prototype);
	      setInternalState$3(that, {
	        type: CONSTRUCTOR_NAME,
	        id: id++,
	        frozen: null
	      });
	      if (!isNullOrUndefined$1(iterable)) iterate$1(iterable, that[ADDER], { that: that, AS_ENTRIES: IS_MAP });
	    });

	    var Prototype = Constructor.prototype;

	    var getInternalState = internalStateGetterFor(CONSTRUCTOR_NAME);

	    var define = function (that, key, value) {
	      var state = getInternalState(that);
	      var data = getWeakData(anObject$4(key), true);
	      if (data === true) uncaughtFrozenStore(state).set(key, value);
	      else data[state.id] = value;
	      return that;
	    };

	    defineBuiltIns$2(Prototype, {
	      // `{ WeakMap, WeakSet }.prototype.delete(key)` methods
	      // https://tc39.es/ecma262/#sec-weakmap.prototype.delete
	      // https://tc39.es/ecma262/#sec-weakset.prototype.delete
	      'delete': function (key) {
	        var state = getInternalState(this);
	        if (!isObject$3(key)) return false;
	        var data = getWeakData(key);
	        if (data === true) return uncaughtFrozenStore(state)['delete'](key);
	        return data && hasOwn$5(data, state.id) && delete data[state.id];
	      },
	      // `{ WeakMap, WeakSet }.prototype.has(key)` methods
	      // https://tc39.es/ecma262/#sec-weakmap.prototype.has
	      // https://tc39.es/ecma262/#sec-weakset.prototype.has
	      has: function has(key) {
	        var state = getInternalState(this);
	        if (!isObject$3(key)) return false;
	        var data = getWeakData(key);
	        if (data === true) return uncaughtFrozenStore(state).has(key);
	        return data && hasOwn$5(data, state.id);
	      }
	    });

	    defineBuiltIns$2(Prototype, IS_MAP ? {
	      // `WeakMap.prototype.get(key)` method
	      // https://tc39.es/ecma262/#sec-weakmap.prototype.get
	      get: function get(key) {
	        var state = getInternalState(this);
	        if (isObject$3(key)) {
	          var data = getWeakData(key);
	          if (data === true) return uncaughtFrozenStore(state).get(key);
	          if (data) return data[state.id];
	        }
	      },
	      // `WeakMap.prototype.set(key, value)` method
	      // https://tc39.es/ecma262/#sec-weakmap.prototype.set
	      set: function set(key, value) {
	        return define(this, key, value);
	      }
	    } : {
	      // `WeakSet.prototype.add(value)` method
	      // https://tc39.es/ecma262/#sec-weakset.prototype.add
	      add: function add(value) {
	        return define(this, value, true);
	      }
	    });

	    return Constructor;
	  }
	};

	var FREEZING = freezing;
	var globalThis$9 = globalThis_1;
	var uncurryThis$9 = functionUncurryThis;
	var defineBuiltIns$1 = defineBuiltIns$5;
	var InternalMetadataModule = internalMetadataExports;
	var collection = collection$3;
	var collectionWeak = collectionWeak$1;
	var isObject$2 = isObject$n;
	var enforceInternalState = internalState.enforce;
	var fails$5 = fails$S;
	var NATIVE_WEAK_MAP = weakMapBasicDetection;

	var $Object = Object;
	// eslint-disable-next-line es/no-array-isarray -- safe
	var isArray = Array.isArray;
	// eslint-disable-next-line es/no-object-isextensible -- safe
	var isExtensible = $Object.isExtensible;
	// eslint-disable-next-line es/no-object-isfrozen -- safe
	var isFrozen = $Object.isFrozen;
	// eslint-disable-next-line es/no-object-issealed -- safe
	var isSealed = $Object.isSealed;
	// eslint-disable-next-line es/no-object-freeze -- safe
	var freeze = $Object.freeze;
	// eslint-disable-next-line es/no-object-seal -- safe
	var seal = $Object.seal;

	var IS_IE11 = !globalThis$9.ActiveXObject && 'ActiveXObject' in globalThis$9;
	var InternalWeakMap;

	var wrapper = function (init) {
	  return function WeakMap() {
	    return init(this, arguments.length ? arguments[0] : undefined);
	  };
	};

	// `WeakMap` constructor
	// https://tc39.es/ecma262/#sec-weakmap-constructor
	var $WeakMap = collection('WeakMap', wrapper, collectionWeak);
	var WeakMapPrototype$1 = $WeakMap.prototype;
	var nativeSet = uncurryThis$9(WeakMapPrototype$1.set);

	// Chakra Edge bug: adding frozen arrays to WeakMap unfreeze them
	var hasMSEdgeFreezingBug = function () {
	  return FREEZING && fails$5(function () {
	    var frozenArray = freeze([]);
	    nativeSet(new $WeakMap(), frozenArray, 1);
	    return !isFrozen(frozenArray);
	  });
	};

	// IE11 WeakMap frozen keys fix
	// We can't use feature detection because it crash some old IE builds
	// https://github.com/zloirock/core-js/issues/485
	if (NATIVE_WEAK_MAP) if (IS_IE11) {
	  InternalWeakMap = collectionWeak.getConstructor(wrapper, 'WeakMap', true);
	  InternalMetadataModule.enable();
	  var nativeDelete = uncurryThis$9(WeakMapPrototype$1['delete']);
	  var nativeHas = uncurryThis$9(WeakMapPrototype$1.has);
	  var nativeGet = uncurryThis$9(WeakMapPrototype$1.get);
	  defineBuiltIns$1(WeakMapPrototype$1, {
	    'delete': function (key) {
	      if (isObject$2(key) && !isExtensible(key)) {
	        var state = enforceInternalState(this);
	        if (!state.frozen) state.frozen = new InternalWeakMap();
	        return nativeDelete(this, key) || state.frozen['delete'](key);
	      } return nativeDelete(this, key);
	    },
	    has: function has(key) {
	      if (isObject$2(key) && !isExtensible(key)) {
	        var state = enforceInternalState(this);
	        if (!state.frozen) state.frozen = new InternalWeakMap();
	        return nativeHas(this, key) || state.frozen.has(key);
	      } return nativeHas(this, key);
	    },
	    get: function get(key) {
	      if (isObject$2(key) && !isExtensible(key)) {
	        var state = enforceInternalState(this);
	        if (!state.frozen) state.frozen = new InternalWeakMap();
	        return nativeHas(this, key) ? nativeGet(this, key) : state.frozen.get(key);
	      } return nativeGet(this, key);
	    },
	    set: function set(key, value) {
	      if (isObject$2(key) && !isExtensible(key)) {
	        var state = enforceInternalState(this);
	        if (!state.frozen) state.frozen = new InternalWeakMap();
	        nativeHas(this, key) ? nativeSet(this, key, value) : state.frozen.set(key, value);
	      } else nativeSet(this, key, value);
	      return this;
	    }
	  });
	// Chakra Edge frozen keys fix
	} else if (hasMSEdgeFreezingBug()) {
	  defineBuiltIns$1(WeakMapPrototype$1, {
	    set: function set(key, value) {
	      var arrayIntegrityLevel;
	      if (isArray(key)) {
	        if (isFrozen(key)) arrayIntegrityLevel = freeze;
	        else if (isSealed(key)) arrayIntegrityLevel = seal;
	      }
	      nativeSet(this, key, value);
	      if (arrayIntegrityLevel) arrayIntegrityLevel(key);
	      return this;
	    }
	  });
	}

	var uncurryThis$8 = functionUncurryThis;

	// eslint-disable-next-line es/no-map -- safe
	var MapPrototype = Map.prototype;

	var mapHelpers = {
	  // eslint-disable-next-line es/no-map -- safe
	  Map: Map,
	  set: uncurryThis$8(MapPrototype.set),
	  get: uncurryThis$8(MapPrototype.get),
	  has: uncurryThis$8(MapPrototype.has),
	  remove: uncurryThis$8(MapPrototype['delete'])};

	var has$5 = mapHelpers.has;

	// Perform ? RequireInternalSlot(M, [[MapData]])
	var aMap$2 = function (it) {
	  has$5(it);
	  return it;
	};

	var $$9 = _export;
	var aMap$1 = aMap$2;
	var MapHelpers$2 = mapHelpers;

	var get$3 = MapHelpers$2.get;
	var has$4 = MapHelpers$2.has;
	var set$3 = MapHelpers$2.set;

	// `Map.prototype.getOrInsert` method
	// https://github.com/tc39/proposal-upsert
	$$9({ target: 'Map', proto: true, real: true, forced: true }, {
	  getOrInsert: function getOrInsert(key, value) {
	    if (has$4(aMap$1(this), key)) return get$3(this, key);
	    set$3(this, key, value);
	    return value;
	  }
	});

	var $$8 = _export;
	var aCallable$1 = aCallable$i;
	var aMap = aMap$2;
	var MapHelpers$1 = mapHelpers;

	var get$2 = MapHelpers$1.get;
	var has$3 = MapHelpers$1.has;
	var set$2 = MapHelpers$1.set;

	// `Map.prototype.getOrInsertComputed` method
	// https://github.com/tc39/proposal-upsert
	$$8({ target: 'Map', proto: true, real: true, forced: true }, {
	  getOrInsertComputed: function getOrInsertComputed(key, callbackfn) {
	    aMap(this);
	    aCallable$1(callbackfn);
	    if (has$3(this, key)) return get$2(this, key);
	    // CanonicalizeKeyedCollectionKey
	    if (key === 0 && 1 / key === -Infinity) key = 0;
	    var value = callbackfn(key);
	    set$2(this, key, value);
	    return value;
	  }
	});

	var uncurryThis$7 = functionUncurryThis;

	// eslint-disable-next-line es/no-weak-map -- safe
	var WeakMapPrototype = WeakMap.prototype;

	var weakMapHelpers = {
	  set: uncurryThis$7(WeakMapPrototype.set),
	  get: uncurryThis$7(WeakMapPrototype.get),
	  has: uncurryThis$7(WeakMapPrototype.has),
	  remove: uncurryThis$7(WeakMapPrototype['delete'])
	};

	var has$2 = weakMapHelpers.has;

	// Perform ? RequireInternalSlot(M, [[WeakMapData]])
	var aWeakMap$2 = function (it) {
	  has$2(it);
	  return it;
	};

	var $$7 = _export;
	var aWeakMap$1 = aWeakMap$2;
	var WeakMapHelpers$1 = weakMapHelpers;

	var get$1 = WeakMapHelpers$1.get;
	var has$1 = WeakMapHelpers$1.has;
	var set$1 = WeakMapHelpers$1.set;

	// `WeakMap.prototype.getOrInsert` method
	// https://github.com/tc39/proposal-upsert
	$$7({ target: 'WeakMap', proto: true, real: true, forced: true }, {
	  getOrInsert: function getOrInsert(key, value) {
	    if (has$1(aWeakMap$1(this), key)) return get$1(this, key);
	    set$1(this, key, value);
	    return value;
	  }
	});

	var $$6 = _export;
	var aCallable = aCallable$i;
	var aWeakMap = aWeakMap$2;
	var WeakMapHelpers = weakMapHelpers;

	var get = WeakMapHelpers.get;
	var has = WeakMapHelpers.has;
	var set = WeakMapHelpers.set;

	// `WeakMap.prototype.getOrInsertComputed` method
	// https://github.com/tc39/proposal-upsert
	$$6({ target: 'WeakMap', proto: true, real: true, forced: true }, {
	  getOrInsertComputed: function getOrInsertComputed(key, callbackfn) {
	    aWeakMap(this);
	    aCallable(callbackfn);
	    if (has(this, key)) return get(this, key);
	    set(this, key); // key validation
	    var value = callbackfn(key);
	    set(this, key, value);
	    return value;
	  }
	});

	// iterable DOM collections
	// flag - `iterable` interface - 'entries', 'keys', 'values', 'forEach' methods
	var domIterables = {
	  CSSRuleList: 0,
	  CSSStyleDeclaration: 0,
	  CSSValueList: 0,
	  ClientRectList: 0,
	  DOMRectList: 0,
	  DOMStringList: 0,
	  DOMTokenList: 1,
	  DataTransferItemList: 0,
	  FileList: 0,
	  HTMLAllCollection: 0,
	  HTMLCollection: 0,
	  HTMLFormElement: 0,
	  HTMLSelectElement: 0,
	  MediaList: 0,
	  MimeTypeArray: 0,
	  NamedNodeMap: 0,
	  NodeList: 1,
	  PaintRequestList: 0,
	  Plugin: 0,
	  PluginArray: 0,
	  SVGLengthList: 0,
	  SVGNumberList: 0,
	  SVGPathSegList: 0,
	  SVGPointList: 0,
	  SVGStringList: 0,
	  SVGTransformList: 0,
	  SourceBufferList: 0,
	  StyleSheetList: 0,
	  TextTrackCueList: 0,
	  TextTrackList: 0,
	  TouchList: 0
	};

	// in old WebKit versions, `element.classList` is not an instance of global `DOMTokenList`
	var documentCreateElement = documentCreateElement$2;

	var classList = documentCreateElement('span').classList;
	var DOMTokenListPrototype$1 = classList && classList.constructor && classList.constructor.prototype;

	var domTokenListPrototype = DOMTokenListPrototype$1 === Object.prototype ? undefined : DOMTokenListPrototype$1;

	var globalThis$8 = globalThis_1;
	var DOMIterables = domIterables;
	var DOMTokenListPrototype = domTokenListPrototype;
	var ArrayIteratorMethods = es_array_iterator;
	var createNonEnumerableProperty$1 = createNonEnumerableProperty$b;
	var setToStringTag$3 = setToStringTag$9;
	var wellKnownSymbol$2 = wellKnownSymbol$w;

	var ITERATOR$2 = wellKnownSymbol$2('iterator');
	var ArrayValues = ArrayIteratorMethods.values;

	var handlePrototype = function (CollectionPrototype, COLLECTION_NAME) {
	  if (CollectionPrototype) {
	    // some Chrome versions have non-configurable methods on DOMTokenList
	    if (CollectionPrototype[ITERATOR$2] !== ArrayValues) try {
	      createNonEnumerableProperty$1(CollectionPrototype, ITERATOR$2, ArrayValues);
	    } catch (error) {
	      CollectionPrototype[ITERATOR$2] = ArrayValues;
	    }
	    setToStringTag$3(CollectionPrototype, COLLECTION_NAME, true);
	    if (DOMIterables[COLLECTION_NAME]) for (var METHOD_NAME in ArrayIteratorMethods) {
	      // some Chrome versions have non-configurable methods on DOMTokenList
	      if (CollectionPrototype[METHOD_NAME] !== ArrayIteratorMethods[METHOD_NAME]) try {
	        createNonEnumerableProperty$1(CollectionPrototype, METHOD_NAME, ArrayIteratorMethods[METHOD_NAME]);
	      } catch (error) {
	        CollectionPrototype[METHOD_NAME] = ArrayIteratorMethods[METHOD_NAME];
	      }
	    }
	  }
	};

	for (var COLLECTION_NAME in DOMIterables) {
	  handlePrototype(globalThis$8[COLLECTION_NAME] && globalThis$8[COLLECTION_NAME].prototype, COLLECTION_NAME);
	}

	handlePrototype(DOMTokenListPrototype, 'DOMTokenList');

	var globalThis$7 = globalThis_1;
	var IS_NODE = environmentIsNode;

	var getBuiltInNodeModule$2 = function (name) {
	  if (IS_NODE) {
	    try {
	      return globalThis$7.process.getBuiltinModule(name);
	    } catch (error) { /* empty */ }
	    try {
	      // eslint-disable-next-line no-new-func -- safe
	      return Function('return require("' + name + '")')();
	    } catch (error) { /* empty */ }
	  }
	};

	var DESCRIPTORS$6 = descriptors;
	var fails$4 = fails$S;
	var anObject$3 = anObject$q;
	var normalizeStringArgument$2 = normalizeStringArgument$5;

	var nativeErrorToString = Error.prototype.toString;

	var INCORRECT_TO_STRING$1 = fails$4(function () {
	  if (DESCRIPTORS$6) {
	    // Chrome 32- incorrectly call accessor
	    // eslint-disable-next-line es/no-object-create, es/no-object-defineproperty -- safe
	    var object = Object.create(Object.defineProperty({}, 'name', { get: function () {
	      return this === object;
	    } }));
	    if (nativeErrorToString.call(object) !== 'true') return true;
	  }
	  // FF10- does not properly handle non-strings
	  return nativeErrorToString.call({ message: 1, name: 2 }) !== '2: 1'
	    // IE8 does not properly handle defaults
	    || nativeErrorToString.call({}) !== 'Error';
	});

	var errorToString$1 = INCORRECT_TO_STRING$1 ? function toString() {
	  var O = anObject$3(this);
	  var name = normalizeStringArgument$2(O.name, 'Error');
	  var message = normalizeStringArgument$2(O.message);
	  return !name ? message : !message ? name : name + ': ' + message;
	} : nativeErrorToString;

	var domExceptionConstants = {
	  IndexSizeError: { s: 'INDEX_SIZE_ERR', c: 1, m: 1 },
	  DOMStringSizeError: { s: 'DOMSTRING_SIZE_ERR', c: 2, m: 0 },
	  HierarchyRequestError: { s: 'HIERARCHY_REQUEST_ERR', c: 3, m: 1 },
	  WrongDocumentError: { s: 'WRONG_DOCUMENT_ERR', c: 4, m: 1 },
	  InvalidCharacterError: { s: 'INVALID_CHARACTER_ERR', c: 5, m: 1 },
	  NoDataAllowedError: { s: 'NO_DATA_ALLOWED_ERR', c: 6, m: 0 },
	  NoModificationAllowedError: { s: 'NO_MODIFICATION_ALLOWED_ERR', c: 7, m: 1 },
	  NotFoundError: { s: 'NOT_FOUND_ERR', c: 8, m: 1 },
	  NotSupportedError: { s: 'NOT_SUPPORTED_ERR', c: 9, m: 1 },
	  InUseAttributeError: { s: 'INUSE_ATTRIBUTE_ERR', c: 10, m: 1 },
	  InvalidStateError: { s: 'INVALID_STATE_ERR', c: 11, m: 1 },
	  SyntaxError: { s: 'SYNTAX_ERR', c: 12, m: 1 },
	  InvalidModificationError: { s: 'INVALID_MODIFICATION_ERR', c: 13, m: 1 },
	  NamespaceError: { s: 'NAMESPACE_ERR', c: 14, m: 1 },
	  InvalidAccessError: { s: 'INVALID_ACCESS_ERR', c: 15, m: 1 },
	  ValidationError: { s: 'VALIDATION_ERR', c: 16, m: 0 },
	  TypeMismatchError: { s: 'TYPE_MISMATCH_ERR', c: 17, m: 1 },
	  SecurityError: { s: 'SECURITY_ERR', c: 18, m: 1 },
	  NetworkError: { s: 'NETWORK_ERR', c: 19, m: 1 },
	  AbortError: { s: 'ABORT_ERR', c: 20, m: 1 },
	  URLMismatchError: { s: 'URL_MISMATCH_ERR', c: 21, m: 1 },
	  QuotaExceededError: { s: 'QUOTA_EXCEEDED_ERR', c: 22, m: 1 },
	  TimeoutError: { s: 'TIMEOUT_ERR', c: 23, m: 1 },
	  InvalidNodeTypeError: { s: 'INVALID_NODE_TYPE_ERR', c: 24, m: 1 },
	  DataCloneError: { s: 'DATA_CLONE_ERR', c: 25, m: 1 }
	};

	var $$5 = _export;
	var getBuiltIn$4 = getBuiltIn$k;
	var getBuiltInNodeModule$1 = getBuiltInNodeModule$2;
	var fails$3 = fails$S;
	var create$1 = objectCreate;
	var createPropertyDescriptor$2 = createPropertyDescriptor$a;
	var defineProperty$1 = objectDefineProperty.f;
	var defineBuiltIn$4 = defineBuiltIn$k;
	var defineBuiltInAccessor$3 = defineBuiltInAccessor$b;
	var hasOwn$4 = hasOwnProperty_1;
	var anInstance$3 = anInstance$9;
	var anObject$2 = anObject$q;
	var errorToString = errorToString$1;
	var normalizeStringArgument$1 = normalizeStringArgument$5;
	var DOMExceptionConstants$1 = domExceptionConstants;
	var clearErrorStack$1 = errorStackClear;
	var InternalStateModule$2 = internalState;
	var DESCRIPTORS$5 = descriptors;

	var DOM_EXCEPTION$2 = 'DOMException';
	var DATA_CLONE_ERR = 'DATA_CLONE_ERR';
	var Error$3 = getBuiltIn$4('Error');
	// NodeJS < 17.0 does not expose `DOMException` to global
	var NativeDOMException$1 = getBuiltIn$4(DOM_EXCEPTION$2) || (function () {
	  try {
	    // NodeJS < 15.0 does not expose `MessageChannel` to global
	    var MessageChannel = getBuiltIn$4('MessageChannel') || getBuiltInNodeModule$1('worker_threads').MessageChannel;
	    // eslint-disable-next-line es/no-weak-map, unicorn/require-post-message-target-origin -- safe
	    new MessageChannel().port1.postMessage(new WeakMap());
	  } catch (error) {
	    if (error.name === DATA_CLONE_ERR && error.code === 25) return error.constructor;
	  }
	})();
	var NativeDOMExceptionPrototype = NativeDOMException$1 && NativeDOMException$1.prototype;
	var ErrorPrototype = Error$3.prototype;
	var setInternalState$2 = InternalStateModule$2.set;
	var getInternalState = InternalStateModule$2.getterFor(DOM_EXCEPTION$2);
	var HAS_STACK = 'stack' in new Error$3(DOM_EXCEPTION$2);

	var codeFor = function (name) {
	  return hasOwn$4(DOMExceptionConstants$1, name) && DOMExceptionConstants$1[name].m ? DOMExceptionConstants$1[name].c : 0;
	};

	var $DOMException$1 = function DOMException() {
	  anInstance$3(this, DOMExceptionPrototype$1);
	  var argumentsLength = arguments.length;
	  var message = normalizeStringArgument$1(argumentsLength < 1 ? undefined : arguments[0]);
	  var name = normalizeStringArgument$1(argumentsLength < 2 ? undefined : arguments[1], 'Error');
	  var code = codeFor(name);
	  setInternalState$2(this, {
	    type: DOM_EXCEPTION$2,
	    name: name,
	    message: message,
	    code: code
	  });
	  if (!DESCRIPTORS$5) {
	    this.name = name;
	    this.message = message;
	    this.code = code;
	  }
	  if (HAS_STACK) {
	    var error = new Error$3(message);
	    error.name = DOM_EXCEPTION$2;
	    defineProperty$1(this, 'stack', createPropertyDescriptor$2(1, clearErrorStack$1(error.stack, 1)));
	  }
	};

	var DOMExceptionPrototype$1 = $DOMException$1.prototype = create$1(ErrorPrototype);

	var createGetterDescriptor = function (get) {
	  return { enumerable: true, configurable: true, get: get };
	};

	var getterFor = function (key) {
	  return createGetterDescriptor(function () {
	    return getInternalState(this)[key];
	  });
	};

	if (DESCRIPTORS$5) {
	  // `DOMException.prototype.code` getter
	  defineBuiltInAccessor$3(DOMExceptionPrototype$1, 'code', getterFor('code'));
	  // `DOMException.prototype.message` getter
	  defineBuiltInAccessor$3(DOMExceptionPrototype$1, 'message', getterFor('message'));
	  // `DOMException.prototype.name` getter
	  defineBuiltInAccessor$3(DOMExceptionPrototype$1, 'name', getterFor('name'));
	}

	defineProperty$1(DOMExceptionPrototype$1, 'constructor', createPropertyDescriptor$2(1, $DOMException$1));

	// FF36- DOMException is a function, but can't be constructed
	var INCORRECT_CONSTRUCTOR = fails$3(function () {
	  return !(new NativeDOMException$1() instanceof Error$3);
	});

	// Safari 10.1 / Chrome 32- / IE8- DOMException.prototype.toString bugs
	var INCORRECT_TO_STRING = INCORRECT_CONSTRUCTOR || fails$3(function () {
	  return ErrorPrototype.toString !== errorToString || String(new NativeDOMException$1(1, 2)) !== '2: 1';
	});

	// Deno 1.6.3- DOMException.prototype.code just missed
	var INCORRECT_CODE = INCORRECT_CONSTRUCTOR || fails$3(function () {
	  return new NativeDOMException$1(1, 'DataCloneError').code !== 25;
	});

	// Deno 1.6.3- DOMException constants just missed
	INCORRECT_CONSTRUCTOR
	  || NativeDOMException$1[DATA_CLONE_ERR] !== 25
	  || NativeDOMExceptionPrototype[DATA_CLONE_ERR] !== 25;

	var FORCED_CONSTRUCTOR$1 = INCORRECT_CONSTRUCTOR;

	// `DOMException` constructor
	// https://webidl.spec.whatwg.org/#idl-DOMException
	$$5({ global: true, constructor: true, forced: FORCED_CONSTRUCTOR$1 }, {
	  DOMException: FORCED_CONSTRUCTOR$1 ? $DOMException$1 : NativeDOMException$1
	});

	var PolyfilledDOMException$1 = getBuiltIn$4(DOM_EXCEPTION$2);
	var PolyfilledDOMExceptionPrototype$1 = PolyfilledDOMException$1.prototype;

	if (INCORRECT_TO_STRING && (NativeDOMException$1 === PolyfilledDOMException$1)) {
	  defineBuiltIn$4(PolyfilledDOMExceptionPrototype$1, 'toString', errorToString);
	}

	if (INCORRECT_CODE && DESCRIPTORS$5 && NativeDOMException$1 === PolyfilledDOMException$1) {
	  defineBuiltInAccessor$3(PolyfilledDOMExceptionPrototype$1, 'code', createGetterDescriptor(function () {
	    return codeFor(anObject$2(this).name);
	  }));
	}

	// `DOMException` constants
	for (var key$2 in DOMExceptionConstants$1) if (hasOwn$4(DOMExceptionConstants$1, key$2)) {
	  var constant$1 = DOMExceptionConstants$1[key$2];
	  var constantName$1 = constant$1.s;
	  var descriptor$1 = createPropertyDescriptor$2(6, constant$1.c);
	  if (!hasOwn$4(PolyfilledDOMException$1, constantName$1)) {
	    defineProperty$1(PolyfilledDOMException$1, constantName$1, descriptor$1);
	  }
	  if (!hasOwn$4(PolyfilledDOMExceptionPrototype$1, constantName$1)) {
	    defineProperty$1(PolyfilledDOMExceptionPrototype$1, constantName$1, descriptor$1);
	  }
	}

	var $$4 = _export;
	var globalThis$6 = globalThis_1;
	var getBuiltIn$3 = getBuiltIn$k;
	var createPropertyDescriptor$1 = createPropertyDescriptor$a;
	var defineProperty = objectDefineProperty.f;
	var hasOwn$3 = hasOwnProperty_1;
	var anInstance$2 = anInstance$9;
	var inheritIfRequired = inheritIfRequired$4;
	var normalizeStringArgument = normalizeStringArgument$5;
	var DOMExceptionConstants = domExceptionConstants;
	var clearErrorStack = errorStackClear;
	var DESCRIPTORS$4 = descriptors;

	var DOM_EXCEPTION$1 = 'DOMException';
	var Error$2 = getBuiltIn$3('Error');
	var NativeDOMException = getBuiltIn$3(DOM_EXCEPTION$1);

	var $DOMException = function DOMException() {
	  anInstance$2(this, DOMExceptionPrototype);
	  var argumentsLength = arguments.length;
	  var message = normalizeStringArgument(argumentsLength < 1 ? undefined : arguments[0]);
	  var name = normalizeStringArgument(argumentsLength < 2 ? undefined : arguments[1], 'Error');
	  var that = new NativeDOMException(message, name);
	  var error = new Error$2(message);
	  error.name = DOM_EXCEPTION$1;
	  defineProperty(that, 'stack', createPropertyDescriptor$1(1, clearErrorStack(error.stack, 1)));
	  inheritIfRequired(that, this, $DOMException);
	  return that;
	};

	var DOMExceptionPrototype = $DOMException.prototype = NativeDOMException.prototype;

	var ERROR_HAS_STACK = 'stack' in new Error$2(DOM_EXCEPTION$1);
	var DOM_EXCEPTION_HAS_STACK = 'stack' in new NativeDOMException(1, 2);

	// eslint-disable-next-line es/no-object-getownpropertydescriptor -- safe
	var descriptor = NativeDOMException && DESCRIPTORS$4 && Object.getOwnPropertyDescriptor(globalThis$6, DOM_EXCEPTION$1);

	// Bun ~ 0.1.1 DOMException have incorrect descriptor and we can't redefine it
	// https://github.com/Jarred-Sumner/bun/issues/399
	var BUGGY_DESCRIPTOR = !!descriptor && !(descriptor.writable && descriptor.configurable);

	var FORCED_CONSTRUCTOR = ERROR_HAS_STACK && !BUGGY_DESCRIPTOR && !DOM_EXCEPTION_HAS_STACK;

	// `DOMException` constructor patch for `.stack` where it's required
	// https://webidl.spec.whatwg.org/#es-DOMException-specialness
	$$4({ global: true, constructor: true, forced: FORCED_CONSTRUCTOR }, { // TODO: fix export logic
	  DOMException: FORCED_CONSTRUCTOR ? $DOMException : NativeDOMException
	});

	var PolyfilledDOMException = getBuiltIn$3(DOM_EXCEPTION$1);
	var PolyfilledDOMExceptionPrototype = PolyfilledDOMException.prototype;

	if (PolyfilledDOMExceptionPrototype.constructor !== PolyfilledDOMException) {
	  {
	    defineProperty(PolyfilledDOMExceptionPrototype, 'constructor', createPropertyDescriptor$1(1, PolyfilledDOMException));
	  }

	  for (var key$1 in DOMExceptionConstants) if (hasOwn$3(DOMExceptionConstants, key$1)) {
	    var constant = DOMExceptionConstants[key$1];
	    var constantName = constant.s;
	    if (!hasOwn$3(PolyfilledDOMException, constantName)) {
	      defineProperty(PolyfilledDOMException, constantName, createPropertyDescriptor$1(6, constant.c));
	    }
	  }
	}

	var getBuiltIn$2 = getBuiltIn$k;
	var setToStringTag$2 = setToStringTag$9;

	var DOM_EXCEPTION = 'DOMException';

	// `DOMException.prototype[@@toStringTag]` property
	setToStringTag$2(getBuiltIn$2(DOM_EXCEPTION), DOM_EXCEPTION);

	var globalThis$5 = globalThis_1;
	var fails$2 = fails$S;
	var V8 = environmentV8Version;
	var ENVIRONMENT = environment;

	var structuredClone$2 = globalThis$5.structuredClone;

	var structuredCloneProperTransfer = !!structuredClone$2 && !fails$2(function () {
	  // prevent V8 ArrayBufferDetaching protector cell invalidation and performance degradation
	  // https://github.com/zloirock/core-js/issues/679
	  if ((ENVIRONMENT === 'DENO' && V8 > 92) || (ENVIRONMENT === 'NODE' && V8 > 94) || (ENVIRONMENT === 'BROWSER' && V8 > 97)) return false;
	  var buffer = new ArrayBuffer(8);
	  var clone = structuredClone$2(buffer, { transfer: [buffer] });
	  return buffer.byteLength !== 0 || clone.byteLength !== 8;
	});

	var globalThis$4 = globalThis_1;
	var getBuiltInNodeModule = getBuiltInNodeModule$2;
	var PROPER_STRUCTURED_CLONE_TRANSFER$1 = structuredCloneProperTransfer;

	var structuredClone$1 = globalThis$4.structuredClone;
	var $ArrayBuffer = globalThis$4.ArrayBuffer;
	var $MessageChannel = globalThis$4.MessageChannel;
	var detach = false;
	var WorkerThreads, channel, buffer, $detach;

	if (PROPER_STRUCTURED_CLONE_TRANSFER$1) {
	  detach = function (transferable) {
	    structuredClone$1(transferable, { transfer: [transferable] });
	  };
	} else if ($ArrayBuffer) try {
	  if (!$MessageChannel) {
	    WorkerThreads = getBuiltInNodeModule('worker_threads');
	    if (WorkerThreads) $MessageChannel = WorkerThreads.MessageChannel;
	  }

	  if ($MessageChannel) {
	    channel = new $MessageChannel();
	    buffer = new $ArrayBuffer(2);

	    $detach = function (transferable) {
	      channel.port1.postMessage(null, [transferable]);
	    };

	    if (buffer.byteLength === 2) {
	      $detach(buffer);
	      if (buffer.byteLength === 0) detach = $detach;
	    }
	  }
	} catch (error) { /* empty */ }

	var detachTransferable$1 = detach;

	var $$3 = _export;
	var globalThis$3 = globalThis_1;
	var getBuiltIn$1 = getBuiltIn$k;
	var uncurryThis$6 = functionUncurryThis;
	var fails$1 = fails$S;
	var uid = uid$5;
	var isCallable$1 = isCallable$w;
	var isConstructor = isConstructor$6;
	var isNullOrUndefined = isNullOrUndefined$b;
	var isObject$1 = isObject$n;
	var isSymbol = isSymbol$6;
	var iterate = iterate$d;
	var anObject$1 = anObject$q;
	var classof$1 = classof$d;
	var hasOwn$2 = hasOwnProperty_1;
	var createProperty = createProperty$6;
	var createNonEnumerableProperty = createNonEnumerableProperty$b;
	var lengthOfArrayLike = lengthOfArrayLike$h;
	var validateArgumentsLength$4 = validateArgumentsLength$6;
	var getRegExpFlags = regexpGetFlags;
	var MapHelpers = mapHelpers;
	var SetHelpers = setHelpers;
	var setIterate = setIterate$1;
	var detachTransferable = detachTransferable$1;
	var ERROR_STACK_INSTALLABLE = errorStackInstallable;
	var PROPER_STRUCTURED_CLONE_TRANSFER = structuredCloneProperTransfer;

	var Object$1 = globalThis$3.Object;
	var Array$1 = globalThis$3.Array;
	var Date$1 = globalThis$3.Date;
	var Error$1 = globalThis$3.Error;
	var TypeError$3 = globalThis$3.TypeError;
	var PerformanceMark = globalThis$3.PerformanceMark;
	var DOMException = getBuiltIn$1('DOMException');
	var Map$1 = MapHelpers.Map;
	var mapHas = MapHelpers.has;
	var mapGet = MapHelpers.get;
	var mapSet = MapHelpers.set;
	var Set$1 = SetHelpers.Set;
	var setAdd = SetHelpers.add;
	var setHas = SetHelpers.has;
	var objectKeys = getBuiltIn$1('Object', 'keys');
	var push$4 = uncurryThis$6([].push);
	var thisBooleanValue = uncurryThis$6(true.valueOf);
	var thisNumberValue = uncurryThis$6(1.0.valueOf);
	var thisStringValue = uncurryThis$6(''.valueOf);
	var thisTimeValue = uncurryThis$6(Date$1.prototype.getTime);
	var PERFORMANCE_MARK = uid('structuredClone');
	var DATA_CLONE_ERROR = 'DataCloneError';
	var TRANSFERRING = 'Transferring';

	var checkBasicSemantic = function (structuredCloneImplementation) {
	  return !fails$1(function () {
	    var set1 = new globalThis$3.Set([7]);
	    var set2 = structuredCloneImplementation(set1);
	    var number = structuredCloneImplementation(Object$1(7));
	    return set2 === set1 || !set2.has(7) || !isObject$1(number) || +number !== 7;
	  }) && structuredCloneImplementation;
	};

	var checkErrorsCloning = function (structuredCloneImplementation, $Error) {
	  return !fails$1(function () {
	    var error = new $Error();
	    var test = structuredCloneImplementation({ a: error, b: error });
	    return !(test && test.a === test.b && test.a instanceof $Error && test.a.stack === error.stack);
	  });
	};

	// https://github.com/whatwg/html/pull/5749
	var checkNewErrorsCloningSemantic = function (structuredCloneImplementation) {
	  return !fails$1(function () {
	    var test = structuredCloneImplementation(new globalThis$3.AggregateError([1], PERFORMANCE_MARK, { cause: 3 }));
	    return test.name !== 'AggregateError' || test.errors[0] !== 1 || test.message !== PERFORMANCE_MARK || test.cause !== 3;
	  });
	};

	// FF94+, Safari 15.4+, Chrome 98+, NodeJS 17.0+, Deno 1.13+
	// FF<103 and Safari implementations can't clone errors
	// https://bugzilla.mozilla.org/show_bug.cgi?id=1556604
	// FF103 can clone errors, but `.stack` of clone is an empty string
	// https://bugzilla.mozilla.org/show_bug.cgi?id=1778762
	// FF104+ fixed it on usual errors, but not on DOMExceptions
	// https://bugzilla.mozilla.org/show_bug.cgi?id=1777321
	// Chrome <102 returns `null` if cloned object contains multiple references to one error
	// https://bugs.chromium.org/p/v8/issues/detail?id=12542
	// NodeJS implementation can't clone DOMExceptions
	// https://github.com/nodejs/node/issues/41038
	// only FF103+ supports new (html/5749) error cloning semantic
	var nativeStructuredClone = globalThis$3.structuredClone;

	var FORCED_REPLACEMENT = !checkErrorsCloning(nativeStructuredClone, Error$1)
	  || !checkErrorsCloning(nativeStructuredClone, DOMException)
	  || !checkNewErrorsCloningSemantic(nativeStructuredClone);

	// Chrome 82+, Safari 14.1+, Deno 1.11+
	// Chrome 78-81 implementation swaps `.name` and `.message` of cloned `DOMException`
	// Chrome returns `null` if cloned object contains multiple references to one error
	// Safari 14.1 implementation doesn't clone some `RegExp` flags, so requires a workaround
	// Safari implementation can't clone errors
	// Deno 1.2-1.10 implementations too naive
	// NodeJS 16.0+ does not have `PerformanceMark` constructor
	// NodeJS <17.2 structured cloning implementation from `performance.mark` is too naive
	// and can't clone, for example, `RegExp` or some boxed primitives
	// https://github.com/nodejs/node/issues/40840
	// no one of those implementations supports new (html/5749) error cloning semantic
	var structuredCloneFromMark = !nativeStructuredClone && checkBasicSemantic(function (value) {
	  return new PerformanceMark(PERFORMANCE_MARK, { detail: value }).detail;
	});

	var nativeRestrictedStructuredClone = checkBasicSemantic(nativeStructuredClone) || structuredCloneFromMark;

	var throwUncloneable = function (type) {
	  throw new DOMException('Uncloneable type: ' + type, DATA_CLONE_ERROR);
	};

	var throwUnpolyfillable = function (type, action) {
	  throw new DOMException((action || 'Cloning') + ' of ' + type + ' cannot be properly polyfilled in this engine', DATA_CLONE_ERROR);
	};

	var tryNativeRestrictedStructuredClone = function (value, type) {
	  if (!nativeRestrictedStructuredClone) throwUnpolyfillable(type);
	  return nativeRestrictedStructuredClone(value);
	};

	var createDataTransfer = function () {
	  var dataTransfer;
	  try {
	    dataTransfer = new globalThis$3.DataTransfer();
	  } catch (error) {
	    try {
	      dataTransfer = new globalThis$3.ClipboardEvent('').clipboardData;
	    } catch (error2) { /* empty */ }
	  }
	  return dataTransfer && dataTransfer.items && dataTransfer.files ? dataTransfer : null;
	};

	var cloneBuffer = function (value, map, $type) {
	  if (mapHas(map, value)) return mapGet(map, value);

	  var type = $type || classof$1(value);
	  var clone, length, options, source, target, i;

	  if (type === 'SharedArrayBuffer') {
	    if (nativeRestrictedStructuredClone) clone = nativeRestrictedStructuredClone(value);
	    // SharedArrayBuffer should use shared memory, we can't polyfill it, so return the original
	    else clone = value;
	  } else {
	    var DataView = globalThis$3.DataView;

	    // `ArrayBuffer#slice` is not available in IE10
	    // `ArrayBuffer#slice` and `DataView` are not available in old FF
	    if (!DataView && !isCallable$1(value.slice)) throwUnpolyfillable('ArrayBuffer');
	    // detached buffers throws in `DataView` and `.slice`
	    try {
	      if (isCallable$1(value.slice) && !value.resizable) {
	        clone = value.slice(0);
	      } else {
	        length = value.byteLength;
	        options = 'maxByteLength' in value ? { maxByteLength: value.maxByteLength } : undefined;
	        // eslint-disable-next-line es/no-resizable-and-growable-arraybuffers -- safe
	        clone = new ArrayBuffer(length, options);
	        source = new DataView(value);
	        target = new DataView(clone);
	        for (i = 0; i < length; i++) {
	          target.setUint8(i, source.getUint8(i));
	        }
	      }
	    } catch (error) {
	      throw new DOMException('ArrayBuffer is detached', DATA_CLONE_ERROR);
	    }
	  }

	  mapSet(map, value, clone);

	  return clone;
	};

	var cloneView = function (value, type, offset, length, map) {
	  var C = globalThis$3[type];
	  // in some old engines like Safari 9, typeof C is 'object'
	  // on Uint8ClampedArray or some other constructors
	  if (!isObject$1(C)) throwUnpolyfillable(type);
	  return new C(cloneBuffer(value.buffer, map), offset, length);
	};

	var structuredCloneInternal = function (value, map) {
	  if (isSymbol(value)) throwUncloneable('Symbol');
	  if (!isObject$1(value)) return value;
	  // effectively preserves circular references
	  if (map) {
	    if (mapHas(map, value)) return mapGet(map, value);
	  } else map = new Map$1();

	  var type = classof$1(value);
	  var C, name, cloned, dataTransfer, i, length, keys, key;

	  switch (type) {
	    case 'Array':
	      cloned = Array$1(lengthOfArrayLike(value));
	      break;
	    case 'Object':
	      cloned = {};
	      break;
	    case 'Map':
	      cloned = new Map$1();
	      break;
	    case 'Set':
	      cloned = new Set$1();
	      break;
	    case 'RegExp':
	      // in this block because of a Safari 14.1 bug
	      // old FF does not clone regexes passed to the constructor, so get the source and flags directly
	      cloned = new RegExp(value.source, getRegExpFlags(value));
	      break;
	    case 'Error':
	      name = value.name;
	      switch (name) {
	        case 'AggregateError':
	          cloned = new (getBuiltIn$1(name))([]);
	          break;
	        case 'EvalError':
	        case 'RangeError':
	        case 'ReferenceError':
	        case 'SuppressedError':
	        case 'SyntaxError':
	        case 'TypeError':
	        case 'URIError':
	          cloned = new (getBuiltIn$1(name))();
	          break;
	        case 'CompileError':
	        case 'LinkError':
	        case 'RuntimeError':
	          cloned = new (getBuiltIn$1('WebAssembly', name))();
	          break;
	        default:
	          cloned = new Error$1();
	      }
	      break;
	    case 'DOMException':
	      cloned = new DOMException(value.message, value.name);
	      break;
	    case 'ArrayBuffer':
	    case 'SharedArrayBuffer':
	      cloned = cloneBuffer(value, map, type);
	      break;
	    case 'DataView':
	    case 'Int8Array':
	    case 'Uint8Array':
	    case 'Uint8ClampedArray':
	    case 'Int16Array':
	    case 'Uint16Array':
	    case 'Int32Array':
	    case 'Uint32Array':
	    case 'Float16Array':
	    case 'Float32Array':
	    case 'Float64Array':
	    case 'BigInt64Array':
	    case 'BigUint64Array':
	      length = type === 'DataView' ? value.byteLength : value.length;
	      cloned = cloneView(value, type, value.byteOffset, length, map);
	      break;
	    case 'DOMQuad':
	      try {
	        cloned = new DOMQuad(
	          structuredCloneInternal(value.p1, map),
	          structuredCloneInternal(value.p2, map),
	          structuredCloneInternal(value.p3, map),
	          structuredCloneInternal(value.p4, map)
	        );
	      } catch (error) {
	        cloned = tryNativeRestrictedStructuredClone(value, type);
	      }
	      break;
	    case 'File':
	      if (nativeRestrictedStructuredClone) try {
	        cloned = nativeRestrictedStructuredClone(value);
	        // NodeJS 20.0.0 bug, https://github.com/nodejs/node/issues/47612
	        if (classof$1(cloned) !== type) cloned = undefined;
	      } catch (error) { /* empty */ }
	      if (!cloned) try {
	        cloned = new File([value], value.name, value);
	      } catch (error) { /* empty */ }
	      if (!cloned) throwUnpolyfillable(type);
	      break;
	    case 'FileList':
	      dataTransfer = createDataTransfer();
	      if (dataTransfer) {
	        for (i = 0, length = lengthOfArrayLike(value); i < length; i++) {
	          dataTransfer.items.add(structuredCloneInternal(value[i], map));
	        }
	        cloned = dataTransfer.files;
	      } else cloned = tryNativeRestrictedStructuredClone(value, type);
	      break;
	    case 'ImageData':
	      // Safari 9 ImageData is a constructor, but typeof ImageData is 'object'
	      try {
	        cloned = new ImageData(
	          structuredCloneInternal(value.data, map),
	          value.width,
	          value.height,
	          { colorSpace: value.colorSpace }
	        );
	      } catch (error) {
	        cloned = tryNativeRestrictedStructuredClone(value, type);
	      } break;
	    default:
	      if (nativeRestrictedStructuredClone) {
	        cloned = nativeRestrictedStructuredClone(value);
	      } else switch (type) {
	        case 'BigInt':
	          // can be a 3rd party polyfill
	          cloned = Object$1(value.valueOf());
	          break;
	        case 'Boolean':
	          cloned = Object$1(thisBooleanValue(value));
	          break;
	        case 'Number':
	          cloned = Object$1(thisNumberValue(value));
	          break;
	        case 'String':
	          cloned = Object$1(thisStringValue(value));
	          break;
	        case 'Date':
	          cloned = new Date$1(thisTimeValue(value));
	          break;
	        case 'Blob':
	          try {
	            cloned = value.slice(0, value.size, value.type);
	          } catch (error) {
	            throwUnpolyfillable(type);
	          } break;
	        case 'DOMPoint':
	        case 'DOMPointReadOnly':
	          C = globalThis$3[type];
	          try {
	            cloned = C.fromPoint
	              ? C.fromPoint(value)
	              : new C(value.x, value.y, value.z, value.w);
	          } catch (error) {
	            throwUnpolyfillable(type);
	          } break;
	        case 'DOMRect':
	        case 'DOMRectReadOnly':
	          C = globalThis$3[type];
	          try {
	            cloned = C.fromRect
	              ? C.fromRect(value)
	              : new C(value.x, value.y, value.width, value.height);
	          } catch (error) {
	            throwUnpolyfillable(type);
	          } break;
	        case 'DOMMatrix':
	        case 'DOMMatrixReadOnly':
	          C = globalThis$3[type];
	          try {
	            cloned = C.fromMatrix
	              ? C.fromMatrix(value)
	              : new C(value);
	          } catch (error) {
	            throwUnpolyfillable(type);
	          } break;
	        case 'AudioData':
	        case 'VideoFrame':
	          if (!isCallable$1(value.clone)) throwUnpolyfillable(type);
	          try {
	            cloned = value.clone();
	          } catch (error) {
	            throwUncloneable(type);
	          } break;
	        case 'CropTarget':
	        case 'CryptoKey':
	        case 'FileSystemDirectoryHandle':
	        case 'FileSystemFileHandle':
	        case 'FileSystemHandle':
	        case 'GPUCompilationInfo':
	        case 'GPUCompilationMessage':
	        case 'ImageBitmap':
	        case 'RTCCertificate':
	        case 'WebAssembly.Module':
	          throwUnpolyfillable(type);
	          // break omitted
	        default:
	          throwUncloneable(type);
	      }
	  }

	  mapSet(map, value, cloned);

	  switch (type) {
	    case 'Array':
	    case 'Object':
	      keys = objectKeys(value);
	      for (i = 0, length = lengthOfArrayLike(keys); i < length; i++) {
	        key = keys[i];
	        createProperty(cloned, key, structuredCloneInternal(value[key], map));
	      } break;
	    case 'Map':
	      value.forEach(function (v, k) {
	        mapSet(cloned, structuredCloneInternal(k, map), structuredCloneInternal(v, map));
	      });
	      break;
	    case 'Set':
	      value.forEach(function (v) {
	        setAdd(cloned, structuredCloneInternal(v, map));
	      });
	      break;
	    case 'Error':
	      createNonEnumerableProperty(cloned, 'message', structuredCloneInternal(value.message, map));
	      if (hasOwn$2(value, 'cause')) {
	        createNonEnumerableProperty(cloned, 'cause', structuredCloneInternal(value.cause, map));
	      }
	      if (name === 'AggregateError') {
	        cloned.errors = structuredCloneInternal(value.errors, map);
	      } else if (name === 'SuppressedError') {
	        cloned.error = structuredCloneInternal(value.error, map);
	        cloned.suppressed = structuredCloneInternal(value.suppressed, map);
	      } // break omitted
	    case 'DOMException':
	      if (ERROR_STACK_INSTALLABLE) {
	        createNonEnumerableProperty(cloned, 'stack', structuredCloneInternal(value.stack, map));
	      }
	  }

	  return cloned;
	};

	var tryToTransfer = function (rawTransfer, map) {
	  if (!isObject$1(rawTransfer)) throw new TypeError$3('Transfer option cannot be converted to a sequence');

	  var transfer = [];

	  iterate(rawTransfer, function (value) {
	    push$4(transfer, anObject$1(value));
	  });

	  var i = 0;
	  var length = lengthOfArrayLike(transfer);
	  var buffers = new Set$1();
	  var value, type, C, transferred, canvas, context;

	  while (i < length) {
	    value = transfer[i++];

	    type = classof$1(value);

	    if (type === 'ArrayBuffer' ? setHas(buffers, value) : mapHas(map, value)) {
	      throw new DOMException('Duplicate transferable', DATA_CLONE_ERROR);
	    }

	    if (type === 'ArrayBuffer') {
	      setAdd(buffers, value);
	      continue;
	    }

	    if (PROPER_STRUCTURED_CLONE_TRANSFER) {
	      transferred = nativeStructuredClone(value, { transfer: [value] });
	    } else switch (type) {
	      case 'ImageBitmap':
	        C = globalThis$3.OffscreenCanvas;
	        if (!isConstructor(C)) throwUnpolyfillable(type, TRANSFERRING);
	        try {
	          canvas = new C(value.width, value.height);
	          context = canvas.getContext('bitmaprenderer');
	          context.transferFromImageBitmap(value);
	          transferred = canvas.transferToImageBitmap();
	        } catch (error) { /* empty */ }
	        break;
	      case 'AudioData':
	      case 'VideoFrame':
	        if (!isCallable$1(value.clone) || !isCallable$1(value.close)) throwUnpolyfillable(type, TRANSFERRING);
	        try {
	          transferred = value.clone();
	          value.close();
	        } catch (error) { /* empty */ }
	        break;
	      case 'MediaSourceHandle':
	      case 'MessagePort':
	      case 'MIDIAccess':
	      case 'OffscreenCanvas':
	      case 'ReadableStream':
	      case 'RTCDataChannel':
	      case 'TransformStream':
	      case 'WebTransportReceiveStream':
	      case 'WebTransportSendStream':
	      case 'WritableStream':
	        throwUnpolyfillable(type, TRANSFERRING);
	    }

	    if (transferred === undefined) throw new DOMException('This object cannot be transferred: ' + type, DATA_CLONE_ERROR);

	    mapSet(map, value, transferred);
	  }

	  return buffers;
	};

	var detachBuffers = function (buffers) {
	  setIterate(buffers, function (buffer) {
	    if (PROPER_STRUCTURED_CLONE_TRANSFER) {
	      nativeRestrictedStructuredClone(buffer, { transfer: [buffer] });
	    } else if (isCallable$1(buffer.transfer)) {
	      buffer.transfer();
	    } else if (detachTransferable) {
	      detachTransferable(buffer);
	    } else {
	      throwUnpolyfillable('ArrayBuffer', TRANSFERRING);
	    }
	  });
	};

	// `structuredClone` method
	// https://html.spec.whatwg.org/multipage/structured-data.html#dom-structuredclone
	$$3({ global: true, enumerable: true, sham: !PROPER_STRUCTURED_CLONE_TRANSFER, forced: FORCED_REPLACEMENT }, {
	  structuredClone: function structuredClone(value /* , { transfer } */) {
	    var options = validateArgumentsLength$4(arguments.length, 1) > 1 && !isNullOrUndefined(arguments[1]) ? anObject$1(arguments[1]) : undefined;
	    var transfer = options ? options.transfer : undefined;
	    var map, buffers;

	    if (transfer !== undefined) {
	      map = new Map$1();
	      buffers = tryToTransfer(transfer, map);
	    }

	    var clone = structuredCloneInternal(value, map);

	    // since of an issue with cloning views of transferred buffers, we a forced to detach them later
	    // https://github.com/zloirock/core-js/issues/1265
	    if (buffers) detachBuffers(buffers);

	    return clone;
	  }
	});

	var fails = fails$S;
	var wellKnownSymbol$1 = wellKnownSymbol$w;
	var DESCRIPTORS$3 = descriptors;
	var IS_PURE = isPure;

	var ITERATOR$1 = wellKnownSymbol$1('iterator');

	var urlConstructorDetection = !fails(function () {
	  // eslint-disable-next-line unicorn/relative-url-style -- required for testing
	  var url = new URL('b?a=1&b=2&c=3', 'https://a');
	  var params = url.searchParams;
	  var params2 = new URLSearchParams('a=1&a=2&b=3');
	  var result = '';
	  url.pathname = 'c%20d';
	  params.forEach(function (value, key) {
	    params['delete']('b');
	    result += key + value;
	  });
	  params2['delete']('a', 2);
	  // `undefined` case is a Chromium 117 bug
	  // https://bugs.chromium.org/p/v8/issues/detail?id=14222
	  params2['delete']('b', undefined);
	  return (IS_PURE && (!url.toJSON || !params2.has('a', 1) || params2.has('a', 2) || !params2.has('a', undefined) || params2.has('b')))
	    || (!params.size && (IS_PURE || !DESCRIPTORS$3))
	    || !params.sort
	    || url.href !== 'https://a/c%20d?a=1&c=3'
	    || params.get('c') !== '3'
	    || String(new URLSearchParams('?a=1')) !== 'a=1'
	    || !params[ITERATOR$1]
	    // throws in Edge
	    || new URL('https://a@b').username !== 'a'
	    || new URLSearchParams(new URLSearchParams('a=b')).get('a') !== 'b'
	    // not punycoded in Edge
	    || new URL('https://тест').host !== 'xn--e1aybc'
	    // not escaped in Chrome 62-
	    || new URL('https://a#б').hash !== '#%D0%B1'
	    // fails in Chrome 66-
	    || result !== 'a1c3'
	    // throws in Safari
	    || new URL('https://x', undefined).host !== 'x';
	});

	// based on https://github.com/bestiejs/punycode.js/blob/master/punycode.js
	var uncurryThis$5 = functionUncurryThis;

	var maxInt = 2147483647; // aka. 0x7FFFFFFF or 2^31-1
	var base = 36;
	var tMin = 1;
	var tMax = 26;
	var skew = 38;
	var damp = 700;
	var initialBias = 72;
	var initialN = 128; // 0x80
	var delimiter = '-'; // '\x2D'
	var regexNonASCII = /[^\0-\u007E]/; // non-ASCII chars
	var regexSeparators = /[.\u3002\uFF0E\uFF61]/g; // RFC 3490 separators
	var OVERFLOW_ERROR = 'Overflow: input needs wider integers to process';
	var baseMinusTMin = base - tMin;

	var $RangeError = RangeError;
	var exec$2 = uncurryThis$5(regexSeparators.exec);
	var floor$1 = Math.floor;
	var fromCharCode$1 = String.fromCharCode;
	var charCodeAt = uncurryThis$5(''.charCodeAt);
	var join$2 = uncurryThis$5([].join);
	var push$3 = uncurryThis$5([].push);
	var replace$2 = uncurryThis$5(''.replace);
	var split$2 = uncurryThis$5(''.split);
	var toLowerCase$1 = uncurryThis$5(''.toLowerCase);

	/**
	 * Creates an array containing the numeric code points of each Unicode
	 * character in the string. While JavaScript uses UCS-2 internally,
	 * this function will convert a pair of surrogate halves (each of which
	 * UCS-2 exposes as separate characters) into a single code point,
	 * matching UTF-16.
	 */
	var ucs2decode = function (string) {
	  var output = [];
	  var counter = 0;
	  var length = string.length;
	  while (counter < length) {
	    var value = charCodeAt(string, counter++);
	    if (value >= 0xD800 && value <= 0xDBFF && counter < length) {
	      // It's a high surrogate, and there is a next character.
	      var extra = charCodeAt(string, counter++);
	      if ((extra & 0xFC00) === 0xDC00) { // Low surrogate.
	        push$3(output, ((value & 0x3FF) << 10) + (extra & 0x3FF) + 0x10000);
	      } else {
	        // It's an unmatched surrogate; only append this code unit, in case the
	        // next code unit is the high surrogate of a surrogate pair.
	        push$3(output, value);
	        counter--;
	      }
	    } else {
	      push$3(output, value);
	    }
	  }
	  return output;
	};

	/**
	 * Converts a digit/integer into a basic code point.
	 */
	var digitToBasic = function (digit) {
	  //  0..25 map to ASCII a..z or A..Z
	  // 26..35 map to ASCII 0..9
	  return digit + 22 + 75 * (digit < 26);
	};

	/**
	 * Bias adaptation function as per section 3.4 of RFC 3492.
	 * https://tools.ietf.org/html/rfc3492#section-3.4
	 */
	var adapt = function (delta, numPoints, firstTime) {
	  var k = 0;
	  delta = firstTime ? floor$1(delta / damp) : delta >> 1;
	  delta += floor$1(delta / numPoints);
	  while (delta > baseMinusTMin * tMax >> 1) {
	    delta = floor$1(delta / baseMinusTMin);
	    k += base;
	  }
	  return floor$1(k + (baseMinusTMin + 1) * delta / (delta + skew));
	};

	/**
	 * Converts a string of Unicode symbols (e.g. a domain name label) to a
	 * Punycode string of ASCII-only symbols.
	 */
	var encode = function (input) {
	  var output = [];

	  // Convert the input in UCS-2 to an array of Unicode code points.
	  input = ucs2decode(input);

	  // Cache the length.
	  var inputLength = input.length;

	  // Initialize the state.
	  var n = initialN;
	  var delta = 0;
	  var bias = initialBias;
	  var i, currentValue;

	  // Handle the basic code points.
	  for (i = 0; i < input.length; i++) {
	    currentValue = input[i];
	    if (currentValue < 0x80) {
	      push$3(output, fromCharCode$1(currentValue));
	    }
	  }

	  var basicLength = output.length; // number of basic code points.
	  var handledCPCount = basicLength; // number of code points that have been handled;

	  // Finish the basic string with a delimiter unless it's empty.
	  if (basicLength) {
	    push$3(output, delimiter);
	  }

	  // Main encoding loop:
	  while (handledCPCount < inputLength) {
	    // All non-basic code points < n have been handled already. Find the next larger one:
	    var m = maxInt;
	    for (i = 0; i < input.length; i++) {
	      currentValue = input[i];
	      if (currentValue >= n && currentValue < m) {
	        m = currentValue;
	      }
	    }

	    // Increase `delta` enough to advance the decoder's <n,i> state to <m,0>, but guard against overflow.
	    var handledCPCountPlusOne = handledCPCount + 1;
	    if (m - n > floor$1((maxInt - delta) / handledCPCountPlusOne)) {
	      throw new $RangeError(OVERFLOW_ERROR);
	    }

	    delta += (m - n) * handledCPCountPlusOne;
	    n = m;

	    for (i = 0; i < input.length; i++) {
	      currentValue = input[i];
	      if (currentValue < n && ++delta > maxInt) {
	        throw new $RangeError(OVERFLOW_ERROR);
	      }
	      if (currentValue === n) {
	        // Represent delta as a generalized variable-length integer.
	        var q = delta;
	        var k = base;
	        while (true) {
	          var t = k <= bias ? tMin : k >= bias + tMax ? tMax : k - bias;
	          if (q < t) break;
	          var qMinusT = q - t;
	          var baseMinusT = base - t;
	          push$3(output, fromCharCode$1(digitToBasic(t + qMinusT % baseMinusT)));
	          q = floor$1(qMinusT / baseMinusT);
	          k += base;
	        }

	        push$3(output, fromCharCode$1(digitToBasic(q)));
	        bias = adapt(delta, handledCPCountPlusOne, handledCPCount === basicLength);
	        delta = 0;
	        handledCPCount++;
	      }
	    }

	    delta++;
	    n++;
	  }
	  return join$2(output, '');
	};

	var stringPunycodeToAscii = function (input) {
	  var encoded = [];
	  var labels = split$2(replace$2(toLowerCase$1(input), regexSeparators, '\u002E'), '.');
	  var i, label;
	  for (i = 0; i < labels.length; i++) {
	    label = labels[i];
	    push$3(encoded, exec$2(regexNonASCII, label) ? 'xn--' + encode(label) : label);
	  }
	  return join$2(encoded, '.');
	};

	// TODO: in core-js@4, move /modules/ dependencies to public entries for better optimization by tools like `preset-env`


	var $$2 = _export;
	var globalThis$2 = globalThis_1;
	var safeGetBuiltIn = safeGetBuiltIn$2;
	var getBuiltIn = getBuiltIn$k;
	var call$1 = functionCall;
	var uncurryThis$4 = functionUncurryThis;
	var DESCRIPTORS$2 = descriptors;
	var USE_NATIVE_URL$1 = urlConstructorDetection;
	var defineBuiltIn$3 = defineBuiltIn$k;
	var defineBuiltInAccessor$2 = defineBuiltInAccessor$b;
	var defineBuiltIns = defineBuiltIns$5;
	var setToStringTag$1 = setToStringTag$9;
	var createIteratorConstructor = iteratorCreateConstructor;
	var InternalStateModule$1 = internalState;
	var anInstance$1 = anInstance$9;
	var isCallable = isCallable$w;
	var hasOwn$1 = hasOwnProperty_1;
	var bind$1 = functionBindContext;
	var classof = classof$d;
	var anObject = anObject$q;
	var isObject = isObject$n;
	var $toString$1 = toString$o;
	var create = objectCreate;
	var createPropertyDescriptor = createPropertyDescriptor$a;
	var getIterator = getIterator$3;
	var getIteratorMethod = getIteratorMethod$4;
	var createIterResultObject = createIterResultObject$6;
	var validateArgumentsLength$3 = validateArgumentsLength$6;
	var wellKnownSymbol = wellKnownSymbol$w;
	var arraySort = arraySort$1;

	var ITERATOR = wellKnownSymbol('iterator');
	var URL_SEARCH_PARAMS = 'URLSearchParams';
	var URL_SEARCH_PARAMS_ITERATOR = URL_SEARCH_PARAMS + 'Iterator';
	var setInternalState$1 = InternalStateModule$1.set;
	var getInternalParamsState = InternalStateModule$1.getterFor(URL_SEARCH_PARAMS);
	var getInternalIteratorState = InternalStateModule$1.getterFor(URL_SEARCH_PARAMS_ITERATOR);

	var nativeFetch = safeGetBuiltIn('fetch');
	var NativeRequest = safeGetBuiltIn('Request');
	var Headers = safeGetBuiltIn('Headers');
	var RequestPrototype = NativeRequest && NativeRequest.prototype;
	var HeadersPrototype = Headers && Headers.prototype;
	var TypeError$2 = globalThis$2.TypeError;
	var encodeURIComponent$1 = globalThis$2.encodeURIComponent;
	var fromCharCode = String.fromCharCode;
	var fromCodePoint = getBuiltIn('String', 'fromCodePoint');
	var $parseInt = parseInt;
	var charAt$1 = uncurryThis$4(''.charAt);
	var join$1 = uncurryThis$4([].join);
	var push$2 = uncurryThis$4([].push);
	var replace$1 = uncurryThis$4(''.replace);
	var shift$1 = uncurryThis$4([].shift);
	var splice = uncurryThis$4([].splice);
	var split$1 = uncurryThis$4(''.split);
	var stringSlice$1 = uncurryThis$4(''.slice);
	var exec$1 = uncurryThis$4(/./.exec);

	var plus = /\+/g;
	var FALLBACK_REPLACER = '\uFFFD';
	var VALID_HEX = /^[0-9a-f]+$/i;

	var parseHexOctet = function (string, start) {
	  var substr = stringSlice$1(string, start, start + 2);
	  if (!exec$1(VALID_HEX, substr)) return NaN;

	  return $parseInt(substr, 16);
	};

	var getLeadingOnes = function (octet) {
	  var count = 0;
	  for (var mask = 0x80; mask > 0 && (octet & mask) !== 0; mask >>= 1) {
	    count++;
	  }
	  return count;
	};

	var utf8Decode = function (octets) {
	  var codePoint = null;

	  switch (octets.length) {
	    case 1:
	      codePoint = octets[0];
	      break;
	    case 2:
	      codePoint = (octets[0] & 0x1F) << 6 | (octets[1] & 0x3F);
	      break;
	    case 3:
	      codePoint = (octets[0] & 0x0F) << 12 | (octets[1] & 0x3F) << 6 | (octets[2] & 0x3F);
	      break;
	    case 4:
	      codePoint = (octets[0] & 0x07) << 18 | (octets[1] & 0x3F) << 12 | (octets[2] & 0x3F) << 6 | (octets[3] & 0x3F);
	      break;
	  }

	  return codePoint > 0x10FFFF ? null : codePoint;
	};

	var decode = function (input) {
	  input = replace$1(input, plus, ' ');
	  var length = input.length;
	  var result = '';
	  var i = 0;

	  while (i < length) {
	    var decodedChar = charAt$1(input, i);

	    if (decodedChar === '%') {
	      if (charAt$1(input, i + 1) === '%' || i + 3 > length) {
	        result += '%';
	        i++;
	        continue;
	      }

	      var octet = parseHexOctet(input, i + 1);

	      // eslint-disable-next-line no-self-compare -- NaN check
	      if (octet !== octet) {
	        result += decodedChar;
	        i++;
	        continue;
	      }

	      i += 2;
	      var byteSequenceLength = getLeadingOnes(octet);

	      if (byteSequenceLength === 0) {
	        decodedChar = fromCharCode(octet);
	      } else {
	        if (byteSequenceLength === 1 || byteSequenceLength > 4) {
	          result += FALLBACK_REPLACER;
	          i++;
	          continue;
	        }

	        var octets = [octet];
	        var sequenceIndex = 1;

	        while (sequenceIndex < byteSequenceLength) {
	          i++;
	          if (i + 3 > length || charAt$1(input, i) !== '%') break;

	          var nextByte = parseHexOctet(input, i + 1);

	          // eslint-disable-next-line no-self-compare -- NaN check
	          if (nextByte !== nextByte) {
	            i += 3;
	            break;
	          }
	          if (nextByte > 191 || nextByte < 128) break;

	          push$2(octets, nextByte);
	          i += 2;
	          sequenceIndex++;
	        }

	        if (octets.length !== byteSequenceLength) {
	          result += FALLBACK_REPLACER;
	          continue;
	        }

	        var codePoint = utf8Decode(octets);
	        if (codePoint === null) {
	          result += FALLBACK_REPLACER;
	        } else {
	          decodedChar = fromCodePoint(codePoint);
	        }
	      }
	    }

	    result += decodedChar;
	    i++;
	  }

	  return result;
	};

	var find = /[!'()~]|%20/g;

	var replacements = {
	  '!': '%21',
	  "'": '%27',
	  '(': '%28',
	  ')': '%29',
	  '~': '%7E',
	  '%20': '+'
	};

	var replacer = function (match) {
	  return replacements[match];
	};

	var serialize = function (it) {
	  return replace$1(encodeURIComponent$1(it), find, replacer);
	};

	var URLSearchParamsIterator = createIteratorConstructor(function Iterator(params, kind) {
	  setInternalState$1(this, {
	    type: URL_SEARCH_PARAMS_ITERATOR,
	    target: getInternalParamsState(params).entries,
	    index: 0,
	    kind: kind
	  });
	}, URL_SEARCH_PARAMS, function next() {
	  var state = getInternalIteratorState(this);
	  var target = state.target;
	  var index = state.index++;
	  if (!target || index >= target.length) {
	    state.target = null;
	    return createIterResultObject(undefined, true);
	  }
	  var entry = target[index];
	  switch (state.kind) {
	    case 'keys': return createIterResultObject(entry.key, false);
	    case 'values': return createIterResultObject(entry.value, false);
	  } return createIterResultObject([entry.key, entry.value], false);
	}, true);

	var URLSearchParamsState = function (init) {
	  this.entries = [];
	  this.url = null;

	  if (init !== undefined) {
	    if (isObject(init)) this.parseObject(init);
	    else this.parseQuery(typeof init == 'string' ? charAt$1(init, 0) === '?' ? stringSlice$1(init, 1) : init : $toString$1(init));
	  }
	};

	URLSearchParamsState.prototype = {
	  type: URL_SEARCH_PARAMS,
	  bindURL: function (url) {
	    this.url = url;
	    this.update();
	  },
	  parseObject: function (object) {
	    var entries = this.entries;
	    var iteratorMethod = getIteratorMethod(object);
	    var iterator, next, step, entryIterator, entryNext, first, second;

	    if (iteratorMethod) {
	      iterator = getIterator(object, iteratorMethod);
	      next = iterator.next;
	      while (!(step = call$1(next, iterator)).done) {
	        entryIterator = getIterator(anObject(step.value));
	        entryNext = entryIterator.next;
	        if (
	          (first = call$1(entryNext, entryIterator)).done ||
	          (second = call$1(entryNext, entryIterator)).done ||
	          !call$1(entryNext, entryIterator).done
	        ) throw new TypeError$2('Expected sequence with length 2');
	        push$2(entries, { key: $toString$1(first.value), value: $toString$1(second.value) });
	      }
	    } else for (var key in object) if (hasOwn$1(object, key)) {
	      push$2(entries, { key: key, value: $toString$1(object[key]) });
	    }
	  },
	  parseQuery: function (query) {
	    if (query) {
	      var entries = this.entries;
	      var attributes = split$1(query, '&');
	      var index = 0;
	      var attribute, entry;
	      while (index < attributes.length) {
	        attribute = attributes[index++];
	        if (attribute.length) {
	          entry = split$1(attribute, '=');
	          push$2(entries, {
	            key: decode(shift$1(entry)),
	            value: decode(join$1(entry, '='))
	          });
	        }
	      }
	    }
	  },
	  serialize: function () {
	    var entries = this.entries;
	    var result = [];
	    var index = 0;
	    var entry;
	    while (index < entries.length) {
	      entry = entries[index++];
	      push$2(result, serialize(entry.key) + '=' + serialize(entry.value));
	    } return join$1(result, '&');
	  },
	  update: function () {
	    this.entries.length = 0;
	    this.parseQuery(this.url.query);
	  },
	  updateURL: function () {
	    if (this.url) this.url.update();
	  }
	};

	// `URLSearchParams` constructor
	// https://url.spec.whatwg.org/#interface-urlsearchparams
	var URLSearchParamsConstructor = function URLSearchParams(/* init */) {
	  anInstance$1(this, URLSearchParamsPrototype$3);
	  var init = arguments.length > 0 ? arguments[0] : undefined;
	  var state = setInternalState$1(this, new URLSearchParamsState(init));
	  if (!DESCRIPTORS$2) this.size = state.entries.length;
	};

	var URLSearchParamsPrototype$3 = URLSearchParamsConstructor.prototype;

	defineBuiltIns(URLSearchParamsPrototype$3, {
	  // `URLSearchParams.prototype.append` method
	  // https://url.spec.whatwg.org/#dom-urlsearchparams-append
	  append: function append(name, value) {
	    var state = getInternalParamsState(this);
	    validateArgumentsLength$3(arguments.length, 2);
	    push$2(state.entries, { key: $toString$1(name), value: $toString$1(value) });
	    if (!DESCRIPTORS$2) this.length++;
	    state.updateURL();
	  },
	  // `URLSearchParams.prototype.delete` method
	  // https://url.spec.whatwg.org/#dom-urlsearchparams-delete
	  'delete': function (name /* , value */) {
	    var state = getInternalParamsState(this);
	    var length = validateArgumentsLength$3(arguments.length, 1);
	    var entries = state.entries;
	    var key = $toString$1(name);
	    var $value = length < 2 ? undefined : arguments[1];
	    var value = $value === undefined ? $value : $toString$1($value);
	    var index = 0;
	    while (index < entries.length) {
	      var entry = entries[index];
	      if (entry.key === key && (value === undefined || entry.value === value)) {
	        splice(entries, index, 1);
	        if (value !== undefined) break;
	      } else index++;
	    }
	    if (!DESCRIPTORS$2) this.size = entries.length;
	    state.updateURL();
	  },
	  // `URLSearchParams.prototype.get` method
	  // https://url.spec.whatwg.org/#dom-urlsearchparams-get
	  get: function get(name) {
	    var entries = getInternalParamsState(this).entries;
	    validateArgumentsLength$3(arguments.length, 1);
	    var key = $toString$1(name);
	    var index = 0;
	    for (; index < entries.length; index++) {
	      if (entries[index].key === key) return entries[index].value;
	    }
	    return null;
	  },
	  // `URLSearchParams.prototype.getAll` method
	  // https://url.spec.whatwg.org/#dom-urlsearchparams-getall
	  getAll: function getAll(name) {
	    var entries = getInternalParamsState(this).entries;
	    validateArgumentsLength$3(arguments.length, 1);
	    var key = $toString$1(name);
	    var result = [];
	    var index = 0;
	    for (; index < entries.length; index++) {
	      if (entries[index].key === key) push$2(result, entries[index].value);
	    }
	    return result;
	  },
	  // `URLSearchParams.prototype.has` method
	  // https://url.spec.whatwg.org/#dom-urlsearchparams-has
	  has: function has(name /* , value */) {
	    var entries = getInternalParamsState(this).entries;
	    var length = validateArgumentsLength$3(arguments.length, 1);
	    var key = $toString$1(name);
	    var $value = length < 2 ? undefined : arguments[1];
	    var value = $value === undefined ? $value : $toString$1($value);
	    var index = 0;
	    while (index < entries.length) {
	      var entry = entries[index++];
	      if (entry.key === key && (value === undefined || entry.value === value)) return true;
	    }
	    return false;
	  },
	  // `URLSearchParams.prototype.set` method
	  // https://url.spec.whatwg.org/#dom-urlsearchparams-set
	  set: function set(name, value) {
	    var state = getInternalParamsState(this);
	    validateArgumentsLength$3(arguments.length, 1);
	    var entries = state.entries;
	    var found = false;
	    var key = $toString$1(name);
	    var val = $toString$1(value);
	    var index = 0;
	    var entry;
	    for (; index < entries.length; index++) {
	      entry = entries[index];
	      if (entry.key === key) {
	        if (found) splice(entries, index--, 1);
	        else {
	          found = true;
	          entry.value = val;
	        }
	      }
	    }
	    if (!found) push$2(entries, { key: key, value: val });
	    if (!DESCRIPTORS$2) this.size = entries.length;
	    state.updateURL();
	  },
	  // `URLSearchParams.prototype.sort` method
	  // https://url.spec.whatwg.org/#dom-urlsearchparams-sort
	  sort: function sort() {
	    var state = getInternalParamsState(this);
	    arraySort(state.entries, function (a, b) {
	      return a.key > b.key ? 1 : -1;
	    });
	    state.updateURL();
	  },
	  // `URLSearchParams.prototype.forEach` method
	  forEach: function forEach(callback /* , thisArg */) {
	    var entries = getInternalParamsState(this).entries;
	    var boundFunction = bind$1(callback, arguments.length > 1 ? arguments[1] : undefined);
	    var index = 0;
	    var entry;
	    while (index < entries.length) {
	      entry = entries[index++];
	      boundFunction(entry.value, entry.key, this);
	    }
	  },
	  // `URLSearchParams.prototype.keys` method
	  keys: function keys() {
	    return new URLSearchParamsIterator(this, 'keys');
	  },
	  // `URLSearchParams.prototype.values` method
	  values: function values() {
	    return new URLSearchParamsIterator(this, 'values');
	  },
	  // `URLSearchParams.prototype.entries` method
	  entries: function entries() {
	    return new URLSearchParamsIterator(this, 'entries');
	  }
	}, { enumerable: true });

	// `URLSearchParams.prototype[@@iterator]` method
	defineBuiltIn$3(URLSearchParamsPrototype$3, ITERATOR, URLSearchParamsPrototype$3.entries, { name: 'entries' });

	// `URLSearchParams.prototype.toString` method
	// https://url.spec.whatwg.org/#urlsearchparams-stringification-behavior
	defineBuiltIn$3(URLSearchParamsPrototype$3, 'toString', function toString() {
	  return getInternalParamsState(this).serialize();
	}, { enumerable: true });

	// `URLSearchParams.prototype.size` getter
	// https://github.com/whatwg/url/pull/734
	if (DESCRIPTORS$2) defineBuiltInAccessor$2(URLSearchParamsPrototype$3, 'size', {
	  get: function size() {
	    return getInternalParamsState(this).entries.length;
	  },
	  configurable: true,
	  enumerable: true
	});

	setToStringTag$1(URLSearchParamsConstructor, URL_SEARCH_PARAMS);

	$$2({ global: true, constructor: true, forced: !USE_NATIVE_URL$1 }, {
	  URLSearchParams: URLSearchParamsConstructor
	});

	// Wrap `fetch` and `Request` for correct work with polyfilled `URLSearchParams`
	if (!USE_NATIVE_URL$1 && isCallable(Headers)) {
	  var headersHas = uncurryThis$4(HeadersPrototype.has);
	  var headersSet = uncurryThis$4(HeadersPrototype.set);

	  var wrapRequestOptions = function (init) {
	    if (isObject(init)) {
	      var body = init.body;
	      var headers;
	      if (classof(body) === URL_SEARCH_PARAMS) {
	        headers = init.headers ? new Headers(init.headers) : new Headers();
	        if (!headersHas(headers, 'content-type')) {
	          headersSet(headers, 'content-type', 'application/x-www-form-urlencoded;charset=UTF-8');
	        }
	        return create(init, {
	          body: createPropertyDescriptor(0, $toString$1(body)),
	          headers: createPropertyDescriptor(0, headers)
	        });
	      }
	    } return init;
	  };

	  if (isCallable(nativeFetch)) {
	    $$2({ global: true, enumerable: true, dontCallGetSet: true, forced: true }, {
	      fetch: function fetch(input /* , init */) {
	        return nativeFetch(input, arguments.length > 1 ? wrapRequestOptions(arguments[1]) : {});
	      }
	    });
	  }

	  if (isCallable(NativeRequest)) {
	    var RequestConstructor = function Request(input /* , init */) {
	      anInstance$1(this, RequestPrototype);
	      return new NativeRequest(input, arguments.length > 1 ? wrapRequestOptions(arguments[1]) : {});
	    };

	    RequestPrototype.constructor = RequestConstructor;
	    RequestConstructor.prototype = RequestPrototype;

	    $$2({ global: true, constructor: true, dontCallGetSet: true, forced: true }, {
	      Request: RequestConstructor
	    });
	  }
	}

	var web_urlSearchParams_constructor = {
	  URLSearchParams: URLSearchParamsConstructor,
	  getState: getInternalParamsState
	};

	// TODO: in core-js@4, move /modules/ dependencies to public entries for better optimization by tools like `preset-env`

	var $$1 = _export;
	var DESCRIPTORS$1 = descriptors;
	var USE_NATIVE_URL = urlConstructorDetection;
	var globalThis$1 = globalThis_1;
	var bind = functionBindContext;
	var uncurryThis$3 = functionUncurryThis;
	var defineBuiltIn$2 = defineBuiltIn$k;
	var defineBuiltInAccessor$1 = defineBuiltInAccessor$b;
	var anInstance = anInstance$9;
	var hasOwn = hasOwnProperty_1;
	var assign = objectAssign;
	var arrayFrom = arrayFrom$1;
	var arraySlice = arraySlice$5;
	var codeAt = stringMultibyte.codeAt;
	var toASCII = stringPunycodeToAscii;
	var $toString = toString$o;
	var setToStringTag = setToStringTag$9;
	var validateArgumentsLength$2 = validateArgumentsLength$6;
	var URLSearchParamsModule = web_urlSearchParams_constructor;
	var InternalStateModule = internalState;

	var setInternalState = InternalStateModule.set;
	var getInternalURLState = InternalStateModule.getterFor('URL');
	var URLSearchParams$1 = URLSearchParamsModule.URLSearchParams;
	var getInternalSearchParamsState = URLSearchParamsModule.getState;

	var NativeURL = globalThis$1.URL;
	var TypeError$1 = globalThis$1.TypeError;
	var parseInt$1 = globalThis$1.parseInt;
	var floor = Math.floor;
	var pow = Math.pow;
	var charAt = uncurryThis$3(''.charAt);
	var exec = uncurryThis$3(/./.exec);
	var join = uncurryThis$3([].join);
	var numberToString = uncurryThis$3(1.0.toString);
	var pop = uncurryThis$3([].pop);
	var push$1 = uncurryThis$3([].push);
	var replace = uncurryThis$3(''.replace);
	var shift = uncurryThis$3([].shift);
	var split = uncurryThis$3(''.split);
	var stringSlice = uncurryThis$3(''.slice);
	var toLowerCase = uncurryThis$3(''.toLowerCase);
	var unshift = uncurryThis$3([].unshift);

	var INVALID_AUTHORITY = 'Invalid authority';
	var INVALID_SCHEME = 'Invalid scheme';
	var INVALID_HOST = 'Invalid host';
	var INVALID_PORT = 'Invalid port';

	var ALPHA = /[a-z]/i;
	// eslint-disable-next-line regexp/no-obscure-range -- safe
	var ALPHANUMERIC = /[\d+-.a-z]/i;
	var DIGIT = /\d/;
	var HEX_START = /^0x/i;
	var OCT = /^[0-7]+$/;
	var DEC = /^\d+$/;
	var HEX = /^[\da-f]+$/i;
	/* eslint-disable regexp/no-control-character -- safe */
	var FORBIDDEN_HOST_CODE_POINT = /[\0\t\n\r #%/:<>?@[\\\]^|]/;
	var FORBIDDEN_HOST_CODE_POINT_EXCLUDING_PERCENT = /[\0\t\n\r #/:<>?@[\\\]^|]/;
	var LEADING_C0_CONTROL_OR_SPACE = /^[\u0000-\u0020]+/;
	var TRAILING_C0_CONTROL_OR_SPACE = /(^|[^\u0000-\u0020])[\u0000-\u0020]+$/;
	var TAB_AND_NEW_LINE = /[\t\n\r]/g;
	/* eslint-enable regexp/no-control-character -- safe */
	var EOF;

	// https://url.spec.whatwg.org/#ipv4-number-parser
	var parseIPv4 = function (input) {
	  var parts = split(input, '.');
	  var partsLength, numbers, index, part, radix, number, ipv4;
	  if (parts.length && parts[parts.length - 1] === '') {
	    parts.length--;
	  }
	  partsLength = parts.length;
	  if (partsLength > 4) return input;
	  numbers = [];
	  for (index = 0; index < partsLength; index++) {
	    part = parts[index];
	    if (part === '') return input;
	    radix = 10;
	    if (part.length > 1 && charAt(part, 0) === '0') {
	      radix = exec(HEX_START, part) ? 16 : 8;
	      part = stringSlice(part, radix === 8 ? 1 : 2);
	    }
	    if (part === '') {
	      number = 0;
	    } else {
	      if (!exec(radix === 10 ? DEC : radix === 8 ? OCT : HEX, part)) return input;
	      number = parseInt$1(part, radix);
	    }
	    push$1(numbers, number);
	  }
	  for (index = 0; index < partsLength; index++) {
	    number = numbers[index];
	    if (index === partsLength - 1) {
	      if (number >= pow(256, 5 - partsLength)) return null;
	    } else if (number > 255) return null;
	  }
	  ipv4 = pop(numbers);
	  for (index = 0; index < numbers.length; index++) {
	    ipv4 += numbers[index] * pow(256, 3 - index);
	  }
	  return ipv4;
	};

	// https://url.spec.whatwg.org/#concept-ipv6-parser
	// eslint-disable-next-line max-statements -- TODO
	var parseIPv6 = function (input) {
	  var address = [0, 0, 0, 0, 0, 0, 0, 0];
	  var pieceIndex = 0;
	  var compress = null;
	  var pointer = 0;
	  var value, length, numbersSeen, ipv4Piece, number, swaps, swap;

	  var chr = function () {
	    return charAt(input, pointer);
	  };

	  if (chr() === ':') {
	    if (charAt(input, 1) !== ':') return;
	    pointer += 2;
	    pieceIndex++;
	    compress = pieceIndex;
	  }
	  while (chr()) {
	    if (pieceIndex === 8) return;
	    if (chr() === ':') {
	      if (compress !== null) return;
	      pointer++;
	      pieceIndex++;
	      compress = pieceIndex;
	      continue;
	    }
	    value = length = 0;
	    while (length < 4 && exec(HEX, chr())) {
	      value = value * 16 + parseInt$1(chr(), 16);
	      pointer++;
	      length++;
	    }
	    if (chr() === '.') {
	      if (length === 0) return;
	      pointer -= length;
	      if (pieceIndex > 6) return;
	      numbersSeen = 0;
	      while (chr()) {
	        ipv4Piece = null;
	        if (numbersSeen > 0) {
	          if (chr() === '.' && numbersSeen < 4) pointer++;
	          else return;
	        }
	        if (!exec(DIGIT, chr())) return;
	        while (exec(DIGIT, chr())) {
	          number = parseInt$1(chr(), 10);
	          if (ipv4Piece === null) ipv4Piece = number;
	          else if (ipv4Piece === 0) return;
	          else ipv4Piece = ipv4Piece * 10 + number;
	          if (ipv4Piece > 255) return;
	          pointer++;
	        }
	        address[pieceIndex] = address[pieceIndex] * 256 + ipv4Piece;
	        numbersSeen++;
	        if (numbersSeen === 2 || numbersSeen === 4) pieceIndex++;
	      }
	      if (numbersSeen !== 4) return;
	      break;
	    } else if (chr() === ':') {
	      pointer++;
	      if (!chr()) return;
	    } else if (chr()) return;
	    address[pieceIndex++] = value;
	  }
	  if (compress !== null) {
	    swaps = pieceIndex - compress;
	    pieceIndex = 7;
	    while (pieceIndex !== 0 && swaps > 0) {
	      swap = address[pieceIndex];
	      address[pieceIndex--] = address[compress + swaps - 1];
	      address[compress + --swaps] = swap;
	    }
	  } else if (pieceIndex !== 8) return;
	  return address;
	};

	var findLongestZeroSequence = function (ipv6) {
	  var maxIndex = null;
	  var maxLength = 1;
	  var currStart = null;
	  var currLength = 0;
	  var index = 0;
	  for (; index < 8; index++) {
	    if (ipv6[index] !== 0) {
	      if (currLength > maxLength) {
	        maxIndex = currStart;
	        maxLength = currLength;
	      }
	      currStart = null;
	      currLength = 0;
	    } else {
	      if (currStart === null) currStart = index;
	      ++currLength;
	    }
	  }
	  return currLength > maxLength ? currStart : maxIndex;
	};

	// https://url.spec.whatwg.org/#host-serializing
	var serializeHost = function (host) {
	  var result, index, compress, ignore0;

	  // ipv4
	  if (typeof host == 'number') {
	    result = [];
	    for (index = 0; index < 4; index++) {
	      unshift(result, host % 256);
	      host = floor(host / 256);
	    }
	    return join(result, '.');
	  }

	  // ipv6
	  if (typeof host == 'object') {
	    result = '';
	    compress = findLongestZeroSequence(host);
	    for (index = 0; index < 8; index++) {
	      if (ignore0 && host[index] === 0) continue;
	      if (ignore0) ignore0 = false;
	      if (compress === index) {
	        result += index ? ':' : '::';
	        ignore0 = true;
	      } else {
	        result += numberToString(host[index], 16);
	        if (index < 7) result += ':';
	      }
	    }
	    return '[' + result + ']';
	  }

	  return host;
	};

	var C0ControlPercentEncodeSet = {};
	var fragmentPercentEncodeSet = assign({}, C0ControlPercentEncodeSet, {
	  ' ': 1, '"': 1, '<': 1, '>': 1, '`': 1
	});
	var pathPercentEncodeSet = assign({}, fragmentPercentEncodeSet, {
	  '#': 1, '?': 1, '{': 1, '}': 1
	});
	var userinfoPercentEncodeSet = assign({}, pathPercentEncodeSet, {
	  '/': 1, ':': 1, ';': 1, '=': 1, '@': 1, '[': 1, '\\': 1, ']': 1, '^': 1, '|': 1
	});

	var percentEncode = function (chr, set) {
	  var code = codeAt(chr, 0);
	  return code > 0x20 && code < 0x7F && !hasOwn(set, chr) ? chr : encodeURIComponent(chr);
	};

	// https://url.spec.whatwg.org/#special-scheme
	var specialSchemes = {
	  ftp: 21,
	  file: null,
	  http: 80,
	  https: 443,
	  ws: 80,
	  wss: 443
	};

	// https://url.spec.whatwg.org/#windows-drive-letter
	var isWindowsDriveLetter = function (string, normalized) {
	  var second;
	  return string.length === 2 && exec(ALPHA, charAt(string, 0))
	    && ((second = charAt(string, 1)) === ':' || (!normalized && second === '|'));
	};

	// https://url.spec.whatwg.org/#start-with-a-windows-drive-letter
	var startsWithWindowsDriveLetter = function (string) {
	  var third;
	  return string.length > 1 && isWindowsDriveLetter(stringSlice(string, 0, 2)) && (
	    string.length === 2 ||
	    ((third = charAt(string, 2)) === '/' || third === '\\' || third === '?' || third === '#')
	  );
	};

	// https://url.spec.whatwg.org/#single-dot-path-segment
	var isSingleDot = function (segment) {
	  return segment === '.' || toLowerCase(segment) === '%2e';
	};

	// https://url.spec.whatwg.org/#double-dot-path-segment
	var isDoubleDot = function (segment) {
	  segment = toLowerCase(segment);
	  return segment === '..' || segment === '%2e.' || segment === '.%2e' || segment === '%2e%2e';
	};

	// States:
	var SCHEME_START = {};
	var SCHEME = {};
	var NO_SCHEME = {};
	var SPECIAL_RELATIVE_OR_AUTHORITY = {};
	var PATH_OR_AUTHORITY = {};
	var RELATIVE = {};
	var RELATIVE_SLASH = {};
	var SPECIAL_AUTHORITY_SLASHES = {};
	var SPECIAL_AUTHORITY_IGNORE_SLASHES = {};
	var AUTHORITY = {};
	var HOST = {};
	var HOSTNAME = {};
	var PORT = {};
	var FILE = {};
	var FILE_SLASH = {};
	var FILE_HOST = {};
	var PATH_START = {};
	var PATH = {};
	var CANNOT_BE_A_BASE_URL_PATH = {};
	var QUERY = {};
	var FRAGMENT = {};

	var URLState = function (url, isBase, base) {
	  var urlString = $toString(url);
	  var baseState, failure, searchParams;
	  if (isBase) {
	    failure = this.parse(urlString);
	    if (failure) throw new TypeError$1(failure);
	    this.searchParams = null;
	  } else {
	    if (base !== undefined) baseState = new URLState(base, true);
	    failure = this.parse(urlString, null, baseState);
	    if (failure) throw new TypeError$1(failure);
	    searchParams = getInternalSearchParamsState(new URLSearchParams$1());
	    searchParams.bindURL(this);
	    this.searchParams = searchParams;
	  }
	};

	URLState.prototype = {
	  type: 'URL',
	  // https://url.spec.whatwg.org/#url-parsing
	  // eslint-disable-next-line max-statements -- TODO
	  parse: function (input, stateOverride, base) {
	    var url = this;
	    var state = stateOverride || SCHEME_START;
	    var pointer = 0;
	    var buffer = '';
	    var seenAt = false;
	    var seenBracket = false;
	    var seenPasswordToken = false;
	    var codePoints, chr, bufferCodePoints, failure;

	    input = $toString(input);

	    if (!stateOverride) {
	      url.scheme = '';
	      url.username = '';
	      url.password = '';
	      url.host = null;
	      url.port = null;
	      url.path = [];
	      url.query = null;
	      url.fragment = null;
	      url.cannotBeABaseURL = false;
	      input = replace(input, LEADING_C0_CONTROL_OR_SPACE, '');
	      input = replace(input, TRAILING_C0_CONTROL_OR_SPACE, '$1');
	    }

	    input = replace(input, TAB_AND_NEW_LINE, '');

	    codePoints = arrayFrom(input);

	    while (pointer <= codePoints.length) {
	      chr = codePoints[pointer];
	      switch (state) {
	        case SCHEME_START:
	          if (chr && exec(ALPHA, chr)) {
	            buffer += toLowerCase(chr);
	            state = SCHEME;
	          } else if (!stateOverride) {
	            state = NO_SCHEME;
	            continue;
	          } else return INVALID_SCHEME;
	          break;

	        case SCHEME:
	          if (chr && (exec(ALPHANUMERIC, chr) || chr === '+' || chr === '-' || chr === '.')) {
	            buffer += toLowerCase(chr);
	          } else if (chr === ':') {
	            if (stateOverride && (
	              (url.isSpecial() !== hasOwn(specialSchemes, buffer)) ||
	              (buffer === 'file' && (url.includesCredentials() || url.port !== null)) ||
	              (url.scheme === 'file' && !url.host)
	            )) return;
	            url.scheme = buffer;
	            if (stateOverride) {
	              if (url.isSpecial() && specialSchemes[url.scheme] === url.port) url.port = null;
	              return;
	            }
	            buffer = '';
	            if (url.scheme === 'file') {
	              state = FILE;
	            } else if (url.isSpecial() && base && base.scheme === url.scheme) {
	              state = SPECIAL_RELATIVE_OR_AUTHORITY;
	            } else if (url.isSpecial()) {
	              state = SPECIAL_AUTHORITY_SLASHES;
	            } else if (codePoints[pointer + 1] === '/') {
	              state = PATH_OR_AUTHORITY;
	              pointer++;
	            } else {
	              url.cannotBeABaseURL = true;
	              push$1(url.path, '');
	              state = CANNOT_BE_A_BASE_URL_PATH;
	            }
	          } else if (!stateOverride) {
	            buffer = '';
	            state = NO_SCHEME;
	            pointer = 0;
	            continue;
	          } else return INVALID_SCHEME;
	          break;

	        case NO_SCHEME:
	          if (!base || (base.cannotBeABaseURL && chr !== '#')) return INVALID_SCHEME;
	          if (base.cannotBeABaseURL && chr === '#') {
	            url.scheme = base.scheme;
	            url.path = arraySlice(base.path);
	            url.query = base.query;
	            url.fragment = '';
	            url.cannotBeABaseURL = true;
	            state = FRAGMENT;
	            break;
	          }
	          state = base.scheme === 'file' ? FILE : RELATIVE;
	          continue;

	        case SPECIAL_RELATIVE_OR_AUTHORITY:
	          if (chr === '/' && codePoints[pointer + 1] === '/') {
	            state = SPECIAL_AUTHORITY_IGNORE_SLASHES;
	            pointer++;
	          } else {
	            state = RELATIVE;
	            continue;
	          } break;

	        case PATH_OR_AUTHORITY:
	          if (chr === '/') {
	            state = AUTHORITY;
	            break;
	          } else {
	            state = PATH;
	            continue;
	          }

	        case RELATIVE:
	          url.scheme = base.scheme;
	          if (chr === EOF) {
	            url.username = base.username;
	            url.password = base.password;
	            url.host = base.host;
	            url.port = base.port;
	            url.path = arraySlice(base.path);
	            url.query = base.query;
	          } else if (chr === '/' || (chr === '\\' && url.isSpecial())) {
	            state = RELATIVE_SLASH;
	          } else if (chr === '?') {
	            url.username = base.username;
	            url.password = base.password;
	            url.host = base.host;
	            url.port = base.port;
	            url.path = arraySlice(base.path);
	            url.query = '';
	            state = QUERY;
	          } else if (chr === '#') {
	            url.username = base.username;
	            url.password = base.password;
	            url.host = base.host;
	            url.port = base.port;
	            url.path = arraySlice(base.path);
	            url.query = base.query;
	            url.fragment = '';
	            state = FRAGMENT;
	          } else {
	            url.username = base.username;
	            url.password = base.password;
	            url.host = base.host;
	            url.port = base.port;
	            url.path = arraySlice(base.path);
	            url.path.length--;
	            state = PATH;
	            continue;
	          } break;

	        case RELATIVE_SLASH:
	          if (url.isSpecial() && (chr === '/' || chr === '\\')) {
	            state = SPECIAL_AUTHORITY_IGNORE_SLASHES;
	          } else if (chr === '/') {
	            state = AUTHORITY;
	          } else {
	            url.username = base.username;
	            url.password = base.password;
	            url.host = base.host;
	            url.port = base.port;
	            state = PATH;
	            continue;
	          } break;

	        case SPECIAL_AUTHORITY_SLASHES:
	          state = SPECIAL_AUTHORITY_IGNORE_SLASHES;
	          if (chr !== '/' || charAt(buffer, pointer + 1) !== '/') continue;
	          pointer++;
	          break;

	        case SPECIAL_AUTHORITY_IGNORE_SLASHES:
	          if (chr !== '/' && chr !== '\\') {
	            state = AUTHORITY;
	            continue;
	          } break;

	        case AUTHORITY:
	          if (chr === '@') {
	            if (seenAt) buffer = '%40' + buffer;
	            seenAt = true;
	            bufferCodePoints = arrayFrom(buffer);
	            for (var i = 0; i < bufferCodePoints.length; i++) {
	              var codePoint = bufferCodePoints[i];
	              if (codePoint === ':' && !seenPasswordToken) {
	                seenPasswordToken = true;
	                continue;
	              }
	              var encodedCodePoints = percentEncode(codePoint, userinfoPercentEncodeSet);
	              if (seenPasswordToken) url.password += encodedCodePoints;
	              else url.username += encodedCodePoints;
	            }
	            buffer = '';
	          } else if (
	            chr === EOF || chr === '/' || chr === '?' || chr === '#' ||
	            (chr === '\\' && url.isSpecial())
	          ) {
	            if (seenAt && buffer === '') return INVALID_AUTHORITY;
	            pointer -= arrayFrom(buffer).length + 1;
	            buffer = '';
	            state = HOST;
	          } else buffer += chr;
	          break;

	        case HOST:
	        case HOSTNAME:
	          if (stateOverride && url.scheme === 'file') {
	            state = FILE_HOST;
	            continue;
	          } else if (chr === ':' && !seenBracket) {
	            if (buffer === '') return INVALID_HOST;
	            failure = url.parseHost(buffer);
	            if (failure) return failure;
	            buffer = '';
	            state = PORT;
	            if (stateOverride === HOSTNAME) return;
	          } else if (
	            chr === EOF || chr === '/' || chr === '?' || chr === '#' ||
	            (chr === '\\' && url.isSpecial())
	          ) {
	            if (url.isSpecial() && buffer === '') return INVALID_HOST;
	            if (stateOverride && buffer === '' && (url.includesCredentials() || url.port !== null)) return;
	            failure = url.parseHost(buffer);
	            if (failure) return failure;
	            buffer = '';
	            state = PATH_START;
	            if (stateOverride) return;
	            continue;
	          } else {
	            if (chr === '[') seenBracket = true;
	            else if (chr === ']') seenBracket = false;
	            buffer += chr;
	          } break;

	        case PORT:
	          if (exec(DIGIT, chr)) {
	            buffer += chr;
	          } else if (
	            chr === EOF || chr === '/' || chr === '?' || chr === '#' ||
	            (chr === '\\' && url.isSpecial()) ||
	            stateOverride
	          ) {
	            if (buffer !== '') {
	              var port = parseInt$1(buffer, 10);
	              if (port > 0xFFFF) return INVALID_PORT;
	              url.port = (url.isSpecial() && port === specialSchemes[url.scheme]) ? null : port;
	              buffer = '';
	            }
	            if (stateOverride) return;
	            state = PATH_START;
	            continue;
	          } else return INVALID_PORT;
	          break;

	        case FILE:
	          url.scheme = 'file';
	          if (chr === '/' || chr === '\\') state = FILE_SLASH;
	          else if (base && base.scheme === 'file') {
	            switch (chr) {
	              case EOF:
	                url.host = base.host;
	                url.path = arraySlice(base.path);
	                url.query = base.query;
	                break;
	              case '?':
	                url.host = base.host;
	                url.path = arraySlice(base.path);
	                url.query = '';
	                state = QUERY;
	                break;
	              case '#':
	                url.host = base.host;
	                url.path = arraySlice(base.path);
	                url.query = base.query;
	                url.fragment = '';
	                state = FRAGMENT;
	                break;
	              default:
	                if (!startsWithWindowsDriveLetter(join(arraySlice(codePoints, pointer), ''))) {
	                  url.host = base.host;
	                  url.path = arraySlice(base.path);
	                  url.shortenPath();
	                }
	                state = PATH;
	                continue;
	            }
	          } else {
	            state = PATH;
	            continue;
	          } break;

	        case FILE_SLASH:
	          if (chr === '/' || chr === '\\') {
	            state = FILE_HOST;
	            break;
	          }
	          if (base && base.scheme === 'file' && !startsWithWindowsDriveLetter(join(arraySlice(codePoints, pointer), ''))) {
	            if (isWindowsDriveLetter(base.path[0], true)) push$1(url.path, base.path[0]);
	            else url.host = base.host;
	          }
	          state = PATH;
	          continue;

	        case FILE_HOST:
	          if (chr === EOF || chr === '/' || chr === '\\' || chr === '?' || chr === '#') {
	            if (!stateOverride && isWindowsDriveLetter(buffer)) {
	              state = PATH;
	            } else if (buffer === '') {
	              url.host = '';
	              if (stateOverride) return;
	              state = PATH_START;
	            } else {
	              failure = url.parseHost(buffer);
	              if (failure) return failure;
	              if (url.host === 'localhost') url.host = '';
	              if (stateOverride) return;
	              buffer = '';
	              state = PATH_START;
	            } continue;
	          } else buffer += chr;
	          break;

	        case PATH_START:
	          if (url.isSpecial()) {
	            state = PATH;
	            if (chr !== '/' && chr !== '\\') continue;
	          } else if (!stateOverride && chr === '?') {
	            url.query = '';
	            state = QUERY;
	          } else if (!stateOverride && chr === '#') {
	            url.fragment = '';
	            state = FRAGMENT;
	          } else if (chr !== EOF) {
	            state = PATH;
	            if (chr !== '/') continue;
	          } break;

	        case PATH:
	          if (
	            chr === EOF || chr === '/' ||
	            (chr === '\\' && url.isSpecial()) ||
	            (!stateOverride && (chr === '?' || chr === '#'))
	          ) {
	            if (isDoubleDot(buffer)) {
	              url.shortenPath();
	              if (chr !== '/' && !(chr === '\\' && url.isSpecial())) {
	                push$1(url.path, '');
	              }
	            } else if (isSingleDot(buffer)) {
	              if (chr !== '/' && !(chr === '\\' && url.isSpecial())) {
	                push$1(url.path, '');
	              }
	            } else {
	              if (url.scheme === 'file' && !url.path.length && isWindowsDriveLetter(buffer)) {
	                if (url.host) url.host = '';
	                buffer = charAt(buffer, 0) + ':'; // normalize windows drive letter
	              }
	              push$1(url.path, buffer);
	            }
	            buffer = '';
	            if (url.scheme === 'file' && (chr === EOF || chr === '?' || chr === '#')) {
	              while (url.path.length > 1 && url.path[0] === '') {
	                shift(url.path);
	              }
	            }
	            if (chr === '?') {
	              url.query = '';
	              state = QUERY;
	            } else if (chr === '#') {
	              url.fragment = '';
	              state = FRAGMENT;
	            }
	          } else {
	            buffer += percentEncode(chr, pathPercentEncodeSet);
	          } break;

	        case CANNOT_BE_A_BASE_URL_PATH:
	          if (chr === '?') {
	            url.query = '';
	            state = QUERY;
	          } else if (chr === '#') {
	            url.fragment = '';
	            state = FRAGMENT;
	          } else if (chr !== EOF) {
	            url.path[0] += percentEncode(chr, C0ControlPercentEncodeSet);
	          } break;

	        case QUERY:
	          if (!stateOverride && chr === '#') {
	            url.fragment = '';
	            state = FRAGMENT;
	          } else if (chr !== EOF) {
	            if (chr === "'" && url.isSpecial()) url.query += '%27';
	            else if (chr === '#') url.query += '%23';
	            else url.query += percentEncode(chr, C0ControlPercentEncodeSet);
	          } break;

	        case FRAGMENT:
	          if (chr !== EOF) url.fragment += percentEncode(chr, fragmentPercentEncodeSet);
	          break;
	      }

	      pointer++;
	    }
	  },
	  // https://url.spec.whatwg.org/#host-parsing
	  parseHost: function (input) {
	    var result, codePoints, index;
	    if (charAt(input, 0) === '[') {
	      if (charAt(input, input.length - 1) !== ']') return INVALID_HOST;
	      result = parseIPv6(stringSlice(input, 1, -1));
	      if (!result) return INVALID_HOST;
	      this.host = result;
	    // opaque host
	    } else if (!this.isSpecial()) {
	      if (exec(FORBIDDEN_HOST_CODE_POINT_EXCLUDING_PERCENT, input)) return INVALID_HOST;
	      result = '';
	      codePoints = arrayFrom(input);
	      for (index = 0; index < codePoints.length; index++) {
	        result += percentEncode(codePoints[index], C0ControlPercentEncodeSet);
	      }
	      this.host = result;
	    } else {
	      input = toASCII(input);
	      if (exec(FORBIDDEN_HOST_CODE_POINT, input)) return INVALID_HOST;
	      result = parseIPv4(input);
	      if (result === null) return INVALID_HOST;
	      this.host = result;
	    }
	  },
	  // https://url.spec.whatwg.org/#cannot-have-a-username-password-port
	  cannotHaveUsernamePasswordPort: function () {
	    return !this.host || this.cannotBeABaseURL || this.scheme === 'file';
	  },
	  // https://url.spec.whatwg.org/#include-credentials
	  includesCredentials: function () {
	    return this.username !== '' || this.password !== '';
	  },
	  // https://url.spec.whatwg.org/#is-special
	  isSpecial: function () {
	    return hasOwn(specialSchemes, this.scheme);
	  },
	  // https://url.spec.whatwg.org/#shorten-a-urls-path
	  shortenPath: function () {
	    var path = this.path;
	    var pathSize = path.length;
	    if (pathSize && (this.scheme !== 'file' || pathSize !== 1 || !isWindowsDriveLetter(path[0], true))) {
	      path.length--;
	    }
	  },
	  // https://url.spec.whatwg.org/#concept-url-serializer
	  serialize: function () {
	    var url = this;
	    var scheme = url.scheme;
	    var username = url.username;
	    var password = url.password;
	    var host = url.host;
	    var port = url.port;
	    var path = url.path;
	    var query = url.query;
	    var fragment = url.fragment;
	    var output = scheme + ':';
	    if (host !== null) {
	      output += '//';
	      if (url.includesCredentials()) {
	        output += username + (password ? ':' + password : '') + '@';
	      }
	      output += serializeHost(host);
	      if (port !== null) output += ':' + port;
	    } else if (scheme === 'file') output += '//';
	    output += url.cannotBeABaseURL ? path[0] : path.length ? '/' + join(path, '/') : '';
	    if (query !== null) output += '?' + query;
	    if (fragment !== null) output += '#' + fragment;
	    return output;
	  },
	  // https://url.spec.whatwg.org/#dom-url-href
	  setHref: function (href) {
	    var failure = this.parse(href);
	    if (failure) throw new TypeError$1(failure);
	    this.searchParams.update();
	  },
	  // https://url.spec.whatwg.org/#dom-url-origin
	  getOrigin: function () {
	    var scheme = this.scheme;
	    var port = this.port;
	    if (scheme === 'blob') try {
	      return new URLConstructor(scheme.path[0]).origin;
	    } catch (error) {
	      return 'null';
	    }
	    if (scheme === 'file' || !this.isSpecial()) return 'null';
	    return scheme + '://' + serializeHost(this.host) + (port !== null ? ':' + port : '');
	  },
	  // https://url.spec.whatwg.org/#dom-url-protocol
	  getProtocol: function () {
	    return this.scheme + ':';
	  },
	  setProtocol: function (protocol) {
	    this.parse($toString(protocol) + ':', SCHEME_START);
	  },
	  // https://url.spec.whatwg.org/#dom-url-username
	  getUsername: function () {
	    return this.username;
	  },
	  setUsername: function (username) {
	    var codePoints = arrayFrom($toString(username));
	    if (this.cannotHaveUsernamePasswordPort()) return;
	    this.username = '';
	    for (var i = 0; i < codePoints.length; i++) {
	      this.username += percentEncode(codePoints[i], userinfoPercentEncodeSet);
	    }
	  },
	  // https://url.spec.whatwg.org/#dom-url-password
	  getPassword: function () {
	    return this.password;
	  },
	  setPassword: function (password) {
	    var codePoints = arrayFrom($toString(password));
	    if (this.cannotHaveUsernamePasswordPort()) return;
	    this.password = '';
	    for (var i = 0; i < codePoints.length; i++) {
	      this.password += percentEncode(codePoints[i], userinfoPercentEncodeSet);
	    }
	  },
	  // https://url.spec.whatwg.org/#dom-url-host
	  getHost: function () {
	    var host = this.host;
	    var port = this.port;
	    return host === null ? ''
	      : port === null ? serializeHost(host)
	      : serializeHost(host) + ':' + port;
	  },
	  setHost: function (host) {
	    if (this.cannotBeABaseURL) return;
	    this.parse(host, HOST);
	  },
	  // https://url.spec.whatwg.org/#dom-url-hostname
	  getHostname: function () {
	    var host = this.host;
	    return host === null ? '' : serializeHost(host);
	  },
	  setHostname: function (hostname) {
	    if (this.cannotBeABaseURL) return;
	    this.parse(hostname, HOSTNAME);
	  },
	  // https://url.spec.whatwg.org/#dom-url-port
	  getPort: function () {
	    var port = this.port;
	    return port === null ? '' : $toString(port);
	  },
	  setPort: function (port) {
	    if (this.cannotHaveUsernamePasswordPort()) return;
	    port = $toString(port);
	    if (port === '') this.port = null;
	    else this.parse(port, PORT);
	  },
	  // https://url.spec.whatwg.org/#dom-url-pathname
	  getPathname: function () {
	    var path = this.path;
	    return this.cannotBeABaseURL ? path[0] : path.length ? '/' + join(path, '/') : '';
	  },
	  setPathname: function (pathname) {
	    if (this.cannotBeABaseURL) return;
	    this.path = [];
	    this.parse(pathname, PATH_START);
	  },
	  // https://url.spec.whatwg.org/#dom-url-search
	  getSearch: function () {
	    var query = this.query;
	    return query ? '?' + query : '';
	  },
	  setSearch: function (search) {
	    search = $toString(search);
	    if (search === '') {
	      this.query = null;
	    } else {
	      if (charAt(search, 0) === '?') search = stringSlice(search, 1);
	      this.query = '';
	      this.parse(search, QUERY);
	    }
	    this.searchParams.update();
	  },
	  // https://url.spec.whatwg.org/#dom-url-searchparams
	  getSearchParams: function () {
	    return this.searchParams.facade;
	  },
	  // https://url.spec.whatwg.org/#dom-url-hash
	  getHash: function () {
	    var fragment = this.fragment;
	    return fragment ? '#' + fragment : '';
	  },
	  setHash: function (hash) {
	    hash = $toString(hash);
	    if (hash === '') {
	      this.fragment = null;
	      return;
	    }
	    if (charAt(hash, 0) === '#') hash = stringSlice(hash, 1);
	    this.fragment = '';
	    this.parse(hash, FRAGMENT);
	  },
	  update: function () {
	    this.query = this.searchParams.serialize() || null;
	  }
	};

	// `URL` constructor
	// https://url.spec.whatwg.org/#url-class
	var URLConstructor = function URL(url /* , base */) {
	  var that = anInstance(this, URLPrototype);
	  var base = validateArgumentsLength$2(arguments.length, 1) > 1 ? arguments[1] : undefined;
	  var state = setInternalState(that, new URLState(url, false, base));
	  if (!DESCRIPTORS$1) {
	    that.href = state.serialize();
	    that.origin = state.getOrigin();
	    that.protocol = state.getProtocol();
	    that.username = state.getUsername();
	    that.password = state.getPassword();
	    that.host = state.getHost();
	    that.hostname = state.getHostname();
	    that.port = state.getPort();
	    that.pathname = state.getPathname();
	    that.search = state.getSearch();
	    that.searchParams = state.getSearchParams();
	    that.hash = state.getHash();
	  }
	};

	var URLPrototype = URLConstructor.prototype;

	var accessorDescriptor = function (getter, setter) {
	  return {
	    get: function () {
	      return getInternalURLState(this)[getter]();
	    },
	    set: setter && function (value) {
	      return getInternalURLState(this)[setter](value);
	    },
	    configurable: true,
	    enumerable: true
	  };
	};

	if (DESCRIPTORS$1) {
	  // `URL.prototype.href` accessors pair
	  // https://url.spec.whatwg.org/#dom-url-href
	  defineBuiltInAccessor$1(URLPrototype, 'href', accessorDescriptor('serialize', 'setHref'));
	  // `URL.prototype.origin` getter
	  // https://url.spec.whatwg.org/#dom-url-origin
	  defineBuiltInAccessor$1(URLPrototype, 'origin', accessorDescriptor('getOrigin'));
	  // `URL.prototype.protocol` accessors pair
	  // https://url.spec.whatwg.org/#dom-url-protocol
	  defineBuiltInAccessor$1(URLPrototype, 'protocol', accessorDescriptor('getProtocol', 'setProtocol'));
	  // `URL.prototype.username` accessors pair
	  // https://url.spec.whatwg.org/#dom-url-username
	  defineBuiltInAccessor$1(URLPrototype, 'username', accessorDescriptor('getUsername', 'setUsername'));
	  // `URL.prototype.password` accessors pair
	  // https://url.spec.whatwg.org/#dom-url-password
	  defineBuiltInAccessor$1(URLPrototype, 'password', accessorDescriptor('getPassword', 'setPassword'));
	  // `URL.prototype.host` accessors pair
	  // https://url.spec.whatwg.org/#dom-url-host
	  defineBuiltInAccessor$1(URLPrototype, 'host', accessorDescriptor('getHost', 'setHost'));
	  // `URL.prototype.hostname` accessors pair
	  // https://url.spec.whatwg.org/#dom-url-hostname
	  defineBuiltInAccessor$1(URLPrototype, 'hostname', accessorDescriptor('getHostname', 'setHostname'));
	  // `URL.prototype.port` accessors pair
	  // https://url.spec.whatwg.org/#dom-url-port
	  defineBuiltInAccessor$1(URLPrototype, 'port', accessorDescriptor('getPort', 'setPort'));
	  // `URL.prototype.pathname` accessors pair
	  // https://url.spec.whatwg.org/#dom-url-pathname
	  defineBuiltInAccessor$1(URLPrototype, 'pathname', accessorDescriptor('getPathname', 'setPathname'));
	  // `URL.prototype.search` accessors pair
	  // https://url.spec.whatwg.org/#dom-url-search
	  defineBuiltInAccessor$1(URLPrototype, 'search', accessorDescriptor('getSearch', 'setSearch'));
	  // `URL.prototype.searchParams` getter
	  // https://url.spec.whatwg.org/#dom-url-searchparams
	  defineBuiltInAccessor$1(URLPrototype, 'searchParams', accessorDescriptor('getSearchParams'));
	  // `URL.prototype.hash` accessors pair
	  // https://url.spec.whatwg.org/#dom-url-hash
	  defineBuiltInAccessor$1(URLPrototype, 'hash', accessorDescriptor('getHash', 'setHash'));
	}

	// `URL.prototype.toJSON` method
	// https://url.spec.whatwg.org/#dom-url-tojson
	defineBuiltIn$2(URLPrototype, 'toJSON', function toJSON() {
	  return getInternalURLState(this).serialize();
	}, { enumerable: true });

	// `URL.prototype.toString` method
	// https://url.spec.whatwg.org/#URL-stringification-behavior
	defineBuiltIn$2(URLPrototype, 'toString', function toString() {
	  return getInternalURLState(this).serialize();
	}, { enumerable: true });

	if (NativeURL) {
	  var nativeCreateObjectURL = NativeURL.createObjectURL;
	  var nativeRevokeObjectURL = NativeURL.revokeObjectURL;
	  // `URL.createObjectURL` method
	  // https://developer.mozilla.org/en-US/docs/Web/API/URL/createObjectURL
	  if (nativeCreateObjectURL) defineBuiltIn$2(URLConstructor, 'createObjectURL', bind(nativeCreateObjectURL, NativeURL));
	  // `URL.revokeObjectURL` method
	  // https://developer.mozilla.org/en-US/docs/Web/API/URL/revokeObjectURL
	  if (nativeRevokeObjectURL) defineBuiltIn$2(URLConstructor, 'revokeObjectURL', bind(nativeRevokeObjectURL, NativeURL));
	}

	setToStringTag(URLConstructor, 'URL');

	$$1({ global: true, constructor: true, forced: !USE_NATIVE_URL, sham: !DESCRIPTORS$1 }, {
	  URL: URLConstructor
	});

	var $ = _export;
	var call = functionCall;

	// `URL.prototype.toJSON` method
	// https://url.spec.whatwg.org/#dom-url-tojson
	$({ target: 'URL', proto: true, enumerable: true }, {
	  toJSON: function toJSON() {
	    return call(URL.prototype.toString, this);
	  }
	});

	var defineBuiltIn$1 = defineBuiltIn$k;
	var uncurryThis$2 = functionUncurryThis;
	var toString$1 = toString$o;
	var validateArgumentsLength$1 = validateArgumentsLength$6;

	var $URLSearchParams$1 = URLSearchParams;
	var URLSearchParamsPrototype$2 = $URLSearchParams$1.prototype;
	var append = uncurryThis$2(URLSearchParamsPrototype$2.append);
	var $delete = uncurryThis$2(URLSearchParamsPrototype$2['delete']);
	var forEach$1 = uncurryThis$2(URLSearchParamsPrototype$2.forEach);
	var push = uncurryThis$2([].push);
	var params$1 = new $URLSearchParams$1('a=1&a=2&b=3');

	params$1['delete']('a', 1);
	// `undefined` case is a Chromium 117 bug
	// https://bugs.chromium.org/p/v8/issues/detail?id=14222
	params$1['delete']('b', undefined);

	if (params$1 + '' !== 'a=2') {
	  defineBuiltIn$1(URLSearchParamsPrototype$2, 'delete', function (name /* , value */) {
	    var length = arguments.length;
	    var $value = length < 2 ? undefined : arguments[1];
	    if (length && $value === undefined) return $delete(this, name);
	    var entries = [];
	    forEach$1(this, function (v, k) { // also validates `this`
	      push(entries, { key: k, value: v });
	    });
	    validateArgumentsLength$1(length, 1);
	    var key = toString$1(name);
	    var value = toString$1($value);
	    var index = 0;
	    var dindex = 0;
	    var found = false;
	    var entriesLength = entries.length;
	    var entry;
	    while (index < entriesLength) {
	      entry = entries[index++];
	      if (found || entry.key === key) {
	        found = true;
	        $delete(this, entry.key);
	      } else dindex++;
	    }
	    while (dindex < entriesLength) {
	      entry = entries[dindex++];
	      if (!(entry.key === key && entry.value === value)) append(this, entry.key, entry.value);
	    }
	  }, { enumerable: true, unsafe: true });
	}

	var defineBuiltIn = defineBuiltIn$k;
	var uncurryThis$1 = functionUncurryThis;
	var toString = toString$o;
	var validateArgumentsLength = validateArgumentsLength$6;

	var $URLSearchParams = URLSearchParams;
	var URLSearchParamsPrototype$1 = $URLSearchParams.prototype;
	var getAll = uncurryThis$1(URLSearchParamsPrototype$1.getAll);
	var $has = uncurryThis$1(URLSearchParamsPrototype$1.has);
	var params = new $URLSearchParams('a=1');

	// `undefined` case is a Chromium 117 bug
	// https://bugs.chromium.org/p/v8/issues/detail?id=14222
	if (params.has('a', 2) || !params.has('a', undefined)) {
	  defineBuiltIn(URLSearchParamsPrototype$1, 'has', function has(name /* , value */) {
	    var length = arguments.length;
	    var $value = length < 2 ? undefined : arguments[1];
	    if (length && $value === undefined) return $has(this, name);
	    var values = getAll(this, name); // also validates `this`
	    validateArgumentsLength(length, 1);
	    var value = toString($value);
	    var index = 0;
	    while (index < values.length) {
	      if (values[index++] === value) return true;
	    } return false;
	  }, { enumerable: true, unsafe: true });
	}

	var DESCRIPTORS = descriptors;
	var uncurryThis = functionUncurryThis;
	var defineBuiltInAccessor = defineBuiltInAccessor$b;

	var URLSearchParamsPrototype = URLSearchParams.prototype;
	var forEach = uncurryThis(URLSearchParamsPrototype.forEach);

	// `URLSearchParams.prototype.size` getter
	// https://github.com/whatwg/url/pull/734
	if (DESCRIPTORS && !('size' in URLSearchParamsPrototype)) {
	  defineBuiltInAccessor(URLSearchParamsPrototype, 'size', {
	    get: function size() {
	      var count = 0;
	      forEach(this, function () { count++; });
	      return count;
	    },
	    configurable: true,
	    enumerable: true
	  });
	}

	function _regenerator() { /*! regenerator-runtime -- Copyright (c) 2014-present, Facebook, Inc. -- license (MIT): https://github.com/babel/babel/blob/main/packages/babel-helpers/LICENSE */ var e, t, r = "function" == typeof Symbol ? Symbol : {}, n = r.iterator || "@@iterator", o = r.toStringTag || "@@toStringTag"; function i(r, n, o, i) { var c = n && n.prototype instanceof Generator ? n : Generator, u = Object.create(c.prototype); return _regeneratorDefine2(u, "_invoke", function (r, n, o) { var i, c, u, f = 0, p = o || [], y = false, G = { p: 0, n: 0, v: e, a: d, f: d.bind(e, 4), d: function d(t, r) { return i = t, c = 0, u = e, G.n = r, a; } }; function d(r, n) { for (c = r, u = n, t = 0; !y && f && !o && t < p.length; t++) { var o, i = p[t], d = G.p, l = i[2]; r > 3 ? (o = l === n) && (u = i[(c = i[4]) ? 5 : (c = 3, 3)], i[4] = i[5] = e) : i[0] <= d && ((o = r < 2 && d < i[1]) ? (c = 0, G.v = n, G.n = i[1]) : d < l && (o = r < 3 || i[0] > n || n > l) && (i[4] = r, i[5] = n, G.n = l, c = 0)); } if (o || r > 1) return a; throw y = true, n; } return function (o, p, l) { if (f > 1) throw TypeError("Generator is already running"); for (y && 1 === p && d(p, l), c = p, u = l; (t = c < 2 ? e : u) || !y;) { i || (c ? (c > 1 && (G.n = c < 3 ? -1 : u), d(c, u)) : G.v = u); try { if (f = 2, i) { if (c || (o = "next"), t = i[o]) { if (!(t = t.call(i, u))) throw TypeError("iterator result is not an object"); if (!t.done) return t; u = t.value, c < 2 && (c = 0); } else 1 === c && (t = i.return) && t.call(i), c < 2 && (u = TypeError("The iterator does not provide a '" + o + "' method"), c = 1); i = e; } else if ((t = (y = G.n < 0) ? u : r.call(n, G)) !== a) break; } catch (t) { i = e, c = 1, u = t; } finally { f = 1; } } return { value: t, done: y }; }; }(r, o, i), true), u; } var a = {}; function Generator() {} function GeneratorFunction() {} function GeneratorFunctionPrototype() {} t = Object.getPrototypeOf; var c = [][n] ? t(t([][n]())) : (_regeneratorDefine2(t = {}, n, function () { return this; }), t), u = GeneratorFunctionPrototype.prototype = Generator.prototype = Object.create(c); function f(e) { return Object.setPrototypeOf ? Object.setPrototypeOf(e, GeneratorFunctionPrototype) : (e.__proto__ = GeneratorFunctionPrototype, _regeneratorDefine2(e, o, "GeneratorFunction")), e.prototype = Object.create(u), e; } return GeneratorFunction.prototype = GeneratorFunctionPrototype, _regeneratorDefine2(u, "constructor", GeneratorFunctionPrototype), _regeneratorDefine2(GeneratorFunctionPrototype, "constructor", GeneratorFunction), GeneratorFunction.displayName = "GeneratorFunction", _regeneratorDefine2(GeneratorFunctionPrototype, o, "GeneratorFunction"), _regeneratorDefine2(u), _regeneratorDefine2(u, o, "Generator"), _regeneratorDefine2(u, n, function () { return this; }), _regeneratorDefine2(u, "toString", function () { return "[object Generator]"; }), (_regenerator = function _regenerator() { return { w: i, m: f }; })(); }
	function _regeneratorDefine2(e, r, n, t) { var i = Object.defineProperty; try { i({}, "", {}); } catch (e) { i = 0; } _regeneratorDefine2 = function _regeneratorDefine(e, r, n, t) { function o(r, n) { _regeneratorDefine2(e, r, function (e) { return this._invoke(r, n, e); }); } r ? i ? i(e, r, { value: n, enumerable: !t, configurable: !t, writable: !t }) : e[r] = n : (o("next", 0), o("throw", 1), o("return", 2)); }, _regeneratorDefine2(e, r, n, t); }
	function asyncGeneratorStep(n, t, e, r, o, a, c) { try { var i = n[a](c), u = i.value; } catch (n) { return void e(n); } i.done ? t(u) : Promise.resolve(u).then(r, o); }
	function _asyncToGenerator(n) { return function () { var t = this, e = arguments; return new Promise(function (r, o) { var a = n.apply(t, e); function _next(n) { asyncGeneratorStep(a, r, o, _next, _throw, "next", n); } function _throw(n) { asyncGeneratorStep(a, r, o, _next, _throw, "throw", n); } _next(void 0); }); }; }
	function _classCallCheck(a, n) { if (!(a instanceof n)) throw new TypeError("Cannot call a class as a function"); }
	function _defineProperties(e, r) { for (var t = 0; t < r.length; t++) { var o = r[t]; o.enumerable = o.enumerable || false, o.configurable = true, "value" in o && (o.writable = true), Object.defineProperty(e, _toPropertyKey(o.key), o); } }
	function _createClass(e, r, t) { return r && _defineProperties(e.prototype, r), Object.defineProperty(e, "prototype", { writable: false }), e; }
	function _classPrivateFieldInitSpec(e, t, a) { _checkPrivateRedeclaration(e, t), t.set(e, a); }
	function _checkPrivateRedeclaration(e, t) { if (t.has(e)) throw new TypeError("Cannot initialize the same private elements twice on an object"); }
	function _classPrivateFieldGet(s, a) { return s.get(_assertClassBrand(s, a)); }
	function _assertClassBrand(e, t, n) { if ("function" == typeof e ? e === t : e.has(t)) return arguments.length < 3 ? t : n; throw new TypeError("Private element is not present on this object"); }
	function _objectWithoutProperties(e, t) { if (null == e) return {}; var o, r, i = _objectWithoutPropertiesLoose(e, t); if (Object.getOwnPropertySymbols) { var n = Object.getOwnPropertySymbols(e); for (r = 0; r < n.length; r++) o = n[r], -1 === t.indexOf(o) && {}.propertyIsEnumerable.call(e, o) && (i[o] = e[o]); } return i; }
	function _objectWithoutPropertiesLoose(r, e) { if (null == r) return {}; var t = {}; for (var n in r) if ({}.hasOwnProperty.call(r, n)) { if (-1 !== e.indexOf(n)) continue; t[n] = r[n]; } return t; }
	function _createForOfIteratorHelper(r, e) { var t = "undefined" != typeof Symbol && r[Symbol.iterator] || r["@@iterator"]; if (!t) { if (Array.isArray(r) || (t = _unsupportedIterableToArray(r)) || e) { t && (r = t); var _n = 0, F = function F() {}; return { s: F, n: function n() { return _n >= r.length ? { done: true } : { done: false, value: r[_n++] }; }, e: function e(r) { throw r; }, f: F }; } throw new TypeError("Invalid attempt to iterate non-iterable instance.\nIn order to be iterable, non-array objects must have a [Symbol.iterator]() method."); } var o, a = true, u = false; return { s: function s() { t = t.call(r); }, n: function n() { var r = t.next(); return a = r.done, r; }, e: function e(r) { u = true, o = r; }, f: function f() { try { a || null == t.return || t.return(); } finally { if (u) throw o; } } }; }
	function _slicedToArray(r, e) { return _arrayWithHoles(r) || _iterableToArrayLimit(r, e) || _unsupportedIterableToArray(r, e) || _nonIterableRest(); }
	function _nonIterableRest() { throw new TypeError("Invalid attempt to destructure non-iterable instance.\nIn order to be iterable, non-array objects must have a [Symbol.iterator]() method."); }
	function _iterableToArrayLimit(r, l) { var t = null == r ? null : "undefined" != typeof Symbol && r[Symbol.iterator] || r["@@iterator"]; if (null != t) { var e, n, i, u, a = [], f = true, o = false; try { if (i = (t = t.call(r)).next, 0 === l) ; else for (; !(f = (e = i.call(t)).done) && (a.push(e.value), a.length !== l); f = !0); } catch (r) { o = true, n = r; } finally { try { if (!f && null != t.return && (u = t.return(), Object(u) !== u)) return; } finally { if (o) throw n; } } return a; } }
	function _arrayWithHoles(r) { if (Array.isArray(r)) return r; }
	function _toConsumableArray(r) { return _arrayWithoutHoles(r) || _iterableToArray(r) || _unsupportedIterableToArray(r) || _nonIterableSpread(); }
	function _nonIterableSpread() { throw new TypeError("Invalid attempt to spread non-iterable instance.\nIn order to be iterable, non-array objects must have a [Symbol.iterator]() method."); }
	function _unsupportedIterableToArray(r, a) { if (r) { if ("string" == typeof r) return _arrayLikeToArray(r, a); var t = {}.toString.call(r).slice(8, -1); return "Object" === t && r.constructor && (t = r.constructor.name), "Map" === t || "Set" === t ? Array.from(r) : "Arguments" === t || /^(?:Ui|I)nt(?:8|16|32)(?:Clamped)?Array$/.test(t) ? _arrayLikeToArray(r, a) : void 0; } }
	function _iterableToArray(r) { if ("undefined" != typeof Symbol && null != r[Symbol.iterator] || null != r["@@iterator"]) return Array.from(r); }
	function _arrayWithoutHoles(r) { if (Array.isArray(r)) return _arrayLikeToArray(r); }
	function _arrayLikeToArray(r, a) { (null == a || a > r.length) && (a = r.length); for (var e = 0, n = Array(a); e < a; e++) n[e] = r[e]; return n; }
	function _typeof(o) { "@babel/helpers - typeof"; return _typeof = "function" == typeof Symbol && "symbol" == typeof Symbol.iterator ? function (o) { return typeof o; } : function (o) { return o && "function" == typeof Symbol && o.constructor === Symbol && o !== Symbol.prototype ? "symbol" : typeof o; }, _typeof(o); }
	function _defineProperty(e, r, t) { return (r = _toPropertyKey(r)) in e ? Object.defineProperty(e, r, { value: t, enumerable: true, configurable: true, writable: true }) : e[r] = t, e; }
	function _toPropertyKey(t) { var i = _toPrimitive(t, "string"); return "symbol" == _typeof(i) ? i : i + ""; }
	function _toPrimitive(t, e) { if ("object" != _typeof(t) || !t) return t; var r; if ("undefined" != typeof Symbol && void 0 !== (r = t[Symbol.toPrimitive])) { var i = r.call(t, e); if ("object" != _typeof(i)) return i; throw new TypeError("@@toPrimitive must return a primitive value."); } return ("string" === e ? String : Number)(t); }
	var sym = Symbol("s");
	var box = _defineProperty({
	  a: 1
	}, sym, 2);
	console.log(Reflect.ownKeys(box).length, _typeof(Reflect.ownKeys(box)[1]), sym.description);
	console.log([1, 2, 3].at(-1), Object.fromEntries([["x", 1]]).x, "abc".padStart(5, "-"), "abc".padEnd(5, "+"));
	console.log([3, 1, 2].toSorted().join(), [1, 2, 3].findLast(function (x) {
	  return x < 3;
	}), Object.hasOwn(box, "a"));
	console.log(Array.from(new Set([1, 1, 2])).length, _toConsumableArray("ab").includes("b"), [NaN].includes(NaN));
	console.log("a-b-c".replaceAll("-", "+"), [[1], [2, [3]]].flat(2).join(), "  x ".trimStart() + "|", " y  ".trimEnd() + "|");
	console.log(Object.entries({
	  a: 1,
	  b: 2
	}).map(function (_ref) {
	  var _ref2 = _slicedToArray(_ref, 2),
	    k = _ref2[0],
	    v = _ref2[1];
	  return k + v;
	}).join(), Object.values({
	  q: 5
	})[0]);
	console.log(new Map([[1, "a"]]).get(1), new WeakMap().has({}), Number.isInteger(5), Math.trunc(-4.5), Math.sign(-3));
	console.log("x".repeat(3), "abc".startsWith("a"), "abc".endsWith("c"), "abc".includes("b"));
	console.log(_toConsumableArray("abc".matchAll(/./g)).length, [1, 2].flatMap(function (x) {
	  return [x, x];
	}).length, "abc".at(-1));
	console.log(JSON.stringify(Object.groupBy([1, 2, 3], function (x) {
	  return x % 2 ? "odd" : "even";
	})));
	console.log(new URLSearchParams("a=1&b=2").get("b"), new URL("https://x.test/p?q=1").searchParams.get("q"));
	console.log(structuredClone({
	  d: [1, 2]
	}).d.length, [1, 2, 3].with(0, 9).join(), [1, 2, 3].toReversed().join());
	console.log(Array.from({
	  length: 3
	}, function (_, i) {
	  return i * 2;
	}).join(), Array.of(7).length, [1, 2, 3].fill(0, 1).join());
	console.log(Object.assign({}, {
	  z: 1
	}).z, [1, 2, 3].find(function (x) {
	  return x > 1;
	}), [1, 2, 3].findIndex(function (x) {
	  return x > 1;
	}));
	console.log("\uD842\uDFB7".codePointAt(0), String.fromCodePoint(134071) === "\uD842\uDFB7", new Error("e", {
	  cause: "c"
	}).cause);
	console.log([5, 1, 10].sort(function (x, y) {
	  return x - y;
	}).join(), [1, 2, 3].reverse().join(), [1, 2, 3, 4].slice(1, 3).join());
	console.log(Number.parseFloat("1.5"), Number.EPSILON > 0, Math.hypot(3, 4), 1234.5678.toFixed(2));
	console.log(JSON.stringify({
	  s: "\u2028"
	}), JSON.stringify([undefined, sym]), String(/a/gi.flags));
	console.log(Object.getOwnPropertyNames(Symbol.prototype).includes("description"), _typeof(Symbol.asyncIterator));
	console.log(new Set([1, 2]).union(new Set([3])).size, new Set([1, 2]).intersection(new Set([2])).size);
	console.log(Promise.withResolvers !== undefined, _typeof(Array.prototype.findLastIndex), "ab".isWellFormed());
	var _iterator = _createForOfIteratorHelper(new Map([["m", 1]])),
	  _step;
	try {
	  for (_iterator.s(); !(_step = _iterator.n()).done;) {
	    var _step$value = _slicedToArray(_step.value, 2),
	      key = _step$value[0],
	      value = _step$value[1];
	    console.log(key, value);
	  }
	} catch (err) {
	  _iterator.e(err);
	} finally {
	  _iterator.f();
	}
	var _a$b$c = {
	    a: 1,
	    b: 2,
	    c: 3
	  },
	  a = _a$b$c.a,
	  rest = _objectWithoutProperties(_a$b$c, ["a"]);
	console.log(a, Object.keys(rest).join(), globalThis.Math === Math);
	var _count = /*#__PURE__*/new WeakMap();
	var Counter = /*#__PURE__*/function () {
	  function Counter() {
	    _classCallCheck(this, Counter);
	    _classPrivateFieldInitSpec(this, _count, 1);
	  }
	  return _createClass(Counter, [{
	    key: "count",
	    get: function get() {
	      return _classPrivateFieldGet(_count, this) + Counter.start;
	    }
	  }]);
	}();
	_defineProperty(Counter, "start", 2);
	console.log(new Counter().count);
	function later() {
	  return _later.apply(this, arguments);
	}
	function _later() {
	  _later = _asyncToGenerator(/*#__PURE__*/_regenerator().m(function _callee() {
	    return _regenerator().w(function (_context) {
	      while (1) switch (_context.n) {
	        case 0:
	          _context.n = 1;
	          return null;
	        case 1:
	          return _context.a(2, 7);
	      }
	    }, _callee);
	  }));
	  return _later.apply(this, arguments);
	}
	later().then(function (value) {
	  return console.log("async", value);
	});
	Promise.allSettled([Promise.resolve(1), Promise.reject(new Error("e"))]).then(function (all) {
	  return console.log(all.map(function (one) {
	    return one.status;
	  }).join());
	});
	Promise.any([Promise.reject(new Error("x")), Promise.resolve(3)]).then(function (value) {
	  return console.log("any", value);
	});
	Promise.resolve(4).finally(function () {
	  return console.log("finally");
	}).then(function (value) {
	  return console.log("promise", value);
	});

})();
