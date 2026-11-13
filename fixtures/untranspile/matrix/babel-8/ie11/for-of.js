function _typeof(o) { "@babel/helpers - typeof"; return _typeof = "function" == typeof Symbol && "symbol" == typeof Symbol.iterator ? function (o) { return typeof o; } : function (o) { return o && "function" == typeof Symbol && o.constructor === Symbol && o !== Symbol.prototype ? "symbol" : typeof o; }, _typeof(o); }
function _slicedToArray(e, i) { return _iterableToArrayLimit(e, i) || _unsupportedIterableToArray(e, i) || _nonIterableRest(); }
function _nonIterableRest() { throw new TypeError("Invalid attempt to destructure non-iterable instance.\nIn order to be iterable, non-array objects must have a [Symbol.iterator]() method."); }
function _iterableToArrayLimit(r, t) { var e = null == r ? null : "undefined" != typeof Symbol && r[Symbol.iterator] || r["@@iterator"]; if (Array.isArray(r) && (null == e || e === ("undefined" != typeof Symbol && [][Symbol.iterator]) || e === []["@@iterator"])) return r; if (null != e) { var l, n, i, a, u = [], o = !0, f = !1; try { if (i = (e = e.call(r)).next, 0 === t) { if (Object(e) !== e) return; o = !1; } else for (; !(o = (l = i.call(e)).done) && (u.push(l.value), u.length !== t); o = !0); } catch (r) { f = !0, n = r; } finally { try { if (!o && null != e.return && (a = e.return(), Object(a) !== a)) return; } finally { if (f) throw n; } } return u; } }
function _defineProperty(r, t, o) { return (t = _toPropertyKey(t)) in r ? Object.defineProperty(r, t, { value: o, enumerable: !0, configurable: !0, writable: !0 }) : r[t] = o, r; }
function _toPropertyKey(r) { var o = _toPrimitive(r, "string"); return "symbol" == _typeof(o) ? o : o + ""; }
function _toPrimitive(t, e) { if ("object" != _typeof(t) || !t) return t; var r; if ("undefined" != typeof Symbol && void 0 !== (r = t[Symbol.toPrimitive])) { var i = r.call(t, e || "default"); if ("object" != _typeof(i)) return i; throw new TypeError("@@toPrimitive must return a primitive value."); } return ("string" === e ? String : Number)(t); }
function _createForOfIteratorHelper(e, t) { var _n = "undefined" != typeof Symbol && e[Symbol.iterator] || e["@@iterator"]; if (!_n) { if (Array.isArray(e) || (_n = _unsupportedIterableToArray(e)) || t && e && "number" == typeof e.length) { _n && (e = _n); var o = 0, F = function F() {}; return { s: F, n: function n() { return o >= e.length ? { done: !0 } : { done: !1, value: e[o++] }; }, e: function e(r) { throw r; }, f: F }; } throw new TypeError("Invalid attempt to iterate non-iterable instance.\nIn order to be iterable, non-array objects must have a [Symbol.iterator]() method."); } var a, u, i = !0; return { s: function s() { _n = _n.call(e); }, n: function n() { i = !0; var r = _n.next(); return r.done ? { done: !0 } : { value: r.value, done: i = !1 }; }, e: function e(r) { a = !0, u = r; }, f: function f() { try { i || null == _n.return || _n.return(); } finally { if (a) throw u; } } }; }
function _unsupportedIterableToArray(t, e) { if (t) { if ("string" == typeof t) return _arrayLikeToArray(t, e); var o = {}.toString.call(t).slice(8, -1); return "Object" === o && t.constructor && (o = t.constructor.name), "Map" === o || "Set" === o ? Array.from(t) : "Arguments" === o || /^(?:Ui|I)nt(?:8|16|32)(?:Clamped)?Array$/.test(o) ? _arrayLikeToArray(t, e) : void 0; } }
function _arrayLikeToArray(r, a) { (null == a || a > r.length) && (a = r.length); for (var e = 0, n = Array(a); e < a; e++) n[e] = r[e]; return n; }
var out = [];
for (var _i = 0, _arr = [1, 2, 3]; _i < _arr.length; _i++) {
  var v = _arr[_i];
  out.push(v * 2);
}
var _iterator = _createForOfIteratorHelper("héllo"),
  _step;
try {
  for (_iterator.s(); !(_step = _iterator.n()).done;) {
    var ch = _step.value;
    out.push(ch);
  }
} catch (err) {
  _iterator.e(err);
} finally {
  _iterator.f();
}
var _iterator2 = _createForOfIteratorHelper(new Map([["a", 1], ["b", 2]])),
  _step2;
try {
  for (_iterator2.s(); !(_step2 = _iterator2.n()).done;) {
    var _step2$value = _slicedToArray(_step2.value, 2),
      k = _step2$value[0],
      _v = _step2$value[1];
    out.push(k + _v);
  }
} catch (err) {
  _iterator2.e(err);
} finally {
  _iterator2.f();
}
var _iterator3 = _createForOfIteratorHelper(new Set([3, 3, 4])),
  _step3;
try {
  for (_iterator3.s(); !(_step3 = _iterator3.n()).done;) {
    var _v2 = _step3.value;
    out.push(_v2);
  }
} catch (err) {
  _iterator3.e(err);
} finally {
  _iterator3.f();
}
console.log(out.join(","));
var custom = _defineProperty({
  from: 1,
  to: 4
}, Symbol.iterator, function () {
  var cur = this.from;
  var last = this.to;
  return {
    next: function next() {
      return cur <= last ? {
        value: cur++,
        done: false
      } : {
        value: undefined,
        done: true
      };
    },
    return: function _return() {
      out.push("closed");
      return {
        done: true
      };
    }
  };
});
var got = [];
var _iterator4 = _createForOfIteratorHelper(custom),
  _step4;
try {
  for (_iterator4.s(); !(_step4 = _iterator4.n()).done;) {
    var _v3 = _step4.value;
    if (_v3 === 3) break;
    got.push(_v3);
  }
} catch (err) {
  _iterator4.e(err);
} finally {
  _iterator4.f();
}
console.log(got.join(","), out[out.length - 1]);
function args() {
  var xs = [];
  var _iterator5 = _createForOfIteratorHelper(arguments),
    _step5;
  try {
    for (_iterator5.s(); !(_step5 = _iterator5.n()).done;) {
      var a = _step5.value;
      xs.push(a);
    }
  } catch (err) {
    _iterator5.e(err);
  } finally {
    _iterator5.f();
  }
  return xs.join("+");
}
console.log(args(1, 2, 3));
var total = 0;
outer: for (var _i2 = 0, _arr2 = [[1, 2], [3, 4], [5, 6]]; _i2 < _arr2.length; _i2++) {
  var row = _arr2[_i2];
  var _iterator6 = _createForOfIteratorHelper(row),
    _step6;
  try {
    for (_iterator6.s(); !(_step6 = _iterator6.n()).done;) {
      var cell = _step6.value;
      if (cell === 4) continue outer;
      if (cell === 6) break outer;
      total += cell;
    }
  } catch (err) {
    _iterator6.e(err);
  } finally {
    _iterator6.f();
  }
}
console.log(total);
