(function () {
  'use strict';

  function _typeof(o) {
    "@babel/helpers - typeof";

    return _typeof = "function" == typeof Symbol && "symbol" == typeof Symbol.iterator ? function (o) {
      return typeof o;
    } : function (o) {
      return o && "function" == typeof Symbol && o.constructor === Symbol && o !== Symbol.prototype ? "symbol" : typeof o;
    }, _typeof(o);
  }

  function _assertThisInitialized(e) {
    if (void 0 === e) throw new ReferenceError("this hasn't been initialised - super() hasn't been called");
    return e;
  }

  function _possibleConstructorReturn(t, e) {
    if (e && ("object" == _typeof(e) || "function" == typeof e)) return e;
    if (void 0 !== e) throw new TypeError("Derived constructors may only return object or undefined");
    return _assertThisInitialized(t);
  }

  function _getPrototypeOf(t) {
    return _getPrototypeOf = Object.setPrototypeOf ? Object.getPrototypeOf.bind() : function (t) {
      return t.__proto__ || Object.getPrototypeOf(t);
    }, _getPrototypeOf(t);
  }

  function _setPrototypeOf(t, e) {
    return _setPrototypeOf = Object.setPrototypeOf ? Object.setPrototypeOf.bind() : function (t, e) {
      return t.__proto__ = e, t;
    }, _setPrototypeOf(t, e);
  }

  function _inherits(t, e) {
    if ("function" != typeof e && null !== e) throw new TypeError("Super expression must either be null or a function");
    t.prototype = Object.create(e && e.prototype, {
      constructor: {
        value: t,
        writable: true,
        configurable: true
      }
    }), Object.defineProperty(t, "prototype", {
      writable: false
    }), e && _setPrototypeOf(t, e);
  }

  function toPrimitive(t, e) {
    if ("object" != _typeof(t) || !t) return t;
    var r;
    if ("undefined" != typeof Symbol && void 0 !== (r = t[Symbol.toPrimitive])) {
      var i = r.call(t, e);
      if ("object" != _typeof(i)) return i;
      throw new TypeError("@@toPrimitive must return a primitive value.");
    }
    return ("string" === e ? String : Number)(t);
  }

  function toPropertyKey(t) {
    var i = toPrimitive(t, "string");
    return "symbol" == _typeof(i) ? i : i + "";
  }

  function _defineProperties(e, r) {
    for (var t = 0; t < r.length; t++) {
      var o = r[t];
      o.enumerable = o.enumerable || false, o.configurable = true, "value" in o && (o.writable = true), Object.defineProperty(e, toPropertyKey(o.key), o);
    }
  }
  function _createClass(e, r, t) {
    return r && _defineProperties(e.prototype, r), Object.defineProperty(e, "prototype", {
      writable: false
    }), e;
  }

  function _classCallCheck(a, n) {
    if (!(a instanceof n)) throw new TypeError("Cannot call a class as a function");
  }

  function _defineProperty(e, r, t) {
    return (r = toPropertyKey(r)) in e ? Object.defineProperty(e, r, {
      value: t,
      enumerable: true,
      configurable: true,
      writable: true
    }) : e[r] = t, e;
  }

  var _Counter;
  var _ref;
  function _callSuper(t, o, e) { return o = _getPrototypeOf(o), _possibleConstructorReturn(t, _isNativeReflectConstruct() ? Reflect.construct(o, e || [], _getPrototypeOf(t).constructor) : o.apply(t, e)); }
  function _isNativeReflectConstruct() { try { var t = !Boolean.prototype.valueOf.call(Reflect.construct(Boolean, [], function () {})); } catch (t) {} return (_isNativeReflectConstruct = function _isNativeReflectConstruct() { return !!t; })(); }
  var seq = 0;
  _ref = "dyn" + seq;
  var Counter = /*#__PURE__*/_createClass(function Counter(step) {
    var _this = this;
    _classCallCheck(this, Counter);
    _defineProperty(this, "count", 0);
    _defineProperty(this, "step", 1);
    _defineProperty(this, "id", ++seq);
    _defineProperty(this, "label", "c" + this.id);
    _defineProperty(this, _ref, "dynamic");
    _defineProperty(this, "inc", function () {
      _this.count += _this.step;
      return _this;
    });
    if (step) this.step = step;
    Counter.instances++;
  });
  _Counter = Counter;
  _defineProperty(Counter, "instances", 0);
  _defineProperty(Counter, "created", _Counter.instances + 1);
  var c = new Counter(2);
  c.inc().inc();
  console.log(c.count, c.label, c.dyn1, Counter.instances, Counter.created);
  var inc = c.inc;
  inc();
  console.log(c.count, Object.keys(c).join(","));
  var Base = /*#__PURE__*/function () {
    function Base() {
      _classCallCheck(this, Base);
      this.setup();
    }
    return _createClass(Base, [{
      key: "setup",
      value: function setup() {}
    }]);
  }();
  var Derived = /*#__PURE__*/function (_Base) {
    function Derived() {
      var _this2;
      _classCallCheck(this, Derived);
      for (var _len = arguments.length, args = new Array(_len), _key = 0; _key < _len; _key++) {
        args[_key] = arguments[_key];
      }
      _this2 = _callSuper(this, Derived, [].concat(args));
      _defineProperty(_this2, "value", 5);
      return _this2;
    }
    _inherits(Derived, _Base);
    return _createClass(Derived, [{
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
      _classCallCheck(this, Shadow);
    }
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
      _classCallCheck(this, Defines);
      for (var _len2 = arguments.length, args = new Array(_len2), _key2 = 0; _key2 < _len2; _key2++) {
        args[_key2] = arguments[_key2];
      }
      _this3 = _callSuper(this, Defines, [].concat(args));
      _defineProperty(_this3, "x", 1);
      return _this3;
    }
    _inherits(Defines, _Shadow);
    return _createClass(Defines);
  }(Shadow);
  var df = new Defines();
  console.log(df.x, df.setterRan === true);

})();
