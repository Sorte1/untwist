function _typeof(o) { "@babel/helpers - typeof"; return _typeof = "function" == typeof Symbol && "symbol" == typeof Symbol.iterator ? function (o) { return typeof o; } : function (o) { return o && "function" == typeof Symbol && o.constructor === Symbol && o !== Symbol.prototype ? "symbol" : typeof o; }, _typeof(o); }
var _Fields, _WithStaticBlock;
var _ref;
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
var counter = 0;
_ref = "k" + counter;
var Fields = /*#__PURE__*/function () {
  "use strict";

  function Fields() {
    _classCallCheck(this, Fields);
    _defineProperty(this, "a", 1);
    _defineProperty(this, "b", this.a + 1);
    _defineProperty(this, _ref, "dyn");
  }
  return _createClass(Fields, [{
    key: "m",
    value: function m() {
      return this.a + this.b;
    }
  }]);
}();
_Fields = Fields;
_defineProperty(Fields, "s", ++counter);
_defineProperty(Fields, "t", _Fields.s + 1);
var f = new Fields();
console.log(f.a, f.b, Fields.s, Fields.t, f.k1, f.m(), Object.keys(f).join(","));
var Parent = /*#__PURE__*/function () {
  "use strict";

  function Parent() {
    _classCallCheck(this, Parent);
    this.init();
  }
  return _createClass(Parent, [{
    key: "init",
    value: function init() {}
  }]);
}();
var Kid = /*#__PURE__*/function (_Parent) {
  "use strict";

  function Kid() {
    var _this;
    _classCallCheck(this, Kid);
    for (var _len = arguments.length, args = new Array(_len), _key = 0; _key < _len; _key++) {
      args[_key] = arguments[_key];
    }
    _this = _callSuper(this, Kid, [].concat(args));
    _defineProperty(_this, "x", 1);
    return _this;
  }
  _inherits(Kid, _Parent);
  return _createClass(Kid, [{
    key: "init",
    value: function init() {
      this.seen = this.x;
    }
  }]);
}(Parent);
console.log(new Kid().seen, new Kid().x);
var WithStaticBlock = /*#__PURE__*/_createClass(function WithStaticBlock() {
  "use strict";

  _classCallCheck(this, WithStaticBlock);
});
_WithStaticBlock = WithStaticBlock;
_defineProperty(WithStaticBlock, "z", void 0);
_WithStaticBlock.z = 42;
console.log(WithStaticBlock.z);
var Arrowed = /*#__PURE__*/_createClass(function Arrowed() {
  "use strict";

  var _this2 = this;
  _classCallCheck(this, Arrowed);
  _defineProperty(this, "name", "arrowed");
  _defineProperty(this, "get", function () {
    return _this2.name;
  });
});
var _Arrowed2 = new Arrowed(),
  get = _Arrowed2.get;
console.log(get());
