function _typeof(o) { "@babel/helpers - typeof"; return _typeof = "function" == typeof Symbol && "symbol" == typeof Symbol.iterator ? function (o) { return typeof o; } : function (o) { return o && "function" == typeof Symbol && o.constructor === Symbol && o !== Symbol.prototype ? "symbol" : typeof o; }, _typeof(o); }
function _callSuper(t, o, e) { return o = _getPrototypeOf(o), _possibleConstructorReturn(t, _isNativeReflectConstruct() ? Reflect.construct(o, e || [], _getPrototypeOf(t).constructor) : o.apply(t, e)); }
function _possibleConstructorReturn(t, e) { if (e && ("object" == _typeof(e) || "function" == typeof e)) return e; if (void 0 !== e) throw new TypeError("Derived constructors may only return object or undefined"); return _assertThisInitialized(t); }
function _assertThisInitialized(e) { if (void 0 === e) throw new ReferenceError("this hasn't been initialised - super() hasn't been called"); return e; }
function _isNativeReflectConstruct() { try { var t = !Boolean.prototype.valueOf.call(Reflect.construct(Boolean, [], function () {})); } catch (t) {} return (_isNativeReflectConstruct = function _isNativeReflectConstruct() { return !!t; })(); }
function _getPrototypeOf(t) { return _getPrototypeOf = Object.setPrototypeOf ? Object.getPrototypeOf.bind() : function (t) { return t.__proto__ || Object.getPrototypeOf(t); }, _getPrototypeOf(t); }
function _inherits(t, e) { if ("function" != typeof e && null !== e) throw new TypeError("Super expression must either be null or a function"); t.prototype = Object.create(e && e.prototype, { constructor: { value: t, writable: !0, configurable: !0 } }), Object.defineProperty(t, "prototype", { writable: !1 }), e && _setPrototypeOf(t, e); }
function _setPrototypeOf(t, e) { return _setPrototypeOf = Object.setPrototypeOf ? Object.setPrototypeOf.bind() : function (t, e) { return t.__proto__ = e, t; }, _setPrototypeOf(t, e); }
function _classCallCheck(a, n) { if (!(a instanceof n)) throw new TypeError("Cannot call a class as a function"); }
function _defineProperties(e, r) { for (var t = 0; t < r.length; t++) { var o = r[t]; o.enumerable = o.enumerable || !1, o.configurable = !0, "value" in o && (o.writable = !0), Object.defineProperty(e, _toPropertyKey(o.key), o); } }
function _createClass(e, r, t) { return r && _defineProperties(e.prototype, r), t && _defineProperties(e, t), Object.defineProperty(e, "prototype", { writable: !1 }), e; }
function _defineProperty(e, r, t) { return (r = _toPropertyKey(r)) in e ? Object.defineProperty(e, r, { value: t, enumerable: !0, configurable: !0, writable: !0 }) : e[r] = t, e; }
function _toPropertyKey(t) { var i = _toPrimitive(t, "string"); return "symbol" == _typeof(i) ? i : i + ""; }
function _toPrimitive(t, r) { if ("object" != _typeof(t) || !t) return t; var e = t[Symbol.toPrimitive]; if (void 0 !== e) { var i = e.call(t, r || "default"); if ("object" != _typeof(i)) return i; throw new TypeError("@@toPrimitive must return a primitive value."); } return ("string" === r ? String : Number)(t); }
var Base = /*#__PURE__*/function () {
  "use strict";

  function Base(v) {
    _classCallCheck(this, Base);
    _defineProperty(this, "tag", "base");
    _defineProperty(this, "empty", void 0);
    this.v = v;
  }
  return _createClass(Base, [{
    key: "val",
    get: function get() {
      return this.v;
    },
    set: function set(x) {
      this.v = x * 2;
    }
  }, {
    key: "greet",
    value: function greet(who) {
      return "hi " + who + " " + this.v;
    }
  }, {
    key: Symbol.iterator,
    value: function value() {
      return [this.v][Symbol.iterator]();
    }
  }, {
    key: "with space",
    value: function value() {
      return 1;
    }
  }], [{
    key: "make",
    value: function make(v) {
      return new this(v);
    }
  }]);
}();
var Child = /*#__PURE__*/function (_Base) {
  "use strict";

  function Child(v, w) {
    var _this;
    _classCallCheck(this, Child);
    var doubled = w * 2;
    _this = _callSuper(this, Child, [v]);
    _defineProperty(_this, "kid", doubled);
    _this.w = w;
    return _this;
  }
  _inherits(Child, _Base);
  return _createClass(Child, [{
    key: "greet",
    value: function greet(who) {
      return "child " + this.w;
    }
  }]);
}(Base);
var Chained = /*#__PURE__*/function (_Base2) {
  "use strict";

  function Chained(v) {
    var _this2;
    _classCallCheck(this, Chained);
    (_this2 = _callSuper(this, Chained, [v])).extra = _this2.v + 1;
    _this2.more = 2;
    return _this2;
  }
  _inherits(Chained, _Base2);
  return _createClass(Chained);
}(Base);
var Wrapped = /*#__PURE__*/function (_Base3) {
  "use strict";

  function Wrapped() {
    var _this3;
    _classCallCheck(this, Wrapped);
    var self = _this3 = _callSuper(this, Wrapped, [7]);
    self.seen = self === _this3;
    return _this3;
  }
  _inherits(Wrapped, _Base3);
  return _createClass(Wrapped);
}(Base);
var Forward = /*#__PURE__*/function (_Base4) {
  "use strict";

  function Forward() {
    _classCallCheck(this, Forward);
    return _callSuper(this, Forward, arguments);
  }
  _inherits(Forward, _Base4);
  return _createClass(Forward);
}(Base);
var Rest = /*#__PURE__*/function (_Base5) {
  "use strict";

  function Rest() {
    var _this4;
    _classCallCheck(this, Rest);
    for (var _len = arguments.length, args = new Array(_len), _key = 0; _key < _len; _key++) {
      args[_key] = arguments[_key];
    }
    _this4 = _callSuper(this, Rest, [].concat(args));
    _defineProperty(_this4, "count", args.length);
    return _this4;
  }
  _inherits(Rest, _Base5);
  return _createClass(Rest);
}(Base);
var Plain = /*#__PURE__*/_createClass(function Plain() {
  "use strict";

  _classCallCheck(this, Plain);
});
var b = new Base(1);
var c = new Child(2, 3);
console.log(b.greet("x"), b.val, (b.val = 5, b.v), b.tag, "empty" in b, [...b].join(), b["with space"]());
console.log(Base.make(4).v, c.greet(), c.kid, c.w, c.v, Object.keys(c).join());
var d = new Chained(5);
console.log(d.extra, d.more, d.v, Object.keys(d).join());
var e = new Wrapped();
console.log(e.seen, e.v, new Forward(9).v, new Rest(1, 2, 3).count, new Rest(6).v);
console.log(new Plain() instanceof Plain, Plain.name, Child.name, Object.getPrototypeOf(Child) === Base);
console.log(Object.getOwnPropertyNames(Base.prototype).length, Object.keys(Base.prototype).length);
var desc = Object.getOwnPropertyDescriptor(Base.prototype, "greet");
console.log(desc.enumerable, desc.writable, desc.configurable);
console.log(Object.getOwnPropertyDescriptor(Base, "prototype").writable);
try {
  Base(1);
} catch (err) {
  console.log(err.constructor.name);
}
