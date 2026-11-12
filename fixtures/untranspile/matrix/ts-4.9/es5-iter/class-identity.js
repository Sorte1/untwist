"use strict";
var __extends = (this && this.__extends) || (function () {
    var extendStatics = function (d, b) {
        extendStatics = Object.setPrototypeOf ||
            ({ __proto__: [] } instanceof Array && function (d, b) { d.__proto__ = b; }) ||
            function (d, b) { for (var p in b) if (Object.prototype.hasOwnProperty.call(b, p)) d[p] = b[p]; };
        return extendStatics(d, b);
    };
    return function (d, b) {
        if (typeof b !== "function" && b !== null)
            throw new TypeError("Class extends value " + String(b) + " is not a constructor or null");
        extendStatics(d, b);
        function __() { this.constructor = d; }
        d.prototype = b === null ? Object.create(b) : (__.prototype = b.prototype, new __());
    };
})();
var __values = (this && this.__values) || function(o) {
    var s = typeof Symbol === "function" && Symbol.iterator, m = s && o[s], i = 0;
    if (m) return m.call(o);
    if (o && typeof o.length === "number") return {
        next: function () {
            if (o && i >= o.length) o = void 0;
            return { value: o && o[i++], done: !o };
        }
    };
    throw new TypeError(s ? "Object is not iterable." : "Symbol.iterator is not defined.");
};
var e_1, _a;
var Shape = /** @class */ (function () {
    function Shape(kind) {
        this.kind = kind;
    }
    Shape.prototype.area = function () {
        return 0;
    };
    Shape.of = function (kind) {
        return new Shape(kind);
    };
    return Shape;
}());
var Square = /** @class */ (function (_super) {
    __extends(Square, _super);
    function Square(side) {
        var _this = _super.call(this, "square") || this;
        _this.side = side;
        return _this;
    }
    Square.prototype.area = function () {
        return this.side * this.side;
    };
    return Square;
}(Shape));
var s = new Square(3);
var seen = [];
for (var key in s)
    seen.push(key);
console.log(seen.join(","), Object.keys(Shape.prototype).length, Object.getOwnPropertyNames(Square.prototype).join(","));
console.log(Object.getOwnPropertyDescriptor(Shape, "prototype").writable, Object.getOwnPropertyDescriptor(Shape.prototype, "area").enumerable);
try {
    for (var _b = __values([function () { return Shape("x"); }, function () { return Square(1); }]), _c = _b.next(); !_c.done; _c = _b.next()) {
        var make = _c.value;
        try {
            make();
            console.log("called without new");
        }
        catch (e) {
            console.log(e.constructor.name);
        }
    }
}
catch (e_1_1) { e_1 = { error: e_1_1 }; }
finally {
    try {
        if (_c && !_c.done && (_a = _b.return)) _a.call(_b);
    }
    finally { if (e_1) throw e_1.error; }
}
console.log(Shape.length, Square.length, typeof Shape.prototype.area.prototype);
