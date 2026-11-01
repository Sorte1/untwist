"use strict";
var _a, _b;
var _c = [1, 2, undefined, 4, 5], p = _c[0], _d = _c[2], q = _d === void 0 ? 7 : _d, r = _c.slice(3);
console.log(p, q, r.join(","));
var _e = new Set(["s1", "s2", "s3"]), m = _e[0], n = _e[1];
console.log(m, n);
var _f = { c: undefined, k1: "kv" }, _g = _f.a, _h = _g === void 0 ? { b: "dflt" } : _g, deep = _h.b, _j = _f.c, c = _j === void 0 ? 3 : _j, _k = "k" + 1, computed = _f[_k];
console.log(deep, c, computed);
function f(_a, _b) {
    var _c = _a === void 0 ? {} : _a, x = _c.x, _d = _c.y, y = _d === void 0 ? 2 : _d;
    var _e = _b === void 0 ? [9] : _b, z = _e[0];
    return [x, y, z].join("/");
}
console.log(f(), f({ x: 1 }, [5]), f.length);
var u, v;
_b = (_a = [1, 2], v = _a[0], u = _a[1], _a), u = _b[0], v = _b[1];
console.log(u, v);
for (var _i = 0, _l = Object.entries({ e1: 1, e2: 2 }); _i < _l.length; _i++) {
    var _m = _l[_i], key = _m[0], value = _m[1];
    console.log(key, value);
}
var order = [];
var src = {
    get first() { order.push("first"); return 1; },
    get second() { order.push("second"); return 2; },
};
var second = src.second, first = src.first;
console.log(order.join(","), first, second);
try {
    var t = null[0];
    console.log(t);
}
catch (e) {
    console.log(e.constructor.name);
}
try {
    var t2 = undefined.t2;
    console.log(t2);
}
catch (e) {
    console.log(e.constructor.name);
}
var str = "ab";
var c1 = str[0], c2 = str[1];
console.log(c1, c2);
var swap = { l: "L", r: "R" };
(swap.r = swap.l, swap.l = swap.r);
console.log(swap.l, swap.r);
var length = "four".length;
console.log(length);
var _o = [[1, [2]], { o: [3] }], _p = _o[0], n1 = _p[0], n2 = _p[1][0], n3 = _o[1].o[0];
console.log(n1 + n2 + n3);
