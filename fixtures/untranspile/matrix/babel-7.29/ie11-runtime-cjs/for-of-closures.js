(function () {
  'use strict';

  var _interopRequireDefault = require("@babel/runtime/helpers/interopRequireDefault");
  var _slicedToArray2 = _interopRequireDefault(require("@babel/runtime/helpers/slicedToArray"));
  function _createForOfIteratorHelper(r, e) { var t = "undefined" != typeof Symbol && r[Symbol.iterator] || r["@@iterator"]; if (!t) { if (Array.isArray(r) || (t = _unsupportedIterableToArray(r)) || e) { t && (r = t); var _n = 0, F = function F() {}; return { s: F, n: function n() { return _n >= r.length ? { done: true } : { done: false, value: r[_n++] }; }, e: function e(r) { throw r; }, f: F }; } throw new TypeError("Invalid attempt to iterate non-iterable instance.\nIn order to be iterable, non-array objects must have a [Symbol.iterator]() method."); } var o, a = true, u = false; return { s: function s() { t = t.call(r); }, n: function n() { var r = t.next(); return a = r.done, r; }, e: function e(r) { u = true, o = r; }, f: function f() { try { a || null == t.return || t.return(); } finally { if (u) throw o; } } }; }
  function _unsupportedIterableToArray(r, a) { if (r) { if ("string" == typeof r) return _arrayLikeToArray(r, a); var t = {}.toString.call(r).slice(8, -1); return "Object" === t && r.constructor && (t = r.constructor.name), "Map" === t || "Set" === t ? Array.from(r) : "Arguments" === t || /^(?:Ui|I)nt(?:8|16|32)(?:Clamped)?Array$/.test(t) ? _arrayLikeToArray(r, a) : void 0; } }
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
      var _step2$value = (0, _slicedToArray2.default)(_step2.value, 2),
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

})();
