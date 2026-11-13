(function () {
  'use strict';

  function _typeof2(obj) { if (typeof Symbol === "function" && typeof Symbol.iterator === "symbol") { _typeof2 = function _typeof2(obj) { return typeof obj; }; } else { _typeof2 = function _typeof2(obj) { return obj && typeof Symbol === "function" && obj.constructor === Symbol && obj !== Symbol.prototype ? "symbol" : typeof obj; }; } return _typeof2(obj); }

  function _typeof(obj) {
    if (typeof Symbol === "function" && _typeof2(Symbol.iterator) === "symbol") {
      _typeof = function _typeof(obj) {
        return _typeof2(obj);
      };
    } else {
      _typeof = function _typeof(obj) {
        return obj && typeof Symbol === "function" && obj.constructor === Symbol && obj !== Symbol.prototype ? "symbol" : _typeof2(obj);
      };
    }

    return _typeof(obj);
  }

  function _assertThisInitialized(self) {
    if (self === void 0) {
      throw new ReferenceError("this hasn't been initialised - super() hasn't been called");
    }

    return self;
  }

  function _possibleConstructorReturn(self, call) {
    if (call && (_typeof(call) === "object" || typeof call === "function")) {
      return call;
    }

    return _assertThisInitialized(self);
  }

  function _getPrototypeOf(o) {
    _getPrototypeOf = Object.setPrototypeOf ? Object.getPrototypeOf : function _getPrototypeOf(o) {
      return o.__proto__ || Object.getPrototypeOf(o);
    };
    return _getPrototypeOf(o);
  }

  function _setPrototypeOf(o, p) {
    _setPrototypeOf = Object.setPrototypeOf || function _setPrototypeOf(o, p) {
      o.__proto__ = p;
      return o;
    };

    return _setPrototypeOf(o, p);
  }

  function _inherits(subClass, superClass) {
    if (typeof superClass !== "function" && superClass !== null) {
      throw new TypeError("Super expression must either be null or a function");
    }

    subClass.prototype = Object.create(superClass && superClass.prototype, {
      constructor: {
        value: subClass,
        writable: true,
        configurable: true
      }
    });
    if (superClass) _setPrototypeOf(subClass, superClass);
  }

  function _defineProperties(target, props) {
    for (var i = 0; i < props.length; i++) {
      var descriptor = props[i];
      descriptor.enumerable = descriptor.enumerable || false;
      descriptor.configurable = true;
      if ("value" in descriptor) descriptor.writable = true;
      Object.defineProperty(target, descriptor.key, descriptor);
    }
  }

  function _createClass(Constructor, protoProps, staticProps) {
    if (protoProps) _defineProperties(Constructor.prototype, protoProps);
    return Constructor;
  }

  function _classCallCheck(instance, Constructor) {
    if (!(instance instanceof Constructor)) {
      throw new TypeError("Cannot call a class as a function");
    }
  }

  function _defineProperty(obj, key, value) {
    if (key in obj) {
      Object.defineProperty(obj, key, {
        value: value,
        enumerable: true,
        configurable: true,
        writable: true
      });
    } else {
      obj[key] = value;
    }

    return obj;
  }

  var seq = 0;

  var _ref = "dyn" + seq;

  var Counter = function Counter(step) {
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
  };

  _defineProperty(Counter, "instances", 0);

  _defineProperty(Counter, "created", Counter.instances + 1);

  var c = new Counter(2);
  c.inc().inc();
  console.log(c.count, c.label, c.dyn1, Counter.instances, Counter.created);
  var inc = c.inc;
  inc();
  console.log(c.count, Object.keys(c).join(","));

  var Base =
  /*#__PURE__*/
  function () {
    function Base() {
      _classCallCheck(this, Base);

      this.setup();
    }

    _createClass(Base, [{
      key: "setup",
      value: function setup() {}
    }]);

    return Base;
  }();

  var Derived =
  /*#__PURE__*/
  function (_Base) {
    _inherits(Derived, _Base);

    function Derived() {
      var _getPrototypeOf2;

      var _this2;

      _classCallCheck(this, Derived);

      for (var _len = arguments.length, args = new Array(_len), _key = 0; _key < _len; _key++) {
        args[_key] = arguments[_key];
      }

      _this2 = _possibleConstructorReturn(this, (_getPrototypeOf2 = _getPrototypeOf(Derived)).call.apply(_getPrototypeOf2, [this].concat(args)));

      _defineProperty(_assertThisInitialized(_assertThisInitialized(_this2)), "value", 5);

      return _this2;
    }

    _createClass(Derived, [{
      key: "setup",
      value: function setup() {
        this.seen = this.value;
      }
    }]);

    return Derived;
  }(Base);

  var dd = new Derived();
  console.log(dd.seen, dd.value);

  var Shadow =
  /*#__PURE__*/
  function () {
    function Shadow() {
      _classCallCheck(this, Shadow);
    }

    _createClass(Shadow, [{
      key: "x",
      set: function set(v) {
        this.setterRan = true;
      }
    }]);

    return Shadow;
  }();

  var Defines =
  /*#__PURE__*/
  function (_Shadow) {
    _inherits(Defines, _Shadow);

    function Defines() {
      var _getPrototypeOf3;

      var _this3;

      _classCallCheck(this, Defines);

      for (var _len2 = arguments.length, args = new Array(_len2), _key2 = 0; _key2 < _len2; _key2++) {
        args[_key2] = arguments[_key2];
      }

      _this3 = _possibleConstructorReturn(this, (_getPrototypeOf3 = _getPrototypeOf(Defines)).call.apply(_getPrototypeOf3, [this].concat(args)));

      _defineProperty(_assertThisInitialized(_assertThisInitialized(_this3)), "x", 1);

      return _this3;
    }

    return Defines;
  }(Shadow);

  var df = new Defines();
  console.log(df.x, df.setterRan === true);

})();
