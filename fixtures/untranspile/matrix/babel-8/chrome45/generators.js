function _typeof(o) { "@babel/helpers - typeof"; return _typeof = "function" == typeof Symbol && "symbol" == typeof Symbol.iterator ? function (o) { return typeof o; } : function (o) { return o && "function" == typeof Symbol && o.constructor === Symbol && o !== Symbol.prototype ? "symbol" : typeof o; }, _typeof(o); }
function _createForOfIteratorHelper(e, t) { var _n = "undefined" != typeof Symbol && e[Symbol.iterator] || e["@@iterator"]; if (!_n) { if (Array.isArray(e) || (_n = _unsupportedIterableToArray(e)) || t && e && "number" == typeof e.length) { _n && (e = _n); var o = 0, F = function F() {}; return { s: F, n: function n() { return o >= e.length ? { done: !0 } : { done: !1, value: e[o++] }; }, e: function e(r) { throw r; }, f: F }; } throw new TypeError("Invalid attempt to iterate non-iterable instance.\nIn order to be iterable, non-array objects must have a [Symbol.iterator]() method."); } var a, u, i = !0; return { s: function s() { _n = _n.call(e); }, n: function n() { i = !0; var r = _n.next(); return r.done ? { done: !0 } : { value: r.value, done: i = !1 }; }, e: function e(r) { a = !0, u = r; }, f: function f() { try { i || null == _n.return || _n.return(); } finally { if (a) throw u; } } }; }
function _toConsumableArray(a) { return _arrayWithoutHoles(a) || _iterableToArray(a) || _unsupportedIterableToArray(a) || _nonIterableSpread(); }
function _nonIterableSpread() { throw new TypeError("Invalid attempt to spread non-iterable instance.\nIn order to be iterable, non-array objects must have a [Symbol.iterator]() method."); }
function _unsupportedIterableToArray(t, e) { if (t) { if ("string" == typeof t) return _arrayLikeToArray(t, e); var o = {}.toString.call(t).slice(8, -1); return "Object" === o && t.constructor && (o = t.constructor.name), "Map" === o || "Set" === o ? Array.from(t) : "Arguments" === o || /^(?:Ui|I)nt(?:8|16|32)(?:Clamped)?Array$/.test(o) ? _arrayLikeToArray(t, e) : void 0; } }
function _iterableToArray(r) { var e = "undefined" != typeof Symbol && r[Symbol.iterator] || r["@@iterator"]; if (Array.isArray(r) && (null == e || e === ("undefined" != typeof Symbol && [][Symbol.iterator]) || e === []["@@iterator"])) return r; if (null != e) { for (var t, o = [], a = e.call(r); !(t = a.next()).done;) o.push(t.value); return o; } }
function _arrayWithoutHoles(a) { if (Array.isArray(a)) return _arrayLikeToArray(a); }
function _arrayLikeToArray(r, a) { (null == a || a > r.length) && (a = r.length); for (var e = 0, n = Array(a); e < a; e++) n[e] = r[e]; return n; }
function _regeneratorValues(e) { if (null != e) { var t = e["function" == typeof Symbol && Symbol.iterator || "@@iterator"], r = 0; if (t) return t.call(e); if ("function" == typeof e.next) return e; if (!isNaN(e.length)) return { next: function next() { return e && r >= e.length && (e = void 0), { value: e && e[r++], done: !e }; } }; } throw new TypeError(_typeof(e) + " is not iterable"); }
var _marked = /*#__PURE__*/_regenerator().m(inner),
  _marked2 = /*#__PURE__*/_regenerator().m(outer),
  _marked3 = /*#__PURE__*/_regenerator().m(thrower),
  _marked4 = /*#__PURE__*/_regenerator().m(fib);
function _regenerator() { /*! regenerator-runtime -- Copyright (c) 2014-present, Facebook, Inc. -- license (MIT): https://github.com/babel/babel/blob/main/packages/babel-helpers/LICENSE */ var r, e, o = "function" == typeof Symbol ? Symbol : {}, n = o.iterator || "@@iterator", a = o.toStringTag || "@@toStringTag"; function i(o, n, a, i) { var u = n && n.prototype instanceof Generator ? n : Generator, f = Object.create(u.prototype); return _regeneratorDefine2(f, "_invoke", function (t, o, n) { var a, i, u, f = 0, p = n || [], y = !1, G = { p: 0, n: 0, v: r, a: l, f: l.bind(r, 4), d: function d(t, e) { return a = t, i = 0, u = r, G.n = e, c; } }; function l(t, o) { for (i = t, u = o, e = 0; !y && f && !n && e < p.length; e++) { var n, a = p[e], l = G.p, F = a[2]; t > 3 ? (n = F === o) && (u = a[(i = a[4]) ? 5 : (i = 3, 3)], a[4] = a[5] = r) : a[0] <= l && ((n = t < 2 && l < a[1]) ? (i = 0, G.v = o, G.n = a[1]) : l < F && (n = t < 3 || a[0] > o || o > F) && (a[4] = t, a[5] = o, G.n = F, i = 0)); } if (n || t > 1) return c; throw y = !0, o; } return function (n, p, F) { if (f > 1) throw TypeError("Generator is already running"); for (y && 1 === p && l(p, F), i = p, u = F; (e = i < 2 ? r : u) || !y;) { a || (i ? (i > 1 && (G.n = i < 3 ? -1 : u), l(i, u)) : G.v = u); try { if (f = 2, a) { if (i || (n = "next"), e = a[n]) { if (!(e = e.call(a, u))) throw TypeError("iterator result is not an object"); if (!e.done) return e; u = e.value, i < 2 && (i = 0); } else 1 === i && (e = a.return) && e.call(a), i < 2 && (u = TypeError("The iterator does not provide a '" + n + "' method"), i = 1); a = r; } else if ((e = (y = G.n < 0) ? u : t.call(o, G)) !== c) break; } catch (t) { a = r, i = 1, u = t; } finally { f = 1; } } return { value: e, done: y }; }; }(o, a, i), !0), f; } var c = {}; function Generator() {} function GeneratorFunction() {} function GeneratorFunctionPrototype() {} e = Object.getPrototypeOf; var u = [][n] ? e(e([][n]())) : (_regeneratorDefine2(e = {}, n, function () { return this; }), e), f = GeneratorFunctionPrototype.prototype = Generator.prototype = Object.create(u); function p(r) { return Object.setPrototypeOf ? Object.setPrototypeOf(r, GeneratorFunctionPrototype) : (r.__proto__ = GeneratorFunctionPrototype, _regeneratorDefine2(r, a, "GeneratorFunction")), r.prototype = Object.create(f), r; } return GeneratorFunction.prototype = GeneratorFunctionPrototype, _regeneratorDefine2(f, "constructor", GeneratorFunctionPrototype), _regeneratorDefine2(GeneratorFunctionPrototype, "constructor", GeneratorFunction), GeneratorFunction.displayName = "GeneratorFunction", _regeneratorDefine2(GeneratorFunctionPrototype, a, "GeneratorFunction"), _regeneratorDefine2(f), _regeneratorDefine2(f, a, "Generator"), _regeneratorDefine2(f, n, function () { return this; }), _regeneratorDefine2(f, "toString", function () { return "[object Generator]"; }), (_regenerator = function _regenerator() { return { w: i, m: p }; })(); }
function _regeneratorDefine2(e, r, n, t) { var i = Object.defineProperty; try { i({}, "", {}); } catch (e) { i = 0; } _regeneratorDefine2 = function _regeneratorDefine(e, r, n, t) { function o(r, n) { _regeneratorDefine2(e, r, function (e) { return this._invoke(r, n, e); }); } r ? i ? i(e, r, { value: n, enumerable: !t, configurable: !t, writable: !t }) : e[r] = n : (o("next", 0), o("throw", 1), o("return", 2)); }, _regeneratorDefine2(e, r, n, t); }
function range(start, end) {
  var step = arguments.length > 2 && arguments[2] !== undefined ? arguments[2] : 1;
  return /*#__PURE__*/_regenerator().m(function _callee() {
    var i;
    return _regenerator().w(function (_context) {
      while (1) switch (_context.n) {
        case 0:
          i = start;
        case 1:
          if (!(i < end)) {
            _context.n = 3;
            break;
          }
          _context.n = 2;
          return i;
        case 2:
          i += step;
          _context.n = 1;
          break;
        case 3:
          return _context.a(2);
      }
    }, _callee);
  })();
}
function inner() {
  var got;
  return _regenerator().w(function (_context2) {
    while (1) switch (_context2.n) {
      case 0:
        _context2.n = 1;
        return "first";
      case 1:
        got = _context2.v;
        _context2.n = 2;
        return "got " + got;
      case 2:
        return _context2.a(2, "inner-done");
    }
  }, _marked);
}
function outer() {
  var r;
  return _regenerator().w(function (_context3) {
    while (1) switch (_context3.p = _context3.n) {
      case 0:
        return _context3.d(_regeneratorValues(inner()), 1);
      case 1:
        r = _context3.v;
        _context3.n = 2;
        return r;
      case 2:
        _context3.p = 2;
        _context3.n = 3;
        return "try";
      case 3:
        _context3.p = 3;
        _context3.n = 4;
        return "cleanup";
      case 4:
        return _context3.f(3);
      case 5:
        return _context3.a(2);
    }
  }, _marked2, null, [[2,, 3, 5]]);
}
console.log(_toConsumableArray(range(0, 5)).join(","), Array.from(range(1, 10, 3)).join(","));
var it = outer();
console.log(JSON.stringify([it.next(), it.next("x"), it.next(), it.next(), it.return(7), it.next(), it.next()]));
var obj = {
  [Symbol.iterator]() {
    return /*#__PURE__*/_regenerator().m(function _callee2() {
      return _regenerator().w(function (_context4) {
        while (1) switch (_context4.n) {
          case 0:
            _context4.n = 1;
            return 1;
          case 1:
            _context4.n = 2;
            return 2;
          case 2:
            return _context4.a(2);
        }
      }, _callee2);
    })();
  }
};
console.log(_toConsumableArray(obj).join(","));
function thrower() {
  var _t;
  return _regenerator().w(function (_context5) {
    while (1) switch (_context5.p = _context5.n) {
      case 0:
        _context5.p = 0;
        _context5.n = 1;
        return 1;
      case 1:
        _context5.n = 3;
        break;
      case 2:
        _context5.p = 2;
        _t = _context5.v;
        _context5.n = 3;
        return "caught " + _t;
      case 3:
        return _context5.a(2);
    }
  }, _marked3, null, [[0, 2]]);
}
var t = thrower();
t.next();
console.log(JSON.stringify(t.throw("boom")), JSON.stringify(t.next()));
function fib() {
  var a, b, _ref;
  return _regenerator().w(function (_context6) {
    while (1) switch (_context6.n) {
      case 0:
        a = 0, b = 1;
      case 1:
        if (!true) {
          _context6.n = 3;
          break;
        }
        _context6.n = 2;
        return a;
      case 2:
        _ref = [b, a + b];
        a = _ref[0];
        b = _ref[1];
        _context6.n = 1;
        break;
      case 3:
        return _context6.a(2);
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
