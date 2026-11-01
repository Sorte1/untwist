function _createForOfIteratorHelper(r, e) { var t = "undefined" != typeof Symbol && r[Symbol.iterator] || r["@@iterator"]; if (!t) { if (Array.isArray(r) || (t = _unsupportedIterableToArray(r)) || e && r && "number" == typeof r.length) { t && (r = t); var _n = 0, F = function F() {}; return { s: F, n: function n() { return _n >= r.length ? { done: !0 } : { done: !1, value: r[_n++] }; }, e: function e(r) { throw r; }, f: F }; } throw new TypeError("Invalid attempt to iterate non-iterable instance.\nIn order to be iterable, non-array objects must have a [Symbol.iterator]() method."); } var o, a = !0, u = !1; return { s: function s() { t = t.call(r); }, n: function n() { var r = t.next(); return a = r.done, r; }, e: function e(r) { u = !0, o = r; }, f: function f() { try { a || null == t.return || t.return(); } finally { if (u) throw o; } } }; }
function _typeof(o) { "@babel/helpers - typeof"; return _typeof = "function" == typeof Symbol && "symbol" == typeof Symbol.iterator ? function (o) { return typeof o; } : function (o) { return o && "function" == typeof Symbol && o.constructor === Symbol && o !== Symbol.prototype ? "symbol" : typeof o; }, _typeof(o); }
function _defineProperty(e, r, t) { return (r = _toPropertyKey(r)) in e ? Object.defineProperty(e, r, { value: t, enumerable: !0, configurable: !0, writable: !0 }) : e[r] = t, e; }
function _toPropertyKey(t) { var i = _toPrimitive(t, "string"); return "symbol" == _typeof(i) ? i : i + ""; }
function _toPrimitive(t, r) { if ("object" != _typeof(t) || !t) return t; var e = t[Symbol.toPrimitive]; if (void 0 !== e) { var i = e.call(t, r || "default"); if ("object" != _typeof(i)) return i; throw new TypeError("@@toPrimitive must return a primitive value."); } return ("string" === r ? String : Number)(t); }
function _regeneratorValues(e) { if (null != e) { var t = e["function" == typeof Symbol && Symbol.iterator || "@@iterator"], r = 0; if (t) return t.call(e); if ("function" == typeof e.next) return e; if (!isNaN(e.length)) return { next: function next() { return e && r >= e.length && (e = void 0), { value: e && e[r++], done: !e }; } }; } throw new TypeError(_typeof(e) + " is not iterable"); }
function _toConsumableArray(r) { return _arrayWithoutHoles(r) || _iterableToArray(r) || _unsupportedIterableToArray(r) || _nonIterableSpread(); }
function _nonIterableSpread() { throw new TypeError("Invalid attempt to spread non-iterable instance.\nIn order to be iterable, non-array objects must have a [Symbol.iterator]() method."); }
function _unsupportedIterableToArray(r, a) { if (r) { if ("string" == typeof r) return _arrayLikeToArray(r, a); var t = {}.toString.call(r).slice(8, -1); return "Object" === t && r.constructor && (t = r.constructor.name), "Map" === t || "Set" === t ? Array.from(r) : "Arguments" === t || /^(?:Ui|I)nt(?:8|16|32)(?:Clamped)?Array$/.test(t) ? _arrayLikeToArray(r, a) : void 0; } }
function _iterableToArray(r) { if ("undefined" != typeof Symbol && null != r[Symbol.iterator] || null != r["@@iterator"]) return Array.from(r); }
function _arrayWithoutHoles(r) { if (Array.isArray(r)) return _arrayLikeToArray(r); }
function _arrayLikeToArray(r, a) { (null == a || a > r.length) && (a = r.length); for (var e = 0, n = Array(a); e < a; e++) n[e] = r[e]; return n; }
function _regenerator() { /*! regenerator-runtime -- Copyright (c) 2014-present, Facebook, Inc. -- license (MIT): https://github.com/babel/babel/blob/main/packages/babel-helpers/LICENSE */ var e, t, r = "function" == typeof Symbol ? Symbol : {}, n = r.iterator || "@@iterator", o = r.toStringTag || "@@toStringTag"; function i(r, n, o, i) { var c = n && n.prototype instanceof Generator ? n : Generator, u = Object.create(c.prototype); return _regeneratorDefine2(u, "_invoke", function (r, n, o) { var i, c, u, f = 0, p = o || [], y = !1, G = { p: 0, n: 0, v: e, a: d, f: d.bind(e, 4), d: function d(t, r) { return i = t, c = 0, u = e, G.n = r, a; } }; function d(r, n) { for (c = r, u = n, t = 0; !y && f && !o && t < p.length; t++) { var o, i = p[t], d = G.p, l = i[2]; r > 3 ? (o = l === n) && (u = i[(c = i[4]) ? 5 : (c = 3, 3)], i[4] = i[5] = e) : i[0] <= d && ((o = r < 2 && d < i[1]) ? (c = 0, G.v = n, G.n = i[1]) : d < l && (o = r < 3 || i[0] > n || n > l) && (i[4] = r, i[5] = n, G.n = l, c = 0)); } if (o || r > 1) return a; throw y = !0, n; } return function (o, p, l) { if (f > 1) throw TypeError("Generator is already running"); for (y && 1 === p && d(p, l), c = p, u = l; (t = c < 2 ? e : u) || !y;) { i || (c ? c < 3 ? (c > 1 && (G.n = -1), d(c, u)) : G.n = u : G.v = u); try { if (f = 2, i) { if (c || (o = "next"), t = i[o]) { if (!(t = t.call(i, u))) throw TypeError("iterator result is not an object"); if (!t.done) return t; u = t.value, c < 2 && (c = 0); } else 1 === c && (t = i.return) && t.call(i), c < 2 && (u = TypeError("The iterator does not provide a '" + o + "' method"), c = 1); i = e; } else if ((t = (y = G.n < 0) ? u : r.call(n, G)) !== a) break; } catch (t) { i = e, c = 1, u = t; } finally { f = 1; } } return { value: t, done: y }; }; }(r, o, i), !0), u; } var a = {}; function Generator() {} function GeneratorFunction() {} function GeneratorFunctionPrototype() {} t = Object.getPrototypeOf; var c = [][n] ? t(t([][n]())) : (_regeneratorDefine2(t = {}, n, function () { return this; }), t), u = GeneratorFunctionPrototype.prototype = Generator.prototype = Object.create(c); function f(e) { return Object.setPrototypeOf ? Object.setPrototypeOf(e, GeneratorFunctionPrototype) : (e.__proto__ = GeneratorFunctionPrototype, _regeneratorDefine2(e, o, "GeneratorFunction")), e.prototype = Object.create(u), e; } return GeneratorFunction.prototype = GeneratorFunctionPrototype, _regeneratorDefine2(u, "constructor", GeneratorFunctionPrototype), _regeneratorDefine2(GeneratorFunctionPrototype, "constructor", GeneratorFunction), GeneratorFunction.displayName = "GeneratorFunction", _regeneratorDefine2(GeneratorFunctionPrototype, o, "GeneratorFunction"), _regeneratorDefine2(u), _regeneratorDefine2(u, o, "Generator"), _regeneratorDefine2(u, n, function () { return this; }), _regeneratorDefine2(u, "toString", function () { return "[object Generator]"; }), (_regenerator = function _regenerator() { return { w: i, m: f }; })(); }
function _regeneratorDefine2(e, r, n, t) { var i = Object.defineProperty; try { i({}, "", {}); } catch (e) { i = 0; } _regeneratorDefine2 = function _regeneratorDefine(e, r, n, t) { function o(r, n) { _regeneratorDefine2(e, r, function (e) { return this._invoke(r, n, e); }); } r ? i ? i(e, r, { value: n, enumerable: !t, configurable: !t, writable: !t }) : e[r] = n : (o("next", 0), o("throw", 1), o("return", 2)); }, _regeneratorDefine2(e, r, n, t); }
var _marked = /*#__PURE__*/_regenerator().m(counter),
  _marked2 = /*#__PURE__*/_regenerator().m(deleg),
  _marked3 = /*#__PURE__*/_regenerator().m(thrower),
  _marked4 = /*#__PURE__*/_regenerator().m(labelled),
  _marked5 = /*#__PURE__*/_regenerator().m(fib);
function counter(n) {
  var i, got;
  return _regenerator().w(function (_context) {
    while (1) switch (_context.p = _context.n) {
      case 0:
        _context.p = 0;
        i = 0;
      case 1:
        if (!(i < n)) {
          _context.n = 4;
          break;
        }
        _context.n = 2;
        return i;
      case 2:
        got = _context.v;
        if (got) console.log("got", got);
      case 3:
        i++;
        _context.n = 1;
        break;
      case 4:
        return _context.a(2, "done");
      case 5:
        _context.p = 5;
        console.log("cleanup");
        return _context.f(5);
      case 6:
        return _context.a(2);
    }
  }, _marked, null, [[0,, 5, 6]]);
}
var it = counter(3);
console.log(JSON.stringify(it.next()), JSON.stringify(it.next("a")));
console.log(JSON.stringify(it.return("early")), JSON.stringify(it.next()));
console.log(_toConsumableArray(counter(2)).join(","));
function deleg() {
  var r;
  return _regenerator().w(function (_context2) {
    while (1) switch (_context2.n) {
      case 0:
        return _context2.d(_regeneratorValues(counter(1)), 1);
      case 1:
        r = _context2.v;
        _context2.n = 2;
        return "r=" + r;
      case 2:
        return _context2.a(2);
    }
  }, _marked2);
}
console.log(_toConsumableArray(deleg()).join(","));
function thrower() {
  var _t;
  return _regenerator().w(function (_context3) {
    while (1) switch (_context3.p = _context3.n) {
      case 0:
        _context3.p = 0;
        _context3.n = 1;
        return 1;
      case 1:
        _context3.n = 3;
        break;
      case 2:
        _context3.p = 2;
        _t = _context3.v;
        _context3.n = 3;
        return "caught " + _t;
      case 3:
        _context3.n = 4;
        return 3;
      case 4:
        return _context3.a(2);
    }
  }, _marked3, null, [[0, 2]]);
}
var t = thrower();
t.next();
console.log(t.throw("x").value, t.next().value, t.next().done);
function labelled() {
  var _i, _arr, a, _i2, _arr2, b;
  return _regenerator().w(function (_context4) {
    while (1) switch (_context4.n) {
      case 0:
        _i = 0, _arr = [1, 2];
      case 1:
        if (!(_i < _arr.length)) {
          _context4.n = 6;
          break;
        }
        a = _arr[_i];
        _i2 = 0, _arr2 = [1, 2];
      case 2:
        if (!(_i2 < _arr2.length)) {
          _context4.n = 5;
          break;
        }
        b = _arr2[_i2];
        if (!(b === 2)) {
          _context4.n = 3;
          break;
        }
        return _context4.a(3, 5);
      case 3:
        _context4.n = 4;
        return a * 10 + b;
      case 4:
        _i2++;
        _context4.n = 2;
        break;
      case 5:
        _i++;
        _context4.n = 1;
        break;
      case 6:
        return _context4.a(2);
    }
  }, _marked4);
}
console.log(_toConsumableArray(labelled()).join(","));
var gen = _defineProperty({}, Symbol.iterator, /*#__PURE__*/_regenerator().m(function _callee() {
  return _regenerator().w(function (_context5) {
    while (1) switch (_context5.n) {
      case 0:
        _context5.n = 1;
        return "x";
      case 1:
        _context5.n = 2;
        return "y";
      case 2:
        return _context5.a(2);
    }
  }, _callee);
}));
console.log(_toConsumableArray(gen).join(""));
console.log(_typeof(counter), counter.name);
try {
  var g2 = counter(1);
  g2.next();
  g2.throw(new TypeError("t"));
} catch (e) {
  console.log(e.constructor.name);
}
function fib() {
  var x, y, _ref;
  return _regenerator().w(function (_context6) {
    while (1) switch (_context6.n) {
      case 0:
        x = 0, y = 1;
      case 1:
        if (!true) {
          _context6.n = 3;
          break;
        }
        _context6.n = 2;
        return x;
      case 2:
        _ref = [y, x + y];
        x = _ref[0];
        y = _ref[1];
        _context6.n = 1;
        break;
      case 3:
        return _context6.a(2);
    }
  }, _marked5);
}
var firsts = [];
var _iterator = _createForOfIteratorHelper(fib()),
  _step;
try {
  for (_iterator.s(); !(_step = _iterator.n()).done;) {
    var v = _step.value;
    if (v > 20) break;
    firsts.push(v);
  }
} catch (err) {
  _iterator.e(err);
} finally {
  _iterator.f();
}
console.log(firsts.join(","));
