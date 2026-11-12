"use strict";

function _typeof(o) { "@babel/helpers - typeof"; return _typeof = "function" == typeof Symbol && "symbol" == typeof Symbol.iterator ? function (o) { return typeof o; } : function (o) { return o && "function" == typeof Symbol && o.constructor === Symbol && o !== Symbol.prototype ? "symbol" : typeof o; }, _typeof(o); }
function _classCallCheck(a, n) { if (!(a instanceof n)) throw new TypeError("Cannot call a class as a function"); }
function _defineProperties(e, r) { for (var t = 0; t < r.length; t++) { var o = r[t]; o.enumerable = o.enumerable || !1, o.configurable = !0, "value" in o && (o.writable = !0), Object.defineProperty(e, _toPropertyKey(o.key), o); } }
function _createClass(e, r, t) { return r && _defineProperties(e.prototype, r), t && _defineProperties(e, t), Object.defineProperty(e, "prototype", { writable: !1 }), e; }
function _toPropertyKey(t) { var i = _toPrimitive(t, "string"); return "symbol" == _typeof(i) ? i : i + ""; }
function _toPrimitive(t, e) { if ("object" != _typeof(t) || !t) return t; var r; if ("undefined" != typeof Symbol && void 0 !== (r = t[Symbol.toPrimitive])) { var i = r.call(t, e || "default"); if ("object" != _typeof(i)) return i; throw new TypeError("@@toPrimitive must return a primitive value."); } return ("string" === e ? String : Number)(t); }
function _regenerator() { /*! regenerator-runtime -- Copyright (c) 2014-present, Facebook, Inc. -- license (MIT): https://github.com/babel/babel/blob/main/packages/babel-helpers/LICENSE */ var e, t, r = "function" == typeof Symbol ? Symbol : {}, n = r.iterator || "@@iterator", o = r.toStringTag || "@@toStringTag"; function i(r, n, o, i) { var c = n && n.prototype instanceof Generator ? n : Generator, u = Object.create(c.prototype); return _regeneratorDefine2(u, "_invoke", function (r, n, o) { var i, c, u, f = 0, p = o || [], y = !1, G = { p: 0, n: 0, v: e, a: d, f: d.bind(e, 4), d: function d(t, r) { return i = t, c = 0, u = e, G.n = r, a; } }; function d(r, n) { for (c = r, u = n, t = 0; !y && f && !o && t < p.length; t++) { var o, i = p[t], d = G.p, l = i[2]; r > 3 ? (o = l === n) && (u = i[(c = i[4]) ? 5 : (c = 3, 3)], i[4] = i[5] = e) : i[0] <= d && ((o = r < 2 && d < i[1]) ? (c = 0, G.v = n, G.n = i[1]) : d < l && (o = r < 3 || i[0] > n || n > l) && (i[4] = r, i[5] = n, G.n = l, c = 0)); } if (o || r > 1) return a; throw y = !0, n; } return function (o, p, l) { if (f > 1) throw TypeError("Generator is already running"); for (y && 1 === p && d(p, l), c = p, u = l; (t = c < 2 ? e : u) || !y;) { i || (c ? (c > 1 && (G.n = c < 3 ? -1 : u), d(c, u)) : G.v = u); try { if (f = 2, i) { if (c || (o = "next"), t = i[o]) { if (!(t = t.call(i, u))) throw TypeError("iterator result is not an object"); if (!t.done) return t; u = t.value, c < 2 && (c = 0); } else 1 === c && (t = i.return) && t.call(i), c < 2 && (u = TypeError("The iterator does not provide a '" + o + "' method"), c = 1); i = e; } else if ((t = (y = G.n < 0) ? u : r.call(n, G)) !== a) break; } catch (t) { i = e, c = 1, u = t; } finally { f = 1; } } return { value: t, done: y }; }; }(r, o, i), !0), u; } var a = {}; function Generator() {} function GeneratorFunction() {} function GeneratorFunctionPrototype() {} t = Object.getPrototypeOf; var c = [][n] ? t(t([][n]())) : (_regeneratorDefine2(t = {}, n, function () { return this; }), t), u = GeneratorFunctionPrototype.prototype = Generator.prototype = Object.create(c); function f(e) { return Object.setPrototypeOf ? Object.setPrototypeOf(e, GeneratorFunctionPrototype) : (e.__proto__ = GeneratorFunctionPrototype, _regeneratorDefine2(e, o, "GeneratorFunction")), e.prototype = Object.create(u), e; } return GeneratorFunction.prototype = GeneratorFunctionPrototype, _regeneratorDefine2(u, "constructor", GeneratorFunctionPrototype), _regeneratorDefine2(GeneratorFunctionPrototype, "constructor", GeneratorFunction), GeneratorFunction.displayName = "GeneratorFunction", _regeneratorDefine2(GeneratorFunctionPrototype, o, "GeneratorFunction"), _regeneratorDefine2(u), _regeneratorDefine2(u, o, "Generator"), _regeneratorDefine2(u, n, function () { return this; }), _regeneratorDefine2(u, "toString", function () { return "[object Generator]"; }), (_regenerator = function _regenerator() { return { w: i, m: f }; })(); }
function _regeneratorDefine2(e, r, n, t) { var i = Object.defineProperty; try { i({}, "", {}); } catch (e) { i = 0; } _regeneratorDefine2 = function _regeneratorDefine(e, r, n, t) { function o(r, n) { _regeneratorDefine2(e, r, function (e) { return this._invoke(r, n, e); }); } r ? i ? i(e, r, { value: n, enumerable: !t, configurable: !t, writable: !t }) : e[r] = n : (o("next", 0), o("throw", 1), o("return", 2)); }, _regeneratorDefine2(e, r, n, t); }
function asyncGeneratorStep(n, t, e, r, o, a, c) { try { var i = n[a](c), u = i.value; } catch (n) { return void e(n); } i.done ? t(u) : Promise.resolve(u).then(r, o); }
function _asyncToGenerator(n) { return function () { var t = this, e = arguments; return new Promise(function (r, o) { var a = n.apply(t, e); function _next(n) { asyncGeneratorStep(a, r, o, _next, _throw, "next", n); } function _throw(n) { asyncGeneratorStep(a, r, o, _next, _throw, "throw", n); } _next(void 0); }); }; }
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
  _fetchTwice = _asyncToGenerator(/*#__PURE__*/_regenerator().m(function _callee4(x) {
    var a, b;
    return _regenerator().w(function (_context4) {
      while (1) switch (_context4.n) {
        case 0:
          log.push("start " + x);
          _context4.n = 1;
          return later(x);
        case 1:
          a = _context4.v;
          _context4.n = 2;
          return later(a + 1);
        case 2:
          b = _context4.v;
          return _context4.a(2, a + b);
      }
    }, _callee4);
  }));
  return _fetchTwice.apply(this, arguments);
}
function guarded() {
  return _guarded.apply(this, arguments);
}
function _guarded() {
  _guarded = _asyncToGenerator(/*#__PURE__*/_regenerator().m(function _callee5() {
    var _t2;
    return _regenerator().w(function (_context5) {
      while (1) switch (_context5.p = _context5.n) {
        case 0:
          _context5.p = 0;
          _context5.n = 1;
          return later(0);
        case 1:
          throw new TypeError("t");
        case 2:
          _context5.p = 2;
          _t2 = _context5.v;
          log.push("caught " + _t2.name);
          return _context5.a(2, "r");
        case 3:
          _context5.p = 3;
          log.push("finally");
          return _context5.f(3);
        case 4:
          return _context5.a(2);
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
    return _asyncToGenerator(/*#__PURE__*/_regenerator().m(function _callee2() {
      var add, sum, i, _t;
      return _regenerator().w(function (_context2) {
        while (1) switch (_context2.n) {
          case 0:
            add = /*#__PURE__*/function () {
              var _ref = _asyncToGenerator(/*#__PURE__*/_regenerator().m(function _callee(k) {
                return _regenerator().w(function (_context) {
                  while (1) switch (_context.n) {
                    case 0:
                      return _context.a(2, _this.base + k + _arguments.length);
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
              _context2.n = 4;
              break;
            }
            _t = sum;
            _context2.n = 2;
            return add(i);
          case 2:
            sum = _t += _context2.v;
          case 3:
            i++;
            _context2.n = 1;
            break;
          case 4:
            return _context2.a(2, sum);
        }
      }, _callee2);
    }))();
  }
};
var Loader = /*#__PURE__*/function () {
  function Loader() {
    _classCallCheck(this, Loader);
    this.items = [];
  }
  return _createClass(Loader, [{
    key: "load",
    value: function () {
      var _load = _asyncToGenerator(/*#__PURE__*/_regenerator().m(function _callee3(v) {
        return _regenerator().w(function (_context3) {
          while (1) switch (_context3.n) {
            case 0:
              _context3.n = 1;
              return null;
            case 1:
              this.items.push(v);
              return _context3.a(2, this.items.length);
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
  _main = _asyncToGenerator(/*#__PURE__*/_regenerator().m(function _callee7() {
    var p, l, arrow, _t4, _t5, _t6, _t7, _t8, _t9, _t0;
    return _regenerator().w(function (_context7) {
      while (1) switch (_context7.n) {
        case 0:
          p = fetchTwice(1);
          log.push("after call");
          _t4 = console;
          _context7.n = 1;
          return p;
        case 1:
          _t5 = _context7.v;
          _context7.n = 2;
          return guarded();
        case 2:
          _t6 = _context7.v;
          _context7.n = 3;
          return service.total(3);
        case 3:
          _t4.log.call(_t4, _t5, _t6, _context7.v);
          l = new Loader();
          _t7 = console;
          _context7.n = 4;
          return l.load("a");
        case 4:
          _t8 = _context7.v;
          _context7.n = 5;
          return l.load("b");
        case 5:
          _t7.log.call(_t7, _t8, _context7.v);
          arrow = /*#__PURE__*/function () {
            var _ref2 = _asyncToGenerator(/*#__PURE__*/_regenerator().m(function _callee6(n) {
              var _t3;
              return _regenerator().w(function (_context6) {
                while (1) switch (_context6.n) {
                  case 0:
                    _context6.n = 1;
                    return later(n);
                  case 1:
                    _t3 = _context6.v;
                    return _context6.a(2, _t3 * 2);
                }
              }, _callee6);
            }));
            return function arrow(_x4) {
              return _ref2.apply(this, arguments);
            };
          }();
          _t9 = console;
          _context7.n = 6;
          return arrow(4);
        case 6:
          _t0 = _context7.v;
          _context7.n = 7;
          return Promise.all([arrow(1), fetchTwice(2)]);
        case 7:
          _t9.log.call(_t9, _t0, _context7.v.join(","));
          console.log(log.join("; "));
        case 8:
          return _context7.a(2);
      }
    }, _callee7);
  }));
  return _main.apply(this, arguments);
}
main().catch(function (e) {
  return console.log("fail", e);
});
