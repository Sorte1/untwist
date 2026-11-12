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
var seq = 0;
var Counter = /** @class */ (function () {
    function Counter(step) {
        var _this = this;
        this.count = 0;
        this.step = 1;
        this.id = ++seq;
        this.label = "c" + this.id;
        this[_a] = "dynamic";
        this.inc = function () {
            _this.count += _this.step;
            return _this;
        };
        if (step)
            this.step = step;
        Counter.instances++;
    }
    var _a;
    _a = "dyn" + seq;
    Counter.instances = 0;
    Counter.created = Counter.instances + 1;
    return Counter;
}());
var c = new Counter(2);
c.inc().inc();
console.log(c.count, c.label, c.dyn1, Counter.instances, Counter.created);
var inc = c.inc;
inc();
console.log(c.count, Object.keys(c).join(","));
var Base = /** @class */ (function () {
    function Base() {
        this.setup();
    }
    Base.prototype.setup = function () { };
    return Base;
}());
var Derived = /** @class */ (function (_super) {
    __extends(Derived, _super);
    function Derived() {
        var _this = _super !== null && _super.apply(this, arguments) || this;
        _this.value = 5;
        return _this;
    }
    Derived.prototype.setup = function () {
        this.seen = this.value;
    };
    return Derived;
}(Base));
var dd = new Derived();
console.log(dd.seen, dd.value);
var Shadow = /** @class */ (function () {
    function Shadow() {
    }
    Object.defineProperty(Shadow.prototype, "x", {
        set: function (v) {
            this.setterRan = true;
        },
        enumerable: false,
        configurable: true
    });
    return Shadow;
}());
var Defines = /** @class */ (function (_super) {
    __extends(Defines, _super);
    function Defines() {
        var _this = _super !== null && _super.apply(this, arguments) || this;
        _this.x = 1;
        return _this;
    }
    return Defines;
}(Shadow));
var df = new Defines();
console.log(df.x, df.setterRan === true);
