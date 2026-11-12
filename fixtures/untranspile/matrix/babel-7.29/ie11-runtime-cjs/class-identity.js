(function () {
  'use strict';

  var _interopRequireDefault = require("@babel/runtime/helpers/interopRequireDefault");
  var _typeof2 = _interopRequireDefault(require("@babel/runtime/helpers/typeof"));
  var _possibleConstructorReturn2 = _interopRequireDefault(require("@babel/runtime/helpers/possibleConstructorReturn"));
  var _getPrototypeOf2 = _interopRequireDefault(require("@babel/runtime/helpers/getPrototypeOf"));
  var _inherits2 = _interopRequireDefault(require("@babel/runtime/helpers/inherits"));
  var _classCallCheck2 = _interopRequireDefault(require("@babel/runtime/helpers/classCallCheck"));
  var _createClass2 = _interopRequireDefault(require("@babel/runtime/helpers/createClass"));
  function _callSuper(t, o, e) { return o = (0, _getPrototypeOf2.default)(o), (0, _possibleConstructorReturn2.default)(t, _isNativeReflectConstruct() ? Reflect.construct(o, e || [], (0, _getPrototypeOf2.default)(t).constructor) : o.apply(t, e)); }
  function _isNativeReflectConstruct() { try { var t = !Boolean.prototype.valueOf.call(Reflect.construct(Boolean, [], function () {})); } catch (t) {} return (_isNativeReflectConstruct = function _isNativeReflectConstruct() { return !!t; })(); }
  var Shape = /*#__PURE__*/function () {
    function Shape(kind) {
      (0, _classCallCheck2.default)(this, Shape);
      this.kind = kind;
    }
    return (0, _createClass2.default)(Shape, [{
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
  }();
  var Square = /*#__PURE__*/function (_Shape) {
    function Square(side) {
      var _this;
      (0, _classCallCheck2.default)(this, Square);
      _this = _callSuper(this, Square, ["square"]);
      _this.side = side;
      return _this;
    }
    (0, _inherits2.default)(Square, _Shape);
    return (0, _createClass2.default)(Square, [{
      key: "area",
      value: function area() {
        return this.side * this.side;
      }
    }]);
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
  console.log(Shape.length, Square.length, (0, _typeof2.default)(Shape.prototype.area.prototype));

})();
