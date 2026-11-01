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
var source = JSON.parse('{"__proto__": {"inherited": 1}, "own": 2}');
var copy = __assign(__assign({}, source), { last: void 0 });
console.log(Object.keys(copy).join(","), Object.getPrototypeOf(copy) === Object.prototype, copy.inherited);
var plain = __assign(__assign({}, { a: 1 }), { b: 2 });
console.log(JSON.stringify(plain));
