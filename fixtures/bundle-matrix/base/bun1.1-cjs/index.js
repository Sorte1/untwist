var __create = Object.create;
var __getProtoOf = Object.getPrototypeOf;
var __defProp = Object.defineProperty;
var __getOwnPropNames = Object.getOwnPropertyNames;
var __hasOwnProp = Object.prototype.hasOwnProperty;
var __toESM = (mod, isNodeMode, target) => {
  target = mod != null ? __create(__getProtoOf(mod)) : {};
  const to = isNodeMode || !mod || !mod.__esModule ? __defProp(target, "default", { value: mod, enumerable: true }) : target;
  for (let key of __getOwnPropNames(mod))
    if (!__hasOwnProp.call(to, key))
      __defProp(to, key, {
        get: () => mod[key],
        enumerable: true
      });
  return to;
};
var __commonJS = (cb, mod) => () => (mod || cb((mod = { exports: {} }).exports, mod), mod.exports);
var __export = (target, all) => {
  for (var name in all)
    __defProp(target, name, {
      get: all[name],
      enumerable: true,
      configurable: true,
      set: (newValue) => all[name] = () => newValue
    });
};
var __esm = (fn, res) => () => (fn && (res = fn(fn = 0)), res);

// ../node_modules/lodash/_baseSlice.js
var require__baseSlice = __commonJS((exports2, module2) => {
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
  module2.exports = baseSlice;
});

// ../node_modules/lodash/eq.js
var require_eq = __commonJS((exports2, module2) => {
  function eq(value, other) {
    return value === other || value !== value && other !== other;
  }
  module2.exports = eq;
});

// ../node_modules/lodash/_freeGlobal.js
var require__freeGlobal = __commonJS((exports2, module2) => {
  var freeGlobal = typeof global == "object" && global && global.Object === Object && global;
  module2.exports = freeGlobal;
});

// ../node_modules/lodash/_root.js
var require__root = __commonJS((exports2, module2) => {
  var freeGlobal = require__freeGlobal();
  var freeSelf = typeof self == "object" && self && self.Object === Object && self;
  var root = freeGlobal || freeSelf || Function("return this")();
  module2.exports = root;
});

// ../node_modules/lodash/_Symbol.js
var require__Symbol = __commonJS((exports2, module2) => {
  var root = require__root();
  var Symbol2 = root.Symbol;
  module2.exports = Symbol2;
});

// ../node_modules/lodash/_getRawTag.js
var require__getRawTag = __commonJS((exports2, module2) => {
  var Symbol2 = require__Symbol();
  var objectProto = Object.prototype;
  var hasOwnProperty = objectProto.hasOwnProperty;
  var nativeObjectToString = objectProto.toString;
  var symToStringTag = Symbol2 ? Symbol2.toStringTag : undefined;
  function getRawTag(value) {
    var isOwn = hasOwnProperty.call(value, symToStringTag), tag = value[symToStringTag];
    try {
      value[symToStringTag] = undefined;
      var unmasked = true;
    } catch (e) {
    }
    var result = nativeObjectToString.call(value);
    if (unmasked) {
      if (isOwn) {
        value[symToStringTag] = tag;
      } else {
        delete value[symToStringTag];
      }
    }
    return result;
  }
  module2.exports = getRawTag;
});

// ../node_modules/lodash/_objectToString.js
var require__objectToString = __commonJS((exports2, module2) => {
  var objectProto = Object.prototype;
  var nativeObjectToString = objectProto.toString;
  function objectToString(value) {
    return nativeObjectToString.call(value);
  }
  module2.exports = objectToString;
});

// ../node_modules/lodash/_baseGetTag.js
var require__baseGetTag = __commonJS((exports2, module2) => {
  var Symbol2 = require__Symbol();
  var getRawTag = require__getRawTag();
  var objectToString = require__objectToString();
  var nullTag = "[object Null]";
  var undefinedTag = "[object Undefined]";
  var symToStringTag = Symbol2 ? Symbol2.toStringTag : undefined;
  function baseGetTag(value) {
    if (value == null) {
      return value === undefined ? undefinedTag : nullTag;
    }
    return symToStringTag && symToStringTag in Object(value) ? getRawTag(value) : objectToString(value);
  }
  module2.exports = baseGetTag;
});

// ../node_modules/lodash/isObject.js
var require_isObject = __commonJS((exports2, module2) => {
  function isObject(value) {
    var type = typeof value;
    return value != null && (type == "object" || type == "function");
  }
  module2.exports = isObject;
});

// ../node_modules/lodash/isFunction.js
var require_isFunction = __commonJS((exports2, module2) => {
  var baseGetTag = require__baseGetTag();
  var isObject = require_isObject();
  var asyncTag = "[object AsyncFunction]";
  var funcTag = "[object Function]";
  var genTag = "[object GeneratorFunction]";
  var proxyTag = "[object Proxy]";
  function isFunction(value) {
    if (!isObject(value)) {
      return false;
    }
    var tag = baseGetTag(value);
    return tag == funcTag || tag == genTag || tag == asyncTag || tag == proxyTag;
  }
  module2.exports = isFunction;
});

// ../node_modules/lodash/isLength.js
var require_isLength = __commonJS((exports2, module2) => {
  var MAX_SAFE_INTEGER = 9007199254740991;
  function isLength(value) {
    return typeof value == "number" && value > -1 && value % 1 == 0 && value <= MAX_SAFE_INTEGER;
  }
  module2.exports = isLength;
});

// ../node_modules/lodash/isArrayLike.js
var require_isArrayLike = __commonJS((exports2, module2) => {
  var isFunction = require_isFunction();
  var isLength = require_isLength();
  function isArrayLike(value) {
    return value != null && isLength(value.length) && !isFunction(value);
  }
  module2.exports = isArrayLike;
});

// ../node_modules/lodash/_isIndex.js
var require__isIndex = __commonJS((exports2, module2) => {
  var MAX_SAFE_INTEGER = 9007199254740991;
  var reIsUint = /^(?:0|[1-9]\d*)$/;
  function isIndex(value, length) {
    var type = typeof value;
    length = length == null ? MAX_SAFE_INTEGER : length;
    return !!length && (type == "number" || type != "symbol" && reIsUint.test(value)) && (value > -1 && value % 1 == 0 && value < length);
  }
  module2.exports = isIndex;
});

// ../node_modules/lodash/_isIterateeCall.js
var require__isIterateeCall = __commonJS((exports2, module2) => {
  var eq = require_eq();
  var isArrayLike = require_isArrayLike();
  var isIndex = require__isIndex();
  var isObject = require_isObject();
  function isIterateeCall(value, index, object) {
    if (!isObject(object)) {
      return false;
    }
    var type = typeof index;
    if (type == "number" ? isArrayLike(object) && isIndex(index, object.length) : type == "string" && (index in object)) {
      return eq(object[index], value);
    }
    return false;
  }
  module2.exports = isIterateeCall;
});

// ../node_modules/lodash/_trimmedEndIndex.js
var require__trimmedEndIndex = __commonJS((exports2, module2) => {
  var reWhitespace = /\s/;
  function trimmedEndIndex(string) {
    var index = string.length;
    while (index-- && reWhitespace.test(string.charAt(index))) {
    }
    return index;
  }
  module2.exports = trimmedEndIndex;
});

// ../node_modules/lodash/_baseTrim.js
var require__baseTrim = __commonJS((exports2, module2) => {
  var trimmedEndIndex = require__trimmedEndIndex();
  var reTrimStart = /^\s+/;
  function baseTrim(string) {
    return string ? string.slice(0, trimmedEndIndex(string) + 1).replace(reTrimStart, "") : string;
  }
  module2.exports = baseTrim;
});

// ../node_modules/lodash/isObjectLike.js
var require_isObjectLike = __commonJS((exports2, module2) => {
  function isObjectLike(value) {
    return value != null && typeof value == "object";
  }
  module2.exports = isObjectLike;
});

// ../node_modules/lodash/isSymbol.js
var require_isSymbol = __commonJS((exports2, module2) => {
  var baseGetTag = require__baseGetTag();
  var isObjectLike = require_isObjectLike();
  var symbolTag = "[object Symbol]";
  function isSymbol(value) {
    return typeof value == "symbol" || isObjectLike(value) && baseGetTag(value) == symbolTag;
  }
  module2.exports = isSymbol;
});

// ../node_modules/lodash/toNumber.js
var require_toNumber = __commonJS((exports2, module2) => {
  var baseTrim = require__baseTrim();
  var isObject = require_isObject();
  var isSymbol = require_isSymbol();
  var NAN = 0 / 0;
  var reIsBadHex = /^[-+]0x[0-9a-f]+$/i;
  var reIsBinary = /^0b[01]+$/i;
  var reIsOctal = /^0o[0-7]+$/i;
  var freeParseInt = parseInt;
  function toNumber(value) {
    if (typeof value == "number") {
      return value;
    }
    if (isSymbol(value)) {
      return NAN;
    }
    if (isObject(value)) {
      var other = typeof value.valueOf == "function" ? value.valueOf() : value;
      value = isObject(other) ? other + "" : other;
    }
    if (typeof value != "string") {
      return value === 0 ? value : +value;
    }
    value = baseTrim(value);
    var isBinary = reIsBinary.test(value);
    return isBinary || reIsOctal.test(value) ? freeParseInt(value.slice(2), isBinary ? 2 : 8) : reIsBadHex.test(value) ? NAN : +value;
  }
  module2.exports = toNumber;
});

// ../node_modules/lodash/toFinite.js
var require_toFinite = __commonJS((exports2, module2) => {
  var toNumber = require_toNumber();
  var INFINITY = 1 / 0;
  var MAX_INTEGER = 179769313486231570000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000;
  function toFinite(value) {
    if (!value) {
      return value === 0 ? value : 0;
    }
    value = toNumber(value);
    if (value === INFINITY || value === -INFINITY) {
      var sign = value < 0 ? -1 : 1;
      return sign * MAX_INTEGER;
    }
    return value === value ? value : 0;
  }
  module2.exports = toFinite;
});

// ../node_modules/lodash/toInteger.js
var require_toInteger = __commonJS((exports2, module2) => {
  var toFinite = require_toFinite();
  function toInteger(value) {
    var result = toFinite(value), remainder = result % 1;
    return result === result ? remainder ? result - remainder : result : 0;
  }
  module2.exports = toInteger;
});

// ../node_modules/lodash/chunk.js
var require_chunk = __commonJS((exports2, module2) => {
  var baseSlice = require__baseSlice();
  var isIterateeCall = require__isIterateeCall();
  var toInteger = require_toInteger();
  var nativeCeil = Math.ceil;
  var nativeMax = Math.max;
  function chunk(array, size, guard) {
    if (guard ? isIterateeCall(array, size, guard) : size === undefined) {
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
  module2.exports = chunk;
});

// cycle-b.js
var require_cycle_b = __commonJS((exports2) => {
  console.log("eval cycle-b");
  var a = require_cycle_a();
  exports2.seen = "b saw " + a.early + "," + a.late;
  exports2.check = function() {
    return "b later " + a.late;
  };
});

// cycle-a.js
var require_cycle_a = __commonJS((exports2) => {
  console.log("eval cycle-a start");
  exports2.early = "a-early";
  var b = require_cycle_b();
  exports2.late = "a-late";
  exports2.report = function() {
    return b.seen + " " + b.check();
  };
  console.log("eval cycle-a end");
});

// lazy.js
var exports_lazy = {};
__export(exports_lazy, {
  value: () => value,
  default: () => lazy_default
});
var value = 42, lazy_default = "lazy-default";
var init_lazy = __esm(() => {
  console.log("eval lazy");
});

// index.js
var import_chunk = __toESM(require_chunk());
// data.json
var data_default = {
  name: "fixture",
  items: [1, 2, 3, 4, 5]
};

// index.js
var import_cycle_a = __toESM(require_cycle_a());

// util.js
console.log("eval util");
var label = (name) => `[${name.padStart(8, ".")}]`;
function total(list) {
  return list.reduce((sum, n) => sum + n, 0);
}

// index.js
var items = data_default.items;
console.log("eval index");
console.log("last " + items[items.length - 1] + " own " + Object.prototype.hasOwnProperty.call(data_default, "name"));
console.log("replace " + "a-b-c".split("-").join("+"));
console.log("chunk " + JSON.stringify(import_chunk.default(items, 2)));
console.log("cycle " + import_cycle_a.report());
console.log("label " + label(data_default.name) + " total " + total(items));
Promise.resolve().then(() => (init_lazy(), exports_lazy)).then((lazy) => {
  console.log("lazy " + lazy.value + " " + lazy.default);
  return Promise.allSettled([Promise.resolve(1), Promise.reject(new Error("no"))]);
}).then((settled) => console.log("settled " + settled.map((s) => s.status).join(",")));
