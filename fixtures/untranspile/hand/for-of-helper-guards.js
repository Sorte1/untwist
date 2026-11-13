"use strict";

function _createForOfIteratorHelper(r, e) { var t = "undefined" != typeof Symbol && r[Symbol.iterator] || r["@@iterator"]; if (!t) { if (Array.isArray(r) || (t = _unsupportedIterableToArray(r)) || e && r && "number" == typeof r.length) { t && (r = t); var _n = 0, F = function F() {}; return { s: F, n: function n() { return _n >= r.length ? { done: !0 } : { done: !1, value: r[_n++] }; }, e: function e(r) { throw r; }, f: F }; } throw new TypeError("Invalid attempt to iterate non-iterable instance.\nIn order to be iterable, non-array objects must have a [Symbol.iterator]() method."); } var o, a = !0, u = !1; return { s: function s() { t = t.call(r); }, n: function n() { var r = t.next(); return a = r.done, r; }, e: function e(r) { u = !0, o = r; }, f: function f() { try { a || null == t.return || t.return(); } finally { if (u) throw o; } } }; }
function _unsupportedIterableToArray(r, a) { if (r) { if ("string" == typeof r) return _arrayLikeToArray(r, a); var t = {}.toString.call(r).slice(8, -1); return "Object" === t && r.constructor && (t = r.constructor.name), "Map" === t || "Set" === t ? Array.from(r) : "Arguments" === t || /^(?:Ui|I)nt(?:8|16|32)(?:Clamped)?Array$/.test(t) ? _arrayLikeToArray(r, a) : void 0; } }
function _arrayLikeToArray(r, a) { (null == a || a > r.length) && (a = r.length); for (var e = 0, n = Array(a); e < a; e++) n[e] = r[e]; return n; }
var log = [];
var items = { length: 2, 0: "a", 1: "b" };
var _iterator = _createForOfIteratorHelper(items, true),
  _step;
try {
  for (_iterator.s(); !(_step = _iterator.n()).done;) {
    var arrayLike = _step.value;
    log.push(arrayLike);
  }
} catch (err) {
  _iterator.e(err);
} finally {
  _iterator.f();
}
var row = [1, 2];
var _iterator2 = _createForOfIteratorHelper(row),
  _step2;
try {
  for (_iterator2.s(); !(_step2 = _iterator2.n()).done;) {
    let row = _step2.value;
    log.push(row);
  }
} catch (err) {
  _iterator2.e(err);
} finally {
  _iterator2.f();
}
var _iterator3 = _createForOfIteratorHelper([3, 4]),
  _step3;
try {
  for (_iterator3.s(); !(_step3 = _iterator3.n()).done;) {
    log.push(_step3.value, _step3.done);
  }
} catch (err) {
  _iterator3.e(err);
} finally {
  _iterator3.f();
}
var _iterator4 = _createForOfIteratorHelper([5, 6]),
  _step4;
log.push(typeof _iterator4.s);
try {
  for (_iterator4.s(); !(_step4 = _iterator4.n()).done;) {
    var kept = _step4.value;
    log.push(kept);
  }
} catch (err) {
  _iterator4.e(err);
} finally {
  _iterator4.f();
}
var _iterator5 = _createForOfIteratorHelper([7, 8]),
  _step5;
try {
  for (_iterator5.s(); !(_step5 = _iterator5.n()).done;) {
    var converted = _step5.value;
    log.push(converted);
  }
} catch (err) {
  _iterator5.e(err);
} finally {
  _iterator5.f();
}
console.log(log.join(","));
