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
var calls = [];
var handlers = [
    function() {
        return calls.push("a");
    },
    function() {
        return calls.push("b");
    }
];
for(var _iterator = _create_for_of_iterator_helper_loose(new Set(handlers)), _step; !(_step = _iterator()).done;){
    var handler = _step.value;
    handler();
}
var counts = {};
for(var _iterator1 = _create_for_of_iterator_helper_loose(new Set([
    "x",
    "y",
    "x"
])), _step1; !(_step1 = _iterator1()).done;){
    var key = _step1.value;
    counts[key] = (counts[key] || 0) + 1;
}
for(var _iterator2 = _create_for_of_iterator_helper_loose(new Set([
    {
        list: [
            1,
            2
        ]
    },
    {
        list: [
            3
        ]
    }
])), _step2; !(_step2 = _iterator2()).done;){
    var entry = _step2.value;
    for(var _iterator3 = _create_for_of_iterator_helper_loose(entry.list), _step3; !(_step3 = _iterator3()).done;){
        var n = _step3.value;
        calls.push(n);
    }
}
var removed = {
    a: 1,
    b: 2,
    c: 3
};
for(var _iterator4 = _create_for_of_iterator_helper_loose(new Set([
    "a",
    "c"
])), _step4; !(_step4 = _iterator4()).done;){
    var key1 = _step4.value;
    delete removed[key1];
}
var total = 0;
var weights = {
    p: 2,
    q: 5
};
for(var _iterator5 = _create_for_of_iterator_helper_loose(new Set([
    "p",
    "q",
    "r"
])), _step5; !(_step5 = _iterator5()).done;){
    var name = _step5.value;
    total += weights[name] || 0;
}
for(var _iterator6 = _create_for_of_iterator_helper_loose(new Set([
    {
        run: function run(k) {
            return calls.push(k);
        }
    }
])), _step6; !(_step6 = _iterator6()).done;){
    var item = _step6.value;
    item.run("ran");
}
console.log(calls.join(), JSON.stringify(counts), JSON.stringify(removed), total);
