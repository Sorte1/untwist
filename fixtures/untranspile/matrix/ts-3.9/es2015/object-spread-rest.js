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
const base = { a: 1, b: 2 };
const extra = { c: 3 };
const merged = Object.assign(Object.assign(Object.assign({}, base), extra), { d: 4 });
const overridden = Object.assign(Object.assign({}, base), { a: 9 });
console.log(JSON.stringify(merged), JSON.stringify(overridden));
const { a } = merged, others = __rest(merged, ["a"]);
console.log(a, JSON.stringify(others));
function without(_a) {
    var { secret } = _a, pub = __rest(_a, ["secret"]);
    return pub;
}
console.log(JSON.stringify(without({ secret: 1, open: 2 })));
const withGetter = Object.assign({ get g() { return "got"; } });
console.log(Object.getOwnPropertyDescriptor(withGetter, "g").value);
const sym = Symbol("s");
const symObj = Object.assign({ [sym]: 1, plain: 2 });
console.log(symObj[sym], symObj.plain);
const _a = symObj, _b = sym, picked = _a[_b], restSym = __rest(_a, [typeof _b === "symbol" ? _b : _b + ""]);
console.log(picked, Object.keys(restSym).join(","));
console.log(JSON.stringify(Object.assign(Object.assign(Object.assign(Object.assign({}, null), undefined), "hi"), [5])));
let order = [];
const tracked = { get x() { order.push("x"); return 1; }, get y() { order.push("y"); return 2; } };
const copy = Object.assign({}, tracked);
console.log(order.join(","), copy.x + copy.y);
