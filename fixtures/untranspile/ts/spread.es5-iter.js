"use strict";
var __assign = (this && this.__assign) || function () {
    __assign = Object.assign || function(t) {
        for (var s, i = 1, n = arguments.length; i < n; i++) {
            s = arguments[i];
            for (var p in s) if (Object.prototype.hasOwnProperty.call(s, p))
                t[p] = s[p];
        }
        return t;
    };
    return __assign.apply(this, arguments);
};
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
var __spreadArray = (this && this.__spreadArray) || function (to, from, pack) {
    if (pack || arguments.length === 2) for (var i = 0, l = from.length, ar; i < l; i++) {
        if (ar || !(i in from)) {
            if (!ar) ar = Array.prototype.slice.call(from, 0, i);
            ar[i] = from[i];
        }
    }
    return to.concat(ar || Array.prototype.slice.call(from));
};
var _a;
function sum() {
    var total = 0;
    for (var i = 0; i < arguments.length; i++)
        total += arguments[i];
    return total;
}
var a = [1, 2, 3];
var b = __spreadArray(__spreadArray(__spreadArray([], __read(a), false), [4], false), [5, 6], false);
console.log(b.join(","), b.length);
console.log(sum.apply(void 0, __spreadArray(__spreadArray(__spreadArray([], __read(a), false), [10], false), [20], false)));
console.log(Math.max.apply(Math, __spreadArray([], __read(new Set([3, 9, 1])), false)));
var holes = [, 1];
var copy = __spreadArray([], __read(holes), false);
console.log(0 in copy, copy.length);
var o1 = { x: 1, y: 2, get z() { return 3; } };
var o2 = __assign(__assign({}, o1), { y: 5, w: 6 });
var zd = Object.getOwnPropertyDescriptor(o2, "z");
console.log(JSON.stringify(o2), Object.keys(o2).join(","), zd.enumerable, typeof zd.get);
var s = Symbol("k");
var o3 = (_a = {}, _a[s] = 1, _a.a = 2, _a);
var o4 = __assign(__assign(__assign(__assign({}, o3), null), undefined), "pq");
console.log(o4[s], Object.getOwnPropertySymbols(o4).length, JSON.stringify(o4));
function rest(first) {
    var others = [];
    for (var _i = 1; _i < arguments.length; _i++) {
        others[_i - 1] = arguments[_i];
    }
    return first + ":" + others.join("|") + ":" + others.length;
}
console.log(rest(1), rest(1, 2, 3), rest.length);
var x = o2.x, others = __rest(o2, ["x"]);
console.log(x, JSON.stringify(others));
function callsThis() {
    var args = [];
    for (var _i = 0; _i < arguments.length; _i++) {
        args[_i] = arguments[_i];
    }
    return this.base + args.length;
}
var holder = { base: 100, m: callsThis };
console.log(holder.m.apply(holder, __spreadArray([], __read(a), false)), holder["m"].apply(holder, __spreadArray(__spreadArray([], __read(a), false), __read(a), false)));
try {
    console.log(__spreadArray([], __read(undefined), false));
}
catch (e) {
    console.log(e.constructor.name);
}
console.log(new (Date.bind.apply(Date, __spreadArray([void 0], [2020, 0, 2], false)))().getFullYear());
var order = [];
var tap = function (v) { return (order.push(v), [v]); };
console.log(sum.apply(void 0, __spreadArray(__spreadArray([], __read(tap(1)), false), __read(tap(2)), false)), order.join(","));
try {
    console.log(__spreadArray([], __read("xy"), false).join("-"));
}
catch (e) {
    console.log(e.constructor.name);
}
