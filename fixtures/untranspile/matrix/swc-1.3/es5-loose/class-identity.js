"use strict";
function _inherits(subClass, superClass) {
    if (typeof superClass !== "function" && superClass !== null) {
        throw new TypeError("Super expression must either be null or a function");
    }
    subClass.prototype = Object.create(superClass && superClass.prototype, {
        constructor: {
            value: subClass,
            writable: true,
            configurable: true
        }
    });
    if (superClass) _set_prototype_of(subClass, superClass);
}
function _set_prototype_of(o, p) {
    _set_prototype_of = Object.setPrototypeOf || function setPrototypeOf(o, p) {
        o.__proto__ = p;
        return o;
    };
    return _set_prototype_of(o, p);
}
function _type_of(obj) {
    "@swc/helpers - typeof";
    return obj && typeof Symbol !== "undefined" && obj.constructor === Symbol ? "symbol" : typeof obj;
}
var Shape = /*#__PURE__*/ function() {
    "use strict";
    function Shape(kind) {
        this.kind = kind;
    }
    var _proto = Shape.prototype;
    _proto.area = function area() {
        return 0;
    };
    Shape.of = function of(kind) {
        return new Shape(kind);
    };
    return Shape;
}();
var Square = /*#__PURE__*/ function(Shape) {
    "use strict";
    _inherits(Square, Shape);
    function Square(side) {
        var _this;
        _this = Shape.call(this, "square") || this;
        _this.side = side;
        return _this;
    }
    var _proto = Square.prototype;
    _proto.area = function area() {
        return this.side * this.side;
    };
    return Square;
}(Shape);
var s = new Square(3);
var seen = [];
for(var key in s)seen.push(key);
console.log(seen.join(","), Object.keys(Shape.prototype).length, Object.getOwnPropertyNames(Square.prototype).join(","));
console.log(Object.getOwnPropertyDescriptor(Shape, "prototype").writable, Object.getOwnPropertyDescriptor(Shape.prototype, "area").enumerable);
for(var _i = 0, _iter = [
    function() {
        return Shape("x");
    },
    function() {
        return Square(1);
    }
]; _i < _iter.length; _i++){
    var make = _iter[_i];
    try {
        make();
        console.log("called without new");
    } catch (e) {
        console.log(e.constructor.name);
    }
}
console.log(Shape.length, Square.length, _type_of(Shape.prototype.area.prototype));
