
function $parcel$interopDefault(a) {
  return a && a.__esModule ? a.default : a;
}

      var $parcel$global = globalThis;
    
var $parcel$modules = {};
var $parcel$inits = {};

var parcelRequire = $parcel$global["parcelRequire9d85"];

if (parcelRequire == null) {
  parcelRequire = function(id) {
    if (id in $parcel$modules) {
      return $parcel$modules[id].exports;
    }
    if (id in $parcel$inits) {
      var init = $parcel$inits[id];
      delete $parcel$inits[id];
      var module = {id: id, exports: {}};
      $parcel$modules[id] = module;
      init.call(module.exports, module, module.exports);
      return module.exports;
    }
    var err = new Error("Cannot find module '" + id + "'");
    err.code = 'MODULE_NOT_FOUND';
    throw err;
  };

  parcelRequire.register = function register(id, init) {
    $parcel$inits[id] = init;
  };

  $parcel$global["parcelRequire9d85"] = parcelRequire;
}

var parcelRegister = parcelRequire.register;
parcelRegister("9vNwv", function(module, exports) {
module.exports = Promise.resolve(require("./lazy.5ceb5e4c.js")).then(()=>parcelRequire("9qF9E"));

});

var $a1109ce096b4022f$exports = {};
var $85f4584a8d631a27$exports = {};
/**
 * The base implementation of `_.slice` without an iteratee call guard.
 *
 * @private
 * @param {Array} array The array to slice.
 * @param {number} [start=0] The start position.
 * @param {number} [end=array.length] The end position.
 * @returns {Array} Returns the slice of `array`.
 */ function $85f4584a8d631a27$var$baseSlice(array, start, end) {
    var index = -1, length = array.length;
    if (start < 0) start = -start > length ? 0 : length + start;
    end = end > length ? length : end;
    if (end < 0) end += length;
    length = start > end ? 0 : end - start >>> 0;
    start >>>= 0;
    var result = Array(length);
    while(++index < length)result[index] = array[index + start];
    return result;
}
$85f4584a8d631a27$exports = $85f4584a8d631a27$var$baseSlice;


var $ac5fe7bd3f4e1559$exports = {};
var $1d3ddc8897b3fe1d$exports = {};
/**
 * Performs a
 * [`SameValueZero`](http://ecma-international.org/ecma-262/7.0/#sec-samevaluezero)
 * comparison between two values to determine if they are equivalent.
 *
 * @static
 * @memberOf _
 * @since 4.0.0
 * @category Lang
 * @param {*} value The value to compare.
 * @param {*} other The other value to compare.
 * @returns {boolean} Returns `true` if the values are equivalent, else `false`.
 * @example
 *
 * var object = { 'a': 1 };
 * var other = { 'a': 1 };
 *
 * _.eq(object, object);
 * // => true
 *
 * _.eq(object, other);
 * // => false
 *
 * _.eq('a', 'a');
 * // => true
 *
 * _.eq('a', Object('a'));
 * // => false
 *
 * _.eq(NaN, NaN);
 * // => true
 */ function $1d3ddc8897b3fe1d$var$eq(value, other) {
    return value === other || value !== value && other !== other;
}
$1d3ddc8897b3fe1d$exports = $1d3ddc8897b3fe1d$var$eq;


var $4e84d8991d2d7795$exports = {};
var $065148dbaaa90171$exports = {};
var $469365a965340e3a$exports = {};
var $af06eef65d40956b$exports = {};
var $fb58540d8e2ef6c1$exports = {};
var $0bed09df65671243$exports = {};
/** Detect free variable `global` from Node.js. */ var $0bed09df65671243$var$freeGlobal = typeof $parcel$global == "object" && $parcel$global && $parcel$global.Object === Object && $parcel$global;
$0bed09df65671243$exports = $0bed09df65671243$var$freeGlobal;


/** Detect free variable `self`. */ var $fb58540d8e2ef6c1$var$freeSelf = typeof self == "object" && self && self.Object === Object && self;
/** Used as a reference to the global object. */ var $fb58540d8e2ef6c1$var$root = $0bed09df65671243$exports || $fb58540d8e2ef6c1$var$freeSelf || Function("return this")();
$fb58540d8e2ef6c1$exports = $fb58540d8e2ef6c1$var$root;


/** Built-in value references. */ var $af06eef65d40956b$var$Symbol = $fb58540d8e2ef6c1$exports.Symbol;
$af06eef65d40956b$exports = $af06eef65d40956b$var$Symbol;


var $017a22860023562f$exports = {};

/** Used for built-in method references. */ var $017a22860023562f$var$objectProto = Object.prototype;
/** Used to check objects for own properties. */ var $017a22860023562f$var$hasOwnProperty = $017a22860023562f$var$objectProto.hasOwnProperty;
/**
 * Used to resolve the
 * [`toStringTag`](http://ecma-international.org/ecma-262/7.0/#sec-object.prototype.tostring)
 * of values.
 */ var $017a22860023562f$var$nativeObjectToString = $017a22860023562f$var$objectProto.toString;
/** Built-in value references. */ var $017a22860023562f$var$symToStringTag = $af06eef65d40956b$exports ? $af06eef65d40956b$exports.toStringTag : undefined;
/**
 * A specialized version of `baseGetTag` which ignores `Symbol.toStringTag` values.
 *
 * @private
 * @param {*} value The value to query.
 * @returns {string} Returns the raw `toStringTag`.
 */ function $017a22860023562f$var$getRawTag(value) {
    var isOwn = $017a22860023562f$var$hasOwnProperty.call(value, $017a22860023562f$var$symToStringTag), tag = value[$017a22860023562f$var$symToStringTag];
    try {
        value[$017a22860023562f$var$symToStringTag] = undefined;
        var unmasked = true;
    } catch (e) {}
    var result = $017a22860023562f$var$nativeObjectToString.call(value);
    if (unmasked) {
        if (isOwn) value[$017a22860023562f$var$symToStringTag] = tag;
        else delete value[$017a22860023562f$var$symToStringTag];
    }
    return result;
}
$017a22860023562f$exports = $017a22860023562f$var$getRawTag;


var $cf8be8c18687f744$exports = {};
/** Used for built-in method references. */ var $cf8be8c18687f744$var$objectProto = Object.prototype;
/**
 * Used to resolve the
 * [`toStringTag`](http://ecma-international.org/ecma-262/7.0/#sec-object.prototype.tostring)
 * of values.
 */ var $cf8be8c18687f744$var$nativeObjectToString = $cf8be8c18687f744$var$objectProto.toString;
/**
 * Converts `value` to a string using `Object.prototype.toString`.
 *
 * @private
 * @param {*} value The value to convert.
 * @returns {string} Returns the converted string.
 */ function $cf8be8c18687f744$var$objectToString(value) {
    return $cf8be8c18687f744$var$nativeObjectToString.call(value);
}
$cf8be8c18687f744$exports = $cf8be8c18687f744$var$objectToString;


/** `Object#toString` result references. */ var $469365a965340e3a$var$nullTag = "[object Null]", $469365a965340e3a$var$undefinedTag = "[object Undefined]";
/** Built-in value references. */ var $469365a965340e3a$var$symToStringTag = $af06eef65d40956b$exports ? $af06eef65d40956b$exports.toStringTag : undefined;
/**
 * The base implementation of `getTag` without fallbacks for buggy environments.
 *
 * @private
 * @param {*} value The value to query.
 * @returns {string} Returns the `toStringTag`.
 */ function $469365a965340e3a$var$baseGetTag(value) {
    if (value == null) return value === undefined ? $469365a965340e3a$var$undefinedTag : $469365a965340e3a$var$nullTag;
    return $469365a965340e3a$var$symToStringTag && $469365a965340e3a$var$symToStringTag in Object(value) ? $017a22860023562f$exports(value) : $cf8be8c18687f744$exports(value);
}
$469365a965340e3a$exports = $469365a965340e3a$var$baseGetTag;


var $6699f190917f4d38$exports = {};
/**
 * Checks if `value` is the
 * [language type](http://www.ecma-international.org/ecma-262/7.0/#sec-ecmascript-language-types)
 * of `Object`. (e.g. arrays, functions, objects, regexes, `new Number(0)`, and `new String('')`)
 *
 * @static
 * @memberOf _
 * @since 0.1.0
 * @category Lang
 * @param {*} value The value to check.
 * @returns {boolean} Returns `true` if `value` is an object, else `false`.
 * @example
 *
 * _.isObject({});
 * // => true
 *
 * _.isObject([1, 2, 3]);
 * // => true
 *
 * _.isObject(_.noop);
 * // => true
 *
 * _.isObject(null);
 * // => false
 */ function $6699f190917f4d38$var$isObject(value) {
    var type = typeof value;
    return value != null && (type == "object" || type == "function");
}
$6699f190917f4d38$exports = $6699f190917f4d38$var$isObject;


/** `Object#toString` result references. */ var $065148dbaaa90171$var$asyncTag = "[object AsyncFunction]", $065148dbaaa90171$var$funcTag = "[object Function]", $065148dbaaa90171$var$genTag = "[object GeneratorFunction]", $065148dbaaa90171$var$proxyTag = "[object Proxy]";
/**
 * Checks if `value` is classified as a `Function` object.
 *
 * @static
 * @memberOf _
 * @since 0.1.0
 * @category Lang
 * @param {*} value The value to check.
 * @returns {boolean} Returns `true` if `value` is a function, else `false`.
 * @example
 *
 * _.isFunction(_);
 * // => true
 *
 * _.isFunction(/abc/);
 * // => false
 */ function $065148dbaaa90171$var$isFunction(value) {
    if (!$6699f190917f4d38$exports(value)) return false;
    // The use of `Object#toString` avoids issues with the `typeof` operator
    // in Safari 9 which returns 'object' for typed arrays and other constructors.
    var tag = $469365a965340e3a$exports(value);
    return tag == $065148dbaaa90171$var$funcTag || tag == $065148dbaaa90171$var$genTag || tag == $065148dbaaa90171$var$asyncTag || tag == $065148dbaaa90171$var$proxyTag;
}
$065148dbaaa90171$exports = $065148dbaaa90171$var$isFunction;


var $11dda4c8974afc1e$exports = {};
/** Used as references for various `Number` constants. */ var $11dda4c8974afc1e$var$MAX_SAFE_INTEGER = 9007199254740991;
/**
 * Checks if `value` is a valid array-like length.
 *
 * **Note:** This method is loosely based on
 * [`ToLength`](http://ecma-international.org/ecma-262/7.0/#sec-tolength).
 *
 * @static
 * @memberOf _
 * @since 4.0.0
 * @category Lang
 * @param {*} value The value to check.
 * @returns {boolean} Returns `true` if `value` is a valid length, else `false`.
 * @example
 *
 * _.isLength(3);
 * // => true
 *
 * _.isLength(Number.MIN_VALUE);
 * // => false
 *
 * _.isLength(Infinity);
 * // => false
 *
 * _.isLength('3');
 * // => false
 */ function $11dda4c8974afc1e$var$isLength(value) {
    return typeof value == "number" && value > -1 && value % 1 == 0 && value <= $11dda4c8974afc1e$var$MAX_SAFE_INTEGER;
}
$11dda4c8974afc1e$exports = $11dda4c8974afc1e$var$isLength;


/**
 * Checks if `value` is array-like. A value is considered array-like if it's
 * not a function and has a `value.length` that's an integer greater than or
 * equal to `0` and less than or equal to `Number.MAX_SAFE_INTEGER`.
 *
 * @static
 * @memberOf _
 * @since 4.0.0
 * @category Lang
 * @param {*} value The value to check.
 * @returns {boolean} Returns `true` if `value` is array-like, else `false`.
 * @example
 *
 * _.isArrayLike([1, 2, 3]);
 * // => true
 *
 * _.isArrayLike(document.body.children);
 * // => true
 *
 * _.isArrayLike('abc');
 * // => true
 *
 * _.isArrayLike(_.noop);
 * // => false
 */ function $4e84d8991d2d7795$var$isArrayLike(value) {
    return value != null && $11dda4c8974afc1e$exports(value.length) && !$065148dbaaa90171$exports(value);
}
$4e84d8991d2d7795$exports = $4e84d8991d2d7795$var$isArrayLike;


var $ea7843c2d32f111b$exports = {};
/** Used as references for various `Number` constants. */ var $ea7843c2d32f111b$var$MAX_SAFE_INTEGER = 9007199254740991;
/** Used to detect unsigned integer values. */ var $ea7843c2d32f111b$var$reIsUint = /^(?:0|[1-9]\d*)$/;
/**
 * Checks if `value` is a valid array-like index.
 *
 * @private
 * @param {*} value The value to check.
 * @param {number} [length=MAX_SAFE_INTEGER] The upper bounds of a valid index.
 * @returns {boolean} Returns `true` if `value` is a valid index, else `false`.
 */ function $ea7843c2d32f111b$var$isIndex(value, length) {
    var type = typeof value;
    length = length == null ? $ea7843c2d32f111b$var$MAX_SAFE_INTEGER : length;
    return !!length && (type == "number" || type != "symbol" && $ea7843c2d32f111b$var$reIsUint.test(value)) && value > -1 && value % 1 == 0 && value < length;
}
$ea7843c2d32f111b$exports = $ea7843c2d32f111b$var$isIndex;



/**
 * Checks if the given arguments are from an iteratee call.
 *
 * @private
 * @param {*} value The potential iteratee value argument.
 * @param {*} index The potential iteratee index or key argument.
 * @param {*} object The potential iteratee object argument.
 * @returns {boolean} Returns `true` if the arguments are from an iteratee call,
 *  else `false`.
 */ function $ac5fe7bd3f4e1559$var$isIterateeCall(value, index, object) {
    if (!$6699f190917f4d38$exports(object)) return false;
    var type = typeof index;
    if (type == "number" ? $4e84d8991d2d7795$exports(object) && $ea7843c2d32f111b$exports(index, object.length) : type == "string" && index in object) return $1d3ddc8897b3fe1d$exports(object[index], value);
    return false;
}
$ac5fe7bd3f4e1559$exports = $ac5fe7bd3f4e1559$var$isIterateeCall;


var $b0a30580fa263d70$exports = {};
var $c7ed7aae9f8eac01$exports = {};
var $4a11240579977c3b$exports = {};
var $ceb3c41204102cb6$exports = {};
var $395cefffc6d7e101$exports = {};
/** Used to match a single whitespace character. */ var $395cefffc6d7e101$var$reWhitespace = /\s/;
/**
 * Used by `_.trim` and `_.trimEnd` to get the index of the last non-whitespace
 * character of `string`.
 *
 * @private
 * @param {string} string The string to inspect.
 * @returns {number} Returns the index of the last non-whitespace character.
 */ function $395cefffc6d7e101$var$trimmedEndIndex(string) {
    var index = string.length;
    while(index-- && $395cefffc6d7e101$var$reWhitespace.test(string.charAt(index)));
    return index;
}
$395cefffc6d7e101$exports = $395cefffc6d7e101$var$trimmedEndIndex;


/** Used to match leading whitespace. */ var $ceb3c41204102cb6$var$reTrimStart = /^\s+/;
/**
 * The base implementation of `_.trim`.
 *
 * @private
 * @param {string} string The string to trim.
 * @returns {string} Returns the trimmed string.
 */ function $ceb3c41204102cb6$var$baseTrim(string) {
    return string ? string.slice(0, $395cefffc6d7e101$exports(string) + 1).replace($ceb3c41204102cb6$var$reTrimStart, "") : string;
}
$ceb3c41204102cb6$exports = $ceb3c41204102cb6$var$baseTrim;



var $a3be48a0f86ca31e$exports = {};

var $b25d60e96a9fb77c$exports = {};
/**
 * Checks if `value` is object-like. A value is object-like if it's not `null`
 * and has a `typeof` result of "object".
 *
 * @static
 * @memberOf _
 * @since 4.0.0
 * @category Lang
 * @param {*} value The value to check.
 * @returns {boolean} Returns `true` if `value` is object-like, else `false`.
 * @example
 *
 * _.isObjectLike({});
 * // => true
 *
 * _.isObjectLike([1, 2, 3]);
 * // => true
 *
 * _.isObjectLike(_.noop);
 * // => false
 *
 * _.isObjectLike(null);
 * // => false
 */ function $b25d60e96a9fb77c$var$isObjectLike(value) {
    return value != null && typeof value == "object";
}
$b25d60e96a9fb77c$exports = $b25d60e96a9fb77c$var$isObjectLike;


/** `Object#toString` result references. */ var $a3be48a0f86ca31e$var$symbolTag = "[object Symbol]";
/**
 * Checks if `value` is classified as a `Symbol` primitive or object.
 *
 * @static
 * @memberOf _
 * @since 4.0.0
 * @category Lang
 * @param {*} value The value to check.
 * @returns {boolean} Returns `true` if `value` is a symbol, else `false`.
 * @example
 *
 * _.isSymbol(Symbol.iterator);
 * // => true
 *
 * _.isSymbol('abc');
 * // => false
 */ function $a3be48a0f86ca31e$var$isSymbol(value) {
    return typeof value == "symbol" || $b25d60e96a9fb77c$exports(value) && $469365a965340e3a$exports(value) == $a3be48a0f86ca31e$var$symbolTag;
}
$a3be48a0f86ca31e$exports = $a3be48a0f86ca31e$var$isSymbol;


/** Used as references for various `Number` constants. */ var $4a11240579977c3b$var$NAN = 0 / 0;
/** Used to detect bad signed hexadecimal string values. */ var $4a11240579977c3b$var$reIsBadHex = /^[-+]0x[0-9a-f]+$/i;
/** Used to detect binary string values. */ var $4a11240579977c3b$var$reIsBinary = /^0b[01]+$/i;
/** Used to detect octal string values. */ var $4a11240579977c3b$var$reIsOctal = /^0o[0-7]+$/i;
/** Built-in method references without a dependency on `root`. */ var $4a11240579977c3b$var$freeParseInt = parseInt;
/**
 * Converts `value` to a number.
 *
 * @static
 * @memberOf _
 * @since 4.0.0
 * @category Lang
 * @param {*} value The value to process.
 * @returns {number} Returns the number.
 * @example
 *
 * _.toNumber(3.2);
 * // => 3.2
 *
 * _.toNumber(Number.MIN_VALUE);
 * // => 5e-324
 *
 * _.toNumber(Infinity);
 * // => Infinity
 *
 * _.toNumber('3.2');
 * // => 3.2
 */ function $4a11240579977c3b$var$toNumber(value) {
    if (typeof value == "number") return value;
    if ($a3be48a0f86ca31e$exports(value)) return $4a11240579977c3b$var$NAN;
    if ($6699f190917f4d38$exports(value)) {
        var other = typeof value.valueOf == "function" ? value.valueOf() : value;
        value = $6699f190917f4d38$exports(other) ? other + "" : other;
    }
    if (typeof value != "string") return value === 0 ? value : +value;
    value = $ceb3c41204102cb6$exports(value);
    var isBinary = $4a11240579977c3b$var$reIsBinary.test(value);
    return isBinary || $4a11240579977c3b$var$reIsOctal.test(value) ? $4a11240579977c3b$var$freeParseInt(value.slice(2), isBinary ? 2 : 8) : $4a11240579977c3b$var$reIsBadHex.test(value) ? $4a11240579977c3b$var$NAN : +value;
}
$4a11240579977c3b$exports = $4a11240579977c3b$var$toNumber;


/** Used as references for various `Number` constants. */ var $c7ed7aae9f8eac01$var$INFINITY = 1 / 0, $c7ed7aae9f8eac01$var$MAX_INTEGER = 1.7976931348623157e+308;
/**
 * Converts `value` to a finite number.
 *
 * @static
 * @memberOf _
 * @since 4.12.0
 * @category Lang
 * @param {*} value The value to convert.
 * @returns {number} Returns the converted number.
 * @example
 *
 * _.toFinite(3.2);
 * // => 3.2
 *
 * _.toFinite(Number.MIN_VALUE);
 * // => 5e-324
 *
 * _.toFinite(Infinity);
 * // => 1.7976931348623157e+308
 *
 * _.toFinite('3.2');
 * // => 3.2
 */ function $c7ed7aae9f8eac01$var$toFinite(value) {
    if (!value) return value === 0 ? value : 0;
    value = $4a11240579977c3b$exports(value);
    if (value === $c7ed7aae9f8eac01$var$INFINITY || value === -$c7ed7aae9f8eac01$var$INFINITY) {
        var sign = value < 0 ? -1 : 1;
        return sign * $c7ed7aae9f8eac01$var$MAX_INTEGER;
    }
    return value === value ? value : 0;
}
$c7ed7aae9f8eac01$exports = $c7ed7aae9f8eac01$var$toFinite;


/**
 * Converts `value` to an integer.
 *
 * **Note:** This method is loosely based on
 * [`ToInteger`](http://www.ecma-international.org/ecma-262/7.0/#sec-tointeger).
 *
 * @static
 * @memberOf _
 * @since 4.0.0
 * @category Lang
 * @param {*} value The value to convert.
 * @returns {number} Returns the converted integer.
 * @example
 *
 * _.toInteger(3.2);
 * // => 3
 *
 * _.toInteger(Number.MIN_VALUE);
 * // => 0
 *
 * _.toInteger(Infinity);
 * // => 1.7976931348623157e+308
 *
 * _.toInteger('3.2');
 * // => 3
 */ function $b0a30580fa263d70$var$toInteger(value) {
    var result = $c7ed7aae9f8eac01$exports(value), remainder = result % 1;
    return result === result ? remainder ? result - remainder : result : 0;
}
$b0a30580fa263d70$exports = $b0a30580fa263d70$var$toInteger;


/* Built-in method references for those with the same name as other `lodash` methods. */ var $a1109ce096b4022f$var$nativeCeil = Math.ceil, $a1109ce096b4022f$var$nativeMax = Math.max;
/**
 * Creates an array of elements split into groups the length of `size`.
 * If `array` can't be split evenly, the final chunk will be the remaining
 * elements.
 *
 * @static
 * @memberOf _
 * @since 3.0.0
 * @category Array
 * @param {Array} array The array to process.
 * @param {number} [size=1] The length of each chunk
 * @param- {Object} [guard] Enables use as an iteratee for methods like `_.map`.
 * @returns {Array} Returns the new array of chunks.
 * @example
 *
 * _.chunk(['a', 'b', 'c', 'd'], 2);
 * // => [['a', 'b'], ['c', 'd']]
 *
 * _.chunk(['a', 'b', 'c', 'd'], 3);
 * // => [['a', 'b', 'c'], ['d']]
 */ function $a1109ce096b4022f$var$chunk(array, size, guard) {
    if (guard ? $ac5fe7bd3f4e1559$exports(array, size, guard) : size === undefined) size = 1;
    else size = $a1109ce096b4022f$var$nativeMax($b0a30580fa263d70$exports(size), 0);
    var length = array == null ? 0 : array.length;
    if (!length || size < 1) return [];
    var index = 0, resIndex = 0, result = Array($a1109ce096b4022f$var$nativeCeil(length / size));
    while(index < length)result[resIndex++] = $85f4584a8d631a27$exports(array, index, index += size);
    return result;
}
$a1109ce096b4022f$exports = $a1109ce096b4022f$var$chunk;


var $35c118d34342a1c7$exports = {};
$35c118d34342a1c7$exports = JSON.parse('{"name":"fixture","items":[1,2,3,4,5]}');


var $1ce139439daf8b97$export$a20a50f149c62f11;
var $1ce139439daf8b97$export$1ee8ee30835eab8b;
var $1ce139439daf8b97$export$f18d5b0af29128b7;
console.log("eval cycle-a start");
$1ce139439daf8b97$export$a20a50f149c62f11 = "a-early";
var $ffa1a2cf627ac571$export$156ac52bf2ea2b93;
var $ffa1a2cf627ac571$export$35dc45686fc2dbd7;
console.log("eval cycle-b");

$ffa1a2cf627ac571$export$156ac52bf2ea2b93 = "b saw " + $1ce139439daf8b97$export$a20a50f149c62f11 + "," + $1ce139439daf8b97$export$1ee8ee30835eab8b;
$ffa1a2cf627ac571$export$35dc45686fc2dbd7 = function() {
    return "b later " + $1ce139439daf8b97$export$1ee8ee30835eab8b;
};


$1ce139439daf8b97$export$1ee8ee30835eab8b = "a-late";
$1ce139439daf8b97$export$f18d5b0af29128b7 = function() {
    return $ffa1a2cf627ac571$export$156ac52bf2ea2b93 + " " + $ffa1a2cf627ac571$export$35dc45686fc2dbd7();
};
console.log("eval cycle-a end");


console.log("eval util");
const $69dd675f95d0e129$export$1237798dc640739a = (name)=>`[${name.padStart(8, ".")}]`;
function $69dd675f95d0e129$export$9086bb9c16b2b5b(list) {
    return list.reduce((sum, n)=>sum + n, 0);
}


const $cc9b0c7af45ee0df$var$items = (0, (/*@__PURE__*/$parcel$interopDefault($35c118d34342a1c7$exports))).items;
console.log("eval index");
console.log("last " + $cc9b0c7af45ee0df$var$items[$cc9b0c7af45ee0df$var$items.length - 1] + " own " + Object.prototype.hasOwnProperty.call((0, (/*@__PURE__*/$parcel$interopDefault($35c118d34342a1c7$exports))), "name"));
console.log("replace " + "a-b-c".split("-").join("+"));
console.log("chunk " + JSON.stringify((0, (/*@__PURE__*/$parcel$interopDefault($a1109ce096b4022f$exports)))($cc9b0c7af45ee0df$var$items, 2)));
console.log("cycle " + (0, $1ce139439daf8b97$export$f18d5b0af29128b7)());
console.log("label " + (0, $69dd675f95d0e129$export$1237798dc640739a)((0, (/*@__PURE__*/$parcel$interopDefault($35c118d34342a1c7$exports))).name) + " total " + (0, $69dd675f95d0e129$export$9086bb9c16b2b5b)($cc9b0c7af45ee0df$var$items));

(parcelRequire("9vNwv")).then((lazy)=>{
    console.log("lazy " + lazy.value + " " + lazy.default);
    return Promise.allSettled([
        Promise.resolve(1),
        Promise.reject(new Error("no"))
    ]);
}).then((settled)=>console.log("settled " + settled.map((s)=>s.status).join(",")));


