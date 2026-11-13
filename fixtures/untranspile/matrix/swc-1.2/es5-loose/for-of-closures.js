"use strict";
var _loop = function(_i, _iter) {
    var v = _iter[_i];
    var doubled = v * 2;
    var note = "n" + v;
    fns.push(function() {
        return doubled + note;
    });
    note += "!";
}, _loop1 = function(_i1, _iter1) {
    var __i = _iter1[_i1], key = __i[0], value = __i[1];
    seen.push(function() {
        return key + value;
    });
}, _loop2 = function(_i2, _iter2) {
    var word = _iter2[_i2];
    var count = 0;
    later.push(function() {
        return word + ++count;
    });
};
var fns = [];
for(var _i = 0, _iter = new Set([
    1,
    2,
    3
]); _i < _iter.length; _i++)_loop(_i, _iter);
console.log(fns.map(function(f) {
    return f();
}).join(","));
var seen = [];
for(var _i1 = 0, _iter1 = new Map([
    [
        "a",
        1
    ],
    [
        "b",
        2
    ]
]); _i1 < _iter1.length; _i1++)_loop1(_i1, _iter1);
console.log(seen.map(function(f) {
    return f();
}).join(","));
var later = [];
for(var _i2 = 0, _iter2 = "abc"; _i2 < _iter2.length; _i2++)_loop2(_i2, _iter2);
console.log(later.map(function(f) {
    return f() + f();
}).join(","));
