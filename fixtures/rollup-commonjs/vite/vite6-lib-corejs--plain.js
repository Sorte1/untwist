var app = (function() {
  "use strict";
  var commonjsGlobal = typeof globalThis !== "undefined" ? globalThis : typeof window !== "undefined" ? window : typeof global !== "undefined" ? global : typeof self !== "undefined" ? self : {};
  function getDefaultExportFromCjs(x) {
    return x && x.__esModule && Object.prototype.hasOwnProperty.call(x, "default") ? x["default"] : x;
  }
  var corejs = {};
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
  var objectGetOwnPropertyNames = {};
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
  var classofRaw;
  var hasRequiredClassofRaw;
  function requireClassofRaw() {
    if (hasRequiredClassofRaw) return classofRaw;
    hasRequiredClassofRaw = 1;
    var uncurryThis = requireFunctionUncurryThis();
    var toString = uncurryThis({}.toString);
    var stringSlice = uncurryThis("".slice);
    classofRaw = function(it) {
      return stringSlice(toString(it), 8, -1);
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
    var classof = requireClassofRaw();
    var $Object = Object;
    var split = uncurryThis("".split);
    indexedObject = fails2(function() {
      return !$Object("z").propertyIsEnumerable(0);
    }) ? function(it) {
      return classof(it) === "String" ? split(it, "") : $Object(it);
    } : $Object;
    return indexedObject;
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
    toAbsoluteIndex = function(index2, length) {
      var integer = toIntegerOrInfinity2(index2);
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
        var index2 = toAbsoluteIndex2(fromIndex, length);
        var value;
        if (IS_INCLUDES && el !== el) while (length > index2) {
          value = O[index2++];
          if (value !== value) return true;
        }
        else for (; length > index2; index2++) {
          if ((IS_INCLUDES || index2 in O) && O[index2] === el) return IS_INCLUDES || index2 || 0;
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
  var hiddenKeys;
  var hasRequiredHiddenKeys;
  function requireHiddenKeys() {
    if (hasRequiredHiddenKeys) return hiddenKeys;
    hasRequiredHiddenKeys = 1;
    hiddenKeys = {};
    return hiddenKeys;
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
      var keys = getOwnPropertyNamesModule.f(anObject2(it));
      var getOwnPropertySymbols = getOwnPropertySymbolsModule.f;
      return getOwnPropertySymbols ? concat(keys, getOwnPropertySymbols(it)) : keys;
    };
    return ownKeys;
  }
  var hasRequiredCorejs;
  function requireCorejs() {
    if (hasRequiredCorejs) return corejs;
    hasRequiredCorejs = 1;
    var ownKeys2 = requireOwnKeys();
    var getBuiltIn2 = requireGetBuiltIn();
    var keys = ownKeys2({ a: 1, b: 2 });
    console.log("own-keys " + keys.join(","));
    console.log("built-in " + typeof getBuiltIn2("Reflect", "ownKeys") + " " + typeof getBuiltIn2("Array"));
    return corejs;
  }
  var corejsExports = requireCorejs();
  const index = /* @__PURE__ */ getDefaultExportFromCjs(corejsExports);
  return index;
})();
