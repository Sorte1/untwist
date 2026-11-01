"use strict";
var __rest = (this && this.__rest) || function (s, e) {
    var t = {};
    for (var p in s) if (Object.prototype.hasOwnProperty.call(s, p) && e.indexOf(p) < 0)
        t[p] = s[p];
    if (s != null && typeof Object.getOwnPropertySymbols === "function")
        for (var i = 0, p = Object.getOwnPropertySymbols(s); i < p.length; i++) {
            if (e.indexOf(p[i]) < 0 && Object.prototype.propertyIsEnumerable.call(s, p[i]))
                t[p[i]] = s[p[i]];
        }
    return t;
};
var _a, _b;
var sym = Symbol.for("s");
var src = (_a = { a: 1, b: 2, c: 3 }, _a[sym] = 4, _a);
var a = src.a, others = __rest(src, ["a"]);
console.log(a, Object.keys(others).join(","), Object.getOwnPropertySymbols(others).length);
function pick(_a) {
    var b = _a.b, rest = __rest(_a, ["b"]);
    return b + ":" + Object.keys(rest).join(",");
}
console.log(pick(src));
var c = src["c"], bee = src.b, remaining = __rest(src, ["c", "b"]);
console.log(c, bee, Object.keys(remaining).join(","));
var _c = [1, 2, 3, 4], head = _c[0], tail = _c.slice(1);
console.log(head, tail.join(","));
var _d = new Map([["k", "v"], ["k2", "v2"]]).keys(), k1 = _d[0], k2 = _d[1];
console.log(k1, k2);
var reads = [];
var watched = {
    get x() { reads.push("x"); return 1; },
    get y() { reads.push("y"); return 2; },
    z: 3,
};
var y = watched.y, fromWatched = __rest(watched, ["y"]);
console.log(y, Object.keys(fromWatched).join(","), reads.join(","));
var box = { p: 1, q: 2 };
var p1 = box.p;
var q1 = box.q;
console.log(p1, q1);
try {
    var only = null.only;
    console.log(only);
}
catch (e) {
    console.log(e.constructor.name);
}
var _e = "xyz", first = _e[0], second = _e[1];
console.log(first, second);
var counted = (_b = {},
    _b[Symbol.iterator] = function () {
        var n = 0;
        return {
            next: function () { n += 1; return { value: n, done: n > 5 }; },
            return: function () { console.log("closed"); return {}; },
        };
    },
    _b);
var one = counted[0], two = counted[1];
console.log(one, two);
