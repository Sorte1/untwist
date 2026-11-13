(() => {
  var __create = Object.create;
  var __defProp = Object.defineProperty;
  var __getOwnPropDesc = Object.getOwnPropertyDescriptor;
  var __getOwnPropNames = Object.getOwnPropertyNames;
  var __getProtoOf = Object.getPrototypeOf;
  var __hasOwnProp = Object.prototype.hasOwnProperty;
  var __name = (target, value2) => __defProp(target, "name", { value: value2, configurable: true });
  var __esm = (fn, res, err) => function __init() {
    if (err) throw err[0];
    try {
      return fn && (res = (0, fn[__getOwnPropNames(fn)[0]])(fn = 0)), res;
    } catch (e) {
      throw err = [e], e;
    }
  };
  var __commonJS = (cb, mod) => function __require() {
    try {
      return mod || (0, cb[__getOwnPropNames(cb)[0]])((mod = { exports: {} }).exports, mod), mod.exports;
    } catch (e) {
      throw mod = 0, e;
    }
  };
  var __export = (target, all) => {
    for (var name in all)
      __defProp(target, name, { get: all[name], enumerable: true });
  };
  var __copyProps = (to, from, except, desc) => {
    if (from && typeof from === "object" || typeof from === "function") {
      for (let key of __getOwnPropNames(from))
        if (!__hasOwnProp.call(to, key) && key !== except)
          __defProp(to, key, { get: () => from[key], enumerable: !(desc = __getOwnPropDesc(from, key)) || desc.enumerable });
    }
    return to;
  };
  var __toESM = (mod, isNodeMode, target) => (target = mod != null ? __create(__getProtoOf(mod)) : {}, __copyProps(
    // If the importer is in node compatibility mode or this is not an ESM
    // file that has been converted to a CommonJS file using a Babel-
    // compatible transform (i.e. "__esModule" has not been set), then set
    // "default" to the CommonJS "module.exports" for node compatibility.
    isNodeMode || !mod || !mod.__esModule ? __defProp(target, "default", { value: mod, enumerable: true }) : target,
    mod
  ));

  // node_modules/lodash/_baseSlice.js
  var require_baseSlice = __commonJS({
    "node_modules/lodash/_baseSlice.js"(exports, module) {
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
      __name(baseSlice, "baseSlice");
      module.exports = baseSlice;
    }
  });

  // node_modules/lodash/eq.js
  var require_eq = __commonJS({
    "node_modules/lodash/eq.js"(exports, module) {
      function eq(value2, other) {
        return value2 === other || value2 !== value2 && other !== other;
      }
      __name(eq, "eq");
      module.exports = eq;
    }
  });

  // node_modules/lodash/_freeGlobal.js
  var require_freeGlobal = __commonJS({
    "node_modules/lodash/_freeGlobal.js"(exports, module) {
      var freeGlobal = typeof global == "object" && global && global.Object === Object && global;
      module.exports = freeGlobal;
    }
  });

  // node_modules/lodash/_root.js
  var require_root = __commonJS({
    "node_modules/lodash/_root.js"(exports, module) {
      var freeGlobal = require_freeGlobal();
      var freeSelf = typeof self == "object" && self && self.Object === Object && self;
      var root = freeGlobal || freeSelf || Function("return this")();
      module.exports = root;
    }
  });

  // node_modules/lodash/_Symbol.js
  var require_Symbol = __commonJS({
    "node_modules/lodash/_Symbol.js"(exports, module) {
      var root = require_root();
      var Symbol2 = root.Symbol;
      module.exports = Symbol2;
    }
  });

  // node_modules/lodash/_getRawTag.js
  var require_getRawTag = __commonJS({
    "node_modules/lodash/_getRawTag.js"(exports, module) {
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
      __name(getRawTag, "getRawTag");
      module.exports = getRawTag;
    }
  });

  // node_modules/lodash/_objectToString.js
  var require_objectToString = __commonJS({
    "node_modules/lodash/_objectToString.js"(exports, module) {
      var objectProto = Object.prototype;
      var nativeObjectToString = objectProto.toString;
      function objectToString(value2) {
        return nativeObjectToString.call(value2);
      }
      __name(objectToString, "objectToString");
      module.exports = objectToString;
    }
  });

  // node_modules/lodash/_baseGetTag.js
  var require_baseGetTag = __commonJS({
    "node_modules/lodash/_baseGetTag.js"(exports, module) {
      var Symbol2 = require_Symbol();
      var getRawTag = require_getRawTag();
      var objectToString = require_objectToString();
      var nullTag = "[object Null]";
      var undefinedTag = "[object Undefined]";
      var symToStringTag = Symbol2 ? Symbol2.toStringTag : void 0;
      function baseGetTag(value2) {
        if (value2 == null) {
          return value2 === void 0 ? undefinedTag : nullTag;
        }
        return symToStringTag && symToStringTag in Object(value2) ? getRawTag(value2) : objectToString(value2);
      }
      __name(baseGetTag, "baseGetTag");
      module.exports = baseGetTag;
    }
  });

  // node_modules/lodash/isObject.js
  var require_isObject = __commonJS({
    "node_modules/lodash/isObject.js"(exports, module) {
      function isObject(value2) {
        var type = typeof value2;
        return value2 != null && (type == "object" || type == "function");
      }
      __name(isObject, "isObject");
      module.exports = isObject;
    }
  });

  // node_modules/lodash/isFunction.js
  var require_isFunction = __commonJS({
    "node_modules/lodash/isFunction.js"(exports, module) {
      var baseGetTag = require_baseGetTag();
      var isObject = require_isObject();
      var asyncTag = "[object AsyncFunction]";
      var funcTag = "[object Function]";
      var genTag = "[object GeneratorFunction]";
      var proxyTag = "[object Proxy]";
      function isFunction(value2) {
        if (!isObject(value2)) {
          return false;
        }
        var tag = baseGetTag(value2);
        return tag == funcTag || tag == genTag || tag == asyncTag || tag == proxyTag;
      }
      __name(isFunction, "isFunction");
      module.exports = isFunction;
    }
  });

  // node_modules/lodash/isLength.js
  var require_isLength = __commonJS({
    "node_modules/lodash/isLength.js"(exports, module) {
      var MAX_SAFE_INTEGER = 9007199254740991;
      function isLength(value2) {
        return typeof value2 == "number" && value2 > -1 && value2 % 1 == 0 && value2 <= MAX_SAFE_INTEGER;
      }
      __name(isLength, "isLength");
      module.exports = isLength;
    }
  });

  // node_modules/lodash/isArrayLike.js
  var require_isArrayLike = __commonJS({
    "node_modules/lodash/isArrayLike.js"(exports, module) {
      var isFunction = require_isFunction();
      var isLength = require_isLength();
      function isArrayLike(value2) {
        return value2 != null && isLength(value2.length) && !isFunction(value2);
      }
      __name(isArrayLike, "isArrayLike");
      module.exports = isArrayLike;
    }
  });

  // node_modules/lodash/_isIndex.js
  var require_isIndex = __commonJS({
    "node_modules/lodash/_isIndex.js"(exports, module) {
      var MAX_SAFE_INTEGER = 9007199254740991;
      var reIsUint = /^(?:0|[1-9]\d*)$/;
      function isIndex(value2, length) {
        var type = typeof value2;
        length = length == null ? MAX_SAFE_INTEGER : length;
        return !!length && (type == "number" || type != "symbol" && reIsUint.test(value2)) && (value2 > -1 && value2 % 1 == 0 && value2 < length);
      }
      __name(isIndex, "isIndex");
      module.exports = isIndex;
    }
  });

  // node_modules/lodash/_isIterateeCall.js
  var require_isIterateeCall = __commonJS({
    "node_modules/lodash/_isIterateeCall.js"(exports, module) {
      var eq = require_eq();
      var isArrayLike = require_isArrayLike();
      var isIndex = require_isIndex();
      var isObject = require_isObject();
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
      __name(isIterateeCall, "isIterateeCall");
      module.exports = isIterateeCall;
    }
  });

  // node_modules/lodash/_trimmedEndIndex.js
  var require_trimmedEndIndex = __commonJS({
    "node_modules/lodash/_trimmedEndIndex.js"(exports, module) {
      var reWhitespace = /\s/;
      function trimmedEndIndex(string) {
        var index = string.length;
        while (index-- && reWhitespace.test(string.charAt(index))) {
        }
        return index;
      }
      __name(trimmedEndIndex, "trimmedEndIndex");
      module.exports = trimmedEndIndex;
    }
  });

  // node_modules/lodash/_baseTrim.js
  var require_baseTrim = __commonJS({
    "node_modules/lodash/_baseTrim.js"(exports, module) {
      var trimmedEndIndex = require_trimmedEndIndex();
      var reTrimStart = /^\s+/;
      function baseTrim(string) {
        return string ? string.slice(0, trimmedEndIndex(string) + 1).replace(reTrimStart, "") : string;
      }
      __name(baseTrim, "baseTrim");
      module.exports = baseTrim;
    }
  });

  // node_modules/lodash/isObjectLike.js
  var require_isObjectLike = __commonJS({
    "node_modules/lodash/isObjectLike.js"(exports, module) {
      function isObjectLike(value2) {
        return value2 != null && typeof value2 == "object";
      }
      __name(isObjectLike, "isObjectLike");
      module.exports = isObjectLike;
    }
  });

  // node_modules/lodash/isSymbol.js
  var require_isSymbol = __commonJS({
    "node_modules/lodash/isSymbol.js"(exports, module) {
      var baseGetTag = require_baseGetTag();
      var isObjectLike = require_isObjectLike();
      var symbolTag = "[object Symbol]";
      function isSymbol(value2) {
        return typeof value2 == "symbol" || isObjectLike(value2) && baseGetTag(value2) == symbolTag;
      }
      __name(isSymbol, "isSymbol");
      module.exports = isSymbol;
    }
  });

  // node_modules/lodash/toNumber.js
  var require_toNumber = __commonJS({
    "node_modules/lodash/toNumber.js"(exports, module) {
      var baseTrim = require_baseTrim();
      var isObject = require_isObject();
      var isSymbol = require_isSymbol();
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
      __name(toNumber, "toNumber");
      module.exports = toNumber;
    }
  });

  // node_modules/lodash/toFinite.js
  var require_toFinite = __commonJS({
    "node_modules/lodash/toFinite.js"(exports, module) {
      var toNumber = require_toNumber();
      var INFINITY = 1 / 0;
      var MAX_INTEGER = 17976931348623157e292;
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
      __name(toFinite, "toFinite");
      module.exports = toFinite;
    }
  });

  // node_modules/lodash/toInteger.js
  var require_toInteger = __commonJS({
    "node_modules/lodash/toInteger.js"(exports, module) {
      var toFinite = require_toFinite();
      function toInteger(value2) {
        var result = toFinite(value2), remainder = result % 1;
        return result === result ? remainder ? result - remainder : result : 0;
      }
      __name(toInteger, "toInteger");
      module.exports = toInteger;
    }
  });

  // node_modules/lodash/chunk.js
  var require_chunk = __commonJS({
    "node_modules/lodash/chunk.js"(exports, module) {
      var baseSlice = require_baseSlice();
      var isIterateeCall = require_isIterateeCall();
      var toInteger = require_toInteger();
      var nativeCeil = Math.ceil;
      var nativeMax = Math.max;
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
      __name(chunk2, "chunk");
      module.exports = chunk2;
    }
  });

  // .src-base-esm/cycle-b.js
  var require_cycle_b = __commonJS({
    ".src-base-esm/cycle-b.js"(exports) {
      console.log("eval cycle-b");
      var a = require_cycle_a();
      exports.seen = "b saw " + a.early + "," + a.late;
      exports.check = function() {
        return "b later " + a.late;
      };
    }
  });

  // .src-base-esm/cycle-a.js
  var require_cycle_a = __commonJS({
    ".src-base-esm/cycle-a.js"(exports) {
      console.log("eval cycle-a start");
      exports.early = "a-early";
      var b = require_cycle_b();
      exports.late = "a-late";
      exports.report = function() {
        return b.seen + " " + b.check();
      };
      console.log("eval cycle-a end");
    }
  });

  // .src-base-esm/lazy.js
  var lazy_exports = {};
  __export(lazy_exports, {
    default: () => lazy_default,
    value: () => value
  });
  var value, lazy_default;
  var init_lazy = __esm({
    ".src-base-esm/lazy.js"() {
      console.log("eval lazy");
      value = 42;
      lazy_default = "lazy-default";
    }
  });

  // .src-base-esm/index.js
  var import_chunk = __toESM(require_chunk());

  // .src-base-esm/data.json
  var data_default = { name: "fixture", items: [1, 2, 3, 4, 5] };

  // .src-base-esm/index.js
  var import_cycle_a = __toESM(require_cycle_a());

  // .src-base-esm/util.js
  console.log("eval util");
  var label = /* @__PURE__ */ __name((name) => `[${name.padStart(8, ".")}]`, "label");
  function total(list) {
    return list.reduce((sum, n) => sum + n, 0);
  }
  __name(total, "total");

  // .src-base-esm/index.js
  var items = data_default.items;
  console.log("eval index");
  console.log("last " + items[items.length - 1] + " own " + Object.prototype.hasOwnProperty.call(data_default, "name"));
  console.log("replace " + "a-b-c".split("-").join("+"));
  console.log("chunk " + JSON.stringify((0, import_chunk.default)(items, 2)));
  console.log("cycle " + (0, import_cycle_a.report)());
  console.log("label " + label(data_default.name) + " total " + total(items));
  Promise.resolve().then(() => (init_lazy(), lazy_exports)).then((lazy) => {
    console.log("lazy " + lazy.value + " " + lazy.default);
    return Promise.allSettled([Promise.resolve(1), Promise.reject(new Error("no"))]);
  }).then((settled) => console.log("settled " + settled.map((s) => s.status).join(",")));
})();
