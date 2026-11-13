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
for (var _i = 0, _a = [function () { return Shape("x"); }, function () { return Square(1); }]; _i < _a.length; _i++) {
    var make = _a[_i];
    try {
        make();
        console.log("called without new");
    }
    catch (e) {
        console.log(e.constructor.name);
    }
}
console.log(Shape.length, Square.length, typeof Shape.prototype.area.prototype);
