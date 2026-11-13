"use strict";

function _typeof(o) { "@babel/helpers - typeof"; return _typeof = "function" == typeof Symbol && "symbol" == typeof Symbol.iterator ? function (o) { return typeof o; } : function (o) { return o && "function" == typeof Symbol && o.constructor === Symbol && o !== Symbol.prototype ? "symbol" : typeof o; }, _typeof(o); }
function _classCallCheck(a, n) { if (!(a instanceof n)) throw new TypeError("Cannot call a class as a function"); }
function _defineProperties(e, r) { for (var t = 0; t < r.length; t++) { var o = r[t]; o.enumerable = o.enumerable || !1, o.configurable = !0, "value" in o && (o.writable = !0), Object.defineProperty(e, _toPropertyKey(o.key), o); } }
function _createClass(e, r, t) { return r && _defineProperties(e.prototype, r), t && _defineProperties(e, t), Object.defineProperty(e, "prototype", { writable: !1 }), e; }
function _toPropertyKey(t) { var i = _toPrimitive(t, "string"); return "symbol" == _typeof(i) ? i : i + ""; }
function _toPrimitive(t, e) { if ("object" != _typeof(t) || !t) return t; var r; if ("undefined" != typeof Symbol && void 0 !== (r = t[Symbol.toPrimitive])) { var i = r.call(t, e || "default"); if ("object" != _typeof(i)) return i; throw new TypeError("@@toPrimitive must return a primitive value."); } return ("string" === e ? String : Number)(t); }
var Counter = /*#__PURE__*/function () {
  function Counter() {
    var start = arguments.length > 0 && arguments[0] !== undefined ? arguments[0] : 0;
    _classCallCheck(this, Counter);
    this.count = start;
  }
  return _createClass(Counter, [{
    key: "stepper",
    value: function stepper() {
      var _this = this;
      var by = arguments.length > 0 && arguments[0] !== undefined ? arguments[0] : 1;
      var label = arguments.length > 1 && arguments[1] !== undefined ? arguments[1] : "n" + this.count;
      return function () {
        _this.count += by;
        return label + ":" + _this.count;
      };
    }
  }]);
}();
function tally() {
  var _arguments = arguments,
    _this2 = this;
  var seed = arguments.length > 0 && arguments[0] !== undefined ? arguments[0] : 10;
  var extra = arguments.length > 1 ? arguments[1] : undefined;
  return function () {
    return _this2.base + seed + (extra !== null && extra !== void 0 ? extra : 0) + _arguments.length;
  };
}
var counter = new Counter(2);
var step = counter.stepper();
console.log(step(), step(), counter.stepper(5, "x")(), Counter.prototype.stepper.length);
console.log(tally.call({
  base: 1
})(), tally.call({
  base: 1
}, 3, 4)(), tally.length);
