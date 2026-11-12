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

  function _assertThisInitialized(e) {
    if (void 0 === e) throw new ReferenceError("this hasn't been initialised - super() hasn't been called");
    return e;
  }

  function _possibleConstructorReturn(t, e) {
    if (e && ("object" == _typeof(e) || "function" == typeof e)) return e;
    if (void 0 !== e) throw new TypeError("Derived constructors may only return object or undefined");
    return _assertThisInitialized(t);
  }

  function _callSuper(e, c, p) {
    return c = _getPrototypeOf(c), _possibleConstructorReturn(e, _isNativeReflectConstruct() ? Reflect.construct(c, p || [], _getPrototypeOf(e).constructor) : c.apply(e, p));
  }

  function _classCallCheck(a, n) {
    if (!(a instanceof n)) throw new TypeError("Cannot call a class as a function");
  }

  function toPrimitive(t, e) {
    if ("object" != _typeof(t) || !t) return t;
    var r;
    if ("undefined" != typeof Symbol && void 0 !== (r = t[Symbol.toPrimitive])) {
      var i = r.call(t, e);
      if ("object" != _typeof(i)) return i;
      throw new TypeError("@@toPrimitive must return a primitive value.");
    }
    return (String )(t);
  }

  function toPropertyKey(r) {
    var o = toPrimitive(r, "string");
    return "symbol" == _typeof(o) ? o : o + "";
  }

  function _defineProperties(r, t) {
    for (var o = 0; o < t.length; o++) {
      var i = t[o];
      i.enumerable = i.enumerable || false, i.configurable = true, "value" in i && (i.writable = true), Object.defineProperty(r, toPropertyKey(i.key), i);
    }
  }
  function _createClass(e, r, t) {
    return r && _defineProperties(e.prototype, r), t && _defineProperties(e, t), Object.defineProperty(e, "prototype", {
      writable: false
    }), e;
  }

  var Shape = /*#__PURE__*/function () {
    function Shape(kind) {
      _classCallCheck(this, Shape);
      this.kind = kind;
    }
    return _createClass(Shape, [{
      key: "area",
      value: function area() {
        return 0;
      }
    }], [{
      key: "of",
      value: function of(kind) {
        return new Shape(kind);
      }
    }]);
  }();
  var Square = /*#__PURE__*/function (_Shape) {
    function Square(side) {
      var _this;
      _classCallCheck(this, Square);
      _this = _callSuper(this, Square, ["square"]);
      _this.side = side;
      return _this;
    }
    _inherits(Square, _Shape);
    return _createClass(Square, [{
      key: "area",
      value: function area() {
        return this.side * this.side;
      }
    }]);
  }(Shape);
  var s = new Square(3);
  var seen = [];
  for (var key in s) seen.push(key);
  console.log(seen.join(","), Object.keys(Shape.prototype).length, Object.getOwnPropertyNames(Square.prototype).join(","));
  console.log(Object.getOwnPropertyDescriptor(Shape, "prototype").writable, Object.getOwnPropertyDescriptor(Shape.prototype, "area").enumerable);
  for (var _i = 0, _arr = [function () {
      return Shape("x");
    }, function () {
      return Square(1);
    }]; _i < _arr.length; _i++) {
    var make = _arr[_i];
    try {
      make();
      console.log("called without new");
    } catch (e) {
      console.log(e.constructor.name);
    }
  }
  console.log(Shape.length, Square.length, _typeof(Shape.prototype.area.prototype));

})();
