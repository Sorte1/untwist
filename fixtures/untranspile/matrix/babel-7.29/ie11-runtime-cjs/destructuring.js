(function () {
  'use strict';

  var _interopRequireDefault = require("@babel/runtime/helpers/interopRequireDefault");
  var _regenerator = _interopRequireDefault(require("@babel/runtime/regenerator"));
  var _slicedToArray2 = _interopRequireDefault(require("@babel/runtime/helpers/slicedToArray"));
  var _objectWithoutProperties2 = _interopRequireDefault(require("@babel/runtime/helpers/objectWithoutProperties"));
  function _arrayLikeToArray(r, a) { (null == a || a > r.length) && (a = r.length); for (var e = 0, n = Array(a); e < a; e++) n[e] = r[e]; return n; }
  var _ref = [1, 2, undefined, 4, 5],
    a = _ref[0],
    _ref$ = _ref[2],
    b = _ref$ === void 0 ? 10 : _ref$,
    rest = _arrayLikeToArray(_ref).slice(3);
  var _x$q$r = {
      x: 1,
      q: 9,
      r: 8
    },
    x = _x$q$r.x,
    _x$q$r$y = _x$q$r.y,
    why = _x$q$r$y === void 0 ? 2 : _x$q$r$y,
    _x$q$r$z = _x$q$r.z,
    _x$q$r$z2 = _x$q$r$z === void 0 ? {
      deep: "d"
    } : _x$q$r$z,
    deep = _x$q$r$z2.deep,
    others = (0, _objectWithoutProperties2.default)(_x$q$r, ["x", "y", "z"]);
  console.log(a, b, rest.join(","), x, why, deep, JSON.stringify(others));
  function point() {
    var _ref2 = arguments.length > 0 && arguments[0] !== undefined ? arguments[0] : {},
      _ref2$x = _ref2.x,
      x = _ref2$x === void 0 ? 0 : _ref2$x,
      _ref2$y = _ref2.y,
      y = _ref2$y === void 0 ? 0 : _ref2$y;
    var _ref3 = arguments.length > 1 && arguments[1] !== undefined ? arguments[1] : [],
      _ref4 = (0, _slicedToArray2.default)(_ref3, 2),
      first = _ref4[0],
      second = _ref4[1];
    return x + y + (first || 0) + (second || 0);
  }
  console.log(point({
    x: 1
  }, [2, 3]), point());
  var m = 1;
  var n = 2;
  var _ref5 = [n, m];
  m = _ref5[0];
  n = _ref5[1];
  console.log(m, n);
  var pairs = [[1, "one"], [2, "two"]];
  for (var _i = 0, _pairs = pairs; _i < _pairs.length; _i++) {
    var _pairs$_i = (0, _slicedToArray2.default)(_pairs[_i], 2),
      num = _pairs$_i[0],
      word = _pairs$_i[1];
    console.log(num, word);
  }
  var _hello = "hello",
    length = _hello.length;
  var _hi = "hi",
    _hi2 = (0, _slicedToArray2.default)(_hi, 2),
    c1 = _hi2[0],
    c2 = _hi2[1];
  console.log(length, c1, c2);
  var data = {
    list: [{
      id: 7,
      tags: ["t1", "t2"]
    }]
  };
  var _data$list = (0, _slicedToArray2.default)(data.list, 1),
    _data$list$ = _data$list[0],
    id = _data$list$.id,
    _data$list$$tags = (0, _slicedToArray2.default)(_data$list$.tags, 2),
    second = _data$list$$tags[1];
  console.log(id, second);
  var key = "dyn";
  var _ref6 = {},
    _ref6$key = _ref6[key],
    dynamic = _ref6$key === void 0 ? "fallback" : _ref6$key;
  console.log(dynamic);
  function swap(_ref7) {
    var _ref8 = (0, _slicedToArray2.default)(_ref7, 2),
      p = _ref8[0],
      q = _ref8[1];
    return [q, p];
  }
  console.log(swap([1, 2]).join(","));
  var _ref9 = /*#__PURE__*/_regenerator.default.mark(function _callee() {
      return _regenerator.default.wrap(function (_context) {
        while (1) switch (_context.prev = _context.next) {
          case 0:
            _context.next = 1;
            return "a";
          case 1:
            _context.next = 2;
            return "b";
          case 2:
            _context.next = 3;
            return "c";
          case 3:
          case "end":
            return _context.stop();
        }
      }, _callee);
    })(),
    _ref0 = (0, _slicedToArray2.default)(_ref9, 2),
    g1 = _ref0[0],
    g2 = _ref0[1];
  console.log(g1, g2);

})();
