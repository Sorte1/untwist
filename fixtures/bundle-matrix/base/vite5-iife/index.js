(function() {
  "use strict";
  var commonjsGlobal = typeof globalThis !== "undefined" ? globalThis : typeof window !== "undefined" ? window : typeof global !== "undefined" ? global : typeof self !== "undefined" ? self : {};
  function getDefaultExportFromCjs(x) {
    return x && x.__esModule && Object.prototype.hasOwnProperty.call(x, "default") ? x["default"] : x;
  }
  function baseSlice$1(array, start, end) {
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
  var _baseSlice = baseSlice$1;
  function eq$1(value2, other) {
    return value2 === other || value2 !== value2 && other !== other;
  }
  var eq_1 = eq$1;
  var freeGlobal$1 = typeof commonjsGlobal == "object" && commonjsGlobal && commonjsGlobal.Object === Object && commonjsGlobal;
  var _freeGlobal = freeGlobal$1;
  var freeGlobal = _freeGlobal;
  var freeSelf = typeof self == "object" && self && self.Object === Object && self;
  var root$1 = freeGlobal || freeSelf || Function("return this")();
  var _root = root$1;
  var root = _root;
  var Symbol$3 = root.Symbol;
  var _Symbol = Symbol$3;
  var Symbol$2 = _Symbol;
  var objectProto$1 = Object.prototype;
  var hasOwnProperty = objectProto$1.hasOwnProperty;
  var nativeObjectToString$1 = objectProto$1.toString;
  var symToStringTag$1 = Symbol$2 ? Symbol$2.toStringTag : void 0;
  function getRawTag$1(value2) {
    var isOwn = hasOwnProperty.call(value2, symToStringTag$1), tag = value2[symToStringTag$1];
    try {
      value2[symToStringTag$1] = void 0;
      var unmasked = true;
    } catch (e) {
    }
    var result = nativeObjectToString$1.call(value2);
    if (unmasked) {
      if (isOwn) {
        value2[symToStringTag$1] = tag;
      } else {
        delete value2[symToStringTag$1];
      }
    }
    return result;
  }
  var _getRawTag = getRawTag$1;
  var objectProto = Object.prototype;
  var nativeObjectToString = objectProto.toString;
  function objectToString$1(value2) {
    return nativeObjectToString.call(value2);
  }
  var _objectToString = objectToString$1;
  var Symbol$1 = _Symbol, getRawTag = _getRawTag, objectToString = _objectToString;
  var nullTag = "[object Null]", undefinedTag = "[object Undefined]";
  var symToStringTag = Symbol$1 ? Symbol$1.toStringTag : void 0;
  function baseGetTag$2(value2) {
    if (value2 == null) {
      return value2 === void 0 ? undefinedTag : nullTag;
    }
    return symToStringTag && symToStringTag in Object(value2) ? getRawTag(value2) : objectToString(value2);
  }
  var _baseGetTag = baseGetTag$2;
  function isObject$3(value2) {
    var type = typeof value2;
    return value2 != null && (type == "object" || type == "function");
  }
  var isObject_1 = isObject$3;
  var baseGetTag$1 = _baseGetTag, isObject$2 = isObject_1;
  var asyncTag = "[object AsyncFunction]", funcTag = "[object Function]", genTag = "[object GeneratorFunction]", proxyTag = "[object Proxy]";
  function isFunction$1(value2) {
    if (!isObject$2(value2)) {
      return false;
    }
    var tag = baseGetTag$1(value2);
    return tag == funcTag || tag == genTag || tag == asyncTag || tag == proxyTag;
  }
  var isFunction_1 = isFunction$1;
  var MAX_SAFE_INTEGER$1 = 9007199254740991;
  function isLength$1(value2) {
    return typeof value2 == "number" && value2 > -1 && value2 % 1 == 0 && value2 <= MAX_SAFE_INTEGER$1;
  }
  var isLength_1 = isLength$1;
  var isFunction = isFunction_1, isLength = isLength_1;
  function isArrayLike$1(value2) {
    return value2 != null && isLength(value2.length) && !isFunction(value2);
  }
  var isArrayLike_1 = isArrayLike$1;
  var MAX_SAFE_INTEGER = 9007199254740991;
  var reIsUint = /^(?:0|[1-9]\d*)$/;
  function isIndex$1(value2, length) {
    var type = typeof value2;
    length = length == null ? MAX_SAFE_INTEGER : length;
    return !!length && (type == "number" || type != "symbol" && reIsUint.test(value2)) && (value2 > -1 && value2 % 1 == 0 && value2 < length);
  }
  var _isIndex = isIndex$1;
  var eq = eq_1, isArrayLike = isArrayLike_1, isIndex = _isIndex, isObject$1 = isObject_1;
  function isIterateeCall$1(value2, index, object) {
    if (!isObject$1(object)) {
      return false;
    }
    var type = typeof index;
    if (type == "number" ? isArrayLike(object) && isIndex(index, object.length) : type == "string" && index in object) {
      return eq(object[index], value2);
    }
    return false;
  }
  var _isIterateeCall = isIterateeCall$1;
  var reWhitespace = /\s/;
  function trimmedEndIndex$1(string) {
    var index = string.length;
    while (index-- && reWhitespace.test(string.charAt(index))) {
    }
    return index;
  }
  var _trimmedEndIndex = trimmedEndIndex$1;
  var trimmedEndIndex = _trimmedEndIndex;
  var reTrimStart = /^\s+/;
  function baseTrim$1(string) {
    return string ? string.slice(0, trimmedEndIndex(string) + 1).replace(reTrimStart, "") : string;
  }
  var _baseTrim = baseTrim$1;
  function isObjectLike$1(value2) {
    return value2 != null && typeof value2 == "object";
  }
  var isObjectLike_1 = isObjectLike$1;
  var baseGetTag = _baseGetTag, isObjectLike = isObjectLike_1;
  var symbolTag = "[object Symbol]";
  function isSymbol$1(value2) {
    return typeof value2 == "symbol" || isObjectLike(value2) && baseGetTag(value2) == symbolTag;
  }
  var isSymbol_1 = isSymbol$1;
  var baseTrim = _baseTrim, isObject = isObject_1, isSymbol = isSymbol_1;
  var NAN = 0 / 0;
  var reIsBadHex = /^[-+]0x[0-9a-f]+$/i;
  var reIsBinary = /^0b[01]+$/i;
  var reIsOctal = /^0o[0-7]+$/i;
  var freeParseInt = parseInt;
  function toNumber$1(value2) {
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
  var toNumber_1 = toNumber$1;
  var toNumber = toNumber_1;
  var INFINITY = 1 / 0, MAX_INTEGER = 17976931348623157e292;
  function toFinite$1(value2) {
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
  var toFinite_1 = toFinite$1;
  var toFinite = toFinite_1;
  function toInteger$1(value2) {
    var result = toFinite(value2), remainder = result % 1;
    return result === result ? remainder ? result - remainder : result : 0;
  }
  var toInteger_1 = toInteger$1;
  var baseSlice = _baseSlice, isIterateeCall = _isIterateeCall, toInteger = toInteger_1;
  var nativeCeil = Math.ceil, nativeMax = Math.max;
  function chunk(array, size, guard) {
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
  var chunk_1 = chunk;
  const chunk$1 = /* @__PURE__ */ getDefaultExportFromCjs(chunk_1);
  const name = "fixture";
  const items$1 = [
    1,
    2,
    3,
    4,
    5
  ];
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
  console.log("chunk " + JSON.stringify(chunk$1(items, 2)));
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
