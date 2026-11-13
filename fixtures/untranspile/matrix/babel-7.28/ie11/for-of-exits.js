"use strict";

function _typeof(o) { "@babel/helpers - typeof"; return _typeof = "function" == typeof Symbol && "symbol" == typeof Symbol.iterator ? function (o) { return typeof o; } : function (o) { return o && "function" == typeof Symbol && o.constructor === Symbol && o !== Symbol.prototype ? "symbol" : typeof o; }, _typeof(o); }
function _slicedToArray(r, e) { return _arrayWithHoles(r) || _iterableToArrayLimit(r, e) || _unsupportedIterableToArray(r, e) || _nonIterableRest(); }
function _nonIterableRest() { throw new TypeError("Invalid attempt to destructure non-iterable instance.\nIn order to be iterable, non-array objects must have a [Symbol.iterator]() method."); }
function _iterableToArrayLimit(r, l) { var t = null == r ? null : "undefined" != typeof Symbol && r[Symbol.iterator] || r["@@iterator"]; if (null != t) { var e, n, i, u, a = [], f = !0, o = !1; try { if (i = (t = t.call(r)).next, 0 === l) { if (Object(t) !== t) return; f = !1; } else for (; !(f = (e = i.call(t)).done) && (a.push(e.value), a.length !== l); f = !0); } catch (r) { o = !0, n = r; } finally { try { if (!f && null != t.return && (u = t.return(), Object(u) !== u)) return; } finally { if (o) throw n; } } return a; } }
function _arrayWithHoles(r) { if (Array.isArray(r)) return r; }
function _toConsumableArray(r) { return _arrayWithoutHoles(r) || _iterableToArray(r) || _unsupportedIterableToArray(r) || _nonIterableSpread(); }
function _nonIterableSpread() { throw new TypeError("Invalid attempt to spread non-iterable instance.\nIn order to be iterable, non-array objects must have a [Symbol.iterator]() method."); }
function _iterableToArray(r) { if ("undefined" != typeof Symbol && null != r[Symbol.iterator] || null != r["@@iterator"]) return Array.from(r); }
function _arrayWithoutHoles(r) { if (Array.isArray(r)) return _arrayLikeToArray(r); }
function _regenerator() { /*! regenerator-runtime -- Copyright (c) 2014-present, Facebook, Inc. -- license (MIT): https://github.com/babel/babel/blob/main/packages/babel-helpers/LICENSE */ var e, t, r = "function" == typeof Symbol ? Symbol : {}, n = r.iterator || "@@iterator", o = r.toStringTag || "@@toStringTag"; function i(r, n, o, i) { var c = n && n.prototype instanceof Generator ? n : Generator, u = Object.create(c.prototype); return _regeneratorDefine2(u, "_invoke", function (r, n, o) { var i, c, u, f = 0, p = o || [], y = !1, G = { p: 0, n: 0, v: e, a: d, f: d.bind(e, 4), d: function d(t, r) { return i = t, c = 0, u = e, G.n = r, a; } }; function d(r, n) { for (c = r, u = n, t = 0; !y && f && !o && t < p.length; t++) { var o, i = p[t], d = G.p, l = i[2]; r > 3 ? (o = l === n) && (u = i[(c = i[4]) ? 5 : (c = 3, 3)], i[4] = i[5] = e) : i[0] <= d && ((o = r < 2 && d < i[1]) ? (c = 0, G.v = n, G.n = i[1]) : d < l && (o = r < 3 || i[0] > n || n > l) && (i[4] = r, i[5] = n, G.n = l, c = 0)); } if (o || r > 1) return a; throw y = !0, n; } return function (o, p, l) { if (f > 1) throw TypeError("Generator is already running"); for (y && 1 === p && d(p, l), c = p, u = l; (t = c < 2 ? e : u) || !y;) { i || (c ? c < 3 ? (c > 1 && (G.n = -1), d(c, u)) : G.n = u : G.v = u); try { if (f = 2, i) { if (c || (o = "next"), t = i[o]) { if (!(t = t.call(i, u))) throw TypeError("iterator result is not an object"); if (!t.done) return t; u = t.value, c < 2 && (c = 0); } else 1 === c && (t = i.return) && t.call(i), c < 2 && (u = TypeError("The iterator does not provide a '" + o + "' method"), c = 1); i = e; } else if ((t = (y = G.n < 0) ? u : r.call(n, G)) !== a) break; } catch (t) { i = e, c = 1, u = t; } finally { f = 1; } } return { value: t, done: y }; }; }(r, o, i), !0), u; } var a = {}; function Generator() {} function GeneratorFunction() {} function GeneratorFunctionPrototype() {} t = Object.getPrototypeOf; var c = [][n] ? t(t([][n]())) : (_regeneratorDefine2(t = {}, n, function () { return this; }), t), u = GeneratorFunctionPrototype.prototype = Generator.prototype = Object.create(c); function f(e) { return Object.setPrototypeOf ? Object.setPrototypeOf(e, GeneratorFunctionPrototype) : (e.__proto__ = GeneratorFunctionPrototype, _regeneratorDefine2(e, o, "GeneratorFunction")), e.prototype = Object.create(u), e; } return GeneratorFunction.prototype = GeneratorFunctionPrototype, _regeneratorDefine2(u, "constructor", GeneratorFunctionPrototype), _regeneratorDefine2(GeneratorFunctionPrototype, "constructor", GeneratorFunction), GeneratorFunction.displayName = "GeneratorFunction", _regeneratorDefine2(GeneratorFunctionPrototype, o, "GeneratorFunction"), _regeneratorDefine2(u), _regeneratorDefine2(u, o, "Generator"), _regeneratorDefine2(u, n, function () { return this; }), _regeneratorDefine2(u, "toString", function () { return "[object Generator]"; }), (_regenerator = function _regenerator() { return { w: i, m: f }; })(); }
function _regeneratorDefine2(e, r, n, t) { var i = Object.defineProperty; try { i({}, "", {}); } catch (e) { i = 0; } _regeneratorDefine2 = function _regeneratorDefine(e, r, n, t) { function o(r, n) { _regeneratorDefine2(e, r, function (e) { return this._invoke(r, n, e); }); } r ? i ? i(e, r, { value: n, enumerable: !t, configurable: !t, writable: !t }) : e[r] = n : (o("next", 0), o("throw", 1), o("return", 2)); }, _regeneratorDefine2(e, r, n, t); }
var _marked = /*#__PURE__*/_regenerator().m(gen);
function _createForOfIteratorHelper(r, e) { var t = "undefined" != typeof Symbol && r[Symbol.iterator] || r["@@iterator"]; if (!t) { if (Array.isArray(r) || (t = _unsupportedIterableToArray(r)) || e && r && "number" == typeof r.length) { t && (r = t); var _n = 0, F = function F() {}; return { s: F, n: function n() { return _n >= r.length ? { done: !0 } : { done: !1, value: r[_n++] }; }, e: function e(r) { throw r; }, f: F }; } throw new TypeError("Invalid attempt to iterate non-iterable instance.\nIn order to be iterable, non-array objects must have a [Symbol.iterator]() method."); } var o, a = !0, u = !1; return { s: function s() { t = t.call(r); }, n: function n() { var r = t.next(); return a = r.done, r; }, e: function e(r) { u = !0, o = r; }, f: function f() { try { a || null == t.return || t.return(); } finally { if (u) throw o; } } }; }
function _unsupportedIterableToArray(r, a) { if (r) { if ("string" == typeof r) return _arrayLikeToArray(r, a); var t = {}.toString.call(r).slice(8, -1); return "Object" === t && r.constructor && (t = r.constructor.name), "Map" === t || "Set" === t ? Array.from(r) : "Arguments" === t || /^(?:Ui|I)nt(?:8|16|32)(?:Clamped)?Array$/.test(t) ? _arrayLikeToArray(r, a) : void 0; } }
function _arrayLikeToArray(r, a) { (null == a || a > r.length) && (a = r.length); for (var e = 0, n = Array(a); e < a; e++) n[e] = r[e]; return n; }
function _defineProperty(e, r, t) { return (r = _toPropertyKey(r)) in e ? Object.defineProperty(e, r, { value: t, enumerable: !0, configurable: !0, writable: !0 }) : e[r] = t, e; }
function _toPropertyKey(t) { var i = _toPrimitive(t, "string"); return "symbol" == _typeof(i) ? i : i + ""; }
function _toPrimitive(t, r) { if ("object" != _typeof(t) || !t) return t; var e = t[Symbol.toPrimitive]; if (void 0 !== e) { var i = e.call(t, r || "default"); if ("object" != _typeof(i)) return i; throw new TypeError("@@toPrimitive must return a primitive value."); } return ("string" === r ? String : Number)(t); }
var log = [];
function counted(name, items) {
  return _defineProperty({}, Symbol.iterator, function () {
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
    var _step8$value = _slicedToArray(_step8.value, 2),
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
  return _regenerator().w(function (_context) {
    while (1) switch (_context.p = _context.n) {
      case 0:
        _context.p = 0;
        _context.n = 1;
        return 1;
      case 1:
        _context.n = 2;
        return 2;
      case 2:
        _context.p = 2;
        log.push("gen:finally");
        return _context.f(2);
      case 3:
        return _context.a(2);
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
var spreadSet = _toConsumableArray(new Set([4, 4, 5]));
var _Map$keys = new Map([["k", 1]]).keys(),
  _Map$keys2 = _slicedToArray(_Map$keys, 1),
  firstKey = _Map$keys2[0];
log.push(spreadSet.join("") + firstKey);
console.log(log.join("\n"));
