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
var Base = /** @class */ (function () {
    function Base(a, b) {
        this.a = a;
    }
    Base.prototype.m = function () { };
    return Base;
}());
var Child = /** @class */ (function (_super) {
    __extends(Child, _super);
    function Child() {
        return _super !== null && _super.apply(this, arguments) || this;
    }
    return Child;
}(Base));
console.log(Base.length, Child.length, Object.getOwnPropertyDescriptor(Base, "prototype").writable);
console.log(Object.getOwnPropertyNames(Child).sort().join(","));
var Errorish = /** @class */ (function (_super) {
    __extends(Errorish, _super);
    function Errorish(m) {
        var _this = _super.call(this, m) || this;
        _this.name = "Errorish";
        return _this;
    }
    return Errorish;
}(Error));
var err = new Errorish("boom");
console.log(err instanceof Errorish, err instanceof Error, err.message, err.name);
var ArrLike = /** @class */ (function (_super) {
    __extends(ArrLike, _super);
    function ArrLike() {
        return _super !== null && _super.apply(this, arguments) || this;
    }
    return ArrLike;
}(Array));
var al = new ArrLike();
al.push(1);
console.log(al.length, al instanceof ArrLike);
console.log(typeof Base.prototype.m.prototype);
try {
    new Base.prototype.m();
    console.log("constructible");
}
catch (e) {
    console.log(e.constructor.name);
}
