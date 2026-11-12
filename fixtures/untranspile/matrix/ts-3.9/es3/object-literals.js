"use strict";
var _a;
var k = "dyn";
var x = 1;
var o = (_a = {
        x: x
    },
    _a[k + "1"] = 2,
    _a.method = function () {
        return "m" + this.x;
    },
    Object.defineProperty(_a, "g", {
        get: function () {
            return "get";
        },
        enumerable: false,
        configurable: true
    }),
    _a["t" + k] = 3,
    _a.__proto__ = { inherited: true },
    _a);
console.log(o.x, o.dyn1, o.method(), o.g, o.tdyn, o.inherited, Object.keys(o).join(","));
var sup = {
    hello: function () {
        return "base";
    }
};
var sub = {
    __proto__: sup,
    hello: function () {
        return _super.hello.call(this) + "+sub";
    }
};
console.log(sub.hello());
