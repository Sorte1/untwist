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
var documentAll = typeof document == "object" && document.all;
var isCallable$2 = typeof documentAll == "undefined" && documentAll !== void 0 ? function(argument) {
  return typeof argument == "function" || argument === documentAll;
} : function(argument) {
  return typeof argument == "function";
};
var globalThis$1 = globalThis_1;
var isCallable$1 = isCallable$2;
var aFunction = function(argument) {
  return isCallable$1(argument) ? argument : void 0;
};
var getBuiltIn$2 = function(namespace, method) {
  return arguments.length < 2 ? aFunction(globalThis$1[namespace]) : globalThis$1[namespace] && globalThis$1[namespace][method];
};
var fails$2 = function(exec) {
  try {
    return !!exec();
  } catch (error) {
    return true;
  }
};
var fails$1 = fails$2;
var functionBindNative = !fails$1(function() {
  var test = (function() {
  }).bind();
  return typeof test != "function" || test.hasOwnProperty("prototype");
});
var NATIVE_BIND = functionBindNative;
var FunctionPrototype = Function.prototype;
var call = FunctionPrototype.call;
var uncurryThisWithBind = NATIVE_BIND && FunctionPrototype.bind.bind(call, call);
var functionUncurryThis = NATIVE_BIND ? uncurryThisWithBind : function(fn) {
  return function() {
    return call.apply(fn, arguments);
  };
};
var objectGetOwnPropertyNames = {};
var isNullOrUndefined$1 = function(it) {
  return it === null || it === void 0;
};
var isNullOrUndefined = isNullOrUndefined$1;
var $TypeError$1 = TypeError;
var requireObjectCoercible$2 = function(it) {
  if (isNullOrUndefined(it)) throw new $TypeError$1("Can't call method on " + it);
  return it;
};
var requireObjectCoercible$1 = requireObjectCoercible$2;
var $Object$1 = Object;
var toObject$1 = function(argument) {
  return $Object$1(requireObjectCoercible$1(argument));
};
var uncurryThis$4 = functionUncurryThis;
var toObject = toObject$1;
var hasOwnProperty = uncurryThis$4({}.hasOwnProperty);
var hasOwnProperty_1 = Object.hasOwn || function hasOwn(it, key) {
  return hasOwnProperty(toObject(it), key);
};
var uncurryThis$3 = functionUncurryThis;
var toString = uncurryThis$3({}.toString);
var stringSlice = uncurryThis$3("".slice);
var classofRaw = function(it) {
  return stringSlice(toString(it), 8, -1);
};
var uncurryThis$2 = functionUncurryThis;
var fails = fails$2;
var classof = classofRaw;
var $Object = Object;
var split = uncurryThis$2("".split);
var indexedObject = fails(function() {
  return !$Object("z").propertyIsEnumerable(0);
}) ? function(it) {
  return classof(it) === "String" ? split(it, "") : $Object(it);
} : $Object;
var IndexedObject = indexedObject;
var requireObjectCoercible = requireObjectCoercible$2;
var toIndexedObject$2 = function(it) {
  return IndexedObject(requireObjectCoercible(it));
};
var ceil = Math.ceil;
var floor = Math.floor;
var mathTrunc = Math.trunc || function trunc(x) {
  var n = +x;
  return (n > 0 ? floor : ceil)(n);
};
var trunc2 = mathTrunc;
var toIntegerOrInfinity$2 = function(argument) {
  var number = +argument;
  return number !== number || number === 0 ? 0 : trunc2(number);
};
var toIntegerOrInfinity$1 = toIntegerOrInfinity$2;
var max = Math.max;
var min$1 = Math.min;
var toAbsoluteIndex$1 = function(index, length) {
  var integer = toIntegerOrInfinity$1(index);
  return integer < 0 ? max(integer + length, 0) : min$1(integer, length);
};
var toIntegerOrInfinity = toIntegerOrInfinity$2;
var min = Math.min;
var toLength$1 = function(argument) {
  var len = toIntegerOrInfinity(argument);
  return len > 0 ? min(len, 9007199254740991) : 0;
};
var toLength = toLength$1;
var lengthOfArrayLike$1 = function(obj) {
  return toLength(obj.length);
};
var toIndexedObject$1 = toIndexedObject$2;
var toAbsoluteIndex = toAbsoluteIndex$1;
var lengthOfArrayLike = lengthOfArrayLike$1;
var createMethod = function(IS_INCLUDES) {
  return function($this, el, fromIndex) {
    var O = toIndexedObject$1($this);
    var length = lengthOfArrayLike(O);
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
var hiddenKeys$2 = {};
var uncurryThis$1 = functionUncurryThis;
var hasOwn2 = hasOwnProperty_1;
var toIndexedObject = toIndexedObject$2;
var indexOf = arrayIncludes.indexOf;
var hiddenKeys$1 = hiddenKeys$2;
var push = uncurryThis$1([].push);
var objectKeysInternal = function(object, names) {
  var O = toIndexedObject(object);
  var i = 0;
  var result = [];
  var key;
  for (key in O) !hasOwn2(hiddenKeys$1, key) && hasOwn2(O, key) && push(result, key);
  while (names.length > i) if (hasOwn2(O, key = names[i++])) {
    ~indexOf(result, key) || push(result, key);
  }
  return result;
};
var enumBugKeys$1 = [
  "constructor",
  "hasOwnProperty",
  "isPrototypeOf",
  "propertyIsEnumerable",
  "toLocaleString",
  "toString",
  "valueOf"
];
var internalObjectKeys = objectKeysInternal;
var enumBugKeys = enumBugKeys$1;
var hiddenKeys = enumBugKeys.concat("length", "prototype");
objectGetOwnPropertyNames.f = Object.getOwnPropertyNames || function getOwnPropertyNames(O) {
  return internalObjectKeys(O, hiddenKeys);
};
var objectGetOwnPropertySymbols = {};
objectGetOwnPropertySymbols.f = Object.getOwnPropertySymbols;
var isCallable = isCallable$2;
var isObject$1 = function(it) {
  return typeof it == "object" ? it !== null : isCallable(it);
};
var isObject = isObject$1;
var $String = String;
var $TypeError = TypeError;
var anObject$1 = function(argument) {
  if (isObject(argument)) return argument;
  throw new $TypeError($String(argument) + " is not an object");
};
var getBuiltIn$1 = getBuiltIn$2;
var uncurryThis = functionUncurryThis;
var getOwnPropertyNamesModule = objectGetOwnPropertyNames;
var getOwnPropertySymbolsModule = objectGetOwnPropertySymbols;
var anObject = anObject$1;
var concat = uncurryThis([].concat);
var ownKeys$1 = getBuiltIn$1("Reflect", "ownKeys") || function ownKeys(it) {
  var keys2 = getOwnPropertyNamesModule.f(anObject(it));
  var getOwnPropertySymbols = getOwnPropertySymbolsModule.f;
  return getOwnPropertySymbols ? concat(keys2, getOwnPropertySymbols(it)) : keys2;
};
var ownKeys2 = ownKeys$1;
var getBuiltIn = getBuiltIn$2;
var keys = ownKeys2({ a: 1, b: 2 });
console.log("own-keys " + keys.join(","));
console.log("built-in " + typeof getBuiltIn("Reflect", "ownKeys") + " " + typeof getBuiltIn("Array"));
