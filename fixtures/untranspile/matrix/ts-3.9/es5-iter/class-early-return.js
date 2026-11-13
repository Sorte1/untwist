"use strict";
var __extends = (this && this.__extends) || (function () {
    var extendStatics = function (d, b) {
        extendStatics = Object.setPrototypeOf ||
            ({ __proto__: [] } instanceof Array && function (d, b) { d.__proto__ = b; }) ||
            function (d, b) { for (var p in b) if (b.hasOwnProperty(p)) d[p] = b[p]; };
        return extendStatics(d, b);
    };
    return function (d, b) {
        extendStatics(d, b);
        function __() { this.constructor = d; }
        d.prototype = b === null ? Object.create(b) : (__.prototype = b.prototype, new __());
    };
})();
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
var Resource = /** @class */ (function () {
    function Resource(name) {
        this.name = name;
    }
    return Resource;
}());
var Pooled = /** @class */ (function (_super) {
    __extends(Pooled, _super);
    function Pooled(name, size) {
        if (size === void 0) { size = 4; }
        var _this = _super.call(this, name) || this;
        _this.size = size;
        if (size > 8) {
            _this.capped = true;
            return _this;
        }
        _this.items = [];
        return _this;
    }
    return Pooled;
}(Resource));
var _a = __read([new Pooled("a"), new Pooled("b", 16)], 2), first = _a[0], second = _a[1];
console.log(first.items.length, second.capped, second.items, first instanceof Resource);
