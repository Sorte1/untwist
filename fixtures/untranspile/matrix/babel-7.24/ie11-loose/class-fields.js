"use strict";

var _Counter;
var _ref;
function _defineProperties(e, r) { for (var t = 0; t < r.length; t++) { var o = r[t]; o.enumerable = o.enumerable || !1, o.configurable = !0, "value" in o && (o.writable = !0), Object.defineProperty(e, _toPropertyKey(o.key), o); } }
function _createClass(e, r, t) { return r && _defineProperties(e.prototype, r), t && _defineProperties(e, t), Object.defineProperty(e, "prototype", { writable: !1 }), e; }
function _toPropertyKey(t) { var i = _toPrimitive(t, "string"); return "symbol" == typeof i ? i : i + ""; }
function _toPrimitive(t, r) { if ("object" != typeof t || !t) return t; var e = t[Symbol.toPrimitive]; if (void 0 !== e) { var i = e.call(t, r || "default"); if ("object" != typeof i) return i; throw new TypeError("@@toPrimitive must return a primitive value."); } return ("string" === r ? String : Number)(t); }
function _inheritsLoose(t, o) { t.prototype = Object.create(o.prototype), t.prototype.constructor = t, _setPrototypeOf(t, o); }
function _setPrototypeOf(t, e) { return _setPrototypeOf = Object.setPrototypeOf ? Object.setPrototypeOf.bind() : function (t, e) { return t.__proto__ = e, t; }, _setPrototypeOf(t, e); }
var seq = 0;
_ref = "dyn" + seq;
var Counter = function Counter(step) {
  var _this = this;
  this.count = 0;
  this.step = 1;
  this.id = ++seq;
  this.label = "c" + this.id;
  this[_ref] = "dynamic";
  this.inc = function () {
    _this.count += _this.step;
    return _this;
  };
  if (step) this.step = step;
  Counter.instances++;
};
_Counter = Counter;
Counter.instances = 0;
Counter.created = _Counter.instances + 1;
var c = new Counter(2);
c.inc().inc();
console.log(c.count, c.label, c.dyn1, Counter.instances, Counter.created);
var inc = c.inc;
inc();
console.log(c.count, Object.keys(c).join(","));
var Base = /*#__PURE__*/function () {
  function Base() {
    this.setup();
  }
  var _proto = Base.prototype;
  _proto.setup = function setup() {};
  return Base;
}();
var Derived = /*#__PURE__*/function (_Base) {
  function Derived() {
    var _this2;
    for (var _len = arguments.length, args = new Array(_len), _key = 0; _key < _len; _key++) {
      args[_key] = arguments[_key];
    }
    _this2 = _Base.call.apply(_Base, [this].concat(args)) || this;
    _this2.value = 5;
    return _this2;
  }
  _inheritsLoose(Derived, _Base);
  var _proto2 = Derived.prototype;
  _proto2.setup = function setup() {
    this.seen = this.value;
  };
  return Derived;
}(Base);
var dd = new Derived();
console.log(dd.seen, dd.value);
var Shadow = /*#__PURE__*/function () {
  function Shadow() {}
  return _createClass(Shadow, [{
    key: "x",
    set: function set(v) {
      this.setterRan = true;
    }
  }]);
}();
var Defines = /*#__PURE__*/function (_Shadow) {
  function Defines() {
    var _this3;
    for (var _len2 = arguments.length, args = new Array(_len2), _key2 = 0; _key2 < _len2; _key2++) {
      args[_key2] = arguments[_key2];
    }
    _this3 = _Shadow.call.apply(_Shadow, [this].concat(args)) || this;
    _this3.x = 1;
    return _this3;
  }
  _inheritsLoose(Defines, _Shadow);
  return Defines;
}(Shadow);
var df = new Defines();
console.log(df.x, df.setterRan === true);
