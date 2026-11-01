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
const sym = Symbol.for("s");
const src = { a: 1, b: 2, c: 3, [sym]: 4 };
const { a } = src, others = __rest(src, ["a"]);
console.log(a, Object.keys(others).join(","), Object.getOwnPropertySymbols(others).length);
function pick(_a) {
    var { b } = _a, rest = __rest(_a, ["b"]);
    return b + ":" + Object.keys(rest).join(",");
}
console.log(pick(src));
const { ["c"]: c, b: bee } = src, remaining = __rest(src, ["c", "b"]);
console.log(c, bee, Object.keys(remaining).join(","));
const [head, ...tail] = [1, 2, 3, 4];
console.log(head, tail.join(","));
const [k1, k2] = new Map([["k", "v"], ["k2", "v2"]]).keys();
console.log(k1, k2);
const reads = [];
const watched = {
    get x() { reads.push("x"); return 1; },
    get y() { reads.push("y"); return 2; },
    z: 3,
};
const { y } = watched, fromWatched = __rest(watched, ["y"]);
console.log(y, Object.keys(fromWatched).join(","), reads.join(","));
const box = { p: 1, q: 2 };
var p1 = box.p;
var q1 = box.q;
console.log(p1, q1);
try {
    const { only } = null;
    console.log(only);
}
catch (e) {
    console.log(e.constructor.name);
}
const [first, second] = "xyz";
console.log(first, second);
const counted = {
    [Symbol.iterator]() {
        let n = 0;
        return {
            next() { n += 1; return { value: n, done: n > 5 }; },
            return() { console.log("closed"); return {}; },
        };
    },
};
const [one, two] = counted;
console.log(one, two);
