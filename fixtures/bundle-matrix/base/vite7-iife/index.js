(function() {
  "use strict";
  var commonjsGlobal = typeof globalThis !== "undefined" ? globalThis : typeof window !== "undefined" ? window : typeof global !== "undefined" ? global : typeof self !== "undefined" ? self : {};
  function getDefaultExportFromCjs(x) {
    return x && x.__esModule && Object.prototype.hasOwnProperty.call(x, "default") ? x["default"] : x;
  }
  var _baseSlice;
  var hasRequired_baseSlice;
  function require_baseSlice() {
    if (hasRequired_baseSlice) return _baseSlice;
    hasRequired_baseSlice = 1;
    function baseSlice(array, start, end) {
      var index = -1, length = array.length;
      if (start < 0) {
        start = -start > length ? 0 : length + start;
      }
      end = end > length ? length : end;
      if (end < 0) {
        end += length;
      }
      length = start > end ? 0 : end - start >>> 0;
      start >>>= 0;
      var result = Array(length);
      while (++index < length) {
        result[index] = array[index + start];
      }
      return result;
    }
    _baseSlice = baseSlice;
    return _baseSlice;
  }
  var eq_1;
  var hasRequiredEq;
  function requireEq() {
    if (hasRequiredEq) return eq_1;
    hasRequiredEq = 1;
    function eq(value2, other) {
      return value2 === other || value2 !== value2 && other !== other;
    }
    eq_1 = eq;
    return eq_1;
  }
  var _freeGlobal;
  var hasRequired_freeGlobal;
  function require_freeGlobal() {
    if (hasRequired_freeGlobal) return _freeGlobal;
    hasRequired_freeGlobal = 1;
    var freeGlobal = typeof commonjsGlobal == "object" && commonjsGlobal && commonjsGlobal.Object === Object && commonjsGlobal;
    _freeGlobal = freeGlobal;
    return _freeGlobal;
  }
  var _root;
  var hasRequired_root;
  function require_root() {
    if (hasRequired_root) return _root;
    hasRequired_root = 1;
    var freeGlobal = require_freeGlobal();
    var freeSelf = typeof self == "object" && self && self.Object === Object && self;
    var root = freeGlobal || freeSelf || Function("return this")();
    _root = root;
    return _root;
  }
  var _Symbol;
  var hasRequired_Symbol;
  function require_Symbol() {
    if (hasRequired_Symbol) return _Symbol;
    hasRequired_Symbol = 1;
    var root = require_root();
    var Symbol2 = root.Symbol;
    _Symbol = Symbol2;
    return _Symbol;
  }
  var _getRawTag;
  var hasRequired_getRawTag;
  function require_getRawTag() {
    if (hasRequired_getRawTag) return _getRawTag;
    hasRequired_getRawTag = 1;
    var Symbol2 = require_Symbol();
    var objectProto = Object.prototype;
    var hasOwnProperty = objectProto.hasOwnProperty;
    var nativeObjectToString = objectProto.toString;
    var symToStringTag = Symbol2 ? Symbol2.toStringTag : void 0;
    function getRawTag(value2) {
      var isOwn = hasOwnProperty.call(value2, symToStringTag), tag = value2[symToStringTag];
      try {
        value2[symToStringTag] = void 0;
        var unmasked = true;
      } catch (e) {
      }
      var result = nativeObjectToString.call(value2);
      if (unmasked) {
        if (isOwn) {
          value2[symToStringTag] = tag;
        } else {
          delete value2[symToStringTag];
        }
      }
      return result;
    }
    _getRawTag = getRawTag;
    return _getRawTag;
  }
  var _objectToString;
  var hasRequired_objectToString;
  function require_objectToString() {
    if (hasRequired_objectToString) return _objectToString;
    hasRequired_objectToString = 1;
    var objectProto = Object.prototype;
    var nativeObjectToString = objectProto.toString;
    function objectToString(value2) {
      return nativeObjectToString.call(value2);
    }
    _objectToString = objectToString;
    return _objectToString;
  }
  var _baseGetTag;
  var hasRequired_baseGetTag;
  function require_baseGetTag() {
    if (hasRequired_baseGetTag) return _baseGetTag;
    hasRequired_baseGetTag = 1;
    var Symbol2 = require_Symbol(), getRawTag = require_getRawTag(), objectToString = require_objectToString();
    var nullTag = "[object Null]", undefinedTag = "[object Undefined]";
    var symToStringTag = Symbol2 ? Symbol2.toStringTag : void 0;
    function baseGetTag(value2) {
      if (value2 == null) {
        return value2 === void 0 ? undefinedTag : nullTag;
      }
      return symToStringTag && symToStringTag in Object(value2) ? getRawTag(value2) : objectToString(value2);
    }
    _baseGetTag = baseGetTag;
    return _baseGetTag;
  }
  var isObject_1;
  var hasRequiredIsObject;
  function requireIsObject() {
    if (hasRequiredIsObject) return isObject_1;
    hasRequiredIsObject = 1;
    function isObject(value2) {
      var type = typeof value2;
      return value2 != null && (type == "object" || type == "function");
    }
    isObject_1 = isObject;
    return isObject_1;
  }
  var isFunction_1;
  var hasRequiredIsFunction;
  function requireIsFunction() {
    if (hasRequiredIsFunction) return isFunction_1;
    hasRequiredIsFunction = 1;
    var baseGetTag = require_baseGetTag(), isObject = requireIsObject();
    var asyncTag = "[object AsyncFunction]", funcTag = "[object Function]", genTag = "[object GeneratorFunction]", proxyTag = "[object Proxy]";
    function isFunction(value2) {
      if (!isObject(value2)) {
        return false;
      }
      var tag = baseGetTag(value2);
      return tag == funcTag || tag == genTag || tag == asyncTag || tag == proxyTag;
    }
    isFunction_1 = isFunction;
    return isFunction_1;
  }
  var isLength_1;
  var hasRequiredIsLength;
  function requireIsLength() {
    if (hasRequiredIsLength) return isLength_1;
    hasRequiredIsLength = 1;
    var MAX_SAFE_INTEGER = 9007199254740991;
    function isLength(value2) {
      return typeof value2 == "number" && value2 > -1 && value2 % 1 == 0 && value2 <= MAX_SAFE_INTEGER;
    }
    isLength_1 = isLength;
    return isLength_1;
  }
  var isArrayLike_1;
  var hasRequiredIsArrayLike;
  function requireIsArrayLike() {
    if (hasRequiredIsArrayLike) return isArrayLike_1;
    hasRequiredIsArrayLike = 1;
    var isFunction = requireIsFunction(), isLength = requireIsLength();
    function isArrayLike(value2) {
      return value2 != null && isLength(value2.length) && !isFunction(value2);
    }
    isArrayLike_1 = isArrayLike;
    return isArrayLike_1;
  }
  var _isIndex;
  var hasRequired_isIndex;
  function require_isIndex() {
    if (hasRequired_isIndex) return _isIndex;
    hasRequired_isIndex = 1;
    var MAX_SAFE_INTEGER = 9007199254740991;
    var reIsUint = /^(?:0|[1-9]\d*)$/;
    function isIndex(value2, length) {
      var type = typeof value2;
      length = length == null ? MAX_SAFE_INTEGER : length;
      return !!length && (type == "number" || type != "symbol" && reIsUint.test(value2)) && (value2 > -1 && value2 % 1 == 0 && value2 < length);
    }
    _isIndex = isIndex;
    return _isIndex;
  }
  var _isIterateeCall;
  var hasRequired_isIterateeCall;
  function require_isIterateeCall() {
    if (hasRequired_isIterateeCall) return _isIterateeCall;
    hasRequired_isIterateeCall = 1;
    var eq = requireEq(), isArrayLike = requireIsArrayLike(), isIndex = require_isIndex(), isObject = requireIsObject();
    function isIterateeCall(value2, index, object) {
      if (!isObject(object)) {
        return false;
      }
      var type = typeof index;
      if (type == "number" ? isArrayLike(object) && isIndex(index, object.length) : type == "string" && index in object) {
        return eq(object[index], value2);
      }
      return false;
    }
    _isIterateeCall = isIterateeCall;
    return _isIterateeCall;
  }
  var _trimmedEndIndex;
  var hasRequired_trimmedEndIndex;
  function require_trimmedEndIndex() {
    if (hasRequired_trimmedEndIndex) return _trimmedEndIndex;
    hasRequired_trimmedEndIndex = 1;
    var reWhitespace = /\s/;
    function trimmedEndIndex(string) {
      var index = string.length;
      while (index-- && reWhitespace.test(string.charAt(index))) {
      }
      return index;
    }
    _trimmedEndIndex = trimmedEndIndex;
    return _trimmedEndIndex;
  }
  var _baseTrim;
  var hasRequired_baseTrim;
  function require_baseTrim() {
    if (hasRequired_baseTrim) return _baseTrim;
    hasRequired_baseTrim = 1;
    var trimmedEndIndex = require_trimmedEndIndex();
    var reTrimStart = /^\s+/;
    function baseTrim(string) {
      return string ? string.slice(0, trimmedEndIndex(string) + 1).replace(reTrimStart, "") : string;
    }
    _baseTrim = baseTrim;
    return _baseTrim;
  }
  var isObjectLike_1;
  var hasRequiredIsObjectLike;
  function requireIsObjectLike() {
    if (hasRequiredIsObjectLike) return isObjectLike_1;
    hasRequiredIsObjectLike = 1;
    function isObjectLike(value2) {
      return value2 != null && typeof value2 == "object";
    }
    isObjectLike_1 = isObjectLike;
    return isObjectLike_1;
  }
  var isSymbol_1;
  var hasRequiredIsSymbol;
  function requireIsSymbol() {
    if (hasRequiredIsSymbol) return isSymbol_1;
    hasRequiredIsSymbol = 1;
    var baseGetTag = require_baseGetTag(), isObjectLike = requireIsObjectLike();
    var symbolTag = "[object Symbol]";
    function isSymbol(value2) {
      return typeof value2 == "symbol" || isObjectLike(value2) && baseGetTag(value2) == symbolTag;
    }
    isSymbol_1 = isSymbol;
    return isSymbol_1;
  }
  var toNumber_1;
  var hasRequiredToNumber;
  function requireToNumber() {
    if (hasRequiredToNumber) return toNumber_1;
    hasRequiredToNumber = 1;
    var baseTrim = require_baseTrim(), isObject = requireIsObject(), isSymbol = requireIsSymbol();
    var NAN = 0 / 0;
    var reIsBadHex = /^[-+]0x[0-9a-f]+$/i;
    var reIsBinary = /^0b[01]+$/i;
    var reIsOctal = /^0o[0-7]+$/i;
    var freeParseInt = parseInt;
    function toNumber(value2) {
      if (typeof value2 == "number") {
        return value2;
      }
      if (isSymbol(value2)) {
        return NAN;
      }
      if (isObject(value2)) {
        var other = typeof value2.valueOf == "function" ? value2.valueOf() : value2;
        value2 = isObject(other) ? other + "" : other;
      }
      if (typeof value2 != "string") {
        return value2 === 0 ? value2 : +value2;
      }
      value2 = baseTrim(value2);
      var isBinary = reIsBinary.test(value2);
      return isBinary || reIsOctal.test(value2) ? freeParseInt(value2.slice(2), isBinary ? 2 : 8) : reIsBadHex.test(value2) ? NAN : +value2;
    }
    toNumber_1 = toNumber;
    return toNumber_1;
  }
  var toFinite_1;
  var hasRequiredToFinite;
  function requireToFinite() {
    if (hasRequiredToFinite) return toFinite_1;
    hasRequiredToFinite = 1;
    var toNumber = requireToNumber();
    var INFINITY = 1 / 0, MAX_INTEGER = 17976931348623157e292;
    function toFinite(value2) {
      if (!value2) {
        return value2 === 0 ? value2 : 0;
      }
      value2 = toNumber(value2);
      if (value2 === INFINITY || value2 === -INFINITY) {
        var sign = value2 < 0 ? -1 : 1;
        return sign * MAX_INTEGER;
      }
      return value2 === value2 ? value2 : 0;
    }
    toFinite_1 = toFinite;
    return toFinite_1;
  }
  var toInteger_1;
  var hasRequiredToInteger;
  function requireToInteger() {
    if (hasRequiredToInteger) return toInteger_1;
    hasRequiredToInteger = 1;
    var toFinite = requireToFinite();
    function toInteger(value2) {
      var result = toFinite(value2), remainder = result % 1;
      return result === result ? remainder ? result - remainder : result : 0;
    }
    toInteger_1 = toInteger;
    return toInteger_1;
  }
  var chunk_1;
  var hasRequiredChunk;
  function requireChunk() {
    if (hasRequiredChunk) return chunk_1;
    hasRequiredChunk = 1;
    var baseSlice = require_baseSlice(), isIterateeCall = require_isIterateeCall(), toInteger = requireToInteger();
    var nativeCeil = Math.ceil, nativeMax = Math.max;
    function chunk2(array, size, guard) {
      if (guard ? isIterateeCall(array, size, guard) : size === void 0) {
        size = 1;
      } else {
        size = nativeMax(toInteger(size), 0);
      }
      var length = array == null ? 0 : array.length;
      if (!length || size < 1) {
        return [];
      }
      var index = 0, resIndex = 0, result = Array(nativeCeil(length / size));
      while (index < length) {
        result[resIndex++] = baseSlice(array, index, index += size);
      }
      return result;
    }
    chunk_1 = chunk2;
    return chunk_1;
  }
  var chunkExports = requireChunk();
  const chunk = /* @__PURE__ */ getDefaultExportFromCjs(chunkExports);
  const name = "fixture";
  const items$1 = [1, 2, 3, 4, 5];
  const data = {
    name,
    items: items$1
  };
  var cycleA = {};
  var cycleB = {};
  var hasRequiredCycleB;
  function requireCycleB() {
    if (hasRequiredCycleB) return cycleB;
    hasRequiredCycleB = 1;
    console.log("eval cycle-b");
    var a = requireCycleA();
    cycleB.seen = "b saw " + a.early + "," + a.late;
    cycleB.check = function() {
      return "b later " + a.late;
    };
    return cycleB;
  }
  var hasRequiredCycleA;
  function requireCycleA() {
    if (hasRequiredCycleA) return cycleA;
    hasRequiredCycleA = 1;
    console.log("eval cycle-a start");
    cycleA.early = "a-early";
    var b = requireCycleB();
    cycleA.late = "a-late";
    cycleA.report = function() {
      return b.seen + " " + b.check();
    };
    console.log("eval cycle-a end");
    return cycleA;
  }
  var cycleAExports = requireCycleA();
  console.log("eval util");
  const label = (name2) => `[${name2.padStart(8, ".")}]`;
  function total(list) {
    return list.reduce((sum, n) => sum + n, 0);
  }
  const items = data.items;
  console.log("eval index");
  console.log("last " + items[items.length - 1] + " own " + Object.prototype.hasOwnProperty.call(data, "name"));
  console.log("replace " + "a-b-c".split("-").join("+"));
  console.log("chunk " + JSON.stringify(chunk(items, 2)));
  console.log("cycle " + cycleAExports.report());
  console.log("label " + label(data.name) + " total " + total(items));
  Promise.resolve().then(() => lazy$1).then((lazy2) => {
    console.log("lazy " + lazy2.value + " " + lazy2.default);
    return Promise.allSettled([Promise.resolve(1), Promise.reject(new Error("no"))]);
  }).then((settled) => console.log("settled " + settled.map((s) => s.status).join(",")));
  console.log("eval lazy");
  const value = 42;
  const lazy = "lazy-default";
  const lazy$1 = /* @__PURE__ */ Object.freeze(/* @__PURE__ */ Object.defineProperty({
    __proto__: null,
    default: lazy,
    value
  }, Symbol.toStringTag, { value: "Module" }));
})();
