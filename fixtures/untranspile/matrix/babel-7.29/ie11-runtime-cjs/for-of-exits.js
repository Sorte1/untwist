(function () {
  'use strict';

  var _regeneratorRuntime2 = require("@babel/runtime/regenerator");
  var _interopRequireDefault = require("@babel/runtime/helpers/interopRequireDefault");
  var _regenerator = _interopRequireDefault(require("@babel/runtime/regenerator"));
  var _slicedToArray2 = _interopRequireDefault(require("@babel/runtime/helpers/slicedToArray"));
  var _toConsumableArray2 = _interopRequireDefault(require("@babel/runtime/helpers/toConsumableArray"));
  var _defineProperty2 = _interopRequireDefault(require("@babel/runtime/helpers/defineProperty"));
  var _marked = /*#__PURE__*/_regeneratorRuntime2.mark(gen);
  function _createForOfIteratorHelper(r, e) { var t = "undefined" != typeof Symbol && r[Symbol.iterator] || r["@@iterator"]; if (!t) { if (Array.isArray(r) || (t = _unsupportedIterableToArray(r)) || e) { t && (r = t); var _n = 0, F = function F() {}; return { s: F, n: function n() { return _n >= r.length ? { done: true } : { done: false, value: r[_n++] }; }, e: function e(r) { throw r; }, f: F }; } throw new TypeError("Invalid attempt to iterate non-iterable instance.\nIn order to be iterable, non-array objects must have a [Symbol.iterator]() method."); } var o, a = true, u = false; return { s: function s() { t = t.call(r); }, n: function n() { var r = t.next(); return a = r.done, r; }, e: function e(r) { u = true, o = r; }, f: function f() { try { a || null == t.return || t.return(); } finally { if (u) throw o; } } }; }
  function _unsupportedIterableToArray(r, a) { if (r) { if ("string" == typeof r) return _arrayLikeToArray(r, a); var t = {}.toString.call(r).slice(8, -1); return "Object" === t && r.constructor && (t = r.constructor.name), "Map" === t || "Set" === t ? Array.from(r) : "Arguments" === t || /^(?:Ui|I)nt(?:8|16|32)(?:Clamped)?Array$/.test(t) ? _arrayLikeToArray(r, a) : void 0; } }
  function _arrayLikeToArray(r, a) { (null == a || a > r.length) && (a = r.length); for (var e = 0, n = Array(a); e < a; e++) n[e] = r[e]; return n; }
  var log = [];
  function counted(name, items) {
    return (0, _defineProperty2.default)({}, Symbol.iterator, function () {
      var at = 0;
      log.push(name + ":open");
      return {
        next: function next() {
          log.push(name + ":next");
          return at < items.length ? {
            value: items[at++],
            done: false
          } : {
            value: undefined,
            done: true
          };
        },
        return: function _return() {
          log.push(name + ":return");
          return {
            done: true
          };
        }
      };
    });
  }
  var _iterator = _createForOfIteratorHelper(counted("all", [1, 2])),
    _step;
  try {
    for (_iterator.s(); !(_step = _iterator.n()).done;) {
      var _v2 = _step.value;
      log.push("all=" + _v2);
    }
  } catch (err) {
    _iterator.e(err);
  } finally {
    _iterator.f();
  }
  var _iterator2 = _createForOfIteratorHelper(counted("brk", [1, 2, 3])),
    _step2;
  try {
    for (_iterator2.s(); !(_step2 = _iterator2.n()).done;) {
      var _v3 = _step2.value;
      if (_v3 === 2) break;
      log.push("brk=" + _v3);
    }
  } catch (err) {
    _iterator2.e(err);
  } finally {
    _iterator2.f();
  }
  function early() {
    var _iterator3 = _createForOfIteratorHelper(counted("ret", [5, 6, 7])),
      _step3;
    try {
      for (_iterator3.s(); !(_step3 = _iterator3.n()).done;) {
        var v = _step3.value;
        if (v === 6) return v;
      }
    } catch (err) {
      _iterator3.e(err);
    } finally {
      _iterator3.f();
    }
    return -1;
  }
  log.push("early=" + early());
  try {
    var _iterator4 = _createForOfIteratorHelper(counted("thr", [1, 2])),
      _step4;
    try {
      for (_iterator4.s(); !(_step4 = _iterator4.n()).done;) {
        var v = _step4.value;
        if (v === 2) throw new Error("boom " + v);
      }
    } catch (err) {
      _iterator4.e(err);
    } finally {
      _iterator4.f();
    }
  } catch (err) {
    log.push("caught " + err.message);
  }
  var skipped = 0;
  var _iterator5 = _createForOfIteratorHelper(counted("cont", [1, 2, 3])),
    _step5;
  try {
    for (_iterator5.s(); !(_step5 = _iterator5.n()).done;) {
      var _v4 = _step5.value;
      if (_v4 === 2) continue;
      skipped += _v4;
    }
  } catch (err) {
    _iterator5.e(err);
  } finally {
    _iterator5.f();
  }
  log.push("cont=" + skipped);
  var _iterator6 = _createForOfIteratorHelper(counted("outer", [1, 2])),
    _step6;
  try {
    outer: for (_iterator6.s(); !(_step6 = _iterator6.n()).done;) {
      var a = _step6.value;
      var _iterator1 = _createForOfIteratorHelper(counted("inner", [10, 20])),
        _step1;
      try {
        for (_iterator1.s(); !(_step1 = _iterator1.n()).done;) {
          var b = _step1.value;
          if (b === 20) continue outer;
          log.push(a + b);
        }
      } catch (err) {
        _iterator1.e(err);
      } finally {
        _iterator1.f();
      }
    }
  } catch (err) {
    _iterator6.e(err);
  } finally {
    _iterator6.f();
  }
  var last;
  var _iterator7 = _createForOfIteratorHelper(counted("assign", [7, 8])),
    _step7;
  try {
    for (_iterator7.s(); !(_step7 = _iterator7.n()).done;) {
      last = _step7.value;
      log.push("seen " + last);
    }
  } catch (err) {
    _iterator7.e(err);
  } finally {
    _iterator7.f();
  }
  log.push("last=" + last);
  var pairs = new Map([["x", 1], ["y", 2]]);
  var _iterator8 = _createForOfIteratorHelper(pairs),
    _step8;
  try {
    for (_iterator8.s(); !(_step8 = _iterator8.n()).done;) {
      var _step8$value = (0, _slicedToArray2.default)(_step8.value, 2),
        key = _step8$value[0],
        value = _step8$value[1];
      log.push(key + value);
    }
  } catch (err) {
    _iterator8.e(err);
  } finally {
    _iterator8.f();
  }
  var fns = [];
  var _loop = function _loop() {
    var v = _arr[_i];
    fns.push(function () {
      return v;
    });
  };
  for (var _i = 0, _arr = [1, 2, 3]; _i < _arr.length; _i++) {
    _loop();
  }
  log.push(fns.map(function (f) {
    return f();
  }).join(""));
  function gen() {
    return _regenerator.default.wrap(function (_context) {
      while (1) switch (_context.prev = _context.next) {
        case 0:
          _context.prev = 0;
          _context.next = 1;
          return 1;
        case 1:
          _context.next = 2;
          return 2;
        case 2:
          _context.prev = 2;
          log.push("gen:finally");
          return _context.finish(2);
        case 3:
        case "end":
          return _context.stop();
      }
    }, _marked, null, [[0,, 2, 3]]);
  }
  var _iterator9 = _createForOfIteratorHelper(gen()),
    _step9;
  try {
    for (_iterator9.s(); !(_step9 = _iterator9.n()).done;) {
      var _v5 = _step9.value;
      if (_v5 === 1) break;
    }
  } catch (err) {
    _iterator9.e(err);
  } finally {
    _iterator9.f();
  }
  try {
    var _iterator0 = _createForOfIteratorHelper(counted("bad", [1])),
      _step0;
    try {
      for (_iterator0.s(); !(_step0 = _iterator0.n()).done;) {
        var _v = _step0.value;
        log.push(_v.missing.field);
      }
    } catch (err) {
      _iterator0.e(err);
    } finally {
      _iterator0.f();
    }
  } catch (err) {
    log.push(err.constructor.name);
  }
  var spreadSet = (0, _toConsumableArray2.default)(new Set([4, 4, 5]));
  var _Map$keys = new Map([["k", 1]]).keys(),
    _Map$keys2 = (0, _slicedToArray2.default)(_Map$keys, 1),
    firstKey = _Map$keys2[0];
  log.push(spreadSet.join("") + firstKey);
  console.log(log.join("\n"));

})();
