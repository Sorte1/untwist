(function () {
  'use strict';

  var _interopRequireDefault = require("@babel/runtime/helpers/interopRequireDefault");
  var _classCallCheck2 = _interopRequireDefault(require("@babel/runtime/helpers/classCallCheck"));
  var _createClass2 = _interopRequireDefault(require("@babel/runtime/helpers/createClass"));
  var _possibleConstructorReturn2 = _interopRequireDefault(require("@babel/runtime/helpers/possibleConstructorReturn"));
  var _getPrototypeOf2 = _interopRequireDefault(require("@babel/runtime/helpers/getPrototypeOf"));
  var _inherits2 = _interopRequireDefault(require("@babel/runtime/helpers/inherits"));
  var _wrapNativeSuper2 = _interopRequireDefault(require("@babel/runtime/helpers/wrapNativeSuper"));
  function _callSuper(t, o, e) { return o = (0, _getPrototypeOf2.default)(o), (0, _possibleConstructorReturn2.default)(t, _isNativeReflectConstruct() ? Reflect.construct(o, e || [], (0, _getPrototypeOf2.default)(t).constructor) : o.apply(t, e)); }
  function _isNativeReflectConstruct() { try { var t = !Boolean.prototype.valueOf.call(Reflect.construct(Boolean, [], function () {})); } catch (t) {} return (_isNativeReflectConstruct = function _isNativeReflectConstruct() { return !!t; })(); }
  var AppError = /*#__PURE__*/function (_Error) {
    function AppError(message, code) {
      var _this;
      (0, _classCallCheck2.default)(this, AppError);
      _this = _callSuper(this, AppError, [message]);
      _this.name = "AppError";
      _this.code = code;
      return _this;
    }
    (0, _inherits2.default)(AppError, _Error);
    return (0, _createClass2.default)(AppError, [{
      key: "describe",
      value: function describe() {
        return this.name + "[" + this.code + "]: " + this.message;
      }
    }]);
  }(/*#__PURE__*/(0, _wrapNativeSuper2.default)(Error));
  var err = new AppError("bad", 7);
  console.log(err instanceof AppError, err instanceof Error, err.message, err.code);
  console.log(typeof err.describe === "function" ? err.describe() : "no method");
  var Stack = /*#__PURE__*/function (_Array) {
    function Stack() {
      (0, _classCallCheck2.default)(this, Stack);
      return _callSuper(this, Stack, arguments);
    }
    (0, _inherits2.default)(Stack, _Array);
    return (0, _createClass2.default)(Stack, [{
      key: "peek",
      value: function peek() {
        return this[this.length - 1];
      }
    }]);
  }(/*#__PURE__*/(0, _wrapNativeSuper2.default)(Array));
  var s = new Stack();
  s.push(1, 2, 3);
  console.log(s.length, s instanceof Stack, typeof s.peek === "function" ? s.peek() : "no method");
  var Registry = /*#__PURE__*/function (_Map) {
    function Registry() {
      (0, _classCallCheck2.default)(this, Registry);
      return _callSuper(this, Registry, arguments);
    }
    (0, _inherits2.default)(Registry, _Map);
    return (0, _createClass2.default)(Registry, [{
      key: "register",
      value: function register(k, v) {
        this.set(k, v);
        return this;
      }
    }]);
  }(/*#__PURE__*/(0, _wrapNativeSuper2.default)(Map));
  var r = new Registry();
  console.log(typeof r.register === "function" ? r.register("a", 1).get("a") : "no method", r instanceof Map);

})();
