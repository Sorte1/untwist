"use strict";
function _array_like_to_array(arr, len) {
    if (len == null || len > arr.length) len = arr.length;
    for(var i = 0, arr2 = new Array(len); i < len; i++)arr2[i] = arr[i];
    return arr2;
}
function _array_with_holes(arr) {
    if (Array.isArray(arr)) return arr;
}
function _define_property(obj, key, value) {
    if (key in obj) {
        Object.defineProperty(obj, key, {
            value: value,
            enumerable: true,
            configurable: true,
            writable: true
        });
    } else {
        obj[key] = value;
    }
    return obj;
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
var out = [];
for(var _i = 0, _iter = [
    1,
    2,
    3
]; _i < _iter.length; _i++){
    var v = _iter[_i];
    out.push(v * 2);
}
var _iteratorNormalCompletion = true, _didIteratorError = false, _iteratorError = undefined;
try {
    for(var _iterator = "h\xe9llo"[Symbol.iterator](), _step; !(_iteratorNormalCompletion = (_step = _iterator.next()).done); _iteratorNormalCompletion = true){
        var ch = _step.value;
        out.push(ch);
    }
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
var _iteratorNormalCompletion1 = true, _didIteratorError1 = false, _iteratorError1 = undefined;
try {
    for(var _iterator1 = new Map([
        [
            "a",
            1
        ],
        [
            "b",
            2
        ]
    ])[Symbol.iterator](), _step1; !(_iteratorNormalCompletion1 = (_step1 = _iterator1.next()).done); _iteratorNormalCompletion1 = true){
        var _step_value = _sliced_to_array(_step1.value, 2), k = _step_value[0], v1 = _step_value[1];
        out.push(k + v1);
    }
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
var _iteratorNormalCompletion2 = true, _didIteratorError2 = false, _iteratorError2 = undefined;
try {
    for(var _iterator2 = new Set([
        3,
        3,
        4
    ])[Symbol.iterator](), _step2; !(_iteratorNormalCompletion2 = (_step2 = _iterator2.next()).done); _iteratorNormalCompletion2 = true){
        var v2 = _step2.value;
        out.push(v2);
    }
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
console.log(out.join(","));
var custom = _define_property({
    from: 1,
    to: 4
}, Symbol.iterator, function() {
    var cur = this.from;
    var last = this.to;
    return {
        next: function() {
            return cur <= last ? {
                value: cur++,
                done: false
            } : {
                value: undefined,
                done: true
            };
        },
        return: function _return() {
            out.push("closed");
            return {
                done: true
            };
        }
    };
});
var got = [];
var _iteratorNormalCompletion3 = true, _didIteratorError3 = false, _iteratorError3 = undefined;
try {
    for(var _iterator3 = custom[Symbol.iterator](), _step3; !(_iteratorNormalCompletion3 = (_step3 = _iterator3.next()).done); _iteratorNormalCompletion3 = true){
        var v3 = _step3.value;
        if (v3 === 3) break;
        got.push(v3);
    }
} catch (err) {
    _didIteratorError3 = true;
    _iteratorError3 = err;
} finally{
    try {
        if (!_iteratorNormalCompletion3 && _iterator3.return != null) {
            _iterator3.return();
        }
    } finally{
        if (_didIteratorError3) {
            throw _iteratorError3;
        }
    }
}
console.log(got.join(","), out[out.length - 1]);
function args() {
    var xs = [];
    var _iteratorNormalCompletion = true, _didIteratorError = false, _iteratorError = undefined;
    try {
        for(var _iterator = arguments[Symbol.iterator](), _step; !(_iteratorNormalCompletion = (_step = _iterator.next()).done); _iteratorNormalCompletion = true){
            var a = _step.value;
            xs.push(a);
        }
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
    return xs.join("+");
}
console.log(args(1, 2, 3));
var total = 0;
outer: for(var _i1 = 0, _iter1 = [
    [
        1,
        2
    ],
    [
        3,
        4
    ],
    [
        5,
        6
    ]
]; _i1 < _iter1.length; _i1++){
    var row = _iter1[_i1];
    var _iteratorNormalCompletion4 = true, _didIteratorError4 = false, _iteratorError4 = undefined;
    try {
        for(var _iterator4 = row[Symbol.iterator](), _step4; !(_iteratorNormalCompletion4 = (_step4 = _iterator4.next()).done); _iteratorNormalCompletion4 = true){
            var cell = _step4.value;
            if (cell === 4) continue outer;
            if (cell === 6) break outer;
            total += cell;
        }
    } catch (err) {
        _didIteratorError4 = true;
        _iteratorError4 = err;
    } finally{
        try {
            if (!_iteratorNormalCompletion4 && _iterator4.return != null) {
                _iterator4.return();
            }
        } finally{
            if (_didIteratorError4) {
                throw _iteratorError4;
            }
        }
    }
}
console.log(total);
