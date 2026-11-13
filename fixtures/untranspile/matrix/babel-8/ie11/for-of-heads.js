function _createForOfIteratorHelper(e, t) { var _n = "undefined" != typeof Symbol && e[Symbol.iterator] || e["@@iterator"]; if (!_n) { if (Array.isArray(e) || (_n = _unsupportedIterableToArray(e)) || t && e && "number" == typeof e.length) { _n && (e = _n); var o = 0, F = function F() {}; return { s: F, n: function n() { return o >= e.length ? { done: !0 } : { done: !1, value: e[o++] }; }, e: function e(r) { throw r; }, f: F }; } throw new TypeError("Invalid attempt to iterate non-iterable instance.\nIn order to be iterable, non-array objects must have a [Symbol.iterator]() method."); } var a, u, i = !0; return { s: function s() { _n = _n.call(e); }, n: function n() { i = !0; var r = _n.next(); return r.done ? { done: !0 } : { value: r.value, done: i = !1 }; }, e: function e(r) { a = !0, u = r; }, f: function f() { try { i || null == _n.return || _n.return(); } finally { if (a) throw u; } } }; }
function _unsupportedIterableToArray(t, e) { if (t) { if ("string" == typeof t) return _arrayLikeToArray(t, e); var o = {}.toString.call(t).slice(8, -1); return "Object" === o && t.constructor && (o = t.constructor.name), "Map" === o || "Set" === o ? Array.from(t) : "Arguments" === o || /^(?:Ui|I)nt(?:8|16|32)(?:Clamped)?Array$/.test(o) ? _arrayLikeToArray(t, e) : void 0; } }
function _arrayLikeToArray(r, a) { (null == a || a > r.length) && (a = r.length); for (var e = 0, n = Array(a); e < a; e++) n[e] = r[e]; return n; }
var calls = [];
var handlers = [function () {
  return calls.push("a");
}, function () {
  return calls.push("b");
}];
var _iterator = _createForOfIteratorHelper(new Set(handlers)),
  _step;
try {
  for (_iterator.s(); !(_step = _iterator.n()).done;) {
    var handler = _step.value;
    handler();
  }
} catch (err) {
  _iterator.e(err);
} finally {
  _iterator.f();
}
var counts = {};
var _iterator2 = _createForOfIteratorHelper(new Set(["x", "y", "x"])),
  _step2;
try {
  for (_iterator2.s(); !(_step2 = _iterator2.n()).done;) {
    var key = _step2.value;
    counts[key] = (counts[key] || 0) + 1;
  }
} catch (err) {
  _iterator2.e(err);
} finally {
  _iterator2.f();
}
var _iterator3 = _createForOfIteratorHelper(new Set([{
    list: [1, 2]
  }, {
    list: [3]
  }])),
  _step3;
try {
  for (_iterator3.s(); !(_step3 = _iterator3.n()).done;) {
    var entry = _step3.value;
    var _iterator7 = _createForOfIteratorHelper(entry.list),
      _step7;
    try {
      for (_iterator7.s(); !(_step7 = _iterator7.n()).done;) {
        var n = _step7.value;
        calls.push(n);
      }
    } catch (err) {
      _iterator7.e(err);
    } finally {
      _iterator7.f();
    }
  }
} catch (err) {
  _iterator3.e(err);
} finally {
  _iterator3.f();
}
var removed = {
  a: 1,
  b: 2,
  c: 3
};
var _iterator4 = _createForOfIteratorHelper(new Set(["a", "c"])),
  _step4;
try {
  for (_iterator4.s(); !(_step4 = _iterator4.n()).done;) {
    var _key = _step4.value;
    delete removed[_key];
  }
} catch (err) {
  _iterator4.e(err);
} finally {
  _iterator4.f();
}
var total = 0;
var weights = {
  p: 2,
  q: 5
};
var _iterator5 = _createForOfIteratorHelper(new Set(["p", "q", "r"])),
  _step5;
try {
  for (_iterator5.s(); !(_step5 = _iterator5.n()).done;) {
    var name = _step5.value;
    total += weights[name] || 0;
  }
} catch (err) {
  _iterator5.e(err);
} finally {
  _iterator5.f();
}
var _iterator6 = _createForOfIteratorHelper(new Set([{
    run: function run(k) {
      return calls.push(k);
    }
  }])),
  _step6;
try {
  for (_iterator6.s(); !(_step6 = _iterator6.n()).done;) {
    var item = _step6.value;
    item.run("ran");
  }
} catch (err) {
  _iterator6.e(err);
} finally {
  _iterator6.f();
}
console.log(calls.join(), JSON.stringify(counts), JSON.stringify(removed), total);
