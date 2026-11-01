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
var _a;
var counter = 0;
var Fields = /** @class */ (function () {
    function Fields() {
        this.a = 1;
        this.b = this.a + 1;
        this[_b] = "dyn";
    }
    Fields.prototype.m = function () {
        return this.a + this.b;
    };
    var _b;
    _b = "k" + counter;
    Fields.s = ++counter;
    Fields.t = Fields.s + 1;
    return Fields;
}());
var f = new Fields();
console.log(f.a, f.b, Fields.s, Fields.t, f.k1, f.m(), Object.keys(f).join(","));
var Parent = /** @class */ (function () {
    function Parent() {
        this.init();
    }
    Parent.prototype.init = function () { };
    return Parent;
}());
var Kid = /** @class */ (function (_super) {
    __extends(Kid, _super);
    function Kid() {
        var _this = _super !== null && _super.apply(this, arguments) || this;
        _this.x = 1;
        return _this;
    }
    Kid.prototype.init = function () {
        this.seen = this.x;
    };
    return Kid;
}(Parent));
console.log(new Kid().seen, new Kid().x);
var WithStaticBlock = /** @class */ (function () {
    function WithStaticBlock() {
    }
    return WithStaticBlock;
}());
_a = WithStaticBlock;
(function () {
    _a.z = 42;
})();
console.log(WithStaticBlock.z);
var Arrowed = /** @class */ (function () {
    function Arrowed() {
        var _this = this;
        this.name = "arrowed";
        this.get = function () { return _this.name; };
    }
    return Arrowed;
}());
var get = new Arrowed().get;
console.log(get());
