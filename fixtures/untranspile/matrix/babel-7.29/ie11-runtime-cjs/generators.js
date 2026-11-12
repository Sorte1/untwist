(function () {
  'use strict';

  var _regeneratorRuntime2 = require("@babel/runtime/regenerator");
  var _interopRequireDefault = require("@babel/runtime/helpers/interopRequireDefault");
  var _defineProperty2 = _interopRequireDefault(require("@babel/runtime/helpers/defineProperty"));
  var _toConsumableArray2 = _interopRequireDefault(require("@babel/runtime/helpers/toConsumableArray"));
  var _regenerator = _interopRequireDefault(require("@babel/runtime/regenerator"));
  function _createForOfIteratorHelper(r, e) { var t = "undefined" != typeof Symbol && r[Symbol.iterator] || r["@@iterator"]; if (!t) { if (Array.isArray(r) || (t = _unsupportedIterableToArray(r)) || e) { t && (r = t); var _n = 0, F = function F() {}; return { s: F, n: function n() { return _n >= r.length ? { done: true } : { done: false, value: r[_n++] }; }, e: function e(r) { throw r; }, f: F }; } throw new TypeError("Invalid attempt to iterate non-iterable instance.\nIn order to be iterable, non-array objects must have a [Symbol.iterator]() method."); } var o, a = true, u = false; return { s: function s() { t = t.call(r); }, n: function n() { var r = t.next(); return a = r.done, r; }, e: function e(r) { u = true, o = r; }, f: function f() { try { a || null == t.return || t.return(); } finally { if (u) throw o; } } }; }
  function _unsupportedIterableToArray(r, a) { if (r) { if ("string" == typeof r) return _arrayLikeToArray(r, a); var t = {}.toString.call(r).slice(8, -1); return "Object" === t && r.constructor && (t = r.constructor.name), "Map" === t || "Set" === t ? Array.from(r) : "Arguments" === t || /^(?:Ui|I)nt(?:8|16|32)(?:Clamped)?Array$/.test(t) ? _arrayLikeToArray(r, a) : void 0; } }
  function _arrayLikeToArray(r, a) { (null == a || a > r.length) && (a = r.length); for (var e = 0, n = Array(a); e < a; e++) n[e] = r[e]; return n; }
  var _marked = /*#__PURE__*/_regeneratorRuntime2.mark(inner),
    _marked2 = /*#__PURE__*/_regeneratorRuntime2.mark(outer),
    _marked3 = /*#__PURE__*/_regeneratorRuntime2.mark(thrower),
    _marked4 = /*#__PURE__*/_regeneratorRuntime2.mark(fib);
  function range(start, end) {
    var step = arguments.length > 2 && arguments[2] !== undefined ? arguments[2] : 1;
    return /*#__PURE__*/_regenerator.default.mark(function _callee() {
      var i;
      return _regenerator.default.wrap(function (_context) {
        while (1) switch (_context.prev = _context.next) {
          case 0:
            i = start;
          case 1:
            if (!(i < end)) {
              _context.next = 3;
              break;
            }
            _context.next = 2;
            return i;
          case 2:
            i += step;
            _context.next = 1;
            break;
          case 3:
          case "end":
            return _context.stop();
        }
      }, _callee);
    })();
  }
  function inner() {
    var got;
    return _regenerator.default.wrap(function (_context2) {
      while (1) switch (_context2.prev = _context2.next) {
        case 0:
          _context2.next = 1;
          return "first";
        case 1:
          got = _context2.sent;
          _context2.next = 2;
          return "got " + got;
        case 2:
          return _context2.abrupt("return", "inner-done");
        case 3:
        case "end":
          return _context2.stop();
      }
    }, _marked);
  }
  function outer() {
    var r;
    return _regenerator.default.wrap(function (_context3) {
      while (1) switch (_context3.prev = _context3.next) {
        case 0:
          return _context3.delegateYield(inner(), "t0", 1);
        case 1:
          r = _context3.t0;
          _context3.next = 2;
          return r;
        case 2:
          _context3.prev = 2;
          _context3.next = 3;
          return "try";
        case 3:
          _context3.prev = 3;
          _context3.next = 4;
          return "cleanup";
        case 4:
          return _context3.finish(3);
        case 5:
        case "end":
          return _context3.stop();
      }
    }, _marked2, null, [[2,, 3, 5]]);
  }
  console.log((0, _toConsumableArray2.default)(range(0, 5)).join(","), Array.from(range(1, 10, 3)).join(","));
  var it = outer();
  console.log(JSON.stringify([it.next(), it.next("x"), it.next(), it.next(), it.return(7), it.next(), it.next()]));
  var obj = (0, _defineProperty2.default)({}, Symbol.iterator, /*#__PURE__*/_regenerator.default.mark(function _callee2() {
    return _regenerator.default.wrap(function (_context4) {
      while (1) switch (_context4.prev = _context4.next) {
        case 0:
          _context4.next = 1;
          return 1;
        case 1:
          _context4.next = 2;
          return 2;
        case 2:
        case "end":
          return _context4.stop();
      }
    }, _callee2);
  }));
  console.log((0, _toConsumableArray2.default)(obj).join(","));
  function thrower() {
    var _t;
    return _regenerator.default.wrap(function (_context5) {
      while (1) switch (_context5.prev = _context5.next) {
        case 0:
          _context5.prev = 0;
          _context5.next = 1;
          return 1;
        case 1:
          _context5.next = 3;
          break;
        case 2:
          _context5.prev = 2;
          _t = _context5["catch"](0);
          _context5.next = 3;
          return "caught " + _t;
        case 3:
        case "end":
          return _context5.stop();
      }
    }, _marked3, null, [[0, 2]]);
  }
  var t = thrower();
  t.next();
  console.log(JSON.stringify(t.throw("boom")), JSON.stringify(t.next()));
  function fib() {
    var a, b, _ref;
    return _regenerator.default.wrap(function (_context6) {
      while (1) switch (_context6.prev = _context6.next) {
        case 0:
          a = 0, b = 1;
        case 1:
          _context6.next = 2;
          return a;
        case 2:
          _ref = [b, a + b];
          a = _ref[0];
          b = _ref[1];
          _context6.next = 1;
          break;
        case 3:
        case "end":
          return _context6.stop();
      }
    }, _marked4);
  }
  var f = fib();
  var first = [];
  var _iterator = _createForOfIteratorHelper(f),
    _step;
  try {
    for (_iterator.s(); !(_step = _iterator.n()).done;) {
      var v = _step.value;
      if (v > 20) break;
      first.push(v);
    }
  } catch (err) {
    _iterator.e(err);
  } finally {
    _iterator.f();
  }
  console.log(first.join(","));

})();
