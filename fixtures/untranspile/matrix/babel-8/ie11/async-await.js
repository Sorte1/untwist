function _typeof(o) { "@babel/helpers - typeof"; return _typeof = "function" == typeof Symbol && "symbol" == typeof Symbol.iterator ? function (o) { return typeof o; } : function (o) { return o && "function" == typeof Symbol && o.constructor === Symbol && o !== Symbol.prototype ? "symbol" : typeof o; }, _typeof(o); }
function _classCallCheck(a, n) { if (!(a instanceof n)) throw new TypeError("Cannot call a class as a function"); }
function _defineProperties(r, t) { for (var o = 0; o < t.length; o++) { var i = t[o]; i.enumerable = i.enumerable || !1, i.configurable = !0, "value" in i && (i.writable = !0), Object.defineProperty(r, _toPropertyKey(i.key), i); } }
function _createClass(e, r, t) { return r && _defineProperties(e.prototype, r), t && _defineProperties(e, t), Object.defineProperty(e, "prototype", { writable: !1 }), e; }
function _toPropertyKey(r) { var o = _toPrimitive(r, "string"); return "symbol" == _typeof(o) ? o : o + ""; }
function _toPrimitive(t, e) { if ("object" != _typeof(t) || !t) return t; var r; if ("undefined" != typeof Symbol && void 0 !== (r = t[Symbol.toPrimitive])) { var i = r.call(t, e || "default"); if ("object" != _typeof(i)) return i; throw new TypeError("@@toPrimitive must return a primitive value."); } return ("string" === e ? String : Number)(t); }
function _regenerator() { /*! regenerator-runtime -- Copyright (c) 2014-present, Facebook, Inc. -- license (MIT): https://github.com/babel/babel/blob/main/packages/babel-helpers/LICENSE */ var r, e, o = "function" == typeof Symbol ? Symbol : {}, n = o.iterator || "@@iterator", a = o.toStringTag || "@@toStringTag"; function i(o, n, a, i) { var u = n && n.prototype instanceof Generator ? n : Generator, f = Object.create(u.prototype); return _regeneratorDefine2(f, "_invoke", function (t, o, n) { var a, i, u, f = 0, p = n || [], y = !1, G = { p: 0, n: 0, v: r, a: l, f: l.bind(r, 4), d: function d(t, e) { return a = t, i = 0, u = r, G.n = e, c; } }; function l(t, o) { for (i = t, u = o, e = 0; !y && f && !n && e < p.length; e++) { var n, a = p[e], l = G.p, F = a[2]; t > 3 ? (n = F === o) && (u = a[(i = a[4]) ? 5 : (i = 3, 3)], a[4] = a[5] = r) : a[0] <= l && ((n = t < 2 && l < a[1]) ? (i = 0, G.v = o, G.n = a[1]) : l < F && (n = t < 3 || a[0] > o || o > F) && (a[4] = t, a[5] = o, G.n = F, i = 0)); } if (n || t > 1) return c; throw y = !0, o; } return function (n, p, F) { if (f > 1) throw TypeError("Generator is already running"); for (y && 1 === p && l(p, F), i = p, u = F; (e = i < 2 ? r : u) || !y;) { a || (i ? (i > 1 && (G.n = i < 3 ? -1 : u), l(i, u)) : G.v = u); try { if (f = 2, a) { if (i || (n = "next"), e = a[n]) { if (!(e = e.call(a, u))) throw TypeError("iterator result is not an object"); if (!e.done) return e; u = e.value, i < 2 && (i = 0); } else 1 === i && (e = a.return) && e.call(a), i < 2 && (u = TypeError("The iterator does not provide a '" + n + "' method"), i = 1); a = r; } else if ((e = (y = G.n < 0) ? u : t.call(o, G)) !== c) break; } catch (t) { a = r, i = 1, u = t; } finally { f = 1; } } return { value: e, done: y }; }; }(o, a, i), !0), f; } var c = {}; function Generator() {} function GeneratorFunction() {} function GeneratorFunctionPrototype() {} e = Object.getPrototypeOf; var u = [][n] ? e(e([][n]())) : (_regeneratorDefine2(e = {}, n, function () { return this; }), e), f = GeneratorFunctionPrototype.prototype = Generator.prototype = Object.create(u); function p(r) { return Object.setPrototypeOf ? Object.setPrototypeOf(r, GeneratorFunctionPrototype) : (r.__proto__ = GeneratorFunctionPrototype, _regeneratorDefine2(r, a, "GeneratorFunction")), r.prototype = Object.create(f), r; } return GeneratorFunction.prototype = GeneratorFunctionPrototype, _regeneratorDefine2(f, "constructor", GeneratorFunctionPrototype), _regeneratorDefine2(GeneratorFunctionPrototype, "constructor", GeneratorFunction), GeneratorFunction.displayName = "GeneratorFunction", _regeneratorDefine2(GeneratorFunctionPrototype, a, "GeneratorFunction"), _regeneratorDefine2(f), _regeneratorDefine2(f, a, "Generator"), _regeneratorDefine2(f, n, function () { return this; }), _regeneratorDefine2(f, "toString", function () { return "[object Generator]"; }), (_regenerator = function _regenerator() { return { w: i, m: p }; })(); }
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
              var _add = _asyncToGenerator(/*#__PURE__*/_regenerator().m(function _callee(k) {
                return _regenerator().w(function (_context) {
                  while (1) switch (_context.n) {
                    case 0:
                      return _context.a(2, _this.base + k + _arguments.length);
                  }
                }, _callee);
              }));
              function add(_x2) {
                return _add.apply(this, arguments);
              }
              return add;
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
    var p, l, arrow, _t4, _t5, _t6, _t7, _t8, _t9, _t0, _t1, _t10, _t11;
    return _regenerator().w(function (_context7) {
      while (1) switch (_context7.n) {
        case 0:
          p = fetchTwice(1);
          log.push("after call");
          _t4 = console;
          _t5 = _t4.log;
          _context7.n = 1;
          return p;
        case 1:
          _t6 = _context7.v;
          _context7.n = 2;
          return guarded();
        case 2:
          _t7 = _context7.v;
          _context7.n = 3;
          return service.total(3);
        case 3:
          _t5.call(_t4, _t6, _t7, _context7.v);
          l = new Loader();
          _t8 = console;
          _t9 = _t8.log;
          _context7.n = 4;
          return l.load("a");
        case 4:
          _t0 = _context7.v;
          _context7.n = 5;
          return l.load("b");
        case 5:
          _t9.call(_t8, _t0, _context7.v);
          arrow = /*#__PURE__*/function () {
            var _arrow = _asyncToGenerator(/*#__PURE__*/_regenerator().m(function _callee6(n) {
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
            function arrow(_x4) {
              return _arrow.apply(this, arguments);
            }
            return arrow;
          }();
          _t1 = console;
          _t10 = _t1.log;
          _context7.n = 6;
          return arrow(4);
        case 6:
          _t11 = _context7.v;
          _context7.n = 7;
          return Promise.all([arrow(1), fetchTwice(2)]);
        case 7:
          _t10.call(_t1, _t11, _context7.v.join(","));
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
