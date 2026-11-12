"use strict";

function _inheritsLoose(t, o) { t.prototype = Object.create(o.prototype), t.prototype.constructor = t, _setPrototypeOf(t, o); }
function _wrapNativeSuper(t) { var r = "function" == typeof Map ? new Map() : void 0; return _wrapNativeSuper = function _wrapNativeSuper(t) { if (null === t || !_isNativeFunction(t)) return t; if ("function" != typeof t) throw new TypeError("Super expression must either be null or a function"); if (void 0 !== r) { if (r.has(t)) return r.get(t); r.set(t, Wrapper); } function Wrapper() { return _construct(t, arguments, _getPrototypeOf(this).constructor); } return Wrapper.prototype = Object.create(t.prototype, { constructor: { value: Wrapper, enumerable: !1, writable: !0, configurable: !0 } }), _setPrototypeOf(Wrapper, t); }, _wrapNativeSuper(t); }
function _construct(t, e, r) { if (_isNativeReflectConstruct()) return Reflect.construct.apply(null, arguments); var o = [null]; o.push.apply(o, e); var p = new (t.bind.apply(t, o))(); return r && _setPrototypeOf(p, r.prototype), p; }
function _isNativeReflectConstruct() { try { var t = !Boolean.prototype.valueOf.call(Reflect.construct(Boolean, [], function () {})); } catch (t) {} return (_isNativeReflectConstruct = function _isNativeReflectConstruct() { return !!t; })(); }
function _isNativeFunction(t) { try { return -1 !== Function.toString.call(t).indexOf("[native code]"); } catch (n) { return "function" == typeof t; } }
function _setPrototypeOf(t, e) { return _setPrototypeOf = Object.setPrototypeOf ? Object.setPrototypeOf.bind() : function (t, e) { return t.__proto__ = e, t; }, _setPrototypeOf(t, e); }
function _getPrototypeOf(t) { return _getPrototypeOf = Object.setPrototypeOf ? Object.getPrototypeOf.bind() : function (t) { return t.__proto__ || Object.getPrototypeOf(t); }, _getPrototypeOf(t); }
var AppError = /*#__PURE__*/function (_Error) {
  function AppError(message, code) {
    var _this;
    _this = _Error.call(this, message) || this;
    _this.name = "AppError";
    _this.code = code;
    return _this;
  }
  _inheritsLoose(AppError, _Error);
  var _proto = AppError.prototype;
  _proto.describe = function describe() {
    return this.name + "[" + this.code + "]: " + this.message;
  };
  return AppError;
}( /*#__PURE__*/_wrapNativeSuper(Error));
var err = new AppError("bad", 7);
console.log(err instanceof AppError, err instanceof Error, err.message, err.code);
console.log(typeof err.describe === "function" ? err.describe() : "no method");
var Stack = /*#__PURE__*/function (_Array) {
  function Stack() {
    return _Array.apply(this, arguments) || this;
  }
  _inheritsLoose(Stack, _Array);
  var _proto2 = Stack.prototype;
  _proto2.peek = function peek() {
    return this[this.length - 1];
  };
  return Stack;
}( /*#__PURE__*/_wrapNativeSuper(Array));
var s = new Stack();
s.push(1, 2, 3);
console.log(s.length, s instanceof Stack, typeof s.peek === "function" ? s.peek() : "no method");
var Registry = /*#__PURE__*/function (_Map) {
  function Registry() {
    return _Map.apply(this, arguments) || this;
  }
  _inheritsLoose(Registry, _Map);
  var _proto3 = Registry.prototype;
  _proto3.register = function register(k, v) {
    this.set(k, v);
    return this;
  };
  return Registry;
}( /*#__PURE__*/_wrapNativeSuper(Map));
var r = new Registry();
console.log(typeof r.register === "function" ? r.register("a", 1).get("a") : "no method", r instanceof Map);
