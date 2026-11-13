"use strict";
function _array_like_to_array(arr, len) {
    if (len == null || len > arr.length) len = arr.length;
    for(var i = 0, arr2 = new Array(len); i < len; i++)arr2[i] = arr[i];
    return arr2;
}
function _array_with_holes(arr) {
    if (Array.isArray(arr)) return arr;
}
function _iterable_to_array_limit(arr, i) {
    var _i = arr == null ? null : typeof Symbol !== "undefined" && arr[Symbol.iterator] || arr["@@iterator"];
    if (_i == null) return;
    var _arr = [];
    var _n = true;
    var _d = false;
    var _s, _e;
    try {
        for(_i = _i.call(arr); !(_n = (_s = _i.next()).done); _n = true){
            _arr.push(_s.value);
            if (i && _arr.length === i) break;
        }
    } catch (err) {
        _d = true;
        _e = err;
    } finally{
        try {
            if (!_n && _i["return"] != null) _i["return"]();
        } finally{
            if (_d) throw _e;
        }
    }
    return _arr;
}
function _non_iterable_rest() {
    throw new TypeError("Invalid attempt to destructure non-iterable instance.\\nIn order to be iterable, non-array objects must have a [Symbol.iterator]() method.");
}
function _sliced_to_array(arr, i) {
    return _array_with_holes(arr) || _iterable_to_array_limit(arr, i) || _unsupported_iterable_to_array(arr, i) || _non_iterable_rest();
}
function _unsupported_iterable_to_array(o, minLen) {
    if (!o) return;
    if (typeof o === "string") return _array_like_to_array(o, minLen);
    var n = Object.prototype.toString.call(o).slice(8, -1);
    if (n === "Object" && o.constructor) n = o.constructor.name;
    if (n === "Map" || n === "Set") return Array.from(n);
    if (n === "Arguments" || /^(?:Ui|I)nt(?:8|16|32)(?:Clamped)?Array$/.test(n)) return _array_like_to_array(o, minLen);
}
var fns = [];
var _iteratorNormalCompletion = true, _didIteratorError = false, _iteratorError = undefined;
try {
    var _loop = function() {
        var v = _step.value;
        var doubled = v * 2;
        var note = "n" + v;
        fns.push(function() {
            return doubled + note;
        });
        note += "!";
    };
    for(var _iterator = new Set([
        1,
        2,
        3
    ])[Symbol.iterator](), _step; !(_iteratorNormalCompletion = (_step = _iterator.next()).done); _iteratorNormalCompletion = true)_loop();
} catch (err) {
    _didIteratorError = true;
    _iteratorError = err;
} finally{
    try {
        if (!_iteratorNormalCompletion && _iterator.return != null) {
            _iterator.return();
        }
    } finally{
        if (_didIteratorError) {
            throw _iteratorError;
        }
    }
}
console.log(fns.map(function(f) {
    return f();
}).join(","));
var seen = [];
var _iteratorNormalCompletion1 = true, _didIteratorError1 = false, _iteratorError1 = undefined;
try {
    var _loop1 = function() {
        var _step_value = _sliced_to_array(_step1.value, 2), key = _step_value[0], value = _step_value[1];
        seen.push(function() {
            return key + value;
        });
    };
    for(var _iterator1 = new Map([
        [
            "a",
            1
        ],
        [
            "b",
            2
        ]
    ])[Symbol.iterator](), _step1; !(_iteratorNormalCompletion1 = (_step1 = _iterator1.next()).done); _iteratorNormalCompletion1 = true)_loop1();
} catch (err) {
    _didIteratorError1 = true;
    _iteratorError1 = err;
} finally{
    try {
        if (!_iteratorNormalCompletion1 && _iterator1.return != null) {
            _iterator1.return();
        }
    } finally{
        if (_didIteratorError1) {
            throw _iteratorError1;
        }
    }
}
console.log(seen.map(function(f) {
    return f();
}).join(","));
var later = [];
var _iteratorNormalCompletion2 = true, _didIteratorError2 = false, _iteratorError2 = undefined;
try {
    var _loop2 = function() {
        var word = _step2.value;
        var count = 0;
        later.push(function() {
            return word + ++count;
        });
    };
    for(var _iterator2 = "abc"[Symbol.iterator](), _step2; !(_iteratorNormalCompletion2 = (_step2 = _iterator2.next()).done); _iteratorNormalCompletion2 = true)_loop2();
} catch (err) {
    _didIteratorError2 = true;
    _iteratorError2 = err;
} finally{
    try {
        if (!_iteratorNormalCompletion2 && _iterator2.return != null) {
            _iterator2.return();
        }
    } finally{
        if (_didIteratorError2) {
            throw _iteratorError2;
        }
    }
}
console.log(later.map(function(f) {
    return f() + f();
}).join(","));
