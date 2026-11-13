(function () {
  'use strict';

  var _interopRequireDefault = require("@babel/runtime/helpers/interopRequireDefault");
  var _getPrototypeOf2 = _interopRequireDefault(require("@babel/runtime/helpers/getPrototypeOf"));
  var _get2 = _interopRequireDefault(require("@babel/runtime/helpers/get"));
  var _defineProperty2 = _interopRequireDefault(require("@babel/runtime/helpers/defineProperty"));
  var _obj;
  function _superPropGet(t, o, e, r) { var p = (0, _get2.default)((0, _getPrototypeOf2.default)(t), o, e); return "function" == typeof p ? function (t) { return p.apply(e, t); } : p; }
  function _defineAccessor(e, r, n, t) { var c = { configurable: true, enumerable: true }; return c[e] = t, Object.defineProperty(r, n, c); }
  var k = "dyn";
  var x = 1;
  var o = (0, _defineProperty2.default)((0, _defineProperty2.default)(_defineAccessor("get", (0, _defineProperty2.default)((0, _defineProperty2.default)({
    x: x
  }, k + "1", 2), "method", function method() {
    return "m" + this.x;
  }), "g", function () {
    return "get";
  }), "t".concat(k), 3), "__proto__", {
    inherited: true
  });
  console.log(o.x, o.dyn1, o.method(), o.g, o.tdyn, o.inherited, Object.keys(o).join(","));
  var sup = {
    hello: function hello() {
      return "base";
    }
  };
  var sub = _obj = {
    __proto__: sup,
    hello: function hello() {
      return _superPropGet(_obj, "hello", this)([]) + "+sub";
    }
  };
  console.log(sub.hello());

})();
