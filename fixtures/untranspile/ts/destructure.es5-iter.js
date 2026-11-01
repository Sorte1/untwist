"use strict";
var __read = (this && this.__read) || function (o, n) {
    var m = typeof Symbol === "function" && o[Symbol.iterator];
    if (!m) return o;
    var i = m.call(o), r, ar = [], e;
    try {
        while ((n === void 0 || n-- > 0) && !(r = i.next()).done) ar.push(r.value);
    }
    catch (error) { e = { error: error }; }
    finally {
        try {
            if (r && !r.done && (m = i["return"])) m.call(i);
        }
        finally { if (e) throw e.error; }
    }
    return ar;
};
var __values = (this && this.__values) || function(o) {
    var s = typeof Symbol === "function" && Symbol.iterator, m = s && o[s], i = 0;
    if (m) return m.call(o);
    if (o && typeof o.length === "number") return {
        next: function () {
            if (o && i >= o.length) o = void 0;
            return { value: o && o[i++], done: !o };
        }
    };
    throw new TypeError(s ? "Object is not iterable." : "Symbol.iterator is not defined.");
};
var _a, _b, _c, e_1, _d;
var _e = __read([1, 2, undefined, 4, 5]), p = _e[0], _f = _e[2], q = _f === void 0 ? 7 : _f, r = _e.slice(3);
console.log(p, q, r.join(","));
var _g = __read(new Set(["s1", "s2", "s3"]), 2), m = _g[0], n = _g[1];
console.log(m, n);
var _h = { c: undefined, k1: "kv" }, _j = _h.a, _k = _j === void 0 ? { b: "dflt" } : _j, deep = _k.b, _l = _h.c, c = _l === void 0 ? 3 : _l, _m = "k" + 1, computed = _h[_m];
console.log(deep, c, computed);
function f(_a, _b) {
    var _c = _a === void 0 ? {} : _a, x = _c.x, _d = _c.y, y = _d === void 0 ? 2 : _d;
    var _e = _b === void 0 ? [9] : _b, _f = __read(_e, 1), z = _f[0];
    return [x, y, z].join("/");
}
console.log(f(), f({ x: 1 }, [5]), f.length);
var u, v;
_c = __read((_a = [1, 2], _b = __read(_a, 2), v = _b[0], u = _b[1], _a), 2), u = _c[0], v = _c[1];
console.log(u, v);
try {
    for (var _o = __values(Object.entries({ e1: 1, e2: 2 })), _p = _o.next(); !_p.done; _p = _o.next()) {
        var _q = __read(_p.value, 2), key = _q[0], value = _q[1];
        console.log(key, value);
    }
}
catch (e_1_1) { e_1 = { error: e_1_1 }; }
finally {
    try {
        if (_p && !_p.done && (_d = _o.return)) _d.call(_o);
    }
    finally { if (e_1) throw e_1.error; }
}
var order = [];
var src = {
    get first() { order.push("first"); return 1; },
    get second() { order.push("second"); return 2; },
};
var second = src.second, first = src.first;
console.log(order.join(","), first, second);
try {
    var _r = __read(null, 1), t = _r[0];
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
var _s = __read(str, 2), c1 = _s[0], c2 = _s[1];
console.log(c1, c2);
var swap = { l: "L", r: "R" };
(swap.r = swap.l, swap.l = swap.r);
console.log(swap.l, swap.r);
var length = "four".length;
console.log(length);
var _t = __read([[1, [2]], { o: [3] }], 2), _u = __read(_t[0], 2), n1 = _u[0], _v = __read(_u[1], 1), n2 = _v[0], _w = __read(_t[1].o, 1), n3 = _w[0];
console.log(n1 + n2 + n3);
