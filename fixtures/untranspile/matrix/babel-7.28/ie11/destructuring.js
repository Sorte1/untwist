"use strict";

function _regenerator() { /*! regenerator-runtime -- Copyright (c) 2014-present, Facebook, Inc. -- license (MIT): https://github.com/babel/babel/blob/main/packages/babel-helpers/LICENSE */ var e, t, r = "function" == typeof Symbol ? Symbol : {}, n = r.iterator || "@@iterator", o = r.toStringTag || "@@toStringTag"; function i(r, n, o, i) { var c = n && n.prototype instanceof Generator ? n : Generator, u = Object.create(c.prototype); return _regeneratorDefine2(u, "_invoke", function (r, n, o) { var i, c, u, f = 0, p = o || [], y = !1, G = { p: 0, n: 0, v: e, a: d, f: d.bind(e, 4), d: function d(t, r) { return i = t, c = 0, u = e, G.n = r, a; } }; function d(r, n) { for (c = r, u = n, t = 0; !y && f && !o && t < p.length; t++) { var o, i = p[t], d = G.p, l = i[2]; r > 3 ? (o = l === n) && (u = i[(c = i[4]) ? 5 : (c = 3, 3)], i[4] = i[5] = e) : i[0] <= d && ((o = r < 2 && d < i[1]) ? (c = 0, G.v = n, G.n = i[1]) : d < l && (o = r < 3 || i[0] > n || n > l) && (i[4] = r, i[5] = n, G.n = l, c = 0)); } if (o || r > 1) return a; throw y = !0, n; } return function (o, p, l) { if (f > 1) throw TypeError("Generator is already running"); for (y && 1 === p && d(p, l), c = p, u = l; (t = c < 2 ? e : u) || !y;) { i || (c ? c < 3 ? (c > 1 && (G.n = -1), d(c, u)) : G.n = u : G.v = u); try { if (f = 2, i) { if (c || (o = "next"), t = i[o]) { if (!(t = t.call(i, u))) throw TypeError("iterator result is not an object"); if (!t.done) return t; u = t.value, c < 2 && (c = 0); } else 1 === c && (t = i.return) && t.call(i), c < 2 && (u = TypeError("The iterator does not provide a '" + o + "' method"), c = 1); i = e; } else if ((t = (y = G.n < 0) ? u : r.call(n, G)) !== a) break; } catch (t) { i = e, c = 1, u = t; } finally { f = 1; } } return { value: t, done: y }; }; }(r, o, i), !0), u; } var a = {}; function Generator() {} function GeneratorFunction() {} function GeneratorFunctionPrototype() {} t = Object.getPrototypeOf; var c = [][n] ? t(t([][n]())) : (_regeneratorDefine2(t = {}, n, function () { return this; }), t), u = GeneratorFunctionPrototype.prototype = Generator.prototype = Object.create(c); function f(e) { return Object.setPrototypeOf ? Object.setPrototypeOf(e, GeneratorFunctionPrototype) : (e.__proto__ = GeneratorFunctionPrototype, _regeneratorDefine2(e, o, "GeneratorFunction")), e.prototype = Object.create(u), e; } return GeneratorFunction.prototype = GeneratorFunctionPrototype, _regeneratorDefine2(u, "constructor", GeneratorFunctionPrototype), _regeneratorDefine2(GeneratorFunctionPrototype, "constructor", GeneratorFunction), GeneratorFunction.displayName = "GeneratorFunction", _regeneratorDefine2(GeneratorFunctionPrototype, o, "GeneratorFunction"), _regeneratorDefine2(u), _regeneratorDefine2(u, o, "Generator"), _regeneratorDefine2(u, n, function () { return this; }), _regeneratorDefine2(u, "toString", function () { return "[object Generator]"; }), (_regenerator = function _regenerator() { return { w: i, m: f }; })(); }
function _regeneratorDefine2(e, r, n, t) { var i = Object.defineProperty; try { i({}, "", {}); } catch (e) { i = 0; } _regeneratorDefine2 = function _regeneratorDefine(e, r, n, t) { function o(r, n) { _regeneratorDefine2(e, r, function (e) { return this._invoke(r, n, e); }); } r ? i ? i(e, r, { value: n, enumerable: !t, configurable: !t, writable: !t }) : e[r] = n : (o("next", 0), o("throw", 1), o("return", 2)); }, _regeneratorDefine2(e, r, n, t); }
function _slicedToArray(r, e) { return _arrayWithHoles(r) || _iterableToArrayLimit(r, e) || _unsupportedIterableToArray(r, e) || _nonIterableRest(); }
function _nonIterableRest() { throw new TypeError("Invalid attempt to destructure non-iterable instance.\nIn order to be iterable, non-array objects must have a [Symbol.iterator]() method."); }
function _unsupportedIterableToArray(r, a) { if (r) { if ("string" == typeof r) return _arrayLikeToArray(r, a); var t = {}.toString.call(r).slice(8, -1); return "Object" === t && r.constructor && (t = r.constructor.name), "Map" === t || "Set" === t ? Array.from(r) : "Arguments" === t || /^(?:Ui|I)nt(?:8|16|32)(?:Clamped)?Array$/.test(t) ? _arrayLikeToArray(r, a) : void 0; } }
function _iterableToArrayLimit(r, l) { var t = null == r ? null : "undefined" != typeof Symbol && r[Symbol.iterator] || r["@@iterator"]; if (null != t) { var e, n, i, u, a = [], f = !0, o = !1; try { if (i = (t = t.call(r)).next, 0 === l) { if (Object(t) !== t) return; f = !1; } else for (; !(f = (e = i.call(t)).done) && (a.push(e.value), a.length !== l); f = !0); } catch (r) { o = !0, n = r; } finally { try { if (!f && null != t.return && (u = t.return(), Object(u) !== u)) return; } finally { if (o) throw n; } } return a; } }
function _arrayWithHoles(r) { if (Array.isArray(r)) return r; }
function _objectWithoutProperties(e, t) { if (null == e) return {}; var o, r, i = _objectWithoutPropertiesLoose(e, t); if (Object.getOwnPropertySymbols) { var n = Object.getOwnPropertySymbols(e); for (r = 0; r < n.length; r++) o = n[r], -1 === t.indexOf(o) && {}.propertyIsEnumerable.call(e, o) && (i[o] = e[o]); } return i; }
function _objectWithoutPropertiesLoose(r, e) { if (null == r) return {}; var t = {}; for (var n in r) if ({}.hasOwnProperty.call(r, n)) { if (-1 !== e.indexOf(n)) continue; t[n] = r[n]; } return t; }
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
  others = _objectWithoutProperties(_x$q$r, ["x", "y", "z"]);
console.log(a, b, rest.join(","), x, why, deep, JSON.stringify(others));
function point() {
  var _ref2 = arguments.length > 0 && arguments[0] !== undefined ? arguments[0] : {},
    _ref2$x = _ref2.x,
    x = _ref2$x === void 0 ? 0 : _ref2$x,
    _ref2$y = _ref2.y,
    y = _ref2$y === void 0 ? 0 : _ref2$y;
  var _ref3 = arguments.length > 1 && arguments[1] !== undefined ? arguments[1] : [],
    _ref4 = _slicedToArray(_ref3, 2),
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
  var _pairs$_i = _slicedToArray(_pairs[_i], 2),
    num = _pairs$_i[0],
    word = _pairs$_i[1];
  console.log(num, word);
}
var _hello = "hello",
  length = _hello.length;
var _hi = "hi",
  _hi2 = _slicedToArray(_hi, 2),
  c1 = _hi2[0],
  c2 = _hi2[1];
console.log(length, c1, c2);
var data = {
  list: [{
    id: 7,
    tags: ["t1", "t2"]
  }]
};
var _data$list = _slicedToArray(data.list, 1),
  _data$list$ = _data$list[0],
  id = _data$list$.id,
  _data$list$$tags = _slicedToArray(_data$list$.tags, 2),
  second = _data$list$$tags[1];
console.log(id, second);
var key = "dyn";
var _ref6 = {},
  _ref6$key = _ref6[key],
  dynamic = _ref6$key === void 0 ? "fallback" : _ref6$key;
console.log(dynamic);
function swap(_ref7) {
  var _ref8 = _slicedToArray(_ref7, 2),
    p = _ref8[0],
    q = _ref8[1];
  return [q, p];
}
console.log(swap([1, 2]).join(","));
var _ref9 = /*#__PURE__*/_regenerator().m(function _callee() {
    return _regenerator().w(function (_context) {
      while (1) switch (_context.n) {
        case 0:
          _context.n = 1;
          return "a";
        case 1:
          _context.n = 2;
          return "b";
        case 2:
          _context.n = 3;
          return "c";
        case 3:
          return _context.a(2);
      }
    }, _callee);
  })(),
  _ref0 = _slicedToArray(_ref9, 2),
  g1 = _ref0[0],
  g2 = _ref0[1];
console.log(g1, g2);
