"use strict";
var _a;
var out = [];
for (var _i = 0, _b = [1, 2, 3]; _i < _b.length; _i++) {
    var v = _b[_i];
    out.push(v * 2);
}
for (var _c = 0, _d = "héllo"; _c < _d.length; _c++) {
    var ch = _d[_c];
    out.push(ch);
}
for (var _e = 0, _f = new Map([["a", 1], ["b", 2]]); _e < _f.length; _e++) {
    var _g = _f[_e], k = _g[0], v = _g[1];
    out.push(k + v);
}
for (var _h = 0, _j = new Set([3, 3, 4]); _h < _j.length; _h++) {
    var v = _j[_h];
    out.push(v);
}
console.log(out.join(","));
var custom = (_a = {
        from: 1,
        to: 4
    },
    _a[Symbol.iterator] = function () {
        var cur = this.from;
        var last = this.to;
        return {
            next: function () { return (cur <= last ? { value: cur++, done: false } : { value: undefined, done: true }); },
            "return": function () {
                out.push("closed");
                return { done: true };
            }
        };
    },
    _a);
var got = [];
for (var _k = 0, custom_1 = custom; _k < custom_1.length; _k++) {
    var v = custom_1[_k];
    if (v === 3)
        break;
    got.push(v);
}
console.log(got.join(","), out[out.length - 1]);
function args() {
    var xs = [];
    for (var _i = 0, arguments_1 = arguments; _i < arguments_1.length; _i++) {
        var a = arguments_1[_i];
        xs.push(a);
    }
    return xs.join("+");
}
console.log(args(1, 2, 3));
var total = 0;
outer: for (var _l = 0, _m = [[1, 2], [3, 4], [5, 6]]; _l < _m.length; _l++) {
    var row = _m[_l];
    for (var _o = 0, row_1 = row; _o < row_1.length; _o++) {
        var cell = row_1[_o];
        if (cell === 4)
            continue outer;
        if (cell === 6)
            break outer;
        total += cell;
    }
}
console.log(total);
