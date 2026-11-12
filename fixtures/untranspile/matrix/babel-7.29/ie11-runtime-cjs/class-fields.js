(function () {
  'use strict';

  var _interopRequireDefault = require("@babel/runtime/helpers/interopRequireDefault");
  var _possibleConstructorReturn2 = _interopRequireDefault(require("@babel/runtime/helpers/possibleConstructorReturn"));
  var _getPrototypeOf2 = _interopRequireDefault(require("@babel/runtime/helpers/getPrototypeOf"));
  var _inherits2 = _interopRequireDefault(require("@babel/runtime/helpers/inherits"));
  var _createClass2 = _interopRequireDefault(require("@babel/runtime/helpers/createClass"));
  var _classCallCheck2 = _interopRequireDefault(require("@babel/runtime/helpers/classCallCheck"));
  var _defineProperty2 = _interopRequireDefault(require("@babel/runtime/helpers/defineProperty"));
  var _Counter;
  var _ref;
  function _callSuper(t, o, e) { return o = (0, _getPrototypeOf2.default)(o), (0, _possibleConstructorReturn2.default)(t, _isNativeReflectConstruct() ? Reflect.construct(o, e || [], (0, _getPrototypeOf2.default)(t).constructor) : o.apply(t, e)); }
  function _isNativeReflectConstruct() { try { var t = !Boolean.prototype.valueOf.call(Reflect.construct(Boolean, [], function () {})); } catch (t) {} return (_isNativeReflectConstruct = function _isNativeReflectConstruct() { return !!t; })(); }
  var seq = 0;
  _ref = "dyn" + seq;
  var Counter = /*#__PURE__*/(0, _createClass2.default)(function Counter(step) {
    var _this = this;
    (0, _classCallCheck2.default)(this, Counter);
    (0, _defineProperty2.default)(this, "count", 0);
    (0, _defineProperty2.default)(this, "step", 1);
    (0, _defineProperty2.default)(this, "id", ++seq);
    (0, _defineProperty2.default)(this, "label", "c" + this.id);
    (0, _defineProperty2.default)(this, _ref, "dynamic");
    (0, _defineProperty2.default)(this, "inc", function () {
      _this.count += _this.step;
      return _this;
    });
    if (step) this.step = step;
    Counter.instances++;
  });
  _Counter = Counter;
  (0, _defineProperty2.default)(Counter, "instances", 0);
  (0, _defineProperty2.default)(Counter, "created", _Counter.instances + 1);
  var c = new Counter(2);
  c.inc().inc();
  console.log(c.count, c.label, c.dyn1, Counter.instances, Counter.created);
  var inc = c.inc;
  inc();
  console.log(c.count, Object.keys(c).join(","));
  var Base = /*#__PURE__*/function () {
    function Base() {
      (0, _classCallCheck2.default)(this, Base);
      this.setup();
    }
    return (0, _createClass2.default)(Base, [{
      key: "setup",
      value: function setup() {}
    }]);
  }();
  var Derived = /*#__PURE__*/function (_Base) {
    function Derived() {
      var _this2;
      (0, _classCallCheck2.default)(this, Derived);
      for (var _len = arguments.length, args = new Array(_len), _key = 0; _key < _len; _key++) {
        args[_key] = arguments[_key];
      }
      _this2 = _callSuper(this, Derived, [].concat(args));
      (0, _defineProperty2.default)(_this2, "value", 5);
      return _this2;
    }
    (0, _inherits2.default)(Derived, _Base);
    return (0, _createClass2.default)(Derived, [{
      key: "setup",
      value: function setup() {
        this.seen = this.value;
      }
    }]);
  }(Base);
  var dd = new Derived();
  console.log(dd.seen, dd.value);
  var Shadow = /*#__PURE__*/function () {
    function Shadow() {
      (0, _classCallCheck2.default)(this, Shadow);
    }
    return (0, _createClass2.default)(Shadow, [{
      key: "x",
      set: function set(v) {
        this.setterRan = true;
      }
    }]);
  }();
  var Defines = /*#__PURE__*/function (_Shadow) {
    function Defines() {
      var _this3;
      (0, _classCallCheck2.default)(this, Defines);
      for (var _len2 = arguments.length, args = new Array(_len2), _key2 = 0; _key2 < _len2; _key2++) {
        args[_key2] = arguments[_key2];
      }
      _this3 = _callSuper(this, Defines, [].concat(args));
      (0, _defineProperty2.default)(_this3, "x", 1);
      return _this3;
    }
    (0, _inherits2.default)(Defines, _Shadow);
    return (0, _createClass2.default)(Defines);
  }(Shadow);
  var df = new Defines();
  console.log(df.x, df.setterRan === true);

})();
