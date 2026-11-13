"use strict";
function _array_like_to_array(arr, len) {
    if (len == null || len > arr.length) len = arr.length;
    for(var i = 0, arr2 = new Array(len); i < len; i++)arr2[i] = arr[i];
    return arr2;
}
function _unsupported_iterable_to_array(o, minLen) {
    if (!o) return;
    if (typeof o === "string") return _array_like_to_array(o, minLen);
    var n = Object.prototype.toString.call(o).slice(8, -1);
    if (n === "Object" && o.constructor) n = o.constructor.name;
    if (n === "Map" || n === "Set") return Array.from(n);
    if (n === "Arguments" || /^(?:Ui|I)nt(?:8|16|32)(?:Clamped)?Array$/.test(n)) return _array_like_to_array(o, minLen);
}
function _create_for_of_iterator_helper_loose(o, allowArrayLike) {
    var it = typeof Symbol !== "undefined" && o[Symbol.iterator] || o["@@iterator"];
    if (it) return (it = it.call(o)).next.bind(it);
    if (Array.isArray(o) || (it = _unsupported_iterable_to_array(o)) || allowArrayLike && o && typeof o.length === "number") {
        if (it) o = it;
        var i = 0;
        return function() {
            if (i >= o.length) {
                return {
                    done: true
                };
            }
            return {
                done: false,
                value: o[i++]
            };
        };
    }
    throw new TypeError("Invalid attempt to iterate non-iterable instance.\\nIn order to be iterable, non-array objects must have a [Symbol.iterator]() method.");
}
var _loop = function() {
    var v = _step.value;
    var doubled = v * 2;
    var note = "n" + v;
    fns.push(function() {
        return doubled + note;
    });
    note += "!";
}, _loop1 = function() {
    var _step_value = _step1.value, key = _step_value[0], value = _step_value[1];
    seen.push(function() {
        return key + value;
    });
}, _loop2 = function() {
    var word = _step2.value;
    var count = 0;
    later.push(function() {
        return word + ++count;
    });
};
var fns = [];
for(var _iterator = _create_for_of_iterator_helper_loose(new Set([
    1,
    2,
    3
])), _step; !(_step = _iterator()).done;)_loop();
console.log(fns.map(function(f) {
    return f();
}).join(","));
var seen = [];
for(var _iterator1 = _create_for_of_iterator_helper_loose(new Map([
    [
        "a",
        1
    ],
    [
        "b",
        2
    ]
])), _step1; !(_step1 = _iterator1()).done;)_loop1();
console.log(seen.map(function(f) {
    return f();
}).join(","));
var later = [];
for(var _iterator2 = _create_for_of_iterator_helper_loose("abc"), _step2; !(_step2 = _iterator2()).done;)_loop2();
console.log(later.map(function(f) {
    return f() + f();
}).join(","));
