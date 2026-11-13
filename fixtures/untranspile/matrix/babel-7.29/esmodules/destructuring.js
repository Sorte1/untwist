"use strict";

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
for (var _ref8 of pairs) {
  var _ref7 = _slicedToArray(_ref8, 2);
  var num = _ref7[0];
  var word = _ref7[1];
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
function swap(_ref9) {
  var _ref0 = _slicedToArray(_ref9, 2),
    p = _ref0[0],
    q = _ref0[1];
  return [q, p];
}
console.log(swap([1, 2]).join(","));
var _ref1 = function* () {
    yield "a";
    yield "b";
    yield "c";
  }(),
  _ref10 = _slicedToArray(_ref1, 2),
  g1 = _ref10[0],
  g2 = _ref10[1];
console.log(g1, g2);
