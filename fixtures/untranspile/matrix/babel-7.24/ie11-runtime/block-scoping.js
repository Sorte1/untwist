(function () {
  'use strict';

  function _createForOfIteratorHelper(r, e) { var t = "undefined" != typeof Symbol && r[Symbol.iterator] || r["@@iterator"]; if (!t) { if (Array.isArray(r) || (t = _unsupportedIterableToArray(r)) || e) { t && (r = t); var _n = 0, F = function F() {}; return { s: F, n: function n() { return _n >= r.length ? { done: true } : { done: false, value: r[_n++] }; }, e: function e(r) { throw r; }, f: F }; } throw new TypeError("Invalid attempt to iterate non-iterable instance.\nIn order to be iterable, non-array objects must have a [Symbol.iterator]() method."); } var o, a = true, u = false; return { s: function s() { t = t.call(r); }, n: function n() { var r = t.next(); return a = r.done, r; }, e: function e(r) { u = true, o = r; }, f: function f() { try { a || null == t.return || t.return(); } finally { if (u) throw o; } } }; }
  function _unsupportedIterableToArray(r, a) { if (r) { if ("string" == typeof r) return _arrayLikeToArray(r, a); var t = {}.toString.call(r).slice(8, -1); return "Object" === t && r.constructor && (t = r.constructor.name), "Map" === t || "Set" === t ? Array.from(r) : "Arguments" === t || /^(?:Ui|I)nt(?:8|16|32)(?:Clamped)?Array$/.test(t) ? _arrayLikeToArray(r, a) : void 0; } }
  function _arrayLikeToArray(r, a) { (null == a || a > r.length) && (a = r.length); for (var e = 0, n = Array(a); e < a; e++) n[e] = r[e]; return n; }
  var fns = [];
  var _loop = function _loop(i) {
    fns.push(function () {
      return i;
    });
  };
  for (var i = 0; i < 3; i++) {
    _loop(i);
  }
  console.log(fns.map(function (f) {
    return f();
  }).join(","));
  var later = [];
  var _loop2 = function _loop2(_i) {
      if (_i === 1) return 0; // continue
      if (_i === 3) return 1; // break
      later.push(function () {
        return _i * 10;
      });
    },
    _ret;
  for (var _i = 0; _i < 4; _i++) {
    _ret = _loop2(_i);
    if (_ret === 0) continue;
    if (_ret === 1) break;
  }
  console.log(later.map(function (f) {
    return f();
  }).join(","));
  function find(list) {
    var _iterator = _createForOfIteratorHelper(list),
      _step;
    try {
      var _loop3 = function _loop3() {
          var item = _step.value;
          var handler = function handler() {
            return item.id;
          };
          if (item.ok) return {
            v: handler()
          };
        },
        _ret2;
      for (_iterator.s(); !(_step = _iterator.n()).done;) {
        _ret2 = _loop3();
        if (_ret2) return _ret2.v;
      }
    } catch (err) {
      _iterator.e(err);
    } finally {
      _iterator.f();
    }
    return -1;
  }
  console.log(find([{
    id: 1
  }, {
    id: 2,
    ok: true
  }]));
  var x = "outer";
  {
    var _x = "inner";
    var y = _x + "!";
    console.log(_x, y);
  }
  console.log(x);
  var ws = [];
  var j = 0;
  var _loop4 = function _loop4() {
    var k = j;
    ws.push(function () {
      return k;
    });
    j++;
  };
  while (j < 3) {
    _loop4();
  }
  console.log(ws.map(function (f) {
    return f();
  }).join(","));
  var _loop5 = function _loop5(_i2) {
    setTimeout(function () {
      return console.log("timer", _i2);
    }, 0);
  };
  for (var _i2 = 0, n = 2; _i2 < n; _i2++) {
    _loop5(_i2);
  }
  switch (1) {
    case 1:
      {
        var s = "case";
        console.log(s);
      }
  }

})();
