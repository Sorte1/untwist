(function () {
  'use strict';

  var _interopRequireDefault = require("@babel/runtime/helpers/interopRequireDefault");
  var _typeof2 = _interopRequireDefault(require("@babel/runtime/helpers/typeof"));
  var _classCallCheck2 = _interopRequireDefault(require("@babel/runtime/helpers/classCallCheck"));
  var _createClass2 = _interopRequireDefault(require("@babel/runtime/helpers/createClass"));
  var Point = /*#__PURE__*/function () {
    function Point(x, y) {
      (0, _classCallCheck2.default)(this, Point);
      this.x = x;
      this.y = y;
    }
    return (0, _createClass2.default)(Point, [{
      key: "length",
      get: function get() {
        return Math.sqrt(this.x * this.x + this.y * this.y);
      },
      set: function set(v) {
        var k = v / this.length;
        this.x *= k;
        this.y *= k;
      }
    }, {
      key: "add",
      value: function add(other) {
        return new Point(this.x + other.x, this.y + other.y);
      }
    }, {
      key: "to" + "Pair",
      value: function value() {
        return [this.x, this.y];
      }
    }], [{
      key: "origin",
      value: function origin() {
        return new this(0, 0);
      }
    }, {
      key: "unit",
      get: function get() {
        return new Point(1, 0);
      }
    }]);
  }();
  var p = new Point(3, 4);
  console.log(p.length, p.add(Point.origin()).toPair().join(","), Point.unit.x);
  p.length = 10;
  console.log(p.x, p.y);
  console.log((0, _typeof2.default)(Point), Point.name);
  var Anon = /*#__PURE__*/function () {
    function Anon() {
      (0, _classCallCheck2.default)(this, Anon);
    }
    return (0, _createClass2.default)(Anon, [{
      key: "hi",
      value: function hi() {
        return "anon";
      }
    }]);
  }();
  console.log(new Anon().hi(), Anon.name);

})();
