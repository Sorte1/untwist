var _ref9;
function _slicedToArray(r, e) { return _arrayWithHoles(r) || _iterableToArrayLimit(r, e) || _unsupportedIterableToArray(r, e) || _nonIterableRest(); }
function _nonIterableRest() { throw new TypeError("Invalid attempt to destructure non-iterable instance.\nIn order to be iterable, non-array objects must have a [Symbol.iterator]() method."); }
function _unsupportedIterableToArray(r, a) { if (r) { if ("string" == typeof r) return _arrayLikeToArray(r, a); var t = {}.toString.call(r).slice(8, -1); return "Object" === t && r.constructor && (t = r.constructor.name), "Map" === t || "Set" === t ? Array.from(r) : "Arguments" === t || /^(?:Ui|I)nt(?:8|16|32)(?:Clamped)?Array$/.test(t) ? _arrayLikeToArray(r, a) : void 0; } }
function _iterableToArrayLimit(r, l) { var t = null == r ? null : "undefined" != typeof Symbol && r[Symbol.iterator] || r["@@iterator"]; if (null != t) { var e, n, i, u, a = [], f = !0, o = !1; try { if (i = (t = t.call(r)).next, 0 === l) { if (Object(t) !== t) return; f = !1; } else for (; !(f = (e = i.call(t)).done) && (a.push(e.value), a.length !== l); f = !0); } catch (r) { o = !0, n = r; } finally { try { if (!f && null != t.return && (u = t.return(), Object(u) !== u)) return; } finally { if (o) throw n; } } return a; } }
function _arrayWithHoles(r) { if (Array.isArray(r)) return r; }
function _arrayLikeToArray(r, a) { (null == a || a > r.length) && (a = r.length); for (var e = 0, n = Array(a); e < a; e++) n[e] = r[e]; return n; }
var _ref = [1, 2, undefined, 4, 5],
  p = _ref[0],
  _ref$ = _ref[2],
  q = _ref$ === void 0 ? 7 : _ref$,
  r = _arrayLikeToArray(_ref).slice(3);
console.log(p, q, r.join(","));
var _Set = new Set(["s1", "s2", "s3"]),
  _Set2 = _slicedToArray(_Set, 2),
  m = _Set2[0],
  n = _Set2[1];
console.log(m, n);
var _c$k = {
    c: undefined,
    k1: "kv"
  },
  _c$k$a = _c$k.a,
  _c$k$a2 = _c$k$a === void 0 ? {
    b: "dflt"
  } : _c$k$a,
  deep = _c$k$a2.b,
  _c$k$c = _c$k.c,
  c = _c$k$c === void 0 ? 3 : _c$k$c,
  computed = _c$k["k" + 1];
console.log(deep, c, computed);
function f() {
  var _ref2 = arguments.length > 0 && arguments[0] !== undefined ? arguments[0] : {},
    x = _ref2.x,
    _ref2$y = _ref2.y,
    y = _ref2$y === void 0 ? 2 : _ref2$y;
  var _ref3 = arguments.length > 1 && arguments[1] !== undefined ? arguments[1] : [9],
    _ref4 = _slicedToArray(_ref3, 1),
    z = _ref4[0];
  return [x, y, z].join("/");
}
console.log(f(), f({
  x: 1
}, [5]), f.length);
var u, v;
var _ref5 = (_ref9 = [1, 2], v = _ref9[0], u = _ref9[1], _ref9);
var _ref6 = _slicedToArray(_ref5, 2);
u = _ref6[0];
v = _ref6[1];
console.log(u, v);
for (var _i = 0, _Object$entries = Object.entries({
    e1: 1,
    e2: 2
  }); _i < _Object$entries.length; _i++) {
  var _Object$entries$_i = _slicedToArray(_Object$entries[_i], 2),
    key = _Object$entries$_i[0],
    value = _Object$entries$_i[1];
  console.log(key, value);
}
var order = [];
var src = {
  get first() {
    order.push("first");
    return 1;
  },
  get second() {
    order.push("second");
    return 2;
  }
};
var second = src.second,
  first = src.first;
console.log(order.join(","), first, second);
try {
  var _ref7 = null,
    _ref8 = _slicedToArray(_ref7, 1),
    t = _ref8[0];
  console.log(t);
} catch (e) {
  console.log(e.constructor.name);
}
try {
  var t2 = undefined.t2;
  console.log(t2);
} catch (e) {
  console.log(e.constructor.name);
}
var str = "ab";
var _str = _slicedToArray(str, 2),
  c1 = _str[0],
  c2 = _str[1];
console.log(c1, c2);
var swap = {
  l: "L",
  r: "R"
};
swap.r = swap.l;
swap.l = swap.r;
console.log(swap.l, swap.r);
var _four = "four",
  length = _four.length;
console.log(length);
var n1 = 1,
  n2 = 2,
  _o$o = _slicedToArray({
    o: [3]
  }.o, 1),
  n3 = _o$o[0];
console.log(n1 + n2 + n3);
