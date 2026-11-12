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
var Shape = /** @class */ (function () {
    function Shape(kind) {
        this.kind = kind;
    }
    return Shape;
}());
var Box = /** @class */ (function (_super) {
    __extends(Box, _super);
    function Box(width, height, label) {
        if (width === void 0) { width = 1; }
        if (height === void 0) { height = width * 2; }
        var _this = _super.call(this, "box") || this;
        _this.width = width;
        _this.height = height;
        _this.label = label;
        return _this;
    }
    Box.prototype.area = function (scale) {
        if (scale === void 0) { scale = 1; }
        return this.width * this.height * scale;
    };
    return Box;
}(Shape));
var Tagged = /** @class */ (function (_super) {
    __extends(Tagged, _super);
    function Tagged(tag) {
        if (tag === void 0) { tag = "t"; }
        var rest = [];
        for (var _i = 1; _i < arguments.length; _i++) {
            rest[_i - 1] = arguments[_i];
        }
        var _this = _super.apply(this, rest) || this;
        _this.tag = tag;
        return _this;
    }
    return Tagged;
}(Box));
function later(step) {
    if (step === void 0) { step = 1; }
    var more = [];
    for (var _i = 1; _i < arguments.length; _i++) {
        more[_i - 1] = arguments[_i];
    }
    var run = function () { return step + more.length; };
    return run();
}
console.log(JSON.stringify(new Box()), JSON.stringify(new Box(3)), JSON.stringify(new Box(3, 4, "x")));
console.log(new Box(2).area(), new Box(2).area(3), Box.length, Box.prototype.area.length);
console.log(JSON.stringify(new Tagged()), JSON.stringify(new Tagged("q", 5)), Tagged.length);
console.log(later(), later(2, 0, 0), later.length);
