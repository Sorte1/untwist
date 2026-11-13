(function() {
  "use strict";
  var commonjsGlobal = typeof globalThis !== "undefined" ? globalThis : typeof window !== "undefined" ? window : typeof global !== "undefined" ? global : typeof self !== "undefined" ? self : {};
  var es_array_at = {};
  var globalThis_1;
  var hasRequiredGlobalThis;
  function requireGlobalThis() {
    if (hasRequiredGlobalThis) return globalThis_1;
    hasRequiredGlobalThis = 1;
    var check = function(it) {
      return it && it.Math === Math && it;
    };
    globalThis_1 = // eslint-disable-next-line es/no-global-this -- safe
    check(typeof globalThis == "object" && globalThis) || check(typeof window == "object" && window) || // eslint-disable-next-line no-restricted-globals -- safe
    check(typeof self == "object" && self) || check(typeof commonjsGlobal == "object" && commonjsGlobal) || check(typeof globalThis_1 == "object" && globalThis_1) || // eslint-disable-next-line no-new-func -- fallback
    /* @__PURE__ */ (function() {
      return this;
    })() || Function("return this")();
    return globalThis_1;
  }
  var objectGetOwnPropertyDescriptor = {};
  var fails;
  var hasRequiredFails;
  function requireFails() {
    if (hasRequiredFails) return fails;
    hasRequiredFails = 1;
    fails = function(exec) {
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
  function requireDescriptors() {
    if (hasRequiredDescriptors) return descriptors;
    hasRequiredDescriptors = 1;
    var fails2 = requireFails();
    descriptors = !fails2(function() {
      return Object.defineProperty({}, 1, { get: function() {
        return 7;
      } })[1] !== 7;
    });
    return descriptors;
  }
  var functionBindNative;
  var hasRequiredFunctionBindNative;
  function requireFunctionBindNative() {
    if (hasRequiredFunctionBindNative) return functionBindNative;
    hasRequiredFunctionBindNative = 1;
    var fails2 = requireFails();
    functionBindNative = !fails2(function() {
      var test = (function() {
      }).bind();
      return typeof test != "function" || test.hasOwnProperty("prototype");
    });
    return functionBindNative;
  }
  var functionCall;
  var hasRequiredFunctionCall;
  function requireFunctionCall() {
    if (hasRequiredFunctionCall) return functionCall;
    hasRequiredFunctionCall = 1;
    var NATIVE_BIND = requireFunctionBindNative();
    var call = Function.prototype.call;
    functionCall = NATIVE_BIND ? call.bind(call) : function() {
      return call.apply(call, arguments);
    };
    return functionCall;
  }
  var objectPropertyIsEnumerable = {};
  var hasRequiredObjectPropertyIsEnumerable;
  function requireObjectPropertyIsEnumerable() {
    if (hasRequiredObjectPropertyIsEnumerable) return objectPropertyIsEnumerable;
    hasRequiredObjectPropertyIsEnumerable = 1;
    var $propertyIsEnumerable = {}.propertyIsEnumerable;
    var getOwnPropertyDescriptor = Object.getOwnPropertyDescriptor;
    var NASHORN_BUG = getOwnPropertyDescriptor && !$propertyIsEnumerable.call({ 1: 2 }, 1);
    objectPropertyIsEnumerable.f = NASHORN_BUG ? function propertyIsEnumerable(V) {
      var descriptor = getOwnPropertyDescriptor(this, V);
      return !!descriptor && descriptor.enumerable;
    } : $propertyIsEnumerable;
    return objectPropertyIsEnumerable;
  }
  var createPropertyDescriptor;
  var hasRequiredCreatePropertyDescriptor;
  function requireCreatePropertyDescriptor() {
    if (hasRequiredCreatePropertyDescriptor) return createPropertyDescriptor;
    hasRequiredCreatePropertyDescriptor = 1;
    createPropertyDescriptor = function(bitmap, value) {
      return {
        enumerable: !(bitmap & 1),
        configurable: !(bitmap & 2),
        writable: !(bitmap & 4),
        value
      };
    };
    return createPropertyDescriptor;
  }
  var functionUncurryThis;
  var hasRequiredFunctionUncurryThis;
  function requireFunctionUncurryThis() {
    if (hasRequiredFunctionUncurryThis) return functionUncurryThis;
    hasRequiredFunctionUncurryThis = 1;
    var NATIVE_BIND = requireFunctionBindNative();
    var FunctionPrototype = Function.prototype;
    var call = FunctionPrototype.call;
    var uncurryThisWithBind = NATIVE_BIND && FunctionPrototype.bind.bind(call, call);
    functionUncurryThis = NATIVE_BIND ? uncurryThisWithBind : function(fn) {
      return function() {
        return call.apply(fn, arguments);
      };
    };
    return functionUncurryThis;
  }
  var classofRaw;
  var hasRequiredClassofRaw;
  function requireClassofRaw() {
    if (hasRequiredClassofRaw) return classofRaw;
    hasRequiredClassofRaw = 1;
    var uncurryThis = requireFunctionUncurryThis();
    var toString2 = uncurryThis({}.toString);
    var stringSlice = uncurryThis("".slice);
    classofRaw = function(it) {
      return stringSlice(toString2(it), 8, -1);
    };
    return classofRaw;
  }
  var indexedObject;
  var hasRequiredIndexedObject;
  function requireIndexedObject() {
    if (hasRequiredIndexedObject) return indexedObject;
    hasRequiredIndexedObject = 1;
    var uncurryThis = requireFunctionUncurryThis();
    var fails2 = requireFails();
    var classof2 = requireClassofRaw();
    var $Object = Object;
    var split = uncurryThis("".split);
    indexedObject = fails2(function() {
      return !$Object("z").propertyIsEnumerable(0);
    }) ? function(it) {
      return classof2(it) === "String" ? split(it, "") : $Object(it);
    } : $Object;
    return indexedObject;
  }
  var isNullOrUndefined;
  var hasRequiredIsNullOrUndefined;
  function requireIsNullOrUndefined() {
    if (hasRequiredIsNullOrUndefined) return isNullOrUndefined;
    hasRequiredIsNullOrUndefined = 1;
    isNullOrUndefined = function(it) {
      return it === null || it === void 0;
    };
    return isNullOrUndefined;
  }
  var requireObjectCoercible;
  var hasRequiredRequireObjectCoercible;
  function requireRequireObjectCoercible() {
    if (hasRequiredRequireObjectCoercible) return requireObjectCoercible;
    hasRequiredRequireObjectCoercible = 1;
    var isNullOrUndefined2 = requireIsNullOrUndefined();
    var $TypeError = TypeError;
    requireObjectCoercible = function(it) {
      if (isNullOrUndefined2(it)) throw new $TypeError("Can't call method on " + it);
      return it;
    };
    return requireObjectCoercible;
  }
  var toIndexedObject;
  var hasRequiredToIndexedObject;
  function requireToIndexedObject() {
    if (hasRequiredToIndexedObject) return toIndexedObject;
    hasRequiredToIndexedObject = 1;
    var IndexedObject = requireIndexedObject();
    var requireObjectCoercible2 = requireRequireObjectCoercible();
    toIndexedObject = function(it) {
      return IndexedObject(requireObjectCoercible2(it));
    };
    return toIndexedObject;
  }
  var isCallable;
  var hasRequiredIsCallable;
  function requireIsCallable() {
    if (hasRequiredIsCallable) return isCallable;
    hasRequiredIsCallable = 1;
    var documentAll = typeof document == "object" && document.all;
    isCallable = typeof documentAll == "undefined" && documentAll !== void 0 ? function(argument) {
      return typeof argument == "function" || argument === documentAll;
    } : function(argument) {
      return typeof argument == "function";
    };
    return isCallable;
  }
  var isObject;
  var hasRequiredIsObject;
  function requireIsObject() {
    if (hasRequiredIsObject) return isObject;
    hasRequiredIsObject = 1;
    var isCallable2 = requireIsCallable();
    isObject = function(it) {
      return typeof it == "object" ? it !== null : isCallable2(it);
    };
    return isObject;
  }
  var getBuiltIn;
  var hasRequiredGetBuiltIn;
  function requireGetBuiltIn() {
    if (hasRequiredGetBuiltIn) return getBuiltIn;
    hasRequiredGetBuiltIn = 1;
    var globalThis2 = requireGlobalThis();
    var isCallable2 = requireIsCallable();
    var aFunction = function(argument) {
      return isCallable2(argument) ? argument : void 0;
    };
    getBuiltIn = function(namespace, method) {
      return arguments.length < 2 ? aFunction(globalThis2[namespace]) : globalThis2[namespace] && globalThis2[namespace][method];
    };
    return getBuiltIn;
  }
  var objectIsPrototypeOf;
  var hasRequiredObjectIsPrototypeOf;
  function requireObjectIsPrototypeOf() {
    if (hasRequiredObjectIsPrototypeOf) return objectIsPrototypeOf;
    hasRequiredObjectIsPrototypeOf = 1;
    var uncurryThis = requireFunctionUncurryThis();
    objectIsPrototypeOf = uncurryThis({}.isPrototypeOf);
    return objectIsPrototypeOf;
  }
  var environmentUserAgent;
  var hasRequiredEnvironmentUserAgent;
  function requireEnvironmentUserAgent() {
    if (hasRequiredEnvironmentUserAgent) return environmentUserAgent;
    hasRequiredEnvironmentUserAgent = 1;
    var globalThis2 = requireGlobalThis();
    var navigator = globalThis2.navigator;
    var userAgent = navigator && navigator.userAgent;
    environmentUserAgent = userAgent ? String(userAgent) : "";
    return environmentUserAgent;
  }
  var environmentV8Version;
  var hasRequiredEnvironmentV8Version;
  function requireEnvironmentV8Version() {
    if (hasRequiredEnvironmentV8Version) return environmentV8Version;
    hasRequiredEnvironmentV8Version = 1;
    var globalThis2 = requireGlobalThis();
    var userAgent = requireEnvironmentUserAgent();
    var process = globalThis2.process;
    var Deno = globalThis2.Deno;
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
    environmentV8Version = version;
    return environmentV8Version;
  }
  var symbolConstructorDetection;
  var hasRequiredSymbolConstructorDetection;
  function requireSymbolConstructorDetection() {
    if (hasRequiredSymbolConstructorDetection) return symbolConstructorDetection;
    hasRequiredSymbolConstructorDetection = 1;
    var V8_VERSION = requireEnvironmentV8Version();
    var fails2 = requireFails();
    var globalThis2 = requireGlobalThis();
    var $String = globalThis2.String;
    symbolConstructorDetection = !!Object.getOwnPropertySymbols && !fails2(function() {
      var symbol = Symbol("symbol detection");
      return !$String(symbol) || !(Object(symbol) instanceof Symbol) || // Chrome 38-40 symbols are not inherited from DOM collections prototypes to instances
      !Symbol.sham && V8_VERSION && V8_VERSION < 41;
    });
    return symbolConstructorDetection;
  }
  var useSymbolAsUid;
  var hasRequiredUseSymbolAsUid;
  function requireUseSymbolAsUid() {
    if (hasRequiredUseSymbolAsUid) return useSymbolAsUid;
    hasRequiredUseSymbolAsUid = 1;
    var NATIVE_SYMBOL = requireSymbolConstructorDetection();
    useSymbolAsUid = NATIVE_SYMBOL && !Symbol.sham && typeof Symbol.iterator == "symbol";
    return useSymbolAsUid;
  }
  var isSymbol;
  var hasRequiredIsSymbol;
  function requireIsSymbol() {
    if (hasRequiredIsSymbol) return isSymbol;
    hasRequiredIsSymbol = 1;
    var getBuiltIn2 = requireGetBuiltIn();
    var isCallable2 = requireIsCallable();
    var isPrototypeOf = requireObjectIsPrototypeOf();
    var USE_SYMBOL_AS_UID = requireUseSymbolAsUid();
    var $Object = Object;
    isSymbol = USE_SYMBOL_AS_UID ? function(it) {
      return typeof it == "symbol";
    } : function(it) {
      var $Symbol = getBuiltIn2("Symbol");
      return isCallable2($Symbol) && isPrototypeOf($Symbol.prototype, $Object(it));
    };
    return isSymbol;
  }
  var tryToString;
  var hasRequiredTryToString;
  function requireTryToString() {
    if (hasRequiredTryToString) return tryToString;
    hasRequiredTryToString = 1;
    var $String = String;
    tryToString = function(argument) {
      try {
        return $String(argument);
      } catch (error) {
        return "Object";
      }
    };
    return tryToString;
  }
  var aCallable;
  var hasRequiredACallable;
  function requireACallable() {
    if (hasRequiredACallable) return aCallable;
    hasRequiredACallable = 1;
    var isCallable2 = requireIsCallable();
    var tryToString2 = requireTryToString();
    var $TypeError = TypeError;
    aCallable = function(argument) {
      if (isCallable2(argument)) return argument;
      throw new $TypeError(tryToString2(argument) + " is not a function");
    };
    return aCallable;
  }
  var getMethod;
  var hasRequiredGetMethod;
  function requireGetMethod() {
    if (hasRequiredGetMethod) return getMethod;
    hasRequiredGetMethod = 1;
    var aCallable2 = requireACallable();
    var isNullOrUndefined2 = requireIsNullOrUndefined();
    getMethod = function(V, P) {
      var func = V[P];
      return isNullOrUndefined2(func) ? void 0 : aCallable2(func);
    };
    return getMethod;
  }
  var ordinaryToPrimitive;
  var hasRequiredOrdinaryToPrimitive;
  function requireOrdinaryToPrimitive() {
    if (hasRequiredOrdinaryToPrimitive) return ordinaryToPrimitive;
    hasRequiredOrdinaryToPrimitive = 1;
    var call = requireFunctionCall();
    var isCallable2 = requireIsCallable();
    var isObject2 = requireIsObject();
    var $TypeError = TypeError;
    ordinaryToPrimitive = function(input, pref) {
      var fn, val;
      if (pref === "string" && isCallable2(fn = input.toString) && !isObject2(val = call(fn, input))) return val;
      if (isCallable2(fn = input.valueOf) && !isObject2(val = call(fn, input))) return val;
      if (pref !== "string" && isCallable2(fn = input.toString) && !isObject2(val = call(fn, input))) return val;
      throw new $TypeError("Can't convert object to primitive value");
    };
    return ordinaryToPrimitive;
  }
  var sharedStore = { exports: {} };
  var isPure;
  var hasRequiredIsPure;
  function requireIsPure() {
    if (hasRequiredIsPure) return isPure;
    hasRequiredIsPure = 1;
    isPure = false;
    return isPure;
  }
  var defineGlobalProperty;
  var hasRequiredDefineGlobalProperty;
  function requireDefineGlobalProperty() {
    if (hasRequiredDefineGlobalProperty) return defineGlobalProperty;
    hasRequiredDefineGlobalProperty = 1;
    var globalThis2 = requireGlobalThis();
    var defineProperty = Object.defineProperty;
    defineGlobalProperty = function(key, value) {
      try {
        defineProperty(globalThis2, key, { value, configurable: true, writable: true });
      } catch (error) {
        globalThis2[key] = value;
      }
      return value;
    };
    return defineGlobalProperty;
  }
  var hasRequiredSharedStore;
  function requireSharedStore() {
    if (hasRequiredSharedStore) return sharedStore.exports;
    hasRequiredSharedStore = 1;
    var IS_PURE = requireIsPure();
    var globalThis2 = requireGlobalThis();
    var defineGlobalProperty2 = requireDefineGlobalProperty();
    var SHARED = "__core-js_shared__";
    var store = sharedStore.exports = globalThis2[SHARED] || defineGlobalProperty2(SHARED, {});
    (store.versions || (store.versions = [])).push({
      version: "3.51.0",
      mode: IS_PURE ? "pure" : "global",
      copyright: "© 2013–2025 Denis Pushkarev (zloirock.ru), 2025–2026 CoreJS Company (core-js.io). All rights reserved.",
      license: "https://github.com/zloirock/core-js/blob/v3.51.0/LICENSE",
      source: "https://github.com/zloirock/core-js"
    });
    return sharedStore.exports;
  }
  var shared;
  var hasRequiredShared;
  function requireShared() {
    if (hasRequiredShared) return shared;
    hasRequiredShared = 1;
    var store = requireSharedStore();
    var create = Object.create || Object;
    shared = function(key, value) {
      return store[key] || (store[key] = value || create(null));
    };
    return shared;
  }
  var toObject;
  var hasRequiredToObject;
  function requireToObject() {
    if (hasRequiredToObject) return toObject;
    hasRequiredToObject = 1;
    var requireObjectCoercible2 = requireRequireObjectCoercible();
    var $Object = Object;
    toObject = function(argument) {
      return $Object(requireObjectCoercible2(argument));
    };
    return toObject;
  }
  var hasOwnProperty_1;
  var hasRequiredHasOwnProperty;
  function requireHasOwnProperty() {
    if (hasRequiredHasOwnProperty) return hasOwnProperty_1;
    hasRequiredHasOwnProperty = 1;
    var uncurryThis = requireFunctionUncurryThis();
    var toObject2 = requireToObject();
    var hasOwnProperty = uncurryThis({}.hasOwnProperty);
    hasOwnProperty_1 = Object.hasOwn || function hasOwn(it, key) {
      return hasOwnProperty(toObject2(it), key);
    };
    return hasOwnProperty_1;
  }
  var uid;
  var hasRequiredUid;
  function requireUid() {
    if (hasRequiredUid) return uid;
    hasRequiredUid = 1;
    var uncurryThis = requireFunctionUncurryThis();
    var id = 0;
    var postfix = Math.random();
    var toString2 = uncurryThis(1.1.toString);
    uid = function(key) {
      return "Symbol(" + (key === void 0 ? "" : key) + ")_" + toString2(++id + postfix, 36);
    };
    return uid;
  }
  var wellKnownSymbol;
  var hasRequiredWellKnownSymbol;
  function requireWellKnownSymbol() {
    if (hasRequiredWellKnownSymbol) return wellKnownSymbol;
    hasRequiredWellKnownSymbol = 1;
    var globalThis2 = requireGlobalThis();
    var shared2 = requireShared();
    var hasOwn = requireHasOwnProperty();
    var uid2 = requireUid();
    var NATIVE_SYMBOL = requireSymbolConstructorDetection();
    var USE_SYMBOL_AS_UID = requireUseSymbolAsUid();
    var Symbol2 = globalThis2.Symbol;
    var WellKnownSymbolsStore = shared2("wks");
    var createWellKnownSymbol = USE_SYMBOL_AS_UID ? Symbol2["for"] || Symbol2 : Symbol2 && Symbol2.withoutSetter || uid2;
    wellKnownSymbol = function(name2) {
      if (!hasOwn(WellKnownSymbolsStore, name2)) {
        WellKnownSymbolsStore[name2] = NATIVE_SYMBOL && hasOwn(Symbol2, name2) ? Symbol2[name2] : createWellKnownSymbol("Symbol." + name2);
      }
      return WellKnownSymbolsStore[name2];
    };
    return wellKnownSymbol;
  }
  var toPrimitive;
  var hasRequiredToPrimitive;
  function requireToPrimitive() {
    if (hasRequiredToPrimitive) return toPrimitive;
    hasRequiredToPrimitive = 1;
    var call = requireFunctionCall();
    var isObject2 = requireIsObject();
    var isSymbol2 = requireIsSymbol();
    var getMethod2 = requireGetMethod();
    var ordinaryToPrimitive2 = requireOrdinaryToPrimitive();
    var wellKnownSymbol2 = requireWellKnownSymbol();
    var $TypeError = TypeError;
    var TO_PRIMITIVE = wellKnownSymbol2("toPrimitive");
    toPrimitive = function(input, pref) {
      if (!isObject2(input) || isSymbol2(input)) return input;
      var exoticToPrim = getMethod2(input, TO_PRIMITIVE);
      var result;
      if (exoticToPrim) {
        if (pref === void 0) pref = "default";
        result = call(exoticToPrim, input, pref);
        if (!isObject2(result) || isSymbol2(result)) return result;
        throw new $TypeError("Can't convert object to primitive value");
      }
      if (pref === void 0) pref = "number";
      return ordinaryToPrimitive2(input, pref);
    };
    return toPrimitive;
  }
  var toPropertyKey;
  var hasRequiredToPropertyKey;
  function requireToPropertyKey() {
    if (hasRequiredToPropertyKey) return toPropertyKey;
    hasRequiredToPropertyKey = 1;
    var toPrimitive2 = requireToPrimitive();
    var isSymbol2 = requireIsSymbol();
    toPropertyKey = function(argument) {
      var key = toPrimitive2(argument, "string");
      return isSymbol2(key) ? key : key + "";
    };
    return toPropertyKey;
  }
  var documentCreateElement;
  var hasRequiredDocumentCreateElement;
  function requireDocumentCreateElement() {
    if (hasRequiredDocumentCreateElement) return documentCreateElement;
    hasRequiredDocumentCreateElement = 1;
    var globalThis2 = requireGlobalThis();
    var isObject2 = requireIsObject();
    var document2 = globalThis2.document;
    var EXISTS = isObject2(document2) && isObject2(document2.createElement);
    documentCreateElement = function(it) {
      return EXISTS ? document2.createElement(it) : {};
    };
    return documentCreateElement;
  }
  var ie8DomDefine;
  var hasRequiredIe8DomDefine;
  function requireIe8DomDefine() {
    if (hasRequiredIe8DomDefine) return ie8DomDefine;
    hasRequiredIe8DomDefine = 1;
    var DESCRIPTORS = requireDescriptors();
    var fails2 = requireFails();
    var createElement = requireDocumentCreateElement();
    ie8DomDefine = !DESCRIPTORS && !fails2(function() {
      return Object.defineProperty(createElement("div"), "a", {
        get: function() {
          return 7;
        }
      }).a !== 7;
    });
    return ie8DomDefine;
  }
  var hasRequiredObjectGetOwnPropertyDescriptor;
  function requireObjectGetOwnPropertyDescriptor() {
    if (hasRequiredObjectGetOwnPropertyDescriptor) return objectGetOwnPropertyDescriptor;
    hasRequiredObjectGetOwnPropertyDescriptor = 1;
    var DESCRIPTORS = requireDescriptors();
    var call = requireFunctionCall();
    var propertyIsEnumerableModule = requireObjectPropertyIsEnumerable();
    var createPropertyDescriptor2 = requireCreatePropertyDescriptor();
    var toIndexedObject2 = requireToIndexedObject();
    var toPropertyKey2 = requireToPropertyKey();
    var hasOwn = requireHasOwnProperty();
    var IE8_DOM_DEFINE = requireIe8DomDefine();
    var $getOwnPropertyDescriptor = Object.getOwnPropertyDescriptor;
    objectGetOwnPropertyDescriptor.f = DESCRIPTORS ? $getOwnPropertyDescriptor : function getOwnPropertyDescriptor(O, P) {
      O = toIndexedObject2(O);
      P = toPropertyKey2(P);
      if (IE8_DOM_DEFINE) try {
        return $getOwnPropertyDescriptor(O, P);
      } catch (error) {
      }
      if (hasOwn(O, P)) return createPropertyDescriptor2(!call(propertyIsEnumerableModule.f, O, P), O[P]);
    };
    return objectGetOwnPropertyDescriptor;
  }
  var objectDefineProperty = {};
  var v8PrototypeDefineBug;
  var hasRequiredV8PrototypeDefineBug;
  function requireV8PrototypeDefineBug() {
    if (hasRequiredV8PrototypeDefineBug) return v8PrototypeDefineBug;
    hasRequiredV8PrototypeDefineBug = 1;
    var DESCRIPTORS = requireDescriptors();
    var fails2 = requireFails();
    v8PrototypeDefineBug = DESCRIPTORS && fails2(function() {
      return Object.defineProperty(function() {
      }, "prototype", {
        value: 42,
        writable: false
      }).prototype !== 42;
    });
    return v8PrototypeDefineBug;
  }
  var anObject;
  var hasRequiredAnObject;
  function requireAnObject() {
    if (hasRequiredAnObject) return anObject;
    hasRequiredAnObject = 1;
    var isObject2 = requireIsObject();
    var $String = String;
    var $TypeError = TypeError;
    anObject = function(argument) {
      if (isObject2(argument)) return argument;
      throw new $TypeError($String(argument) + " is not an object");
    };
    return anObject;
  }
  var hasRequiredObjectDefineProperty;
  function requireObjectDefineProperty() {
    if (hasRequiredObjectDefineProperty) return objectDefineProperty;
    hasRequiredObjectDefineProperty = 1;
    var DESCRIPTORS = requireDescriptors();
    var IE8_DOM_DEFINE = requireIe8DomDefine();
    var V8_PROTOTYPE_DEFINE_BUG = requireV8PrototypeDefineBug();
    var anObject2 = requireAnObject();
    var toPropertyKey2 = requireToPropertyKey();
    var $TypeError = TypeError;
    var $defineProperty = Object.defineProperty;
    var $getOwnPropertyDescriptor = Object.getOwnPropertyDescriptor;
    var ENUMERABLE = "enumerable";
    var CONFIGURABLE = "configurable";
    var WRITABLE = "writable";
    objectDefineProperty.f = DESCRIPTORS ? V8_PROTOTYPE_DEFINE_BUG ? function defineProperty(O, P, Attributes) {
      anObject2(O);
      P = toPropertyKey2(P);
      anObject2(Attributes);
      if (typeof O === "function" && P === "prototype" && "value" in Attributes && WRITABLE in Attributes && !Attributes[WRITABLE]) {
        var current = $getOwnPropertyDescriptor(O, P);
        if (current && current[WRITABLE]) {
          O[P] = Attributes.value;
          Attributes = {
            configurable: CONFIGURABLE in Attributes ? Attributes[CONFIGURABLE] : current[CONFIGURABLE],
            enumerable: ENUMERABLE in Attributes ? Attributes[ENUMERABLE] : current[ENUMERABLE],
            writable: false
          };
        }
      }
      return $defineProperty(O, P, Attributes);
    } : $defineProperty : function defineProperty(O, P, Attributes) {
      anObject2(O);
      P = toPropertyKey2(P);
      anObject2(Attributes);
      if (IE8_DOM_DEFINE) try {
        return $defineProperty(O, P, Attributes);
      } catch (error) {
      }
      if ("get" in Attributes || "set" in Attributes) throw new $TypeError("Accessors not supported");
      if ("value" in Attributes) O[P] = Attributes.value;
      return O;
    };
    return objectDefineProperty;
  }
  var createNonEnumerableProperty;
  var hasRequiredCreateNonEnumerableProperty;
  function requireCreateNonEnumerableProperty() {
    if (hasRequiredCreateNonEnumerableProperty) return createNonEnumerableProperty;
    hasRequiredCreateNonEnumerableProperty = 1;
    var DESCRIPTORS = requireDescriptors();
    var definePropertyModule = requireObjectDefineProperty();
    var createPropertyDescriptor2 = requireCreatePropertyDescriptor();
    createNonEnumerableProperty = DESCRIPTORS ? function(object, key, value) {
      return definePropertyModule.f(object, key, createPropertyDescriptor2(1, value));
    } : function(object, key, value) {
      object[key] = value;
      return object;
    };
    return createNonEnumerableProperty;
  }
  var makeBuiltIn = { exports: {} };
  var functionName;
  var hasRequiredFunctionName;
  function requireFunctionName() {
    if (hasRequiredFunctionName) return functionName;
    hasRequiredFunctionName = 1;
    var DESCRIPTORS = requireDescriptors();
    var hasOwn = requireHasOwnProperty();
    var FunctionPrototype = Function.prototype;
    var getDescriptor = DESCRIPTORS && Object.getOwnPropertyDescriptor;
    var EXISTS = hasOwn(FunctionPrototype, "name");
    var PROPER = EXISTS && (function something() {
    }).name === "something";
    var CONFIGURABLE = EXISTS && (!DESCRIPTORS || DESCRIPTORS && getDescriptor(FunctionPrototype, "name").configurable);
    functionName = {
      EXISTS,
      PROPER,
      CONFIGURABLE
    };
    return functionName;
  }
  var inspectSource;
  var hasRequiredInspectSource;
  function requireInspectSource() {
    if (hasRequiredInspectSource) return inspectSource;
    hasRequiredInspectSource = 1;
    var uncurryThis = requireFunctionUncurryThis();
    var isCallable2 = requireIsCallable();
    var store = requireSharedStore();
    var functionToString = uncurryThis(Function.toString);
    if (!isCallable2(store.inspectSource)) {
      store.inspectSource = function(it) {
        return functionToString(it);
      };
    }
    inspectSource = store.inspectSource;
    return inspectSource;
  }
  var weakMapBasicDetection;
  var hasRequiredWeakMapBasicDetection;
  function requireWeakMapBasicDetection() {
    if (hasRequiredWeakMapBasicDetection) return weakMapBasicDetection;
    hasRequiredWeakMapBasicDetection = 1;
    var globalThis2 = requireGlobalThis();
    var isCallable2 = requireIsCallable();
    var WeakMap = globalThis2.WeakMap;
    weakMapBasicDetection = isCallable2(WeakMap) && /native code/.test(String(WeakMap));
    return weakMapBasicDetection;
  }
  var sharedKey;
  var hasRequiredSharedKey;
  function requireSharedKey() {
    if (hasRequiredSharedKey) return sharedKey;
    hasRequiredSharedKey = 1;
    var shared2 = requireShared();
    var uid2 = requireUid();
    var keys2 = shared2("keys");
    sharedKey = function(key) {
      return keys2[key] || (keys2[key] = uid2(key));
    };
    return sharedKey;
  }
  var hiddenKeys;
  var hasRequiredHiddenKeys;
  function requireHiddenKeys() {
    if (hasRequiredHiddenKeys) return hiddenKeys;
    hasRequiredHiddenKeys = 1;
    hiddenKeys = {};
    return hiddenKeys;
  }
  var internalState;
  var hasRequiredInternalState;
  function requireInternalState() {
    if (hasRequiredInternalState) return internalState;
    hasRequiredInternalState = 1;
    var NATIVE_WEAK_MAP = requireWeakMapBasicDetection();
    var globalThis2 = requireGlobalThis();
    var isObject2 = requireIsObject();
    var createNonEnumerableProperty2 = requireCreateNonEnumerableProperty();
    var hasOwn = requireHasOwnProperty();
    var shared2 = requireSharedStore();
    var sharedKey2 = requireSharedKey();
    var hiddenKeys2 = requireHiddenKeys();
    var OBJECT_ALREADY_INITIALIZED = "Object already initialized";
    var TypeError2 = globalThis2.TypeError;
    var WeakMap = globalThis2.WeakMap;
    var set, get, has;
    var enforce = function(it) {
      return has(it) ? get(it) : set(it, {});
    };
    var getterFor = function(TYPE) {
      return function(it) {
        var state;
        if (!isObject2(it) || (state = get(it)).type !== TYPE) {
          throw new TypeError2("Incompatible receiver, " + TYPE + " required");
        }
        return state;
      };
    };
    if (NATIVE_WEAK_MAP || shared2.state) {
      var store = shared2.state || (shared2.state = new WeakMap());
      store.get = store.get;
      store.has = store.has;
      store.set = store.set;
      set = function(it, metadata) {
        if (store.has(it)) throw new TypeError2(OBJECT_ALREADY_INITIALIZED);
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
      var STATE = sharedKey2("state");
      hiddenKeys2[STATE] = true;
      set = function(it, metadata) {
        if (hasOwn(it, STATE)) throw new TypeError2(OBJECT_ALREADY_INITIALIZED);
        metadata.facade = it;
        createNonEnumerableProperty2(it, STATE, metadata);
        return metadata;
      };
      get = function(it) {
        return hasOwn(it, STATE) ? it[STATE] : {};
      };
      has = function(it) {
        return hasOwn(it, STATE);
      };
    }
    internalState = {
      set,
      get,
      has,
      enforce,
      getterFor
    };
    return internalState;
  }
  var hasRequiredMakeBuiltIn;
  function requireMakeBuiltIn() {
    if (hasRequiredMakeBuiltIn) return makeBuiltIn.exports;
    hasRequiredMakeBuiltIn = 1;
    var uncurryThis = requireFunctionUncurryThis();
    var fails2 = requireFails();
    var isCallable2 = requireIsCallable();
    var hasOwn = requireHasOwnProperty();
    var DESCRIPTORS = requireDescriptors();
    var CONFIGURABLE_FUNCTION_NAME = requireFunctionName().CONFIGURABLE;
    var inspectSource2 = requireInspectSource();
    var InternalStateModule = requireInternalState();
    var enforceInternalState = InternalStateModule.enforce;
    var getInternalState = InternalStateModule.get;
    var $String = String;
    var defineProperty = Object.defineProperty;
    var stringSlice = uncurryThis("".slice);
    var replace = uncurryThis("".replace);
    var join = uncurryThis([].join);
    var CONFIGURABLE_LENGTH = DESCRIPTORS && !fails2(function() {
      return defineProperty(function() {
      }, "length", { value: 8 }).length !== 8;
    });
    var TEMPLATE = String(String).split("String");
    var makeBuiltIn$1 = makeBuiltIn.exports = function(value, name2, options) {
      if (stringSlice($String(name2), 0, 7) === "Symbol(") {
        name2 = "[" + replace($String(name2), /^Symbol\(([^)]*)\).*$/, "$1") + "]";
      }
      if (options && options.getter) name2 = "get " + name2;
      if (options && options.setter) name2 = "set " + name2;
      if (!hasOwn(value, "name") || CONFIGURABLE_FUNCTION_NAME && value.name !== name2) {
        if (DESCRIPTORS) defineProperty(value, "name", { value: name2, configurable: true });
        else value.name = name2;
      }
      if (CONFIGURABLE_LENGTH && options && hasOwn(options, "arity") && value.length !== options.arity) {
        defineProperty(value, "length", { value: options.arity });
      }
      try {
        if (options && hasOwn(options, "constructor") && options.constructor) {
          if (DESCRIPTORS) defineProperty(value, "prototype", { writable: false });
        } else if (value.prototype) value.prototype = void 0;
      } catch (error) {
      }
      var state = enforceInternalState(value);
      if (!hasOwn(state, "source")) {
        state.source = join(TEMPLATE, typeof name2 == "string" ? name2 : "");
      }
      return value;
    };
    Function.prototype.toString = makeBuiltIn$1(function toString2() {
      return isCallable2(this) && getInternalState(this).source || inspectSource2(this);
    }, "toString");
    return makeBuiltIn.exports;
  }
  var defineBuiltIn;
  var hasRequiredDefineBuiltIn;
  function requireDefineBuiltIn() {
    if (hasRequiredDefineBuiltIn) return defineBuiltIn;
    hasRequiredDefineBuiltIn = 1;
    var isCallable2 = requireIsCallable();
    var definePropertyModule = requireObjectDefineProperty();
    var makeBuiltIn2 = requireMakeBuiltIn();
    var defineGlobalProperty2 = requireDefineGlobalProperty();
    defineBuiltIn = function(O, key, value, options) {
      if (!options) options = {};
      var simple = options.enumerable;
      var name2 = options.name !== void 0 ? options.name : key;
      if (isCallable2(value)) makeBuiltIn2(value, name2, options);
      if (options.global) {
        if (simple) O[key] = value;
        else defineGlobalProperty2(key, value);
      } else {
        try {
          if (!options.unsafe) delete O[key];
          else if (O[key]) simple = true;
        } catch (error) {
        }
        if (simple) O[key] = value;
        else definePropertyModule.f(O, key, {
          value,
          enumerable: false,
          configurable: !options.nonConfigurable,
          writable: !options.nonWritable
        });
      }
      return O;
    };
    return defineBuiltIn;
  }
  var objectGetOwnPropertyNames = {};
  var mathTrunc;
  var hasRequiredMathTrunc;
  function requireMathTrunc() {
    if (hasRequiredMathTrunc) return mathTrunc;
    hasRequiredMathTrunc = 1;
    var ceil = Math.ceil;
    var floor = Math.floor;
    mathTrunc = Math.trunc || function trunc(x) {
      var n = +x;
      return (n > 0 ? floor : ceil)(n);
    };
    return mathTrunc;
  }
  var toIntegerOrInfinity;
  var hasRequiredToIntegerOrInfinity;
  function requireToIntegerOrInfinity() {
    if (hasRequiredToIntegerOrInfinity) return toIntegerOrInfinity;
    hasRequiredToIntegerOrInfinity = 1;
    var trunc = requireMathTrunc();
    toIntegerOrInfinity = function(argument) {
      var number = +argument;
      return number !== number || number === 0 ? 0 : trunc(number);
    };
    return toIntegerOrInfinity;
  }
  var toAbsoluteIndex;
  var hasRequiredToAbsoluteIndex;
  function requireToAbsoluteIndex() {
    if (hasRequiredToAbsoluteIndex) return toAbsoluteIndex;
    hasRequiredToAbsoluteIndex = 1;
    var toIntegerOrInfinity2 = requireToIntegerOrInfinity();
    var max = Math.max;
    var min = Math.min;
    toAbsoluteIndex = function(index, length) {
      var integer = toIntegerOrInfinity2(index);
      return integer < 0 ? max(integer + length, 0) : min(integer, length);
    };
    return toAbsoluteIndex;
  }
  var toLength;
  var hasRequiredToLength;
  function requireToLength() {
    if (hasRequiredToLength) return toLength;
    hasRequiredToLength = 1;
    var toIntegerOrInfinity2 = requireToIntegerOrInfinity();
    var min = Math.min;
    toLength = function(argument) {
      var len = toIntegerOrInfinity2(argument);
      return len > 0 ? min(len, 9007199254740991) : 0;
    };
    return toLength;
  }
  var lengthOfArrayLike;
  var hasRequiredLengthOfArrayLike;
  function requireLengthOfArrayLike() {
    if (hasRequiredLengthOfArrayLike) return lengthOfArrayLike;
    hasRequiredLengthOfArrayLike = 1;
    var toLength2 = requireToLength();
    lengthOfArrayLike = function(obj) {
      return toLength2(obj.length);
    };
    return lengthOfArrayLike;
  }
  var arrayIncludes;
  var hasRequiredArrayIncludes;
  function requireArrayIncludes() {
    if (hasRequiredArrayIncludes) return arrayIncludes;
    hasRequiredArrayIncludes = 1;
    var toIndexedObject2 = requireToIndexedObject();
    var toAbsoluteIndex2 = requireToAbsoluteIndex();
    var lengthOfArrayLike2 = requireLengthOfArrayLike();
    var createMethod = function(IS_INCLUDES) {
      return function($this, el, fromIndex) {
        var O = toIndexedObject2($this);
        var length = lengthOfArrayLike2(O);
        if (length === 0) return !IS_INCLUDES && -1;
        var index = toAbsoluteIndex2(fromIndex, length);
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
  function requireObjectKeysInternal() {
    if (hasRequiredObjectKeysInternal) return objectKeysInternal;
    hasRequiredObjectKeysInternal = 1;
    var uncurryThis = requireFunctionUncurryThis();
    var hasOwn = requireHasOwnProperty();
    var toIndexedObject2 = requireToIndexedObject();
    var indexOf = requireArrayIncludes().indexOf;
    var hiddenKeys2 = requireHiddenKeys();
    var push = uncurryThis([].push);
    objectKeysInternal = function(object, names) {
      var O = toIndexedObject2(object);
      var i = 0;
      var result = [];
      var key;
      for (key in O) !hasOwn(hiddenKeys2, key) && hasOwn(O, key) && push(result, key);
      while (names.length > i) if (hasOwn(O, key = names[i++])) {
        ~indexOf(result, key) || push(result, key);
      }
      return result;
    };
    return objectKeysInternal;
  }
  var enumBugKeys;
  var hasRequiredEnumBugKeys;
  function requireEnumBugKeys() {
    if (hasRequiredEnumBugKeys) return enumBugKeys;
    hasRequiredEnumBugKeys = 1;
    enumBugKeys = [
      "constructor",
      "hasOwnProperty",
      "isPrototypeOf",
      "propertyIsEnumerable",
      "toLocaleString",
      "toString",
      "valueOf"
    ];
    return enumBugKeys;
  }
  var hasRequiredObjectGetOwnPropertyNames;
  function requireObjectGetOwnPropertyNames() {
    if (hasRequiredObjectGetOwnPropertyNames) return objectGetOwnPropertyNames;
    hasRequiredObjectGetOwnPropertyNames = 1;
    var internalObjectKeys = requireObjectKeysInternal();
    var enumBugKeys2 = requireEnumBugKeys();
    var hiddenKeys2 = enumBugKeys2.concat("length", "prototype");
    objectGetOwnPropertyNames.f = Object.getOwnPropertyNames || function getOwnPropertyNames(O) {
      return internalObjectKeys(O, hiddenKeys2);
    };
    return objectGetOwnPropertyNames;
  }
  var objectGetOwnPropertySymbols = {};
  var hasRequiredObjectGetOwnPropertySymbols;
  function requireObjectGetOwnPropertySymbols() {
    if (hasRequiredObjectGetOwnPropertySymbols) return objectGetOwnPropertySymbols;
    hasRequiredObjectGetOwnPropertySymbols = 1;
    objectGetOwnPropertySymbols.f = Object.getOwnPropertySymbols;
    return objectGetOwnPropertySymbols;
  }
  var ownKeys;
  var hasRequiredOwnKeys;
  function requireOwnKeys() {
    if (hasRequiredOwnKeys) return ownKeys;
    hasRequiredOwnKeys = 1;
    var getBuiltIn2 = requireGetBuiltIn();
    var uncurryThis = requireFunctionUncurryThis();
    var getOwnPropertyNamesModule = requireObjectGetOwnPropertyNames();
    var getOwnPropertySymbolsModule = requireObjectGetOwnPropertySymbols();
    var anObject2 = requireAnObject();
    var concat = uncurryThis([].concat);
    ownKeys = getBuiltIn2("Reflect", "ownKeys") || function ownKeys2(it) {
      var keys2 = getOwnPropertyNamesModule.f(anObject2(it));
      var getOwnPropertySymbols = getOwnPropertySymbolsModule.f;
      return getOwnPropertySymbols ? concat(keys2, getOwnPropertySymbols(it)) : keys2;
    };
    return ownKeys;
  }
  var copyConstructorProperties;
  var hasRequiredCopyConstructorProperties;
  function requireCopyConstructorProperties() {
    if (hasRequiredCopyConstructorProperties) return copyConstructorProperties;
    hasRequiredCopyConstructorProperties = 1;
    var hasOwn = requireHasOwnProperty();
    var ownKeys2 = requireOwnKeys();
    var getOwnPropertyDescriptorModule = requireObjectGetOwnPropertyDescriptor();
    var definePropertyModule = requireObjectDefineProperty();
    copyConstructorProperties = function(target, source, exceptions) {
      var keys2 = ownKeys2(source);
      var defineProperty = definePropertyModule.f;
      var getOwnPropertyDescriptor = getOwnPropertyDescriptorModule.f;
      for (var i = 0; i < keys2.length; i++) {
        var key = keys2[i];
        if (!hasOwn(target, key) && !(exceptions && hasOwn(exceptions, key))) {
          defineProperty(target, key, getOwnPropertyDescriptor(source, key));
        }
      }
    };
    return copyConstructorProperties;
  }
  var isForced_1;
  var hasRequiredIsForced;
  function requireIsForced() {
    if (hasRequiredIsForced) return isForced_1;
    hasRequiredIsForced = 1;
    var fails2 = requireFails();
    var isCallable2 = requireIsCallable();
    var replacement = /#|\.prototype\./;
    var isForced = function(feature, detection) {
      var value = data2[normalize(feature)];
      return value === POLYFILL ? true : value === NATIVE ? false : isCallable2(detection) ? fails2(detection) : !!detection;
    };
    var normalize = isForced.normalize = function(string) {
      return String(string).replace(replacement, ".").toLowerCase();
    };
    var data2 = isForced.data = {};
    var NATIVE = isForced.NATIVE = "N";
    var POLYFILL = isForced.POLYFILL = "P";
    isForced_1 = isForced;
    return isForced_1;
  }
  var _export;
  var hasRequired_export;
  function require_export() {
    if (hasRequired_export) return _export;
    hasRequired_export = 1;
    var globalThis2 = requireGlobalThis();
    var getOwnPropertyDescriptor = requireObjectGetOwnPropertyDescriptor().f;
    var createNonEnumerableProperty2 = requireCreateNonEnumerableProperty();
    var defineBuiltIn2 = requireDefineBuiltIn();
    var defineGlobalProperty2 = requireDefineGlobalProperty();
    var copyConstructorProperties2 = requireCopyConstructorProperties();
    var isForced = requireIsForced();
    _export = function(options, source) {
      var TARGET = options.target;
      var GLOBAL = options.global;
      var STATIC = options.stat;
      var FORCED, target, key, targetProperty, sourceProperty, descriptor;
      if (GLOBAL) {
        target = globalThis2;
      } else if (STATIC) {
        target = globalThis2[TARGET] || defineGlobalProperty2(TARGET, {});
      } else {
        target = globalThis2[TARGET] && globalThis2[TARGET].prototype;
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
          copyConstructorProperties2(sourceProperty, targetProperty);
        }
        if (options.sham || targetProperty && targetProperty.sham) {
          createNonEnumerableProperty2(sourceProperty, "sham", true);
        }
        defineBuiltIn2(target, key, sourceProperty, options);
      }
    };
    return _export;
  }
  var objectDefineProperties = {};
  var objectKeys;
  var hasRequiredObjectKeys;
  function requireObjectKeys() {
    if (hasRequiredObjectKeys) return objectKeys;
    hasRequiredObjectKeys = 1;
    var internalObjectKeys = requireObjectKeysInternal();
    var enumBugKeys2 = requireEnumBugKeys();
    objectKeys = Object.keys || function keys2(O) {
      return internalObjectKeys(O, enumBugKeys2);
    };
    return objectKeys;
  }
  var hasRequiredObjectDefineProperties;
  function requireObjectDefineProperties() {
    if (hasRequiredObjectDefineProperties) return objectDefineProperties;
    hasRequiredObjectDefineProperties = 1;
    var DESCRIPTORS = requireDescriptors();
    var V8_PROTOTYPE_DEFINE_BUG = requireV8PrototypeDefineBug();
    var definePropertyModule = requireObjectDefineProperty();
    var anObject2 = requireAnObject();
    var toIndexedObject2 = requireToIndexedObject();
    var objectKeys2 = requireObjectKeys();
    objectDefineProperties.f = DESCRIPTORS && !V8_PROTOTYPE_DEFINE_BUG ? Object.defineProperties : function defineProperties(O, Properties) {
      anObject2(O);
      var props = toIndexedObject2(Properties);
      var keys2 = objectKeys2(Properties);
      var length = keys2.length;
      var index = 0;
      var key;
      while (length > index) definePropertyModule.f(O, key = keys2[index++], props[key]);
      return O;
    };
    return objectDefineProperties;
  }
  var html;
  var hasRequiredHtml;
  function requireHtml() {
    if (hasRequiredHtml) return html;
    hasRequiredHtml = 1;
    var getBuiltIn2 = requireGetBuiltIn();
    html = getBuiltIn2("document", "documentElement");
    return html;
  }
  var objectCreate;
  var hasRequiredObjectCreate;
  function requireObjectCreate() {
    if (hasRequiredObjectCreate) return objectCreate;
    hasRequiredObjectCreate = 1;
    var anObject2 = requireAnObject();
    var definePropertiesModule = requireObjectDefineProperties();
    var enumBugKeys2 = requireEnumBugKeys();
    var hiddenKeys2 = requireHiddenKeys();
    var html2 = requireHtml();
    var documentCreateElement2 = requireDocumentCreateElement();
    var sharedKey2 = requireSharedKey();
    var GT = ">";
    var LT = "<";
    var PROTOTYPE = "prototype";
    var SCRIPT = "script";
    var IE_PROTO = sharedKey2("IE_PROTO");
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
      var iframe = documentCreateElement2("iframe");
      var JS = "java" + SCRIPT + ":";
      var iframeDocument;
      iframe.style.display = "none";
      html2.appendChild(iframe);
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
      var length = enumBugKeys2.length;
      while (length--) delete NullProtoObject[PROTOTYPE][enumBugKeys2[length]];
      return NullProtoObject();
    };
    hiddenKeys2[IE_PROTO] = true;
    objectCreate = Object.create || function create(O, Properties) {
      var result;
      if (O !== null) {
        EmptyConstructor[PROTOTYPE] = anObject2(O);
        result = new EmptyConstructor();
        EmptyConstructor[PROTOTYPE] = null;
        result[IE_PROTO] = O;
      } else result = NullProtoObject();
      return Properties === void 0 ? result : definePropertiesModule.f(result, Properties);
    };
    return objectCreate;
  }
  var addToUnscopables;
  var hasRequiredAddToUnscopables;
  function requireAddToUnscopables() {
    if (hasRequiredAddToUnscopables) return addToUnscopables;
    hasRequiredAddToUnscopables = 1;
    var wellKnownSymbol2 = requireWellKnownSymbol();
    var create = requireObjectCreate();
    var defineProperty = requireObjectDefineProperty().f;
    var UNSCOPABLES = wellKnownSymbol2("unscopables");
    var ArrayPrototype = Array.prototype;
    if (ArrayPrototype[UNSCOPABLES] === void 0) {
      defineProperty(ArrayPrototype, UNSCOPABLES, {
        configurable: true,
        value: create(null)
      });
    }
    addToUnscopables = function(key) {
      ArrayPrototype[UNSCOPABLES][key] = true;
    };
    return addToUnscopables;
  }
  var hasRequiredEs_array_at;
  function requireEs_array_at() {
    if (hasRequiredEs_array_at) return es_array_at;
    hasRequiredEs_array_at = 1;
    var $ = require_export();
    var toObject2 = requireToObject();
    var lengthOfArrayLike2 = requireLengthOfArrayLike();
    var toIntegerOrInfinity2 = requireToIntegerOrInfinity();
    var addToUnscopables2 = requireAddToUnscopables();
    $({ target: "Array", proto: true }, {
      at: function at(index) {
        var O = toObject2(this);
        var len = lengthOfArrayLike2(O);
        var relativeIndex = toIntegerOrInfinity2(index);
        var k = relativeIndex >= 0 ? relativeIndex : len + relativeIndex;
        return k < 0 || k >= len ? void 0 : O[k];
      }
    });
    addToUnscopables2("at");
    return es_array_at;
  }
  requireEs_array_at();
  var es_object_hasOwn = {};
  var hasRequiredEs_object_hasOwn;
  function requireEs_object_hasOwn() {
    if (hasRequiredEs_object_hasOwn) return es_object_hasOwn;
    hasRequiredEs_object_hasOwn = 1;
    var $ = require_export();
    var hasOwn = requireHasOwnProperty();
    $({ target: "Object", stat: true }, {
      hasOwn
    });
    return es_object_hasOwn;
  }
  requireEs_object_hasOwn();
  var es_string_replaceAll = {};
  var isRegexp;
  var hasRequiredIsRegexp;
  function requireIsRegexp() {
    if (hasRequiredIsRegexp) return isRegexp;
    hasRequiredIsRegexp = 1;
    var isObject2 = requireIsObject();
    var classof2 = requireClassofRaw();
    var wellKnownSymbol2 = requireWellKnownSymbol();
    var MATCH = wellKnownSymbol2("match");
    isRegexp = function(it) {
      var isRegExp;
      return isObject2(it) && ((isRegExp = it[MATCH]) !== void 0 ? !!isRegExp : classof2(it) === "RegExp");
    };
    return isRegexp;
  }
  var toStringTagSupport;
  var hasRequiredToStringTagSupport;
  function requireToStringTagSupport() {
    if (hasRequiredToStringTagSupport) return toStringTagSupport;
    hasRequiredToStringTagSupport = 1;
    var wellKnownSymbol2 = requireWellKnownSymbol();
    var TO_STRING_TAG = wellKnownSymbol2("toStringTag");
    var test = {};
    test[TO_STRING_TAG] = "z";
    toStringTagSupport = String(test) === "[object z]";
    return toStringTagSupport;
  }
  var classof;
  var hasRequiredClassof;
  function requireClassof() {
    if (hasRequiredClassof) return classof;
    hasRequiredClassof = 1;
    var TO_STRING_TAG_SUPPORT = requireToStringTagSupport();
    var isCallable2 = requireIsCallable();
    var classofRaw2 = requireClassofRaw();
    var wellKnownSymbol2 = requireWellKnownSymbol();
    var TO_STRING_TAG = wellKnownSymbol2("toStringTag");
    var $Object = Object;
    var CORRECT_ARGUMENTS = classofRaw2(/* @__PURE__ */ (function() {
      return arguments;
    })()) === "Arguments";
    var tryGet = function(it, key) {
      try {
        return it[key];
      } catch (error) {
      }
    };
    classof = TO_STRING_TAG_SUPPORT ? classofRaw2 : function(it) {
      var O, tag, result;
      return it === void 0 ? "Undefined" : it === null ? "Null" : typeof (tag = tryGet(O = $Object(it), TO_STRING_TAG)) == "string" ? tag : CORRECT_ARGUMENTS ? classofRaw2(O) : (result = classofRaw2(O)) === "Object" && isCallable2(O.callee) ? "Arguments" : result;
    };
    return classof;
  }
  var toString;
  var hasRequiredToString;
  function requireToString() {
    if (hasRequiredToString) return toString;
    hasRequiredToString = 1;
    var classof2 = requireClassof();
    var $String = String;
    toString = function(argument) {
      if (classof2(argument) === "Symbol") throw new TypeError("Cannot convert a Symbol value to a string");
      return $String(argument);
    };
    return toString;
  }
  var regexpFlagsDetection;
  var hasRequiredRegexpFlagsDetection;
  function requireRegexpFlagsDetection() {
    if (hasRequiredRegexpFlagsDetection) return regexpFlagsDetection;
    hasRequiredRegexpFlagsDetection = 1;
    var globalThis2 = requireGlobalThis();
    var fails2 = requireFails();
    var RegExp2 = globalThis2.RegExp;
    var FLAGS_GETTER_IS_CORRECT = !fails2(function() {
      var INDICES_SUPPORT = true;
      try {
        RegExp2(".", "d");
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
      var result = Object.getOwnPropertyDescriptor(RegExp2.prototype, "flags").get.call(O);
      return result !== expected || calls !== expected;
    });
    regexpFlagsDetection = { correct: FLAGS_GETTER_IS_CORRECT };
    return regexpFlagsDetection;
  }
  var regexpFlags;
  var hasRequiredRegexpFlags;
  function requireRegexpFlags() {
    if (hasRequiredRegexpFlags) return regexpFlags;
    hasRequiredRegexpFlags = 1;
    var anObject2 = requireAnObject();
    regexpFlags = function() {
      var that = anObject2(this);
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
    return regexpFlags;
  }
  var regexpGetFlags;
  var hasRequiredRegexpGetFlags;
  function requireRegexpGetFlags() {
    if (hasRequiredRegexpGetFlags) return regexpGetFlags;
    hasRequiredRegexpGetFlags = 1;
    var call = requireFunctionCall();
    var hasOwn = requireHasOwnProperty();
    var isPrototypeOf = requireObjectIsPrototypeOf();
    var regExpFlagsDetection = requireRegexpFlagsDetection();
    var regExpFlagsGetterImplementation = requireRegexpFlags();
    var RegExpPrototype = RegExp.prototype;
    regexpGetFlags = regExpFlagsDetection.correct ? function(it) {
      return it.flags;
    } : function(it) {
      return !regExpFlagsDetection.correct && isPrototypeOf(RegExpPrototype, it) && !hasOwn(it, "flags") ? call(regExpFlagsGetterImplementation, it) : it.flags;
    };
    return regexpGetFlags;
  }
  var getSubstitution;
  var hasRequiredGetSubstitution;
  function requireGetSubstitution() {
    if (hasRequiredGetSubstitution) return getSubstitution;
    hasRequiredGetSubstitution = 1;
    var uncurryThis = requireFunctionUncurryThis();
    var toObject2 = requireToObject();
    var floor = Math.floor;
    var charAt = uncurryThis("".charAt);
    var replace = uncurryThis("".replace);
    var stringSlice = uncurryThis("".slice);
    var SUBSTITUTION_SYMBOLS = /\$([$&'`]|\d{1,2}|<[^>]*>)/g;
    var SUBSTITUTION_SYMBOLS_NO_NAMED = /\$([$&'`]|\d{1,2})/g;
    getSubstitution = function(matched, str, position, captures, namedCaptures, replacement) {
      var tailPos = position + matched.length;
      var m = captures.length;
      var symbols = SUBSTITUTION_SYMBOLS_NO_NAMED;
      if (namedCaptures !== void 0) {
        namedCaptures = toObject2(namedCaptures);
        symbols = SUBSTITUTION_SYMBOLS;
      }
      return replace(replacement, symbols, function(match, ch) {
        var capture;
        switch (charAt(ch, 0)) {
          case "$":
            return "$";
          case "&":
            return matched;
          case "`":
            return stringSlice(str, 0, position);
          case "'":
            return stringSlice(str, tailPos);
          case "<":
            capture = namedCaptures[stringSlice(ch, 1, -1)];
            break;
          default:
            var n = +ch;
            if (n === 0) return match;
            if (n > m) {
              var f = floor(n / 10);
              if (f === 0) return match;
              if (f <= m) return captures[f - 1] === void 0 ? charAt(ch, 1) : captures[f - 1] + charAt(ch, 1);
              return match;
            }
            capture = captures[n - 1];
        }
        return capture === void 0 ? "" : capture;
      });
    };
    return getSubstitution;
  }
  var hasRequiredEs_string_replaceAll;
  function requireEs_string_replaceAll() {
    if (hasRequiredEs_string_replaceAll) return es_string_replaceAll;
    hasRequiredEs_string_replaceAll = 1;
    var $ = require_export();
    var call = requireFunctionCall();
    var uncurryThis = requireFunctionUncurryThis();
    var requireObjectCoercible2 = requireRequireObjectCoercible();
    var isCallable2 = requireIsCallable();
    var isObject2 = requireIsObject();
    var isRegExp = requireIsRegexp();
    var toString2 = requireToString();
    var getMethod2 = requireGetMethod();
    var getRegExpFlags = requireRegexpGetFlags();
    var getSubstitution2 = requireGetSubstitution();
    var wellKnownSymbol2 = requireWellKnownSymbol();
    var IS_PURE = requireIsPure();
    var REPLACE = wellKnownSymbol2("replace");
    var $TypeError = TypeError;
    var indexOf = uncurryThis("".indexOf);
    var replace = uncurryThis("".replace);
    var stringSlice = uncurryThis("".slice);
    var max = Math.max;
    $({ target: "String", proto: true }, {
      replaceAll: function replaceAll(searchValue, replaceValue) {
        var O = requireObjectCoercible2(this);
        var IS_REG_EXP, flags, replacer, string, searchString, functionalReplace, searchLength, advanceBy, position, replacement;
        var endOfLastMatch = 0;
        var result = "";
        if (isObject2(searchValue)) {
          IS_REG_EXP = isRegExp(searchValue);
          if (IS_REG_EXP) {
            flags = toString2(requireObjectCoercible2(getRegExpFlags(searchValue)));
            if (!~indexOf(flags, "g")) throw new $TypeError("`.replaceAll` does not allow non-global regexes");
          }
          replacer = getMethod2(searchValue, REPLACE);
          if (replacer) return call(replacer, searchValue, O, replaceValue);
          if (IS_PURE && IS_REG_EXP) return replace(toString2(O), searchValue, replaceValue);
        }
        string = toString2(O);
        searchString = toString2(searchValue);
        functionalReplace = isCallable2(replaceValue);
        if (!functionalReplace) replaceValue = toString2(replaceValue);
        searchLength = searchString.length;
        advanceBy = max(1, searchLength);
        position = indexOf(string, searchString);
        while (position !== -1) {
          replacement = functionalReplace ? toString2(replaceValue(searchString, position, string)) : getSubstitution2(searchString, string, position, [], void 0, replaceValue);
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
  const name = "corejs";
  const items = [1, 2, 3];
  const data = {
    name,
    items
  };
  var describe = {};
  var keys;
  var hasRequiredKeys;
  function requireKeys() {
    if (hasRequiredKeys) return keys;
    hasRequiredKeys = 1;
    requireEs_string_replaceAll();
    console.log("eval keys");
    keys = function keys2(value) {
      return Object.keys(value).map(function(key) {
        return key.replaceAll("a", "A");
      });
    };
    return keys;
  }
  var hasRequiredDescribe;
  function requireDescribe() {
    if (hasRequiredDescribe) return describe;
    hasRequiredDescribe = 1;
    requireEs_object_hasOwn();
    var keys2 = requireKeys();
    console.log("eval describe");
    describe.describe = function(value) {
      return keys2(value).join("|") + " " + Object.hasOwn(value, "items");
    };
    return describe;
  }
  var describeExports = requireDescribe();
  console.log("eval index");
  console.log("at " + data.items.at(-1) + " hasOwn " + Object.hasOwn(data, "name"));
  console.log("replaceAll " + "a-b-c".replaceAll("-", "+"));
  console.log("describe " + describeExports.describe(data));
})();
