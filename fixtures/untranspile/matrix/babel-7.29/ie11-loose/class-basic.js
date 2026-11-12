"use strict";

function _defineAccessor(e, r, n, t) { var c = { configurable: !0, enumerable: !0 }; return c[e] = t, Object.defineProperty(r, n, c); }
var Point = /*#__PURE__*/function () {
  function Point(x, y) {
    this.x = x;
    this.y = y;
  }
  var _proto = Point.prototype;
  _defineAccessor("get", _proto, "length", function () {
    return Math.sqrt(this.x * this.x + this.y * this.y);
  });
  _defineAccessor("set", _proto, "length", function (v) {
    var k = v / this.length;
    this.x *= k;
    this.y *= k;
  });
  _proto.add = function add(other) {
    return new Point(this.x + other.x, this.y + other.y);
  };
  _proto["to" + "Pair"] = function () {
    return [this.x, this.y];
  };
  Point.origin = function origin() {
    return new this(0, 0);
  };
  _defineAccessor("get", Point, "unit", function () {
    return new Point(1, 0);
  });
  return Point;
}();
var p = new Point(3, 4);
console.log(p.length, p.add(Point.origin()).toPair().join(","), Point.unit.x);
p.length = 10;
console.log(p.x, p.y);
console.log(typeof Point, Point.name);
var Anon = /*#__PURE__*/function () {
  function Anon() {}
  var _proto2 = Anon.prototype;
  _proto2.hi = function hi() {
    return "anon";
  };
  return Anon;
}();
console.log(new Anon().hi(), Anon.name);
