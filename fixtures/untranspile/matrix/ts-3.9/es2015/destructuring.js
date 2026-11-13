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
const [a, , b = 10, ...rest] = [1, 2, undefined, 4, 5];
const _a = { x: 1, q: 9, r: 8 }, { x, y: why = 2, z: { deep } = { deep: "d" } } = _a, others = __rest(_a, ["x", "y", "z"]);
console.log(a, b, rest.join(","), x, why, deep, JSON.stringify(others));
function point({ x = 0, y = 0 } = {}, [first, second] = []) {
    return x + y + (first || 0) + (second || 0);
}
console.log(point({ x: 1 }, [2, 3]), point());
let m = 1;
let n = 2;
[m, n] = [n, m];
console.log(m, n);
const pairs = [[1, "one"], [2, "two"]];
for (const [num, word] of pairs)
    console.log(num, word);
const { length } = "hello";
const [c1, c2] = "hi";
console.log(length, c1, c2);
const data = { list: [{ id: 7, tags: ["t1", "t2"] }] };
const { list: [{ id, tags: [, second] }] } = data;
console.log(id, second);
const key = "dyn";
const { [key]: dynamic = "fallback" } = {};
console.log(dynamic);
function swap([p, q]) {
    return [q, p];
}
console.log(swap([1, 2]).join(","));
const [g1, g2] = (function* () { yield "a"; yield "b"; yield "c"; })();
console.log(g1, g2);
