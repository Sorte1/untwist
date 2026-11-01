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
    function Base(v) {
        this.v = v;
    }
    Object.defineProperty(Base.prototype, "val", {
        get: function () {
            return this.v;
        },
        set: function (x) {
            this.v = x * 2;
        },
        enumerable: false,
        configurable: true
    });
    Base.prototype.greet = function (who) {
        return "hi " + who + " " + this.v;
    };
    Base.make = function (v) {
        return new this(v);
    };
    Object.defineProperty(Base, "kind", {
        get: function () {
            return "base";
        },
        enumerable: false,
        configurable: true
    });
    Base.prototype["comp" + "uted"] = function () {
        return "c";
    };
    return Base;
}());
var Child = /** @class */ (function (_super) {
    __extends(Child, _super);
    function Child(v, w) {
        var _this = _super.call(this, v) || this;
        _this.w = w;
        return _this;
    }
    Child.prototype.greet = function (who) {
        return _super.prototype.greet.call(this, who).toUpperCase() + this.w;
    };
    Child.make = function (v) {
        return _super.make.call(this, v);
    };
    return Child;
}(Base));
var c = new Child(1, 2);
console.log(c.greet("x"), c.val, ((c.val = 5), c.v), c.computed());
console.log(Child.make(3) instanceof Child, Child.kind, Object.getPrototypeOf(Child) === Base);
console.log(Base.name, Child.name, typeof Base);
console.log(Object.getOwnPropertyNames(Base.prototype).sort().join(","), Object.keys(Base.prototype).length);
var d = Object.getOwnPropertyDescriptor(Base.prototype, "greet");
console.log(d.enumerable, d.writable, d.configurable);
console.log(Object.getOwnPropertyDescriptor(Base, "make").enumerable);
try {
    Base(1);
}
catch (e) {
    console.log(e.constructor.name);
}
var Anon = /** @class */ (function () {
    function Anon() {
    }
    return Anon;
}());
console.log(Anon.name);
var Counter = /** @class */ (function () {
    function Counter() {
        this.n = 0;
    }
    Counter.prototype.inc = function () {
        this.n++;
        return this;
    };
    return Counter;
}());
console.log(new Counter().inc().inc().n);
function Legacy(x) {
    this.x = x;
}
Legacy.prototype.get = function () {
    return this.x;
};
var FromLegacy = /** @class */ (function (_super) {
    __extends(FromLegacy, _super);
    function FromLegacy() {
        return _super !== null && _super.apply(this, arguments) || this;
    }
    FromLegacy.prototype.get = function () {
        return _super.prototype.get.call(this) + 1;
    };
    return FromLegacy;
}(Legacy));
console.log(new FromLegacy(4).get());
