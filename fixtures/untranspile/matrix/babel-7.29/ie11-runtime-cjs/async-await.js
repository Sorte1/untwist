(function () {
  'use strict';

  var _interopRequireDefault = require("@babel/runtime/helpers/interopRequireDefault");
  var _regenerator = _interopRequireDefault(require("@babel/runtime/regenerator"));
  var _classCallCheck2 = _interopRequireDefault(require("@babel/runtime/helpers/classCallCheck"));
  var _createClass2 = _interopRequireDefault(require("@babel/runtime/helpers/createClass"));
  var _asyncToGenerator2 = _interopRequireDefault(require("@babel/runtime/helpers/asyncToGenerator"));
  var log = [];
  var later = function later(v) {
    return new Promise(function (resolve) {
      return setTimeout(function () {
        return resolve(v);
      }, 0);
    });
  };
  function fetchTwice(_x) {
    return _fetchTwice.apply(this, arguments);
  }
  function _fetchTwice() {
    _fetchTwice = (0, _asyncToGenerator2.default)(/*#__PURE__*/_regenerator.default.mark(function _callee4(x) {
      var a, b;
      return _regenerator.default.wrap(function (_context4) {
        while (1) switch (_context4.prev = _context4.next) {
          case 0:
            log.push("start " + x);
            _context4.next = 1;
            return later(x);
          case 1:
            a = _context4.sent;
            _context4.next = 2;
            return later(a + 1);
          case 2:
            b = _context4.sent;
            return _context4.abrupt("return", a + b);
          case 3:
          case "end":
            return _context4.stop();
        }
      }, _callee4);
    }));
    return _fetchTwice.apply(this, arguments);
  }
  function guarded() {
    return _guarded.apply(this, arguments);
  }
  function _guarded() {
    _guarded = (0, _asyncToGenerator2.default)(/*#__PURE__*/_regenerator.default.mark(function _callee5() {
      var _t2;
      return _regenerator.default.wrap(function (_context5) {
        while (1) switch (_context5.prev = _context5.next) {
          case 0:
            _context5.prev = 0;
            _context5.next = 1;
            return later(0);
          case 1:
            throw new TypeError("t");
          case 2:
            _context5.prev = 2;
            _t2 = _context5["catch"](0);
            log.push("caught " + _t2.name);
            return _context5.abrupt("return", "r");
          case 3:
            _context5.prev = 3;
            log.push("finally");
            return _context5.finish(3);
          case 4:
          case "end":
            return _context5.stop();
        }
      }, _callee5, null, [[0, 2, 3, 4]]);
    }));
    return _guarded.apply(this, arguments);
  }
  var service = {
    base: 100,
    total: function total(n) {
      var _arguments = arguments,
        _this = this;
      return (0, _asyncToGenerator2.default)(/*#__PURE__*/_regenerator.default.mark(function _callee2() {
        var add, sum, i, _t;
        return _regenerator.default.wrap(function (_context2) {
          while (1) switch (_context2.prev = _context2.next) {
            case 0:
              add = /*#__PURE__*/function () {
                var _ref = (0, _asyncToGenerator2.default)(/*#__PURE__*/_regenerator.default.mark(function _callee(k) {
                  return _regenerator.default.wrap(function (_context) {
                    while (1) switch (_context.prev = _context.next) {
                      case 0:
                        return _context.abrupt("return", _this.base + k + _arguments.length);
                      case 1:
                      case "end":
                        return _context.stop();
                    }
                  }, _callee);
                }));
                return function add(_x2) {
                  return _ref.apply(this, arguments);
                };
              }();
              sum = 0;
              i = 0;
            case 1:
              if (!(i < n)) {
                _context2.next = 4;
                break;
              }
              _t = sum;
              _context2.next = 2;
              return add(i);
            case 2:
              sum = _t += _context2.sent;
            case 3:
              i++;
              _context2.next = 1;
              break;
            case 4:
              return _context2.abrupt("return", sum);
            case 5:
            case "end":
              return _context2.stop();
          }
        }, _callee2);
      }))();
    }
  };
  var Loader = /*#__PURE__*/function () {
    function Loader() {
      (0, _classCallCheck2.default)(this, Loader);
      this.items = [];
    }
    return (0, _createClass2.default)(Loader, [{
      key: "load",
      value: function () {
        var _load = (0, _asyncToGenerator2.default)(/*#__PURE__*/_regenerator.default.mark(function _callee3(v) {
          return _regenerator.default.wrap(function (_context3) {
            while (1) switch (_context3.prev = _context3.next) {
              case 0:
                _context3.next = 1;
                return null;
              case 1:
                this.items.push(v);
                return _context3.abrupt("return", this.items.length);
              case 2:
              case "end":
                return _context3.stop();
            }
          }, _callee3, this);
        }));
        function load(_x3) {
          return _load.apply(this, arguments);
        }
        return load;
      }()
    }]);
  }();
  function main() {
    return _main.apply(this, arguments);
  }
  function _main() {
    _main = (0, _asyncToGenerator2.default)(/*#__PURE__*/_regenerator.default.mark(function _callee7() {
      var p, l, arrow, _t4, _t5, _t6, _t7, _t8, _t9, _t0;
      return _regenerator.default.wrap(function (_context7) {
        while (1) switch (_context7.prev = _context7.next) {
          case 0:
            p = fetchTwice(1);
            log.push("after call");
            _t4 = console;
            _context7.next = 1;
            return p;
          case 1:
            _t5 = _context7.sent;
            _context7.next = 2;
            return guarded();
          case 2:
            _t6 = _context7.sent;
            _context7.next = 3;
            return service.total(3);
          case 3:
            _t4.log.call(_t4, _t5, _t6, _context7.sent);
            l = new Loader();
            _t7 = console;
            _context7.next = 4;
            return l.load("a");
          case 4:
            _t8 = _context7.sent;
            _context7.next = 5;
            return l.load("b");
          case 5:
            _t7.log.call(_t7, _t8, _context7.sent);
            arrow = /*#__PURE__*/function () {
              var _ref2 = (0, _asyncToGenerator2.default)(/*#__PURE__*/_regenerator.default.mark(function _callee6(n) {
                var _t3;
                return _regenerator.default.wrap(function (_context6) {
                  while (1) switch (_context6.prev = _context6.next) {
                    case 0:
                      _context6.next = 1;
                      return later(n);
                    case 1:
                      _t3 = _context6.sent;
                      return _context6.abrupt("return", _t3 * 2);
                    case 2:
                    case "end":
                      return _context6.stop();
                  }
                }, _callee6);
              }));
              return function arrow(_x4) {
                return _ref2.apply(this, arguments);
              };
            }();
            _t9 = console;
            _context7.next = 6;
            return arrow(4);
          case 6:
            _t0 = _context7.sent;
            _context7.next = 7;
            return Promise.all([arrow(1), fetchTwice(2)]);
          case 7:
            _t9.log.call(_t9, _t0, _context7.sent.join(","));
            console.log(log.join("; "));
          case 8:
          case "end":
            return _context7.stop();
        }
      }, _callee7);
    }));
    return _main.apply(this, arguments);
  }
  main().catch(function (e) {
    return console.log("fail", e);
  });

})();
