"use strict";

function _defineProperties(e, r) { for (var t = 0; t < r.length; t++) { var o = r[t]; o.enumerable = o.enumerable || !1, o.configurable = !0, "value" in o && (o.writable = !0), Object.defineProperty(e, _toPropertyKey(o.key), o); } }
function _createClass(e, r, t) { return r && _defineProperties(e.prototype, r), t && _defineProperties(e, t), Object.defineProperty(e, "prototype", { writable: !1 }), e; }
function _toPropertyKey(t) { var i = _toPrimitive(t, "string"); return "symbol" == typeof i ? i : i + ""; }
function _toPrimitive(t, r) { if ("object" != typeof t || !t) return t; var e = t[Symbol.toPrimitive]; if (void 0 !== e) { var i = e.call(t, r || "default"); if ("object" != typeof i) return i; throw new TypeError("@@toPrimitive must return a primitive value."); } return ("string" === r ? String : Number)(t); }
var Point = /*#__PURE__*/function () {
  function Point(x, y) {
    this.x = x;
    this.y = y;
  }
  var _proto = Point.prototype;
  _proto.add = function add(other) {
    return new Point(this.x + other.x, this.y + other.y);
  };
  _proto["to" + "Pair"] = function () {
    return [this.x, this.y];
  };
  Point.origin = function origin() {
    return new this(0, 0);
  };
  return _createClass(Point, [{
    key: "length",
    get: function get() {
      return Math.sqrt(this.x * this.x + this.y * this.y);
    },
    set: function set(v) {
      var k = v / this.length;
      this.x *= k;
      this.y *= k;
    }
  }], [{
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
