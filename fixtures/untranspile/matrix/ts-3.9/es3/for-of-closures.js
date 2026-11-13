"use strict";
var fns = [];
var _loop_1 = function (v) {
    var doubled = v * 2;
    var note = "n" + v;
    fns.push(function () { return doubled + note; });
    note += "!";
};
for (var _i = 0, _a = new Set([1, 2, 3]); _i < _a.length; _i++) {
    var v = _a[_i];
    _loop_1(v);
}
console.log(fns.map(function (f) { return f(); }).join(","));
var seen = [];
var _loop_2 = function (key, value) {
    seen.push(function () { return key + value; });
};
for (var _b = 0, _c = new Map([["a", 1], ["b", 2]]); _b < _c.length; _b++) {
    var _d = _c[_b], key = _d[0], value = _d[1];
    _loop_2(key, value);
}
console.log(seen.map(function (f) { return f(); }).join(","));
var later = [];
var _loop_3 = function (word) {
    var count = 0;
    later.push(function () { return word + ++count; });
};
for (var _e = 0, _f = "abc"; _e < _f.length; _e++) {
    var word = _f[_e];
    _loop_3(word);
}
console.log(later.map(function (f) { return f() + f(); }).join(","));
