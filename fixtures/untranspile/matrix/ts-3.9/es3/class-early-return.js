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
var _a = [new Pooled("a"), new Pooled("b", 16)], first = _a[0], second = _a[1];
console.log(first.items.length, second.capped, second.items, first instanceof Resource);
