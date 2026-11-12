"use strict";

function _typeof(obj) { if (typeof Symbol === "function" && typeof Symbol.iterator === "symbol") { _typeof = function _typeof(obj) { return typeof obj; }; } else { _typeof = function _typeof(obj) { return obj && typeof Symbol === "function" && obj.constructor === Symbol && obj !== Symbol.prototype ? "symbol" : typeof obj; }; } return _typeof(obj); }

function _possibleConstructorReturn(self, call) { if (call && (_typeof(call) === "object" || typeof call === "function")) { return call; } return _assertThisInitialized(self); }

function _assertThisInitialized(self) { if (self === void 0) { throw new ReferenceError("this hasn't been initialised - super() hasn't been called"); } return self; }

function _getPrototypeOf(o) { _getPrototypeOf = Object.setPrototypeOf ? Object.getPrototypeOf : function _getPrototypeOf(o) { return o.__proto__ || Object.getPrototypeOf(o); }; return _getPrototypeOf(o); }

function _inherits(subClass, superClass) { if (typeof superClass !== "function" && superClass !== null) { throw new TypeError("Super expression must either be null or a function"); } subClass.prototype = Object.create(superClass && superClass.prototype, { constructor: { value: subClass, writable: true, configurable: true } }); if (superClass) _setPrototypeOf(subClass, superClass); }

function _setPrototypeOf(o, p) { _setPrototypeOf = Object.setPrototypeOf || function _setPrototypeOf(o, p) { o.__proto__ = p; return o; }; return _setPrototypeOf(o, p); }

function _classCallCheck(instance, Constructor) { if (!(instance instanceof Constructor)) { throw new TypeError("Cannot call a class as a function"); } }

function _defineProperties(target, props) { for (var i = 0; i < props.length; i++) { var descriptor = props[i]; descriptor.enumerable = descriptor.enumerable || false; descriptor.configurable = true; if ("value" in descriptor) descriptor.writable = true; Object.defineProperty(target, descriptor.key, descriptor); } }

function _createClass(Constructor, protoProps, staticProps) { if (protoProps) _defineProperties(Constructor.prototype, protoProps); if (staticProps) _defineProperties(Constructor, staticProps); return Constructor; }

var Shape =
/*#__PURE__*/
function () {
  function Shape(kind) {
    _classCallCheck(this, Shape);

    this.kind = kind;
  }

  _createClass(Shape, [{
    key: "area",
    value: function area() {
      return 0;
    }
  }], [{
    key: "of",
    value: function of(kind) {
      return new Shape(kind);
    }
  }]);

  return Shape;
}();

var Square =
/*#__PURE__*/
function (_Shape) {
  _inherits(Square, _Shape);

  function Square(side) {
    var _this;

    _classCallCheck(this, Square);

    _this = _possibleConstructorReturn(this, _getPrototypeOf(Square).call(this, "square"));
    _this.side = side;
    return _this;
  }

  _createClass(Square, [{
    key: "area",
    value: function area() {
      return this.side * this.side;
    }
  }]);

  return Square;
}(Shape);

var s = new Square(3);
var seen = [];

for (var key in s) {
  seen.push(key);
}

console.log(seen.join(","), Object.keys(Shape.prototype).length, Object.getOwnPropertyNames(Square.prototype).join(","));
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
