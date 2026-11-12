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
    return r && _defineProperties(e.prototype, r), Object.defineProperty(e, "prototype", {
      writable: false
    }), e;
  }

  var counter = {
    count: 0,
    start: function start() {
      var _this = this;
      [1, 2, 3].forEach(function (n) {
        _this.count += n;
      });
      return this.count;
    },
    nested: function nested() {
      var _this2 = this;
      var outer = function outer() {
        var inner = function inner() {
          return _this2.count;
        };
        return inner();
      };
      return outer();
    },
    argsLater: function argsLater() {
      var _arguments = arguments;
      var f = function f() {
        return _arguments[0] + _arguments.length;
      };
      return f();
    }
  };
  console.log(counter.start(), counter.nested(), counter.argsLater("a", "b"));
  function Timer() {
    var _this3 = this;
    this.ticks = 0;
    var tick = function tick() {
      return ++_this3.ticks;
    };
    tick();
    tick();
  }
  console.log(new Timer().ticks);
  var Widget = /*#__PURE__*/function () {
    function Widget() {
      var _this4 = this;
      _classCallCheck(this, Widget);
      this.name = "w";
      this.handler = function () {
        return _this4.name;
      };
    }
    return _createClass(Widget, [{
      key: "render",
      value: function render() {
        var _this5 = this;
        return [1].map(function () {
          return _this5.name + _this5.handler();
        })[0];
      }
    }]);
  }();
  console.log(new Widget().render());
  var id = function id(x) {
    return x;
  };
  var obj = function obj(a, b) {
    return {
      a: a,
      b: b
    };
  };
  console.log(id(5), JSON.stringify(obj(1, 2)));
  function outerFn() {
    var _newtarget = this instanceof outerFn ? this.constructor : void 0;
    return function () {
      return _typeof(_newtarget);
    };
  }
  console.log(outerFn()(), new outerFn()());

})();
