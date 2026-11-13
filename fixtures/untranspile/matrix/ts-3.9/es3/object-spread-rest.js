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
var _a;
var base = { a: 1, b: 2 };
var extra = { c: 3 };
var merged = __assign(__assign(__assign({}, base), extra), { d: 4 });
var overridden = __assign(__assign({}, base), { a: 9 });
console.log(JSON.stringify(merged), JSON.stringify(overridden));
var a = merged.a, others = __rest(merged, ["a"]);
console.log(a, JSON.stringify(others));
function without(_a) {
    var secret = _a.secret, pub = __rest(_a, ["secret"]);
    return pub;
}
console.log(JSON.stringify(without({ secret: 1, open: 2 })));
var withGetter = __assign({ get g() { return "got"; } });
console.log(Object.getOwnPropertyDescriptor(withGetter, "g").value);
var sym = Symbol("s");
var symObj = __assign((_a = {}, _a[sym] = 1, _a.plain = 2, _a));
console.log(symObj[sym], symObj.plain);
var _b = symObj, _c = sym, picked = _b[_c], restSym = __rest(_b, [typeof _c === "symbol" ? _c : _c + ""]);
console.log(picked, Object.keys(restSym).join(","));
console.log(JSON.stringify(__assign(__assign(__assign(__assign({}, null), undefined), "hi"), [5])));
var order = [];
var tracked = { get x() { order.push("x"); return 1; }, get y() { order.push("y"); return 2; } };
var copy = __assign({}, tracked);
console.log(order.join(","), copy.x + copy.y);
