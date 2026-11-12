(function () {
  'use strict';

  var _regeneratorRuntime2 = require("@babel/runtime/regenerator");
  var _interopRequireDefault = require("@babel/runtime/helpers/interopRequireDefault");
  var _regenerator = _interopRequireDefault(require("@babel/runtime/regenerator"));
  var _construct2 = _interopRequireDefault(require("@babel/runtime/helpers/construct"));
  var _createClass2 = _interopRequireDefault(require("@babel/runtime/helpers/createClass"));
  var _classCallCheck2 = _interopRequireDefault(require("@babel/runtime/helpers/classCallCheck"));
  var _toConsumableArray2 = _interopRequireDefault(require("@babel/runtime/helpers/toConsumableArray"));
  var _obj$vals;
  var _marked = /*#__PURE__*/_regeneratorRuntime2.mark(g);
  function sum() {
    for (var _len = arguments.length, nums = new Array(_len), _key = 0; _key < _len; _key++) {
      nums[_key] = arguments[_key];
    }
    return nums.reduce(function (a, b) {
      return a + b;
    }, 0);
  }
  var a = [1, 2];
  var b = [3, 4];
  console.log([].concat(a, b, [5]).join(","), sum.apply(void 0, a.concat(b)), sum.apply(void 0, [0].concat(a)));
  function attempt(label, f) {
    try {
      console.log(label, f());
    } catch (e) {
      console.log(label, e.constructor.name);
    }
  }
  attempt("string", function () {
    return (0, _toConsumableArray2.default)("abc").join("-");
  });
  attempt("set", function () {
    return (0, _toConsumableArray2.default)(new Set([1, 1, 2])).join(",");
  });
  var holes = [, 1];
  console.log([].concat(holes).length, 0 in [].concat(holes));
  console.log(Math.max.apply(Math, a.concat(b)));
  var Box = /*#__PURE__*/(0, _createClass2.default)(function Box() {
    (0, _classCallCheck2.default)(this, Box);
    for (var _len2 = arguments.length, items = new Array(_len2), _key2 = 0; _key2 < _len2; _key2++) {
      items[_key2] = arguments[_key2];
    }
    this.items = items;
  });
  console.log((0, _construct2.default)(Box, a.concat([9])).items.join(","));
  var obj = {
    vals: [7, 8],
    total: function total() {
      return sum.apply(void 0, (0, _toConsumableArray2.default)(this.vals));
    }
  };
  console.log(obj.total(), (_obj$vals = obj.vals).concat.apply(_obj$vals, [[9], [10]]).join(","));
  function g() {
    return _regenerator.default.wrap(function (_context) {
      while (1) switch (_context.prev = _context.next) {
        case 0:
          _context.next = 1;
          return "x";
        case 1:
          _context.next = 2;
          return "y";
        case 2:
        case "end":
          return _context.stop();
      }
    }, _marked);
  }
  attempt("generator", function () {
    return [].concat((0, _toConsumableArray2.default)(g()), (0, _toConsumableArray2.default)(g())).join("");
  });

})();
