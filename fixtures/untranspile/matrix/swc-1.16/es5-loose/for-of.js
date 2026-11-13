"use strict";
function _array_like_to_array(arr, len) {
    if (len == null || len > arr.length) len = arr.length;
    for(var i = 0, arr2 = new Array(len); i < len; i++)arr2[i] = arr[i];
    return arr2;
}
function _create_for_of_iterator_helper_loose(o, allowArrayLike) {
    var it = typeof Symbol !== "undefined" && o[Symbol.iterator] || o["@@iterator"];
    if (it) return (it = it.call(o)).next.bind(it);
    if (Array.isArray(o) || (it = _unsupported_iterable_to_array(o)) || allowArrayLike && o && typeof o.length === "number") {
        if (it) o = it;
        var i = 0;
        return function() {
            if (i >= o.length) return {
                done: true
            };
            return {
                done: false,
                value: o[i++]
            };
        };
    }
    throw new TypeError("Invalid attempt to iterate non-iterable instance.\nIn order to be iterable, non-array objects must have a [Symbol.iterator]() method.");
}
function _unsupported_iterable_to_array(o, minLen) {
    if (!o) return;
    if (typeof o === "string") return _array_like_to_array(o, minLen);
    var n = Object.prototype.toString.call(o).slice(8, -1);
    if (n === "Object" && o.constructor) n = o.constructor.name;
    if (n === "Map" || n === "Set") return Array.from(o);
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
for(var _iterator = _create_for_of_iterator_helper_loose("héllo"), _step; !(_step = _iterator()).done;){
    var ch = _step.value;
    out.push(ch);
}
for(var _iterator1 = _create_for_of_iterator_helper_loose(new Map([
    [
        "a",
        1
    ],
    [
        "b",
        2
    ]
])), _step1; !(_step1 = _iterator1()).done;){
    var _step_value = _step1.value, k = _step_value[0], v1 = _step_value[1];
    out.push(k + v1);
}
for(var _iterator2 = _create_for_of_iterator_helper_loose(new Set([
    3,
    3,
    4
])), _step2; !(_step2 = _iterator2()).done;){
    var v2 = _step2.value;
    out.push(v2);
}
console.log(out.join(","));
var _obj;
var custom = (_obj = {
    from: 1,
    to: 4
}, _obj[Symbol.iterator] = function() {
    var cur = this.from;
    var last = this.to;
    return {
        next: function next() {
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
}, _obj);
var got = [];
for(var _iterator3 = _create_for_of_iterator_helper_loose(custom), _step3; !(_step3 = _iterator3()).done;){
    var v3 = _step3.value;
    if (v3 === 3) break;
    got.push(v3);
}
console.log(got.join(","), out[out.length - 1]);
function args() {
    var xs = [];
    for(var _iterator = _create_for_of_iterator_helper_loose(arguments), _step; !(_step = _iterator()).done;){
        var a = _step.value;
        xs.push(a);
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
    for(var _iterator4 = _create_for_of_iterator_helper_loose(row), _step4; !(_step4 = _iterator4()).done;){
        var cell = _step4.value;
        if (cell === 4) continue outer;
        if (cell === 6) break outer;
        total += cell;
    }
}
console.log(total);
