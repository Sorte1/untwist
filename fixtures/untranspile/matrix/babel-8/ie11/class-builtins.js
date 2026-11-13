function _typeof(o) { "@babel/helpers - typeof"; return _typeof = "function" == typeof Symbol && "symbol" == typeof Symbol.iterator ? function (o) { return typeof o; } : function (o) { return o && "function" == typeof Symbol && o.constructor === Symbol && o !== Symbol.prototype ? "symbol" : typeof o; }, _typeof(o); }
function _classCallCheck(a, n) { if (!(a instanceof n)) throw new TypeError("Cannot call a class as a function"); }
function _defineProperties(r, t) { for (var o = 0; o < t.length; o++) { var i = t[o]; i.enumerable = i.enumerable || !1, i.configurable = !0, "value" in i && (i.writable = !0), Object.defineProperty(r, _toPropertyKey(i.key), i); } }
function _createClass(e, r, t) { return r && _defineProperties(e.prototype, r), t && _defineProperties(e, t), Object.defineProperty(e, "prototype", { writable: !1 }), e; }
function _toPropertyKey(r) { var o = _toPrimitive(r, "string"); return "symbol" == _typeof(o) ? o : o + ""; }
function _toPrimitive(t, e) { if ("object" != _typeof(t) || !t) return t; var r; if ("undefined" != typeof Symbol && void 0 !== (r = t[Symbol.toPrimitive])) { var i = r.call(t, e || "default"); if ("object" != _typeof(i)) return i; throw new TypeError("@@toPrimitive must return a primitive value."); } return ("string" === e ? String : Number)(t); }
function _inherits(t, r) { if ("function" != typeof r && null !== r) throw new TypeError("Super expression must either be null or a function"); t.prototype = Object.create(r && r.prototype, { constructor: { value: t, writable: !0, configurable: !0 } }), Object.defineProperty(t, "prototype", { writable: !1 }), r && _setPrototypeOf(t, r); }
function _callSuper(e, c, p) { return c = _getPrototypeOf(c), _possibleConstructorReturn(e, _isNativeReflectConstruct() ? Reflect.construct(c, p || [], _getPrototypeOf(e).constructor) : c.apply(e, p)); }
function _possibleConstructorReturn(t, e) { if (e && ("object" == _typeof(e) || "function" == typeof e)) return e; if (void 0 !== e) throw new TypeError("Derived constructors may only return object or undefined"); return _assertThisInitialized(t); }
function _assertThisInitialized(e) { if (void 0 === e) throw new ReferenceError("this hasn't been initialised - super() hasn't been called"); return e; }
function _wrapNativeSuper(p) { var n = "function" == typeof Map ? new Map() : void 0; return _wrapNativeSuper = function _wrapNativeSuper(p) { if (null === p || !_isNativeFunction(p)) return p; if ("function" != typeof p) throw new TypeError("Super expression must either be null or a function"); if (void 0 !== n) { if (n.has(p)) return n.get(p); n.set(p, Wrapper); } function Wrapper() { return _construct(p, arguments, _getPrototypeOf(this).constructor); } return Wrapper.prototype = Object.create(p.prototype, { constructor: { value: Wrapper, enumerable: !1, writable: !0, configurable: !0 } }), _setPrototypeOf(Wrapper, p); }, _wrapNativeSuper(p); }
function _construct(e, o, p) { if (_isNativeReflectConstruct()) return Reflect.construct.apply(null, arguments); var n = [null]; n.push.apply(n, o); var l = new (e.bind.apply(e, n))(); return p && _setPrototypeOf(l, p.prototype), l; }
function _isNativeReflectConstruct() { try { var t = !Boolean.prototype.valueOf.call(Reflect.construct(Boolean, [], function () {})); } catch (t) {} return (_isNativeReflectConstruct = function _isNativeReflectConstruct() { return !!t; })(); }
function _isNativeFunction(t) { try { return -1 !== Function.toString.call(t).indexOf("[native code]"); } catch (n) { return "function" == typeof t; } }
function _setPrototypeOf(t, e) { return _setPrototypeOf = Object.setPrototypeOf ? Object.setPrototypeOf.bind() : function (t, e) { return t.__proto__ = e, t; }, _setPrototypeOf(t, e); }
function _getPrototypeOf(t) { return _getPrototypeOf = Object.setPrototypeOf ? Object.getPrototypeOf.bind() : function (t) { return t.__proto__ || Object.getPrototypeOf(t); }, _getPrototypeOf(t); }
var AppError = /*#__PURE__*/function (_Error) {
  function AppError(message, code) {
    var _this;
    _classCallCheck(this, AppError);
    _this = _callSuper(this, AppError, [message]);
    _this.name = "AppError";
    _this.code = code;
    return _this;
  }
  _inherits(AppError, _Error);
  return _createClass(AppError, [{
    key: "describe",
    value: function describe() {
      return this.name + "[" + this.code + "]: " + this.message;
    }
  }]);
}(/*#__PURE__*/_wrapNativeSuper(Error));
var err = new AppError("bad", 7);
console.log(err instanceof AppError, err instanceof Error, err.message, err.code);
console.log(typeof err.describe === "function" ? err.describe() : "no method");
var Stack = /*#__PURE__*/function (_Array) {
  function Stack() {
    _classCallCheck(this, Stack);
    return _callSuper(this, Stack, arguments);
  }
  _inherits(Stack, _Array);
  return _createClass(Stack, [{
    key: "peek",
    value: function peek() {
      return this[this.length - 1];
    }
  }]);
}(/*#__PURE__*/_wrapNativeSuper(Array));
var s = new Stack();
s.push(1, 2, 3);
console.log(s.length, s instanceof Stack, typeof s.peek === "function" ? s.peek() : "no method");
var Registry = /*#__PURE__*/function (_Map) {
  function Registry() {
    _classCallCheck(this, Registry);
    return _callSuper(this, Registry, arguments);
  }
  _inherits(Registry, _Map);
  return _createClass(Registry, [{
    key: "register",
    value: function register(k, v) {
      this.set(k, v);
      return this;
    }
  }]);
}(/*#__PURE__*/_wrapNativeSuper(Map));
var r = new Registry();
console.log(typeof r.register === "function" ? r.register("a", 1).get("a") : "no method", r instanceof Map);
