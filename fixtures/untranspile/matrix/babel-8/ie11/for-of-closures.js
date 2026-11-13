function _slicedToArray(e, i) { return _iterableToArrayLimit(e, i) || _unsupportedIterableToArray(e, i) || _nonIterableRest(); }
function _nonIterableRest() { throw new TypeError("Invalid attempt to destructure non-iterable instance.\nIn order to be iterable, non-array objects must have a [Symbol.iterator]() method."); }
function _iterableToArrayLimit(r, t) { var e = null == r ? null : "undefined" != typeof Symbol && r[Symbol.iterator] || r["@@iterator"]; if (Array.isArray(r) && (null == e || e === ("undefined" != typeof Symbol && [][Symbol.iterator]) || e === []["@@iterator"])) return r; if (null != e) { var l, n, i, a, u = [], o = !0, f = !1; try { if (i = (e = e.call(r)).next, 0 === t) { if (Object(e) !== e) return; o = !1; } else for (; !(o = (l = i.call(e)).done) && (u.push(l.value), u.length !== t); o = !0); } catch (r) { f = !0, n = r; } finally { try { if (!o && null != e.return && (a = e.return(), Object(a) !== a)) return; } finally { if (f) throw n; } } return u; } }
function _createForOfIteratorHelper(e, t) { var _n = "undefined" != typeof Symbol && e[Symbol.iterator] || e["@@iterator"]; if (!_n) { if (Array.isArray(e) || (_n = _unsupportedIterableToArray(e)) || t && e && "number" == typeof e.length) { _n && (e = _n); var o = 0, F = function F() {}; return { s: F, n: function n() { return o >= e.length ? { done: !0 } : { done: !1, value: e[o++] }; }, e: function e(r) { throw r; }, f: F }; } throw new TypeError("Invalid attempt to iterate non-iterable instance.\nIn order to be iterable, non-array objects must have a [Symbol.iterator]() method."); } var a, u, i = !0; return { s: function s() { _n = _n.call(e); }, n: function n() { i = !0; var r = _n.next(); return r.done ? { done: !0 } : { value: r.value, done: i = !1 }; }, e: function e(r) { a = !0, u = r; }, f: function f() { try { i || null == _n.return || _n.return(); } finally { if (a) throw u; } } }; }
function _unsupportedIterableToArray(t, e) { if (t) { if ("string" == typeof t) return _arrayLikeToArray(t, e); var o = {}.toString.call(t).slice(8, -1); return "Object" === o && t.constructor && (o = t.constructor.name), "Map" === o || "Set" === o ? Array.from(t) : "Arguments" === o || /^(?:Ui|I)nt(?:8|16|32)(?:Clamped)?Array$/.test(o) ? _arrayLikeToArray(t, e) : void 0; } }
function _arrayLikeToArray(r, a) { (null == a || a > r.length) && (a = r.length); for (var e = 0, n = Array(a); e < a; e++) n[e] = r[e]; return n; }
var fns = [];
var _iterator = _createForOfIteratorHelper(new Set([1, 2, 3])),
  _step;
try {
  var _loop = function _loop() {
    var v = _step.value;
    var doubled = v * 2;
    var note = "n" + v;
    fns.push(function () {
      return doubled + note;
    });
    note += "!";
  };
  for (_iterator.s(); !(_step = _iterator.n()).done;) {
    _loop();
  }
} catch (err) {
  _iterator.e(err);
} finally {
  _iterator.f();
}
console.log(fns.map(function (f) {
  return f();
}).join(","));
var seen = [];
var _iterator2 = _createForOfIteratorHelper(new Map([["a", 1], ["b", 2]])),
  _step2;
try {
  var _loop2 = function _loop2() {
    var _step2$value = _slicedToArray(_step2.value, 2),
      key = _step2$value[0],
      value = _step2$value[1];
    seen.push(function () {
      return key + value;
    });
  };
  for (_iterator2.s(); !(_step2 = _iterator2.n()).done;) {
    _loop2();
  }
} catch (err) {
  _iterator2.e(err);
} finally {
  _iterator2.f();
}
console.log(seen.map(function (f) {
  return f();
}).join(","));
var later = [];
var _iterator3 = _createForOfIteratorHelper("abc"),
  _step3;
try {
  var _loop3 = function _loop3() {
    var word = _step3.value;
    var count = 0;
    later.push(function () {
      return word + ++count;
    });
  };
  for (_iterator3.s(); !(_step3 = _iterator3.n()).done;) {
    _loop3();
  }
} catch (err) {
  _iterator3.e(err);
} finally {
  _iterator3.f();
}
console.log(later.map(function (f) {
  return f() + f();
}).join(","));
