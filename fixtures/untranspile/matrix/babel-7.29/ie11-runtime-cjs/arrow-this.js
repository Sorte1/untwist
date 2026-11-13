(function () {
  'use strict';

  var _interopRequireDefault = require("@babel/runtime/helpers/interopRequireDefault");
  var _typeof2 = _interopRequireDefault(require("@babel/runtime/helpers/typeof"));
  var _classCallCheck2 = _interopRequireDefault(require("@babel/runtime/helpers/classCallCheck"));
  var _createClass2 = _interopRequireDefault(require("@babel/runtime/helpers/createClass"));
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
      (0, _classCallCheck2.default)(this, Widget);
      this.name = "w";
      this.handler = function () {
        return _this4.name;
      };
    }
    return (0, _createClass2.default)(Widget, [{
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
      return (0, _typeof2.default)(_newtarget);
    };
  }
  console.log(outerFn()(), new outerFn()());

})();
