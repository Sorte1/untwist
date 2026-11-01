function _callSuper(t, o, e) { return o = _getPrototypeOf(o), _possibleConstructorReturn(t, _isNativeReflectConstruct() ? Reflect.construct(o, e || [], _getPrototypeOf(t).constructor) : o.apply(t, e)); }
function _possibleConstructorReturn(t, e) { if (e && ("object" == _typeof(e) || "function" == typeof e)) return e; if (void 0 !== e) throw new TypeError("Derived constructors may only return object or undefined"); return _assertThisInitialized(t); }
function _assertThisInitialized(e) { if (void 0 === e) throw new ReferenceError("this hasn't been initialised - super() hasn't been called"); return e; }
function _isNativeReflectConstruct() { try { var t = !Boolean.prototype.valueOf.call(Reflect.construct(Boolean, [], function () {})); } catch (t) {} return (_isNativeReflectConstruct = function _isNativeReflectConstruct() { return !!t; })(); }
function _superPropGet(t, o, e, r) { var p = _get(_getPrototypeOf(1 & r ? t.prototype : t), o, e); return 2 & r && "function" == typeof p ? function (t) { return p.apply(e, t); } : p; }
function _get() { return _get = "undefined" != typeof Reflect && Reflect.get ? Reflect.get.bind() : function (e, t, r) { var p = _superPropBase(e, t); if (p) { var n = Object.getOwnPropertyDescriptor(p, t); return n.get ? n.get.call(arguments.length < 3 ? e : r) : n.value; } }, _get.apply(null, arguments); }
function _superPropBase(t, o) { for (; !{}.hasOwnProperty.call(t, o) && null !== (t = _getPrototypeOf(t));); return t; }
function _getPrototypeOf(t) { return _getPrototypeOf = Object.setPrototypeOf ? Object.getPrototypeOf.bind() : function (t) { return t.__proto__ || Object.getPrototypeOf(t); }, _getPrototypeOf(t); }
function _inherits(t, e) { if ("function" != typeof e && null !== e) throw new TypeError("Super expression must either be null or a function"); t.prototype = Object.create(e && e.prototype, { constructor: { value: t, writable: !0, configurable: !0 } }), Object.defineProperty(t, "prototype", { writable: !1 }), e && _setPrototypeOf(t, e); }
function _setPrototypeOf(t, e) { return _setPrototypeOf = Object.setPrototypeOf ? Object.setPrototypeOf.bind() : function (t, e) { return t.__proto__ = e, t; }, _setPrototypeOf(t, e); }
function _typeof(o) { "@babel/helpers - typeof"; return _typeof = "function" == typeof Symbol && "symbol" == typeof Symbol.iterator ? function (o) { return typeof o; } : function (o) { return o && "function" == typeof Symbol && o.constructor === Symbol && o !== Symbol.prototype ? "symbol" : typeof o; }, _typeof(o); }
function _classCallCheck(a, n) { if (!(a instanceof n)) throw new TypeError("Cannot call a class as a function"); }
function _defineProperties(e, r) { for (var t = 0; t < r.length; t++) { var o = r[t]; o.enumerable = o.enumerable || !1, o.configurable = !0, "value" in o && (o.writable = !0), Object.defineProperty(e, _toPropertyKey(o.key), o); } }
function _createClass(e, r, t) { return r && _defineProperties(e.prototype, r), t && _defineProperties(e, t), Object.defineProperty(e, "prototype", { writable: !1 }), e; }
function _toPropertyKey(t) { var i = _toPrimitive(t, "string"); return "symbol" == _typeof(i) ? i : i + ""; }
function _toPrimitive(t, r) { if ("object" != _typeof(t) || !t) return t; var e = t[Symbol.toPrimitive]; if (void 0 !== e) { var i = e.call(t, r || "default"); if ("object" != _typeof(i)) return i; throw new TypeError("@@toPrimitive must return a primitive value."); } return ("string" === r ? String : Number)(t); }
var Base = /*#__PURE__*/function () {
  "use strict";

  function Base(v) {
    _classCallCheck(this, Base);
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
    key: "comp" + "uted",
    value: function value() {
      return "c";
    }
  }], [{
    key: "make",
    value: function make(v) {
      return new this(v);
    }
  }, {
    key: "kind",
    get: function get() {
      return "base";
    }
  }]);
}();
var Child = /*#__PURE__*/function (_Base) {
  "use strict";

  function Child(v, w) {
    var _this;
    _classCallCheck(this, Child);
    _this = _callSuper(this, Child, [v]);
    _this.w = w;
    return _this;
  }
  _inherits(Child, _Base);
  return _createClass(Child, [{
    key: "greet",
    value: function greet(who) {
      return _superPropGet(Child, "greet", this, 3)([who]).toUpperCase() + this.w;
    }
  }], [{
    key: "make",
    value: function make(v) {
      return _superPropGet(Child, "make", this, 2)([v]);
    }
  }]);
}(Base);
var c = new Child(1, 2);
console.log(c.greet("x"), c.val, (c.val = 5, c.v), c.computed());
console.log(Child.make(3) instanceof Child, Child.kind, Object.getPrototypeOf(Child) === Base);
console.log(Base.name, Child.name, _typeof(Base));
console.log(Object.getOwnPropertyNames(Base.prototype).sort().join(","), Object.keys(Base.prototype).length);
var d = Object.getOwnPropertyDescriptor(Base.prototype, "greet");
console.log(d.enumerable, d.writable, d.configurable);
console.log(Object.getOwnPropertyDescriptor(Base, "make").enumerable);
try {
  Base(1);
} catch (e) {
  console.log(e.constructor.name);
}
var Anon = /*#__PURE__*/_createClass(function Anon() {
  "use strict";

  _classCallCheck(this, Anon);
});
console.log(Anon.name);
var Counter = /*#__PURE__*/function () {
  "use strict";

  function Counter() {
    _classCallCheck(this, Counter);
    this.n = 0;
  }
  return _createClass(Counter, [{
    key: "inc",
    value: function inc() {
      this.n++;
      return this;
    }
  }]);
}();
console.log(new Counter().inc().inc().n);
function Legacy(x) {
  this.x = x;
}
Legacy.prototype.get = function () {
  return this.x;
};
var FromLegacy = /*#__PURE__*/function (_Legacy) {
  "use strict";

  function FromLegacy() {
    _classCallCheck(this, FromLegacy);
    return _callSuper(this, FromLegacy, arguments);
  }
  _inherits(FromLegacy, _Legacy);
  return _createClass(FromLegacy, [{
    key: "get",
    value: function get() {
      return _superPropGet(FromLegacy, "get", this, 3)([]) + 1;
    }
  }]);
}(Legacy);
console.log(new FromLegacy(4).get());
