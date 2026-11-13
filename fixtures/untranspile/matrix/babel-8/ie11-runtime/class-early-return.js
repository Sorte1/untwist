(function () {
  'use strict';

  function _setPrototypeOf(t, e) {
    return _setPrototypeOf = Object.setPrototypeOf ? Object.setPrototypeOf.bind() : function (t, e) {
      return t.__proto__ = e, t;
    }, _setPrototypeOf(t, e);
  }

  function _inherits(t, r) {
    if ("function" != typeof r && null !== r) throw new TypeError("Super expression must either be null or a function");
    t.prototype = Object.create(r && r.prototype, {
      constructor: {
        value: t,
        writable: true,
        configurable: true
      }
    }), Object.defineProperty(t, "prototype", {
      writable: false
    }), r && _setPrototypeOf(t, r);
  }

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

  function _isNativeReflectConstruct() {
    try {
      var t = !Boolean.prototype.valueOf.call(Reflect.construct(Boolean, [], function () {}));
    } catch (t) {}
    return (_isNativeReflectConstruct = function _isNativeReflectConstruct() {
      return !!t;
    })();
  }

  function _callSuper(e, c, p) {
    return c = _getPrototypeOf(c), _possibleConstructorReturn(e, _isNativeReflectConstruct() ? Reflect.construct(c, p || [], _getPrototypeOf(e).constructor) : c.apply(e, p));
  }

  function _createClass(e, r, t) {
    return Object.defineProperty(e, "prototype", {
      writable: false
    }), e;
  }

  function _classCallCheck(a, n) {
    if (!(a instanceof n)) throw new TypeError("Cannot call a class as a function");
  }

  var Resource = /*#__PURE__*/_createClass(function Resource(name) {
    _classCallCheck(this, Resource);
    this.name = name;
  });
  var Pooled = /*#__PURE__*/function (_Resource) {
    function Pooled(name) {
      var _this;
      var size = arguments.length > 1 && arguments[1] !== undefined ? arguments[1] : 4;
      _classCallCheck(this, Pooled);
      _this = _callSuper(this, Pooled, [name]);
      _this.size = size;
      if (size > 8) {
        _this.capped = true;
        return _possibleConstructorReturn(_this);
      }
      _this.items = [];
      return _this;
    }
    _inherits(Pooled, _Resource);
    return _createClass(Pooled);
  }(Resource);
  var first = new Pooled("a"),
    second = new Pooled("b", 16);
  console.log(first.items.length, second.capped, second.items, first instanceof Resource);

})();
