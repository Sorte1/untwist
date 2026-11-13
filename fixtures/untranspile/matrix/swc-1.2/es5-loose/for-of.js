"use strict";
var out = [];
for(var _i = 0, _iter = [
    1,
    2,
    3
]; _i < _iter.length; _i++){
    var v = _iter[_i];
    out.push(v * 2);
}
for(var _i1 = 0, _iter1 = "h\xe9llo"; _i1 < _iter1.length; _i1++){
    var ch = _iter1[_i1];
    out.push(ch);
}
for(var _i2 = 0, _iter2 = new Map([
    [
        "a",
        1
    ],
    [
        "b",
        2
    ]
]); _i2 < _iter2.length; _i2++){
    var __i = _iter2[_i2], k = __i[0], v1 = __i[1];
    out.push(k + v1);
}
for(var _i3 = 0, _iter3 = new Set([
    3,
    3,
    4
]); _i3 < _iter3.length; _i3++){
    var v2 = _iter3[_i3];
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
}, _obj);
var got = [];
for(var _i4 = 0; _i4 < custom.length; _i4++){
    var v3 = custom[_i4];
    if (v3 === 3) break;
    got.push(v3);
}
console.log(got.join(","), out[out.length - 1]);
function args() {
    var xs = [];
    for(var _i = 0; _i < arguments.length; _i++){
        var a = arguments[_i];
        xs.push(a);
    }
    return xs.join("+");
}
console.log(args(1, 2, 3));
var total = 0;
outer: for(var _i5 = 0, _iter4 = [
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
]; _i5 < _iter4.length; _i5++){
    var row = _iter4[_i5];
    for(var _i6 = 0; _i6 < row.length; _i6++){
        var cell = row[_i6];
        if (cell === 4) continue outer;
        if (cell === 6) break outer;
        total += cell;
    }
}
console.log(total);
