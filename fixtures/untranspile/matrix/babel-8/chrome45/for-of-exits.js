function _slicedToArray(e, i) { return _iterableToArrayLimit(e, i) || _unsupportedIterableToArray(e, i) || _nonIterableRest(); }
function _nonIterableRest() { throw new TypeError("Invalid attempt to destructure non-iterable instance.\nIn order to be iterable, non-array objects must have a [Symbol.iterator]() method."); }
function _iterableToArrayLimit(r, t) { var e = null == r ? null : "undefined" != typeof Symbol && r[Symbol.iterator] || r["@@iterator"]; if (Array.isArray(r) && (null == e || e === ("undefined" != typeof Symbol && [][Symbol.iterator]) || e === []["@@iterator"])) return r; if (null != e) { var l, n, i, a, u = [], o = !0, f = !1; try { if (i = (e = e.call(r)).next, 0 === t) { if (Object(e) !== e) return; o = !1; } else for (; !(o = (l = i.call(e)).done) && (u.push(l.value), u.length !== t); o = !0); } catch (r) { f = !0, n = r; } finally { try { if (!o && null != e.return && (a = e.return(), Object(a) !== a)) return; } finally { if (f) throw n; } } return u; } }
function _toConsumableArray(a) { return _arrayWithoutHoles(a) || _iterableToArray(a) || _unsupportedIterableToArray(a) || _nonIterableSpread(); }
function _nonIterableSpread() { throw new TypeError("Invalid attempt to spread non-iterable instance.\nIn order to be iterable, non-array objects must have a [Symbol.iterator]() method."); }
function _iterableToArray(r) { var e = "undefined" != typeof Symbol && r[Symbol.iterator] || r["@@iterator"]; if (Array.isArray(r) && (null == e || e === ("undefined" != typeof Symbol && [][Symbol.iterator]) || e === []["@@iterator"])) return r; if (null != e) { for (var t, o = [], a = e.call(r); !(t = a.next()).done;) o.push(t.value); return o; } }
function _arrayWithoutHoles(a) { if (Array.isArray(a)) return _arrayLikeToArray(a); }
function _regenerator() { /*! regenerator-runtime -- Copyright (c) 2014-present, Facebook, Inc. -- license (MIT): https://github.com/babel/babel/blob/main/packages/babel-helpers/LICENSE */ var r, e, o = "function" == typeof Symbol ? Symbol : {}, n = o.iterator || "@@iterator", a = o.toStringTag || "@@toStringTag"; function i(o, n, a, i) { var u = n && n.prototype instanceof Generator ? n : Generator, f = Object.create(u.prototype); return _regeneratorDefine2(f, "_invoke", function (t, o, n) { var a, i, u, f = 0, p = n || [], y = !1, G = { p: 0, n: 0, v: r, a: l, f: l.bind(r, 4), d: function d(t, e) { return a = t, i = 0, u = r, G.n = e, c; } }; function l(t, o) { for (i = t, u = o, e = 0; !y && f && !n && e < p.length; e++) { var n, a = p[e], l = G.p, F = a[2]; t > 3 ? (n = F === o) && (u = a[(i = a[4]) ? 5 : (i = 3, 3)], a[4] = a[5] = r) : a[0] <= l && ((n = t < 2 && l < a[1]) ? (i = 0, G.v = o, G.n = a[1]) : l < F && (n = t < 3 || a[0] > o || o > F) && (a[4] = t, a[5] = o, G.n = F, i = 0)); } if (n || t > 1) return c; throw y = !0, o; } return function (n, p, F) { if (f > 1) throw TypeError("Generator is already running"); for (y && 1 === p && l(p, F), i = p, u = F; (e = i < 2 ? r : u) || !y;) { a || (i ? (i > 1 && (G.n = i < 3 ? -1 : u), l(i, u)) : G.v = u); try { if (f = 2, a) { if (i || (n = "next"), e = a[n]) { if (!(e = e.call(a, u))) throw TypeError("iterator result is not an object"); if (!e.done) return e; u = e.value, i < 2 && (i = 0); } else 1 === i && (e = a.return) && e.call(a), i < 2 && (u = TypeError("The iterator does not provide a '" + n + "' method"), i = 1); a = r; } else if ((e = (y = G.n < 0) ? u : t.call(o, G)) !== c) break; } catch (t) { a = r, i = 1, u = t; } finally { f = 1; } } return { value: e, done: y }; }; }(o, a, i), !0), f; } var c = {}; function Generator() {} function GeneratorFunction() {} function GeneratorFunctionPrototype() {} e = Object.getPrototypeOf; var u = [][n] ? e(e([][n]())) : (_regeneratorDefine2(e = {}, n, function () { return this; }), e), f = GeneratorFunctionPrototype.prototype = Generator.prototype = Object.create(u); function p(r) { return Object.setPrototypeOf ? Object.setPrototypeOf(r, GeneratorFunctionPrototype) : (r.__proto__ = GeneratorFunctionPrototype, _regeneratorDefine2(r, a, "GeneratorFunction")), r.prototype = Object.create(f), r; } return GeneratorFunction.prototype = GeneratorFunctionPrototype, _regeneratorDefine2(f, "constructor", GeneratorFunctionPrototype), _regeneratorDefine2(GeneratorFunctionPrototype, "constructor", GeneratorFunction), GeneratorFunction.displayName = "GeneratorFunction", _regeneratorDefine2(GeneratorFunctionPrototype, a, "GeneratorFunction"), _regeneratorDefine2(f), _regeneratorDefine2(f, a, "Generator"), _regeneratorDefine2(f, n, function () { return this; }), _regeneratorDefine2(f, "toString", function () { return "[object Generator]"; }), (_regenerator = function _regenerator() { return { w: i, m: p }; })(); }
function _regeneratorDefine2(e, r, n, t) { var i = Object.defineProperty; try { i({}, "", {}); } catch (e) { i = 0; } _regeneratorDefine2 = function _regeneratorDefine(e, r, n, t) { function o(r, n) { _regeneratorDefine2(e, r, function (e) { return this._invoke(r, n, e); }); } r ? i ? i(e, r, { value: n, enumerable: !t, configurable: !t, writable: !t }) : e[r] = n : (o("next", 0), o("throw", 1), o("return", 2)); }, _regeneratorDefine2(e, r, n, t); }
var _marked = /*#__PURE__*/_regenerator().m(gen);
function _createForOfIteratorHelper(e, t) { var _n = "undefined" != typeof Symbol && e[Symbol.iterator] || e["@@iterator"]; if (!_n) { if (Array.isArray(e) || (_n = _unsupportedIterableToArray(e)) || t && e && "number" == typeof e.length) { _n && (e = _n); var o = 0, F = function F() {}; return { s: F, n: function n() { return o >= e.length ? { done: !0 } : { done: !1, value: e[o++] }; }, e: function e(r) { throw r; }, f: F }; } throw new TypeError("Invalid attempt to iterate non-iterable instance.\nIn order to be iterable, non-array objects must have a [Symbol.iterator]() method."); } var a, u, i = !0; return { s: function s() { _n = _n.call(e); }, n: function n() { i = !0; var r = _n.next(); return r.done ? { done: !0 } : { value: r.value, done: i = !1 }; }, e: function e(r) { a = !0, u = r; }, f: function f() { try { i || null == _n.return || _n.return(); } finally { if (a) throw u; } } }; }
function _unsupportedIterableToArray(t, e) { if (t) { if ("string" == typeof t) return _arrayLikeToArray(t, e); var o = {}.toString.call(t).slice(8, -1); return "Object" === o && t.constructor && (o = t.constructor.name), "Map" === o || "Set" === o ? Array.from(t) : "Arguments" === o || /^(?:Ui|I)nt(?:8|16|32)(?:Clamped)?Array$/.test(o) ? _arrayLikeToArray(t, e) : void 0; } }
function _arrayLikeToArray(r, a) { (null == a || a > r.length) && (a = r.length); for (var e = 0, n = Array(a); e < a; e++) n[e] = r[e]; return n; }
var log = [];
function counted(name, items) {
  return {
    [Symbol.iterator]() {
      var at = 0;
      log.push(name + ":open");
      return {
        next() {
          log.push(name + ":next");
          return at < items.length ? {
            value: items[at++],
            done: false
          } : {
            value: undefined,
            done: true
          };
        },
        return() {
          log.push(name + ":return");
          return {
            done: true
          };
        }
      };
    }
  };
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
