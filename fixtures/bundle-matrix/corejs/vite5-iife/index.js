(function() {
  "use strict";
  var commonjsGlobal = typeof globalThis !== "undefined" ? globalThis : typeof window !== "undefined" ? window : typeof global !== "undefined" ? global : typeof self !== "undefined" ? self : {};
  var check = function(it) {
    return it && it.Math === Math && it;
  };
  var globalThis_1 = (
    // eslint-disable-next-line es/no-global-this -- safe
    check(typeof globalThis == "object" && globalThis) || check(typeof window == "object" && window) || // eslint-disable-next-line no-restricted-globals -- safe
    check(typeof self == "object" && self) || check(typeof commonjsGlobal == "object" && commonjsGlobal) || check(typeof commonjsGlobal == "object" && commonjsGlobal) || // eslint-disable-next-line no-new-func -- fallback
    /* @__PURE__ */ function() {
      return this;
    }() || Function("return this")()
  );
  var objectGetOwnPropertyDescriptor = {};
  var fails$9 = function(exec) {
    try {
      return !!exec();
    } catch (error) {
      return true;
    }
  };
  var fails$8 = fails$9;
  var descriptors = !fails$8(function() {
    return Object.defineProperty({}, 1, { get: function() {
      return 7;
    } })[1] !== 7;
  });
  var fails$7 = fails$9;
  var functionBindNative = !fails$7(function() {
    var test2 = (function() {
    }).bind();
    return typeof test2 != "function" || test2.hasOwnProperty("prototype");
  });
  var NATIVE_BIND$1 = functionBindNative;
  var call$6 = Function.prototype.call;
  var functionCall = NATIVE_BIND$1 ? call$6.bind(call$6) : function() {
    return call$6.apply(call$6, arguments);
  };
  var objectPropertyIsEnumerable = {};
  var $propertyIsEnumerable = {}.propertyIsEnumerable;
  var getOwnPropertyDescriptor$1 = Object.getOwnPropertyDescriptor;
  var NASHORN_BUG = getOwnPropertyDescriptor$1 && !$propertyIsEnumerable.call({ 1: 2 }, 1);
  objectPropertyIsEnumerable.f = NASHORN_BUG ? function propertyIsEnumerable(V) {
    var descriptor = getOwnPropertyDescriptor$1(this, V);
    return !!descriptor && descriptor.enumerable;
  } : $propertyIsEnumerable;
  var createPropertyDescriptor$2 = function(bitmap, value) {
    return {
      enumerable: !(bitmap & 1),
      configurable: !(bitmap & 2),
      writable: !(bitmap & 4),
      value
    };
  };
  var NATIVE_BIND = functionBindNative;
  var FunctionPrototype$1 = Function.prototype;
  var call$5 = FunctionPrototype$1.call;
  var uncurryThisWithBind = NATIVE_BIND && FunctionPrototype$1.bind.bind(call$5, call$5);
  var functionUncurryThis = NATIVE_BIND ? uncurryThisWithBind : function(fn) {
    return function() {
      return call$5.apply(fn, arguments);
    };
  };
  var uncurryThis$a = functionUncurryThis;
  var toString$3 = uncurryThis$a({}.toString);
  var stringSlice$3 = uncurryThis$a("".slice);
  var classofRaw$1 = function(it) {
    return stringSlice$3(toString$3(it), 8, -1);
  };
  var uncurryThis$9 = functionUncurryThis;
  var fails$6 = fails$9;
  var classof$3 = classofRaw$1;
  var $Object$3 = Object;
  var split = uncurryThis$9("".split);
  var indexedObject = fails$6(function() {
    return !$Object$3("z").propertyIsEnumerable(0);
  }) ? function(it) {
    return classof$3(it) === "String" ? split(it, "") : $Object$3(it);
  } : $Object$3;
  var isNullOrUndefined$2 = function(it) {
    return it === null || it === void 0;
  };
  var isNullOrUndefined$1 = isNullOrUndefined$2;
  var $TypeError$6 = TypeError;
  var requireObjectCoercible$3 = function(it) {
    if (isNullOrUndefined$1(it)) throw new $TypeError$6("Can't call method on " + it);
    return it;
  };
  var IndexedObject = indexedObject;
  var requireObjectCoercible$2 = requireObjectCoercible$3;
  var toIndexedObject$4 = function(it) {
    return IndexedObject(requireObjectCoercible$2(it));
  };
  var documentAll = typeof document == "object" && document.all;
  var isCallable$c = typeof documentAll == "undefined" && documentAll !== void 0 ? function(argument) {
    return typeof argument == "function" || argument === documentAll;
  } : function(argument) {
    return typeof argument == "function";
  };
  var isCallable$b = isCallable$c;
  var isObject$6 = function(it) {
    return typeof it == "object" ? it !== null : isCallable$b(it);
  };
  var globalThis$c = globalThis_1;
  var isCallable$a = isCallable$c;
  var aFunction = function(argument) {
    return isCallable$a(argument) ? argument : void 0;
  };
  var getBuiltIn$3 = function(namespace, method) {
    return arguments.length < 2 ? aFunction(globalThis$c[namespace]) : globalThis$c[namespace] && globalThis$c[namespace][method];
  };
  var uncurryThis$8 = functionUncurryThis;
  var objectIsPrototypeOf = uncurryThis$8({}.isPrototypeOf);
  var globalThis$b = globalThis_1;
  var navigator = globalThis$b.navigator;
  var userAgent$1 = navigator && navigator.userAgent;
  var environmentUserAgent = userAgent$1 ? String(userAgent$1) : "";
  var globalThis$a = globalThis_1;
  var userAgent = environmentUserAgent;
  var process = globalThis$a.process;
  var Deno = globalThis$a.Deno;
  var versions = process && process.versions || Deno && Deno.version;
  var v8 = versions && versions.v8;
  var match, version;
  if (v8) {
    match = v8.split(".");
    version = match[0] > 0 && match[0] < 4 ? 1 : +(match[0] + match[1]);
  }
  if (!version && userAgent) {
    match = userAgent.match(/Edge\/(\d+)/);
    if (!match || match[1] >= 74) {
      match = userAgent.match(/Chrome\/(\d+)/);
      if (match) version = +match[1];
    }
  }
  var environmentV8Version = version;
  var V8_VERSION = environmentV8Version;
  var fails$5 = fails$9;
  var globalThis$9 = globalThis_1;
  var $String$4 = globalThis$9.String;
  var symbolConstructorDetection = !!Object.getOwnPropertySymbols && !fails$5(function() {
    var symbol = Symbol("symbol detection");
    return !$String$4(symbol) || !(Object(symbol) instanceof Symbol) || // Chrome 38-40 symbols are not inherited from DOM collections prototypes to instances
    !Symbol.sham && V8_VERSION && V8_VERSION < 41;
  });
  var NATIVE_SYMBOL$1 = symbolConstructorDetection;
  var useSymbolAsUid = NATIVE_SYMBOL$1 && !Symbol.sham && typeof Symbol.iterator == "symbol";
  var getBuiltIn$2 = getBuiltIn$3;
  var isCallable$9 = isCallable$c;
  var isPrototypeOf$1 = objectIsPrototypeOf;
  var USE_SYMBOL_AS_UID$1 = useSymbolAsUid;
  var $Object$2 = Object;
  var isSymbol$2 = USE_SYMBOL_AS_UID$1 ? function(it) {
    return typeof it == "symbol";
  } : function(it) {
    var $Symbol = getBuiltIn$2("Symbol");
    return isCallable$9($Symbol) && isPrototypeOf$1($Symbol.prototype, $Object$2(it));
  };
  var $String$3 = String;
  var tryToString$1 = function(argument) {
    try {
      return $String$3(argument);
    } catch (error) {
      return "Object";
    }
  };
  var isCallable$8 = isCallable$c;
  var tryToString = tryToString$1;
  var $TypeError$5 = TypeError;
  var aCallable$1 = function(argument) {
    if (isCallable$8(argument)) return argument;
    throw new $TypeError$5(tryToString(argument) + " is not a function");
  };
  var aCallable = aCallable$1;
  var isNullOrUndefined = isNullOrUndefined$2;
  var getMethod$2 = function(V, P) {
    var func = V[P];
    return isNullOrUndefined(func) ? void 0 : aCallable(func);
  };
  var call$4 = functionCall;
  var isCallable$7 = isCallable$c;
  var isObject$5 = isObject$6;
  var $TypeError$4 = TypeError;
  var ordinaryToPrimitive$1 = function(input, pref) {
    var fn, val;
    if (pref === "string" && isCallable$7(fn = input.toString) && !isObject$5(val = call$4(fn, input))) return val;
    if (isCallable$7(fn = input.valueOf) && !isObject$5(val = call$4(fn, input))) return val;
    if (pref !== "string" && isCallable$7(fn = input.toString) && !isObject$5(val = call$4(fn, input))) return val;
    throw new $TypeError$4("Can't convert object to primitive value");
  };
  var sharedStore = { exports: {} };
  var globalThis$8 = globalThis_1;
  var defineProperty$2 = Object.defineProperty;
  var defineGlobalProperty$3 = function(key, value) {
    try {
      defineProperty$2(globalThis$8, key, { value, configurable: true, writable: true });
    } catch (error) {
      globalThis$8[key] = value;
    }
    return value;
  };
  var globalThis$7 = globalThis_1;
  var defineGlobalProperty$2 = defineGlobalProperty$3;
  var SHARED = "__core-js_shared__";
  var store$3 = sharedStore.exports = globalThis$7[SHARED] || defineGlobalProperty$2(SHARED, {});
  (store$3.versions || (store$3.versions = [])).push({
    version: "3.51.0",
    mode: "global",
    copyright: "© 2013–2025 Denis Pushkarev (zloirock.ru), 2025–2026 CoreJS Company (core-js.io). All rights reserved.",
    license: "https://github.com/zloirock/core-js/blob/v3.51.0/LICENSE",
    source: "https://github.com/zloirock/core-js"
  });
  var sharedStoreExports = sharedStore.exports;
  var store$2 = sharedStoreExports;
  var create$1 = Object.create || Object;
  var shared$3 = function(key, value) {
    return store$2[key] || (store$2[key] = value || create$1(null));
  };
  var requireObjectCoercible$1 = requireObjectCoercible$3;
  var $Object$1 = Object;
  var toObject$3 = function(argument) {
    return $Object$1(requireObjectCoercible$1(argument));
  };
  var uncurryThis$7 = functionUncurryThis;
  var toObject$2 = toObject$3;
  var hasOwnProperty = uncurryThis$7({}.hasOwnProperty);
  var hasOwnProperty_1 = Object.hasOwn || function hasOwn2(it, key) {
    return hasOwnProperty(toObject$2(it), key);
  };
  var uncurryThis$6 = functionUncurryThis;
  var id = 0;
  var postfix = Math.random();
  var toString$2 = uncurryThis$6(1.1.toString);
  var uid$2 = function(key) {
    return "Symbol(" + (key === void 0 ? "" : key) + ")_" + toString$2(++id + postfix, 36);
  };
  var globalThis$6 = globalThis_1;
  var shared$2 = shared$3;
  var hasOwn$8 = hasOwnProperty_1;
  var uid$1 = uid$2;
  var NATIVE_SYMBOL = symbolConstructorDetection;
  var USE_SYMBOL_AS_UID = useSymbolAsUid;
  var Symbol$1 = globalThis$6.Symbol;
  var WellKnownSymbolsStore = shared$2("wks");
  var createWellKnownSymbol = USE_SYMBOL_AS_UID ? Symbol$1["for"] || Symbol$1 : Symbol$1 && Symbol$1.withoutSetter || uid$1;
  var wellKnownSymbol$6 = function(name2) {
    if (!hasOwn$8(WellKnownSymbolsStore, name2)) {
      WellKnownSymbolsStore[name2] = NATIVE_SYMBOL && hasOwn$8(Symbol$1, name2) ? Symbol$1[name2] : createWellKnownSymbol("Symbol." + name2);
    }
    return WellKnownSymbolsStore[name2];
  };
  var call$3 = functionCall;
  var isObject$4 = isObject$6;
  var isSymbol$1 = isSymbol$2;
  var getMethod$1 = getMethod$2;
  var ordinaryToPrimitive = ordinaryToPrimitive$1;
  var wellKnownSymbol$5 = wellKnownSymbol$6;
  var $TypeError$3 = TypeError;
  var TO_PRIMITIVE = wellKnownSymbol$5("toPrimitive");
  var toPrimitive$1 = function(input, pref) {
    if (!isObject$4(input) || isSymbol$1(input)) return input;
    var exoticToPrim = getMethod$1(input, TO_PRIMITIVE);
    var result;
    if (exoticToPrim) {
      if (pref === void 0) pref = "default";
      result = call$3(exoticToPrim, input, pref);
      if (!isObject$4(result) || isSymbol$1(result)) return result;
      throw new $TypeError$3("Can't convert object to primitive value");
    }
    if (pref === void 0) pref = "number";
    return ordinaryToPrimitive(input, pref);
  };
  var toPrimitive = toPrimitive$1;
  var isSymbol = isSymbol$2;
  var toPropertyKey$2 = function(argument) {
    var key = toPrimitive(argument, "string");
    return isSymbol(key) ? key : key + "";
  };
  var globalThis$5 = globalThis_1;
  var isObject$3 = isObject$6;
  var document$1 = globalThis$5.document;
  var EXISTS$1 = isObject$3(document$1) && isObject$3(document$1.createElement);
  var documentCreateElement$1 = function(it) {
    return EXISTS$1 ? document$1.createElement(it) : {};
  };
  var DESCRIPTORS$7 = descriptors;
  var fails$4 = fails$9;
  var createElement = documentCreateElement$1;
  var ie8DomDefine = !DESCRIPTORS$7 && !fails$4(function() {
    return Object.defineProperty(createElement("div"), "a", {
      get: function() {
        return 7;
      }
    }).a !== 7;
  });
  var DESCRIPTORS$6 = descriptors;
  var call$2 = functionCall;
  var propertyIsEnumerableModule = objectPropertyIsEnumerable;
  var createPropertyDescriptor$1 = createPropertyDescriptor$2;
  var toIndexedObject$3 = toIndexedObject$4;
  var toPropertyKey$1 = toPropertyKey$2;
  var hasOwn$7 = hasOwnProperty_1;
  var IE8_DOM_DEFINE$1 = ie8DomDefine;
  var $getOwnPropertyDescriptor$1 = Object.getOwnPropertyDescriptor;
  objectGetOwnPropertyDescriptor.f = DESCRIPTORS$6 ? $getOwnPropertyDescriptor$1 : function getOwnPropertyDescriptor2(O, P) {
    O = toIndexedObject$3(O);
    P = toPropertyKey$1(P);
    if (IE8_DOM_DEFINE$1) try {
      return $getOwnPropertyDescriptor$1(O, P);
    } catch (error) {
    }
    if (hasOwn$7(O, P)) return createPropertyDescriptor$1(!call$2(propertyIsEnumerableModule.f, O, P), O[P]);
  };
  var objectDefineProperty = {};
  var DESCRIPTORS$5 = descriptors;
  var fails$3 = fails$9;
  var v8PrototypeDefineBug = DESCRIPTORS$5 && fails$3(function() {
    return Object.defineProperty(function() {
    }, "prototype", {
      value: 42,
      writable: false
    }).prototype !== 42;
  });
  var isObject$2 = isObject$6;
  var $String$2 = String;
  var $TypeError$2 = TypeError;
  var anObject$5 = function(argument) {
    if (isObject$2(argument)) return argument;
    throw new $TypeError$2($String$2(argument) + " is not an object");
  };
  var DESCRIPTORS$4 = descriptors;
  var IE8_DOM_DEFINE = ie8DomDefine;
  var V8_PROTOTYPE_DEFINE_BUG$1 = v8PrototypeDefineBug;
  var anObject$4 = anObject$5;
  var toPropertyKey = toPropertyKey$2;
  var $TypeError$1 = TypeError;
  var $defineProperty = Object.defineProperty;
  var $getOwnPropertyDescriptor = Object.getOwnPropertyDescriptor;
  var ENUMERABLE = "enumerable";
  var CONFIGURABLE$1 = "configurable";
  var WRITABLE = "writable";
  objectDefineProperty.f = DESCRIPTORS$4 ? V8_PROTOTYPE_DEFINE_BUG$1 ? function defineProperty2(O, P, Attributes) {
    anObject$4(O);
    P = toPropertyKey(P);
    anObject$4(Attributes);
    if (typeof O === "function" && P === "prototype" && "value" in Attributes && WRITABLE in Attributes && !Attributes[WRITABLE]) {
      var current = $getOwnPropertyDescriptor(O, P);
      if (current && current[WRITABLE]) {
        O[P] = Attributes.value;
        Attributes = {
          configurable: CONFIGURABLE$1 in Attributes ? Attributes[CONFIGURABLE$1] : current[CONFIGURABLE$1],
          enumerable: ENUMERABLE in Attributes ? Attributes[ENUMERABLE] : current[ENUMERABLE],
          writable: false
        };
      }
    }
    return $defineProperty(O, P, Attributes);
  } : $defineProperty : function defineProperty2(O, P, Attributes) {
    anObject$4(O);
    P = toPropertyKey(P);
    anObject$4(Attributes);
    if (IE8_DOM_DEFINE) try {
      return $defineProperty(O, P, Attributes);
    } catch (error) {
    }
    if ("get" in Attributes || "set" in Attributes) throw new $TypeError$1("Accessors not supported");
    if ("value" in Attributes) O[P] = Attributes.value;
    return O;
  };
  var DESCRIPTORS$3 = descriptors;
  var definePropertyModule$3 = objectDefineProperty;
  var createPropertyDescriptor = createPropertyDescriptor$2;
  var createNonEnumerableProperty$2 = DESCRIPTORS$3 ? function(object, key, value) {
    return definePropertyModule$3.f(object, key, createPropertyDescriptor(1, value));
  } : function(object, key, value) {
    object[key] = value;
    return object;
  };
  var makeBuiltIn$2 = { exports: {} };
  var DESCRIPTORS$2 = descriptors;
  var hasOwn$6 = hasOwnProperty_1;
  var FunctionPrototype = Function.prototype;
  var getDescriptor = DESCRIPTORS$2 && Object.getOwnPropertyDescriptor;
  var EXISTS = hasOwn$6(FunctionPrototype, "name");
  var CONFIGURABLE = EXISTS && (!DESCRIPTORS$2 || DESCRIPTORS$2 && getDescriptor(FunctionPrototype, "name").configurable);
  var functionName = {
    CONFIGURABLE
  };
  var uncurryThis$5 = functionUncurryThis;
  var isCallable$6 = isCallable$c;
  var store$1 = sharedStoreExports;
  var functionToString = uncurryThis$5(Function.toString);
  if (!isCallable$6(store$1.inspectSource)) {
    store$1.inspectSource = function(it) {
      return functionToString(it);
    };
  }
  var inspectSource$1 = store$1.inspectSource;
  var globalThis$4 = globalThis_1;
  var isCallable$5 = isCallable$c;
  var WeakMap$1 = globalThis$4.WeakMap;
  var weakMapBasicDetection = isCallable$5(WeakMap$1) && /native code/.test(String(WeakMap$1));
  var shared$1 = shared$3;
  var uid = uid$2;
  var keys$2 = shared$1("keys");
  var sharedKey$2 = function(key) {
    return keys$2[key] || (keys$2[key] = uid(key));
  };
  var hiddenKeys$4 = {};
  var NATIVE_WEAK_MAP = weakMapBasicDetection;
  var globalThis$3 = globalThis_1;
  var createNonEnumerableProperty$1 = createNonEnumerableProperty$2;
  var hasOwn$5 = hasOwnProperty_1;
  var shared = sharedStoreExports;
  var sharedKey$1 = sharedKey$2;
  var hiddenKeys$3 = hiddenKeys$4;
  var OBJECT_ALREADY_INITIALIZED = "Object already initialized";
  var TypeError$1 = globalThis$3.TypeError;
  var WeakMap = globalThis$3.WeakMap;
  var set, get, has;
  var enforce = function(it) {
    return has(it) ? get(it) : set(it, {});
  };
  if (NATIVE_WEAK_MAP || shared.state) {
    var store = shared.state || (shared.state = new WeakMap());
    store.get = store.get;
    store.has = store.has;
    store.set = store.set;
    set = function(it, metadata) {
      if (store.has(it)) throw new TypeError$1(OBJECT_ALREADY_INITIALIZED);
      metadata.facade = it;
      store.set(it, metadata);
      return metadata;
    };
    get = function(it) {
      return store.get(it) || {};
    };
    has = function(it) {
      return store.has(it);
    };
  } else {
    var STATE = sharedKey$1("state");
    hiddenKeys$3[STATE] = true;
    set = function(it, metadata) {
      if (hasOwn$5(it, STATE)) throw new TypeError$1(OBJECT_ALREADY_INITIALIZED);
      metadata.facade = it;
      createNonEnumerableProperty$1(it, STATE, metadata);
      return metadata;
    };
    get = function(it) {
      return hasOwn$5(it, STATE) ? it[STATE] : {};
    };
    has = function(it) {
      return hasOwn$5(it, STATE);
    };
  }
  var internalState = {
    get,
    enforce
  };
  var uncurryThis$4 = functionUncurryThis;
  var fails$2 = fails$9;
  var isCallable$4 = isCallable$c;
  var hasOwn$4 = hasOwnProperty_1;
  var DESCRIPTORS$1 = descriptors;
  var CONFIGURABLE_FUNCTION_NAME = functionName.CONFIGURABLE;
  var inspectSource = inspectSource$1;
  var InternalStateModule = internalState;
  var enforceInternalState = InternalStateModule.enforce;
  var getInternalState = InternalStateModule.get;
  var $String$1 = String;
  var defineProperty$1 = Object.defineProperty;
  var stringSlice$2 = uncurryThis$4("".slice);
  var replace$1 = uncurryThis$4("".replace);
  var join = uncurryThis$4([].join);
  var CONFIGURABLE_LENGTH = DESCRIPTORS$1 && !fails$2(function() {
    return defineProperty$1(function() {
    }, "length", { value: 8 }).length !== 8;
  });
  var TEMPLATE = String(String).split("String");
  var makeBuiltIn$1 = makeBuiltIn$2.exports = function(value, name2, options) {
    if (stringSlice$2($String$1(name2), 0, 7) === "Symbol(") {
      name2 = "[" + replace$1($String$1(name2), /^Symbol\(([^)]*)\).*$/, "$1") + "]";
    }
    if (options && options.getter) name2 = "get " + name2;
    if (options && options.setter) name2 = "set " + name2;
    if (!hasOwn$4(value, "name") || CONFIGURABLE_FUNCTION_NAME && value.name !== name2) {
      if (DESCRIPTORS$1) defineProperty$1(value, "name", { value: name2, configurable: true });
      else value.name = name2;
    }
    if (CONFIGURABLE_LENGTH && options && hasOwn$4(options, "arity") && value.length !== options.arity) {
      defineProperty$1(value, "length", { value: options.arity });
    }
    try {
      if (options && hasOwn$4(options, "constructor") && options.constructor) {
        if (DESCRIPTORS$1) defineProperty$1(value, "prototype", { writable: false });
      } else if (value.prototype) value.prototype = void 0;
    } catch (error) {
    }
    var state = enforceInternalState(value);
    if (!hasOwn$4(state, "source")) {
      state.source = join(TEMPLATE, typeof name2 == "string" ? name2 : "");
    }
    return value;
  };
  Function.prototype.toString = makeBuiltIn$1(function toString2() {
    return isCallable$4(this) && getInternalState(this).source || inspectSource(this);
  }, "toString");
  var makeBuiltInExports = makeBuiltIn$2.exports;
  var isCallable$3 = isCallable$c;
  var definePropertyModule$2 = objectDefineProperty;
  var makeBuiltIn = makeBuiltInExports;
  var defineGlobalProperty$1 = defineGlobalProperty$3;
  var defineBuiltIn$1 = function(O, key, value, options) {
    if (!options) options = {};
    var simple = options.enumerable;
    var name2 = options.name !== void 0 ? options.name : key;
    if (isCallable$3(value)) makeBuiltIn(value, name2, options);
    if (options.global) {
      if (simple) O[key] = value;
      else defineGlobalProperty$1(key, value);
    } else {
      try {
        if (!options.unsafe) delete O[key];
        else if (O[key]) simple = true;
      } catch (error) {
      }
      if (simple) O[key] = value;
      else definePropertyModule$2.f(O, key, {
        value,
        enumerable: false,
        configurable: !options.nonConfigurable,
        writable: !options.nonWritable
      });
    }
    return O;
  };
  var objectGetOwnPropertyNames = {};
  var ceil = Math.ceil;
  var floor$1 = Math.floor;
  var mathTrunc = Math.trunc || function trunc2(x) {
    var n = +x;
    return (n > 0 ? floor$1 : ceil)(n);
  };
  var trunc = mathTrunc;
  var toIntegerOrInfinity$3 = function(argument) {
    var number = +argument;
    return number !== number || number === 0 ? 0 : trunc(number);
  };
  var toIntegerOrInfinity$2 = toIntegerOrInfinity$3;
  var max$1 = Math.max;
  var min$1 = Math.min;
  var toAbsoluteIndex$1 = function(index, length) {
    var integer = toIntegerOrInfinity$2(index);
    return integer < 0 ? max$1(integer + length, 0) : min$1(integer, length);
  };
  var toIntegerOrInfinity$1 = toIntegerOrInfinity$3;
  var min = Math.min;
  var toLength$1 = function(argument) {
    var len = toIntegerOrInfinity$1(argument);
    return len > 0 ? min(len, 9007199254740991) : 0;
  };
  var toLength = toLength$1;
  var lengthOfArrayLike$2 = function(obj) {
    return toLength(obj.length);
  };
  var toIndexedObject$2 = toIndexedObject$4;
  var toAbsoluteIndex = toAbsoluteIndex$1;
  var lengthOfArrayLike$1 = lengthOfArrayLike$2;
  var createMethod = function(IS_INCLUDES) {
    return function($this, el, fromIndex) {
      var O = toIndexedObject$2($this);
      var length = lengthOfArrayLike$1(O);
      if (length === 0) return !IS_INCLUDES && -1;
      var index = toAbsoluteIndex(fromIndex, length);
      var value;
      if (IS_INCLUDES && el !== el) while (length > index) {
        value = O[index++];
        if (value !== value) return true;
      }
      else for (; length > index; index++) {
        if ((IS_INCLUDES || index in O) && O[index] === el) return IS_INCLUDES || index || 0;
      }
      return !IS_INCLUDES && -1;
    };
  };
  var arrayIncludes = {
    // `Array.prototype.indexOf` method
    // https://tc39.es/ecma262/#sec-array.prototype.indexof
    indexOf: createMethod(false)
  };
  var uncurryThis$3 = functionUncurryThis;
  var hasOwn$3 = hasOwnProperty_1;
  var toIndexedObject$1 = toIndexedObject$4;
  var indexOf$1 = arrayIncludes.indexOf;
  var hiddenKeys$2 = hiddenKeys$4;
  var push = uncurryThis$3([].push);
  var objectKeysInternal = function(object, names) {
    var O = toIndexedObject$1(object);
    var i = 0;
    var result = [];
    var key;
    for (key in O) !hasOwn$3(hiddenKeys$2, key) && hasOwn$3(O, key) && push(result, key);
    while (names.length > i) if (hasOwn$3(O, key = names[i++])) {
      ~indexOf$1(result, key) || push(result, key);
    }
    return result;
  };
  var enumBugKeys$3 = [
    "constructor",
    "hasOwnProperty",
    "isPrototypeOf",
    "propertyIsEnumerable",
    "toLocaleString",
    "toString",
    "valueOf"
  ];
  var internalObjectKeys$1 = objectKeysInternal;
  var enumBugKeys$2 = enumBugKeys$3;
  var hiddenKeys$1 = enumBugKeys$2.concat("length", "prototype");
  objectGetOwnPropertyNames.f = Object.getOwnPropertyNames || function getOwnPropertyNames(O) {
    return internalObjectKeys$1(O, hiddenKeys$1);
  };
  var objectGetOwnPropertySymbols = {};
  objectGetOwnPropertySymbols.f = Object.getOwnPropertySymbols;
  var getBuiltIn$1 = getBuiltIn$3;
  var uncurryThis$2 = functionUncurryThis;
  var getOwnPropertyNamesModule = objectGetOwnPropertyNames;
  var getOwnPropertySymbolsModule = objectGetOwnPropertySymbols;
  var anObject$3 = anObject$5;
  var concat = uncurryThis$2([].concat);
  var ownKeys$1 = getBuiltIn$1("Reflect", "ownKeys") || function ownKeys2(it) {
    var keys2 = getOwnPropertyNamesModule.f(anObject$3(it));
    var getOwnPropertySymbols = getOwnPropertySymbolsModule.f;
    return getOwnPropertySymbols ? concat(keys2, getOwnPropertySymbols(it)) : keys2;
  };
  var hasOwn$2 = hasOwnProperty_1;
  var ownKeys = ownKeys$1;
  var getOwnPropertyDescriptorModule = objectGetOwnPropertyDescriptor;
  var definePropertyModule$1 = objectDefineProperty;
  var copyConstructorProperties$1 = function(target, source, exceptions) {
    var keys2 = ownKeys(source);
    var defineProperty2 = definePropertyModule$1.f;
    var getOwnPropertyDescriptor2 = getOwnPropertyDescriptorModule.f;
    for (var i = 0; i < keys2.length; i++) {
      var key = keys2[i];
      if (!hasOwn$2(target, key) && !(exceptions && hasOwn$2(exceptions, key))) {
        defineProperty2(target, key, getOwnPropertyDescriptor2(source, key));
      }
    }
  };
  var fails$1 = fails$9;
  var isCallable$2 = isCallable$c;
  var replacement = /#|\.prototype\./;
  var isForced$1 = function(feature, detection) {
    var value = data$1[normalize(feature)];
    return value === POLYFILL ? true : value === NATIVE ? false : isCallable$2(detection) ? fails$1(detection) : !!detection;
  };
  var normalize = isForced$1.normalize = function(string) {
    return String(string).replace(replacement, ".").toLowerCase();
  };
  var data$1 = isForced$1.data = {};
  var NATIVE = isForced$1.NATIVE = "N";
  var POLYFILL = isForced$1.POLYFILL = "P";
  var isForced_1 = isForced$1;
  var globalThis$2 = globalThis_1;
  var getOwnPropertyDescriptor = objectGetOwnPropertyDescriptor.f;
  var createNonEnumerableProperty = createNonEnumerableProperty$2;
  var defineBuiltIn = defineBuiltIn$1;
  var defineGlobalProperty = defineGlobalProperty$3;
  var copyConstructorProperties = copyConstructorProperties$1;
  var isForced = isForced_1;
  var _export = function(options, source) {
    var TARGET = options.target;
    var GLOBAL = options.global;
    var STATIC = options.stat;
    var FORCED, target, key, targetProperty, sourceProperty, descriptor;
    if (GLOBAL) {
      target = globalThis$2;
    } else if (STATIC) {
      target = globalThis$2[TARGET] || defineGlobalProperty(TARGET, {});
    } else {
      target = globalThis$2[TARGET] && globalThis$2[TARGET].prototype;
    }
    if (target) for (key in source) {
      sourceProperty = source[key];
      if (options.dontCallGetSet) {
        descriptor = getOwnPropertyDescriptor(target, key);
        targetProperty = descriptor && descriptor.value;
      } else targetProperty = target[key];
      FORCED = isForced(GLOBAL ? key : TARGET + (STATIC ? "." : "#") + key, options.forced);
      if (!FORCED && targetProperty !== void 0) {
        if (typeof sourceProperty == typeof targetProperty) continue;
        copyConstructorProperties(sourceProperty, targetProperty);
      }
      if (options.sham || targetProperty && targetProperty.sham) {
        createNonEnumerableProperty(sourceProperty, "sham", true);
      }
      defineBuiltIn(target, key, sourceProperty, options);
    }
  };
  var objectDefineProperties = {};
  var internalObjectKeys = objectKeysInternal;
  var enumBugKeys$1 = enumBugKeys$3;
  var objectKeys$1 = Object.keys || function keys2(O) {
    return internalObjectKeys(O, enumBugKeys$1);
  };
  var DESCRIPTORS = descriptors;
  var V8_PROTOTYPE_DEFINE_BUG = v8PrototypeDefineBug;
  var definePropertyModule = objectDefineProperty;
  var anObject$2 = anObject$5;
  var toIndexedObject = toIndexedObject$4;
  var objectKeys = objectKeys$1;
  objectDefineProperties.f = DESCRIPTORS && !V8_PROTOTYPE_DEFINE_BUG ? Object.defineProperties : function defineProperties(O, Properties) {
    anObject$2(O);
    var props = toIndexedObject(Properties);
    var keys2 = objectKeys(Properties);
    var length = keys2.length;
    var index = 0;
    var key;
    while (length > index) definePropertyModule.f(O, key = keys2[index++], props[key]);
    return O;
  };
  var getBuiltIn = getBuiltIn$3;
  var html$1 = getBuiltIn("document", "documentElement");
  var anObject$1 = anObject$5;
  var definePropertiesModule = objectDefineProperties;
  var enumBugKeys = enumBugKeys$3;
  var hiddenKeys = hiddenKeys$4;
  var html = html$1;
  var documentCreateElement = documentCreateElement$1;
  var sharedKey = sharedKey$2;
  var GT = ">";
  var LT = "<";
  var PROTOTYPE = "prototype";
  var SCRIPT = "script";
  var IE_PROTO = sharedKey("IE_PROTO");
  var EmptyConstructor = function() {
  };
  var scriptTag = function(content) {
    return LT + SCRIPT + GT + content + LT + "/" + SCRIPT + GT;
  };
  var NullProtoObjectViaActiveX = function(activeXDocument2) {
    activeXDocument2.write(scriptTag(""));
    activeXDocument2.close();
    var temp = activeXDocument2.parentWindow.Object;
    activeXDocument2 = null;
    return temp;
  };
  var NullProtoObjectViaIFrame = function() {
    var iframe = documentCreateElement("iframe");
    var JS = "java" + SCRIPT + ":";
    var iframeDocument;
    iframe.style.display = "none";
    html.appendChild(iframe);
    iframe.src = String(JS);
    iframeDocument = iframe.contentWindow.document;
    iframeDocument.open();
    iframeDocument.write(scriptTag("document.F=Object"));
    iframeDocument.close();
    return iframeDocument.F;
  };
  var activeXDocument;
  var NullProtoObject = function() {
    try {
      activeXDocument = new ActiveXObject("htmlfile");
    } catch (error) {
    }
    NullProtoObject = typeof document != "undefined" ? document.domain && activeXDocument ? NullProtoObjectViaActiveX(activeXDocument) : NullProtoObjectViaIFrame() : NullProtoObjectViaActiveX(activeXDocument);
    var length = enumBugKeys.length;
    while (length--) delete NullProtoObject[PROTOTYPE][enumBugKeys[length]];
    return NullProtoObject();
  };
  hiddenKeys[IE_PROTO] = true;
  var objectCreate = Object.create || function create2(O, Properties) {
    var result;
    if (O !== null) {
      EmptyConstructor[PROTOTYPE] = anObject$1(O);
      result = new EmptyConstructor();
      EmptyConstructor[PROTOTYPE] = null;
      result[IE_PROTO] = O;
    } else result = NullProtoObject();
    return Properties === void 0 ? result : definePropertiesModule.f(result, Properties);
  };
  var wellKnownSymbol$4 = wellKnownSymbol$6;
  var create = objectCreate;
  var defineProperty = objectDefineProperty.f;
  var UNSCOPABLES = wellKnownSymbol$4("unscopables");
  var ArrayPrototype = Array.prototype;
  if (ArrayPrototype[UNSCOPABLES] === void 0) {
    defineProperty(ArrayPrototype, UNSCOPABLES, {
      configurable: true,
      value: create(null)
    });
  }
  var addToUnscopables$1 = function(key) {
    ArrayPrototype[UNSCOPABLES][key] = true;
  };
  var $$2 = _export;
  var toObject$1 = toObject$3;
  var lengthOfArrayLike = lengthOfArrayLike$2;
  var toIntegerOrInfinity = toIntegerOrInfinity$3;
  var addToUnscopables = addToUnscopables$1;
  $$2({ target: "Array", proto: true }, {
    at: function at(index) {
      var O = toObject$1(this);
      var len = lengthOfArrayLike(O);
      var relativeIndex = toIntegerOrInfinity(index);
      var k = relativeIndex >= 0 ? relativeIndex : len + relativeIndex;
      return k < 0 || k >= len ? void 0 : O[k];
    }
  });
  addToUnscopables("at");
  var $$1 = _export;
  var hasOwn$1 = hasOwnProperty_1;
  $$1({ target: "Object", stat: true }, {
    hasOwn: hasOwn$1
  });
  var isObject$1 = isObject$6;
  var classof$2 = classofRaw$1;
  var wellKnownSymbol$3 = wellKnownSymbol$6;
  var MATCH = wellKnownSymbol$3("match");
  var isRegexp = function(it) {
    var isRegExp2;
    return isObject$1(it) && ((isRegExp2 = it[MATCH]) !== void 0 ? !!isRegExp2 : classof$2(it) === "RegExp");
  };
  var wellKnownSymbol$2 = wellKnownSymbol$6;
  var TO_STRING_TAG$1 = wellKnownSymbol$2("toStringTag");
  var test = {};
  test[TO_STRING_TAG$1] = "z";
  var toStringTagSupport = String(test) === "[object z]";
  var TO_STRING_TAG_SUPPORT = toStringTagSupport;
  var isCallable$1 = isCallable$c;
  var classofRaw = classofRaw$1;
  var wellKnownSymbol$1 = wellKnownSymbol$6;
  var TO_STRING_TAG = wellKnownSymbol$1("toStringTag");
  var $Object = Object;
  var CORRECT_ARGUMENTS = classofRaw(/* @__PURE__ */ function() {
    return arguments;
  }()) === "Arguments";
  var tryGet = function(it, key) {
    try {
      return it[key];
    } catch (error) {
    }
  };
  var classof$1 = TO_STRING_TAG_SUPPORT ? classofRaw : function(it) {
    var O, tag, result;
    return it === void 0 ? "Undefined" : it === null ? "Null" : typeof (tag = tryGet(O = $Object(it), TO_STRING_TAG)) == "string" ? tag : CORRECT_ARGUMENTS ? classofRaw(O) : (result = classofRaw(O)) === "Object" && isCallable$1(O.callee) ? "Arguments" : result;
  };
  var classof = classof$1;
  var $String = String;
  var toString$1 = function(argument) {
    if (classof(argument) === "Symbol") throw new TypeError("Cannot convert a Symbol value to a string");
    return $String(argument);
  };
  var globalThis$1 = globalThis_1;
  var fails = fails$9;
  var RegExp$1 = globalThis$1.RegExp;
  var FLAGS_GETTER_IS_CORRECT = !fails(function() {
    var INDICES_SUPPORT = true;
    try {
      RegExp$1(".", "d");
    } catch (error) {
      INDICES_SUPPORT = false;
    }
    var O = {};
    var calls = "";
    var expected = INDICES_SUPPORT ? "dgimsy" : "gimsy";
    var addGetter = function(key2, chr) {
      Object.defineProperty(O, key2, { get: function() {
        calls += chr;
        return true;
      } });
    };
    var pairs = {
      dotAll: "s",
      global: "g",
      ignoreCase: "i",
      multiline: "m",
      sticky: "y"
    };
    if (INDICES_SUPPORT) pairs.hasIndices = "d";
    for (var key in pairs) addGetter(key, pairs[key]);
    var result = Object.getOwnPropertyDescriptor(RegExp$1.prototype, "flags").get.call(O);
    return result !== expected || calls !== expected;
  });
  var regexpFlagsDetection = { correct: FLAGS_GETTER_IS_CORRECT };
  var anObject = anObject$5;
  var regexpFlags = function() {
    var that = anObject(this);
    var result = "";
    if (that.hasIndices) result += "d";
    if (that.global) result += "g";
    if (that.ignoreCase) result += "i";
    if (that.multiline) result += "m";
    if (that.dotAll) result += "s";
    if (that.unicode) result += "u";
    if (that.unicodeSets) result += "v";
    if (that.sticky) result += "y";
    return result;
  };
  var call$1 = functionCall;
  var hasOwn = hasOwnProperty_1;
  var isPrototypeOf = objectIsPrototypeOf;
  var regExpFlagsDetection = regexpFlagsDetection;
  var regExpFlagsGetterImplementation = regexpFlags;
  var RegExpPrototype = RegExp.prototype;
  var regexpGetFlags = regExpFlagsDetection.correct ? function(it) {
    return it.flags;
  } : function(it) {
    return !regExpFlagsDetection.correct && isPrototypeOf(RegExpPrototype, it) && !hasOwn(it, "flags") ? call$1(regExpFlagsGetterImplementation, it) : it.flags;
  };
  var uncurryThis$1 = functionUncurryThis;
  var toObject = toObject$3;
  var floor = Math.floor;
  var charAt = uncurryThis$1("".charAt);
  var replace = uncurryThis$1("".replace);
  var stringSlice$1 = uncurryThis$1("".slice);
  var SUBSTITUTION_SYMBOLS = /\$([$&'`]|\d{1,2}|<[^>]*>)/g;
  var SUBSTITUTION_SYMBOLS_NO_NAMED = /\$([$&'`]|\d{1,2})/g;
  var getSubstitution$1 = function(matched, str, position, captures, namedCaptures, replacement2) {
    var tailPos = position + matched.length;
    var m = captures.length;
    var symbols = SUBSTITUTION_SYMBOLS_NO_NAMED;
    if (namedCaptures !== void 0) {
      namedCaptures = toObject(namedCaptures);
      symbols = SUBSTITUTION_SYMBOLS;
    }
    return replace(replacement2, symbols, function(match2, ch) {
      var capture;
      switch (charAt(ch, 0)) {
        case "$":
          return "$";
        case "&":
          return matched;
        case "`":
          return stringSlice$1(str, 0, position);
        case "'":
          return stringSlice$1(str, tailPos);
        case "<":
          capture = namedCaptures[stringSlice$1(ch, 1, -1)];
          break;
        default:
          var n = +ch;
          if (n === 0) return match2;
          if (n > m) {
            var f = floor(n / 10);
            if (f === 0) return match2;
            if (f <= m) return captures[f - 1] === void 0 ? charAt(ch, 1) : captures[f - 1] + charAt(ch, 1);
            return match2;
          }
          capture = captures[n - 1];
      }
      return capture === void 0 ? "" : capture;
    });
  };
  var $ = _export;
  var call = functionCall;
  var uncurryThis = functionUncurryThis;
  var requireObjectCoercible = requireObjectCoercible$3;
  var isCallable = isCallable$c;
  var isObject = isObject$6;
  var isRegExp = isRegexp;
  var toString = toString$1;
  var getMethod = getMethod$2;
  var getRegExpFlags = regexpGetFlags;
  var getSubstitution = getSubstitution$1;
  var wellKnownSymbol = wellKnownSymbol$6;
  var REPLACE = wellKnownSymbol("replace");
  var $TypeError = TypeError;
  var indexOf = uncurryThis("".indexOf);
  uncurryThis("".replace);
  var stringSlice = uncurryThis("".slice);
  var max = Math.max;
  $({ target: "String", proto: true }, {
    replaceAll: function replaceAll(searchValue, replaceValue) {
      var O = requireObjectCoercible(this);
      var IS_REG_EXP, flags, replacer, string, searchString, functionalReplace, searchLength, advanceBy, position, replacement2;
      var endOfLastMatch = 0;
      var result = "";
      if (isObject(searchValue)) {
        IS_REG_EXP = isRegExp(searchValue);
        if (IS_REG_EXP) {
          flags = toString(requireObjectCoercible(getRegExpFlags(searchValue)));
          if (!~indexOf(flags, "g")) throw new $TypeError("`.replaceAll` does not allow non-global regexes");
        }
        replacer = getMethod(searchValue, REPLACE);
        if (replacer) return call(replacer, searchValue, O, replaceValue);
      }
      string = toString(O);
      searchString = toString(searchValue);
      functionalReplace = isCallable(replaceValue);
      if (!functionalReplace) replaceValue = toString(replaceValue);
      searchLength = searchString.length;
      advanceBy = max(1, searchLength);
      position = indexOf(string, searchString);
      while (position !== -1) {
        replacement2 = functionalReplace ? toString(replaceValue(searchString, position, string)) : getSubstitution(searchString, string, position, [], void 0, replaceValue);
        result += stringSlice(string, endOfLastMatch, position) + replacement2;
        endOfLastMatch = position + searchLength;
        position = position + advanceBy > string.length ? -1 : indexOf(string, searchString, position + advanceBy);
      }
      if (endOfLastMatch < string.length) {
        result += stringSlice(string, endOfLastMatch);
      }
      return result;
    }
  });
  const name = "corejs";
  const items = [
    1,
    2,
    3
  ];
  const data = {
    name,
    items
  };
  console.log("eval keys");
  var keys$1 = function keys2(value) {
    return Object.keys(value).map(function(key) {
      return key.replaceAll("a", "A");
    });
  };
  var keys = keys$1;
  console.log("eval describe");
  var describe_1 = function(value) {
    return keys(value).join("|") + " " + Object.hasOwn(value, "items");
  };
  console.log("eval index");
  console.log("at " + data.items.at(-1) + " hasOwn " + Object.hasOwn(data, "name"));
  console.log("replaceAll " + "a-b-c".replaceAll("-", "+"));
  console.log("describe " + describe_1(data));
})();
