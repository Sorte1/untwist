function _typeof(o) { "@babel/helpers - typeof"; return _typeof = "function" == typeof Symbol && "symbol" == typeof Symbol.iterator ? function (o) { return typeof o; } : function (o) { return o && "function" == typeof Symbol && o.constructor === Symbol && o !== Symbol.prototype ? "symbol" : typeof o; }, _typeof(o); }
function _regenerator() { /*! regenerator-runtime -- Copyright (c) 2014-present, Facebook, Inc. -- license (MIT): https://github.com/babel/babel/blob/main/packages/babel-helpers/LICENSE */ var e, t, r = "function" == typeof Symbol ? Symbol : {}, n = r.iterator || "@@iterator", o = r.toStringTag || "@@toStringTag"; function i(r, n, o, i) { var c = n && n.prototype instanceof Generator ? n : Generator, u = Object.create(c.prototype); return _regeneratorDefine2(u, "_invoke", function (r, n, o) { var i, c, u, f = 0, p = o || [], y = !1, G = { p: 0, n: 0, v: e, a: d, f: d.bind(e, 4), d: function d(t, r) { return i = t, c = 0, u = e, G.n = r, a; } }; function d(r, n) { for (c = r, u = n, t = 0; !y && f && !o && t < p.length; t++) { var o, i = p[t], d = G.p, l = i[2]; r > 3 ? (o = l === n) && (u = i[(c = i[4]) ? 5 : (c = 3, 3)], i[4] = i[5] = e) : i[0] <= d && ((o = r < 2 && d < i[1]) ? (c = 0, G.v = n, G.n = i[1]) : d < l && (o = r < 3 || i[0] > n || n > l) && (i[4] = r, i[5] = n, G.n = l, c = 0)); } if (o || r > 1) return a; throw y = !0, n; } return function (o, p, l) { if (f > 1) throw TypeError("Generator is already running"); for (y && 1 === p && d(p, l), c = p, u = l; (t = c < 2 ? e : u) || !y;) { i || (c ? c < 3 ? (c > 1 && (G.n = -1), d(c, u)) : G.n = u : G.v = u); try { if (f = 2, i) { if (c || (o = "next"), t = i[o]) { if (!(t = t.call(i, u))) throw TypeError("iterator result is not an object"); if (!t.done) return t; u = t.value, c < 2 && (c = 0); } else 1 === c && (t = i.return) && t.call(i), c < 2 && (u = TypeError("The iterator does not provide a '" + o + "' method"), c = 1); i = e; } else if ((t = (y = G.n < 0) ? u : r.call(n, G)) !== a) break; } catch (t) { i = e, c = 1, u = t; } finally { f = 1; } } return { value: t, done: y }; }; }(r, o, i), !0), u; } var a = {}; function Generator() {} function GeneratorFunction() {} function GeneratorFunctionPrototype() {} t = Object.getPrototypeOf; var c = [][n] ? t(t([][n]())) : (_regeneratorDefine2(t = {}, n, function () { return this; }), t), u = GeneratorFunctionPrototype.prototype = Generator.prototype = Object.create(c); function f(e) { return Object.setPrototypeOf ? Object.setPrototypeOf(e, GeneratorFunctionPrototype) : (e.__proto__ = GeneratorFunctionPrototype, _regeneratorDefine2(e, o, "GeneratorFunction")), e.prototype = Object.create(u), e; } return GeneratorFunction.prototype = GeneratorFunctionPrototype, _regeneratorDefine2(u, "constructor", GeneratorFunctionPrototype), _regeneratorDefine2(GeneratorFunctionPrototype, "constructor", GeneratorFunction), GeneratorFunction.displayName = "GeneratorFunction", _regeneratorDefine2(GeneratorFunctionPrototype, o, "GeneratorFunction"), _regeneratorDefine2(u), _regeneratorDefine2(u, o, "Generator"), _regeneratorDefine2(u, n, function () { return this; }), _regeneratorDefine2(u, "toString", function () { return "[object Generator]"; }), (_regenerator = function _regenerator() { return { w: i, m: f }; })(); }
function _regeneratorDefine2(e, r, n, t) { var i = Object.defineProperty; try { i({}, "", {}); } catch (e) { i = 0; } _regeneratorDefine2 = function _regeneratorDefine(e, r, n, t) { function o(r, n) { _regeneratorDefine2(e, r, function (e) { return this._invoke(r, n, e); }); } r ? i ? i(e, r, { value: n, enumerable: !t, configurable: !t, writable: !t }) : e[r] = n : (o("next", 0), o("throw", 1), o("return", 2)); }, _regeneratorDefine2(e, r, n, t); }
function asyncGeneratorStep(n, t, e, r, o, a, c) { try { var i = n[a](c), u = i.value; } catch (n) { return void e(n); } i.done ? t(u) : Promise.resolve(u).then(r, o); }
function _asyncToGenerator(n) { return function () { var t = this, e = arguments; return new Promise(function (r, o) { var a = n.apply(t, e); function _next(n) { asyncGeneratorStep(a, r, o, _next, _throw, "next", n); } function _throw(n) { asyncGeneratorStep(a, r, o, _next, _throw, "throw", n); } _next(void 0); }); }; }
var log = [];
var sleep = function sleep(v) {
  return new Promise(function (resolve) {
    return setTimeout(function () {
      return resolve(v);
    }, 0);
  });
};
function one(_x) {
  return _one.apply(this, arguments);
}
function _one() {
  _one = _asyncToGenerator(/*#__PURE__*/_regenerator().m(function _callee3(x) {
    var y;
    return _regenerator().w(function (_context3) {
      while (1) switch (_context3.n) {
        case 0:
          log.push("one-start " + x);
          _context3.n = 1;
          return x;
        case 1:
          y = _context3.v;
          log.push("one-after " + y);
          return _context3.a(2, y * 2);
      }
    }, _callee3);
  }));
  return _one.apply(this, arguments);
}
function throws() {
  return _throws.apply(this, arguments);
}
function _throws() {
  _throws = _asyncToGenerator(/*#__PURE__*/_regenerator().m(function _callee4() {
    return _regenerator().w(function (_context4) {
      while (1) switch (_context4.n) {
        case 0:
          _context4.n = 1;
          return null;
        case 1:
          throw new RangeError("r");
        case 2:
          return _context4.a(2);
      }
    }, _callee4);
  }));
  return _throws.apply(this, arguments);
}
function withFinally() {
  return _withFinally.apply(this, arguments);
}
function _withFinally() {
  _withFinally = _asyncToGenerator(/*#__PURE__*/_regenerator().m(function _callee5() {
    var _t;
    return _regenerator().w(function (_context5) {
      while (1) switch (_context5.p = _context5.n) {
        case 0:
          _context5.p = 0;
          _context5.n = 1;
          return throws();
        case 1:
          _context5.n = 3;
          break;
        case 2:
          _context5.p = 2;
          _t = _context5.v;
          log.push("caught " + _t.constructor.name);
          return _context5.a(2, "ret");
        case 3:
          _context5.p = 3;
          log.push("finally");
          return _context5.f(3);
        case 4:
          return _context5.a(2);
      }
    }, _callee5, null, [[0, 2, 3, 4]]);
  }));
  return _withFinally.apply(this, arguments);
}
var obj = {
  base: 10,
  m: function m(a) {
    var _arguments = arguments,
      _this = this;
    return _asyncToGenerator(/*#__PURE__*/_regenerator().m(function _callee2() {
      var f;
      return _regenerator().w(function (_context2) {
        while (1) switch (_context2.n) {
          case 0:
            f = /*#__PURE__*/function () {
              var _ref = _asyncToGenerator(/*#__PURE__*/_regenerator().m(function _callee() {
                return _regenerator().w(function (_context) {
                  while (1) switch (_context.n) {
                    case 0:
                      return _context.a(2, _this.base + a + _arguments.length);
                  }
                }, _callee);
              }));
              return function f() {
                return _ref.apply(this, arguments);
              };
            }();
            _context2.n = 1;
            return f();
          case 1:
            return _context2.a(2, _context2.v);
        }
      }, _callee2);
    }))();
  }
};
function loop() {
  return _loop.apply(this, arguments);
}
function _loop() {
  _loop = _asyncToGenerator(/*#__PURE__*/_regenerator().m(function _callee6() {
    var s, i, _t2;
    return _regenerator().w(function (_context6) {
      while (1) switch (_context6.n) {
        case 0:
          s = 0;
          i = 0;
        case 1:
          if (!(i < 3)) {
            _context6.n = 4;
            break;
          }
          _t2 = s;
          _context6.n = 2;
          return sleep(i);
        case 2:
          s = _t2 += _context6.v;
        case 3:
          i++;
          _context6.n = 1;
          break;
        case 4:
          return _context6.a(2, s);
      }
    }, _callee6);
  }));
  return _loop.apply(this, arguments);
}
function main() {
  return _main.apply(this, arguments);
}
function _main() {
  _main = _asyncToGenerator(/*#__PURE__*/_regenerator().m(function _callee8() {
    var p, arrow, _t4, _t5, _t6, _t7, _t8, _t9;
    return _regenerator().w(function (_context8) {
      while (1) switch (_context8.n) {
        case 0:
          p = one(1);
          log.push("sync after call");
          _t4 = console;
          _context8.n = 1;
          return p;
        case 1:
          _t5 = _context8.v;
          _context8.n = 2;
          return withFinally();
        case 2:
          _t6 = _context8.v;
          _context8.n = 3;
          return obj.m(5);
        case 3:
          _t7 = _context8.v;
          _context8.n = 4;
          return loop();
        case 4:
          _t4.log.call(_t4, _t5, _t6, _t7, _context8.v);
          console.log(one.name, _typeof(one));
          console.log(log.join("; "));
          arrow = /*#__PURE__*/function () {
            var _ref2 = _asyncToGenerator(/*#__PURE__*/_regenerator().m(function _callee7(n) {
              var _t3;
              return _regenerator().w(function (_context7) {
                while (1) switch (_context7.n) {
                  case 0:
                    _context7.n = 1;
                    return sleep(n);
                  case 1:
                    _t3 = _context7.v;
                    return _context7.a(2, _t3 + 1);
                }
              }, _callee7);
            }));
            return function arrow(_x2) {
              return _ref2.apply(this, arguments);
            };
          }();
          _t8 = console;
          _context8.n = 5;
          return arrow(1);
        case 5:
          _t9 = _context8.v;
          _context8.n = 6;
          return Promise.all([one(2), arrow(3)]).then(function (v) {
            return v.join(",");
          });
        case 6:
          _t8.log.call(_t8, _t9, _context8.v);
        case 7:
          return _context8.a(2);
      }
    }, _callee8);
  }));
  return _main.apply(this, arguments);
}
main().catch(function (e) {
  return console.log("fail", e.constructor.name);
});
