"use strict";

function _inheritsLoose(t, o) { t.prototype = Object.create(o.prototype), t.prototype.constructor = t, _setPrototypeOf(t, o); }
function _setPrototypeOf(t, e) { return _setPrototypeOf = Object.setPrototypeOf ? Object.setPrototypeOf.bind() : function (t, e) { return t.__proto__ = e, t; }, _setPrototypeOf(t, e); }
var Shape = /*#__PURE__*/function () {
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
var Square = /*#__PURE__*/function (_Shape) {
  function Square(side) {
    var _this;
    _this = _Shape.call(this, "square") || this;
    _this.side = side;
    return _this;
  }
  _inheritsLoose(Square, _Shape);
  var _proto2 = Square.prototype;
  _proto2.area = function area() {
    return this.side * this.side;
  };
  return Square;
}(Shape);
var s = new Square(3);
var seen = [];
for (var key in s) seen.push(key);
console.log(seen.join(","), Object.keys(Shape.prototype).length, Object.getOwnPropertyNames(Square.prototype).join(","));
console.log(Object.getOwnPropertyDescriptor(Shape, "prototype").writable, Object.getOwnPropertyDescriptor(Shape.prototype, "area").enumerable);
for (var _i = 0, _arr = [function () {
    return Shape("x");
  }, function () {
    return Square(1);
  }]; _i < _arr.length; _i++) {
  var make = _arr[_i];
  try {
    make();
    console.log("called without new");
  } catch (e) {
    console.log(e.constructor.name);
  }
}
console.log(Shape.length, Square.length, typeof Shape.prototype.area.prototype);
