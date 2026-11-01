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
function sum() {
    var total = 0;
    for (var i = 0; i < arguments.length; i++)
        total += arguments[i];
    return total;
}
const a = [1, 2, 3];
const b = [...a, 4, ...[5, 6]];
console.log(b.join(","), b.length);
console.log(sum(...a, 10, ...[20]));
console.log(Math.max(...new Set([3, 9, 1])));
const holes = [, 1];
const copy = [...holes];
console.log(0 in copy, copy.length);
const o1 = { x: 1, y: 2, get z() { return 3; } };
const o2 = Object.assign(Object.assign({}, o1), { y: 5, w: 6 });
const zd = Object.getOwnPropertyDescriptor(o2, "z");
console.log(JSON.stringify(o2), Object.keys(o2).join(","), zd.enumerable, typeof zd.get);
const s = Symbol("k");
const o3 = { [s]: 1, a: 2 };
const o4 = Object.assign(Object.assign(Object.assign(Object.assign({}, o3), null), undefined), "pq");
console.log(o4[s], Object.getOwnPropertySymbols(o4).length, JSON.stringify(o4));
function rest(first, ...others) {
    return first + ":" + others.join("|") + ":" + others.length;
}
console.log(rest(1), rest(1, 2, 3), rest.length);
const { x } = o2, others = __rest(o2, ["x"]);
console.log(x, JSON.stringify(others));
function callsThis(...args) {
    return this.base + args.length;
}
const holder = { base: 100, m: callsThis };
console.log(holder.m(...a), holder["m"](...a, ...a));
try {
    console.log([...undefined]);
}
catch (e) {
    console.log(e.constructor.name);
}
console.log(new Date(...[2020, 0, 2]).getFullYear());
const order = [];
const tap = (v) => (order.push(v), [v]);
console.log(sum(...tap(1), ...tap(2)), order.join(","));
try {
    console.log([..."xy"].join("-"));
}
catch (e) {
    console.log(e.constructor.name);
}
