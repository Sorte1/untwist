"use strict";

function _slicedToArray(r, e) { return _arrayWithHoles(r) || _iterableToArrayLimit(r, e) || _unsupportedIterableToArray(r, e) || _nonIterableRest(); }
function _nonIterableRest() { throw new TypeError("Invalid attempt to destructure non-iterable instance.\nIn order to be iterable, non-array objects must have a [Symbol.iterator]() method."); }
function _iterableToArrayLimit(r, l) { var t = null == r ? null : "undefined" != typeof Symbol && r[Symbol.iterator] || r["@@iterator"]; if (null != t) { var e, n, i, u, a = [], f = !0, o = !1; try { if (i = (t = t.call(r)).next, 0 === l) { if (Object(t) !== t) return; f = !1; } else for (; !(f = (e = i.call(t)).done) && (a.push(e.value), a.length !== l); f = !0); } catch (r) { o = !0, n = r; } finally { try { if (!f && null != t.return && (u = t.return(), Object(u) !== u)) return; } finally { if (o) throw n; } } return a; } }
function _arrayWithHoles(r) { if (Array.isArray(r)) return r; }
function _createForOfIteratorHelper(r, e) { var t = "undefined" != typeof Symbol && r[Symbol.iterator] || r["@@iterator"]; if (!t) { if (Array.isArray(r) || (t = _unsupportedIterableToArray(r)) || e && r && "number" == typeof r.length) { t && (r = t); var _n = 0, F = function F() {}; return { s: F, n: function n() { return _n >= r.length ? { done: !0 } : { done: !1, value: r[_n++] }; }, e: function e(r) { throw r; }, f: F }; } throw new TypeError("Invalid attempt to iterate non-iterable instance.\nIn order to be iterable, non-array objects must have a [Symbol.iterator]() method."); } var o, a = !0, u = !1; return { s: function s() { t = t.call(r); }, n: function n() { var r = t.next(); return a = r.done, r; }, e: function e(r) { u = !0, o = r; }, f: function f() { try { a || null == t.return || t.return(); } finally { if (u) throw o; } } }; }
function _unsupportedIterableToArray(r, a) { if (r) { if ("string" == typeof r) return _arrayLikeToArray(r, a); var t = {}.toString.call(r).slice(8, -1); return "Object" === t && r.constructor && (t = r.constructor.name), "Map" === t || "Set" === t ? Array.from(r) : "Arguments" === t || /^(?:Ui|I)nt(?:8|16|32)(?:Clamped)?Array$/.test(t) ? _arrayLikeToArray(r, a) : void 0; } }
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
var custom = {
  from: 1,
  to: 4,
  [Symbol.iterator]() {
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
      return() {
        out.push("closed");
        return {
          done: true
        };
      }
    };
  }
};
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
