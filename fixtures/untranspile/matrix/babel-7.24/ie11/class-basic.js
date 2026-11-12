"use strict";

function _typeof(o) { "@babel/helpers - typeof"; return _typeof = "function" == typeof Symbol && "symbol" == typeof Symbol.iterator ? function (o) { return typeof o; } : function (o) { return o && "function" == typeof Symbol && o.constructor === Symbol && o !== Symbol.prototype ? "symbol" : typeof o; }, _typeof(o); }
function _classCallCheck(a, n) { if (!(a instanceof n)) throw new TypeError("Cannot call a class as a function"); }
function _defineProperties(e, r) { for (var t = 0; t < r.length; t++) { var o = r[t]; o.enumerable = o.enumerable || !1, o.configurable = !0, "value" in o && (o.writable = !0), Object.defineProperty(e, _toPropertyKey(o.key), o); } }
function _createClass(e, r, t) { return r && _defineProperties(e.prototype, r), t && _defineProperties(e, t), Object.defineProperty(e, "prototype", { writable: !1 }), e; }
function _toPropertyKey(t) { var i = _toPrimitive(t, "string"); return "symbol" == _typeof(i) ? i : i + ""; }
function _toPrimitive(t, r) { if ("object" != _typeof(t) || !t) return t; var e = t[Symbol.toPrimitive]; if (void 0 !== e) { var i = e.call(t, r || "default"); if ("object" != _typeof(i)) return i; throw new TypeError("@@toPrimitive must return a primitive value."); } return ("string" === r ? String : Number)(t); }
var Point = /*#__PURE__*/function () {
  function Point(x, y) {
    _classCallCheck(this, Point);
    this.x = x;
    this.y = y;
  }
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
console.log(_typeof(Point), Point.name);
var Anon = /*#__PURE__*/function () {
  function Anon() {
    _classCallCheck(this, Anon);
  }
  return _createClass(Anon, [{
    key: "hi",
    value: function hi() {
      return "anon";
    }
  }]);
}();
console.log(new Anon().hi(), Anon.name);
