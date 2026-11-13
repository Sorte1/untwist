"use strict";

var _typeof = typeof Symbol === "function" && typeof Symbol.iterator === "symbol" ? function (obj) { return typeof obj; } : function (obj) { return obj && typeof Symbol === "function" && obj.constructor === Symbol && obj !== Symbol.prototype ? "symbol" : typeof obj; };

function _possibleConstructorReturn(self, call) { if (!self) { throw new ReferenceError("this hasn't been initialised - super() hasn't been called"); } return call && (typeof call === "object" || typeof call === "function") ? call : self; }

function _inherits(subClass, superClass) { if (typeof superClass !== "function" && superClass !== null) { throw new TypeError("Super expression must either be null or a function, not " + typeof superClass); } subClass.prototype = Object.create(superClass && superClass.prototype, { constructor: { value: subClass, enumerable: false, writable: true, configurable: true } }); if (superClass) Object.setPrototypeOf ? Object.setPrototypeOf(subClass, superClass) : subClass.__proto__ = superClass; }

function _classCallCheck(instance, Constructor) { if (!(instance instanceof Constructor)) { throw new TypeError("Cannot call a class as a function"); } }

var Shape = function () {
  function Shape(kind) {
    _classCallCheck(this, Shape);

    this.kind = kind;
  }

  Shape.prototype.area = function area() {
    return 0;
  };

  Shape.of = function of(kind) {
    return new Shape(kind);
  };

  return Shape;
}();

var Square = function (_Shape) {
  _inherits(Square, _Shape);

  function Square(side) {
    _classCallCheck(this, Square);

    var _this = _possibleConstructorReturn(this, _Shape.call(this, "square"));

    _this.side = side;
    return _this;
  }

  Square.prototype.area = function area() {
    return this.side * this.side;
  };

  return Square;
}(Shape);

var s = new Square(3);
var seen = [];
for (var key in s) {
  seen.push(key);
}console.log(seen.join(","), Object.keys(Shape.prototype).length, Object.getOwnPropertyNames(Square.prototype).join(","));
console.log(Object.getOwnPropertyDescriptor(Shape, "prototype").writable, Object.getOwnPropertyDescriptor(Shape.prototype, "area").enumerable);
var _arr = [function () {
  return Shape("x");
}, function () {
  return Square(1);
}];
for (var _i = 0; _i < _arr.length; _i++) {
  var make = _arr[_i];
  try {
    make();
    console.log("called without new");
  } catch (e) {
    console.log(e.constructor.name);
  }
}
console.log(Shape.length, Square.length, _typeof(Shape.prototype.area.prototype));
