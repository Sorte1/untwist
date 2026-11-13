(function () {
  'use strict';

  function _typeof(obj) {
    "@babel/helpers - typeof";

    return _typeof = "function" == typeof Symbol && "symbol" == typeof Symbol.iterator ? function (obj) {
      return typeof obj;
    } : function (obj) {
      return obj && "function" == typeof Symbol && obj.constructor === Symbol && obj !== Symbol.prototype ? "symbol" : typeof obj;
    }, _typeof(obj);
  }

  function _classCallCheck(instance, Constructor) {
    if (!(instance instanceof Constructor)) {
      throw new TypeError("Cannot call a class as a function");
    }
  }

  function _toPrimitive(input, hint) {
    if (_typeof(input) !== "object" || input === null) return input;
    var prim = input[Symbol.toPrimitive];
    if (prim !== undefined) {
      var res = prim.call(input, hint);
      if (_typeof(res) !== "object") return res;
      throw new TypeError("@@toPrimitive must return a primitive value.");
    }
    return (String )(input);
  }

  function _toPropertyKey(arg) {
    var key = _toPrimitive(arg, "string");
    return _typeof(key) === "symbol" ? key : String(key);
  }

  function _defineProperties(target, props) {
    for (var i = 0; i < props.length; i++) {
      var descriptor = props[i];
      descriptor.enumerable = descriptor.enumerable || false;
      descriptor.configurable = true;
      if ("value" in descriptor) descriptor.writable = true;
      Object.defineProperty(target, _toPropertyKey(descriptor.key), descriptor);
    }
  }
  function _createClass(Constructor, protoProps, staticProps) {
    if (protoProps) _defineProperties(Constructor.prototype, protoProps);
    Object.defineProperty(Constructor, "prototype", {
      writable: false
    });
    return Constructor;
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
    _createClass(Widget, [{
      key: "render",
      value: function render() {
        var _this5 = this;
        return [1].map(function () {
          return _this5.name + _this5.handler();
        })[0];
      }
    }]);
    return Widget;
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
