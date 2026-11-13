"use strict";

function _regenerator() { /*! regenerator-runtime -- Copyright (c) 2014-present, Facebook, Inc. -- license (MIT): https://github.com/babel/babel/blob/main/packages/babel-helpers/LICENSE */ var e, t, r = "function" == typeof Symbol ? Symbol : {}, n = r.iterator || "@@iterator", o = r.toStringTag || "@@toStringTag"; function i(r, n, o, i) { var c = n && n.prototype instanceof Generator ? n : Generator, u = Object.create(c.prototype); return _regeneratorDefine2(u, "_invoke", function (r, n, o) { var i, c, u, f = 0, p = o || [], y = !1, G = { p: 0, n: 0, v: e, a: d, f: d.bind(e, 4), d: function d(t, r) { return i = t, c = 0, u = e, G.n = r, a; } }; function d(r, n) { for (c = r, u = n, t = 0; !y && f && !o && t < p.length; t++) { var o, i = p[t], d = G.p, l = i[2]; r > 3 ? (o = l === n) && (u = i[(c = i[4]) ? 5 : (c = 3, 3)], i[4] = i[5] = e) : i[0] <= d && ((o = r < 2 && d < i[1]) ? (c = 0, G.v = n, G.n = i[1]) : d < l && (o = r < 3 || i[0] > n || n > l) && (i[4] = r, i[5] = n, G.n = l, c = 0)); } if (o || r > 1) return a; throw y = !0, n; } return function (o, p, l) { if (f > 1) throw TypeError("Generator is already running"); for (y && 1 === p && d(p, l), c = p, u = l; (t = c < 2 ? e : u) || !y;) { i || (c ? (c > 1 && (G.n = c < 3 ? -1 : u), d(c, u)) : G.v = u); try { if (f = 2, i) { if (c || (o = "next"), t = i[o]) { if (!(t = t.call(i, u))) throw TypeError("iterator result is not an object"); if (!t.done) return t; u = t.value, c < 2 && (c = 0); } else 1 === c && (t = i.return) && t.call(i), c < 2 && (u = TypeError("The iterator does not provide a '" + o + "' method"), c = 1); i = e; } else if ((t = (y = G.n < 0) ? u : r.call(n, G)) !== a) break; } catch (t) { i = e, c = 1, u = t; } finally { f = 1; } } return { value: t, done: y }; }; }(r, o, i), !0), u; } var a = {}; function Generator() {} function GeneratorFunction() {} function GeneratorFunctionPrototype() {} t = Object.getPrototypeOf; var c = [][n] ? t(t([][n]())) : (_regeneratorDefine2(t = {}, n, function () { return this; }), t), u = GeneratorFunctionPrototype.prototype = Generator.prototype = Object.create(c); function f(e) { return Object.setPrototypeOf ? Object.setPrototypeOf(e, GeneratorFunctionPrototype) : (e.__proto__ = GeneratorFunctionPrototype, _regeneratorDefine2(e, o, "GeneratorFunction")), e.prototype = Object.create(u), e; } return GeneratorFunction.prototype = GeneratorFunctionPrototype, _regeneratorDefine2(u, "constructor", GeneratorFunctionPrototype), _regeneratorDefine2(GeneratorFunctionPrototype, "constructor", GeneratorFunction), GeneratorFunction.displayName = "GeneratorFunction", _regeneratorDefine2(GeneratorFunctionPrototype, o, "GeneratorFunction"), _regeneratorDefine2(u), _regeneratorDefine2(u, o, "Generator"), _regeneratorDefine2(u, n, function () { return this; }), _regeneratorDefine2(u, "toString", function () { return "[object Generator]"; }), (_regenerator = function _regenerator() { return { w: i, m: f }; })(); }
function _regeneratorDefine2(e, r, n, t) { var i = Object.defineProperty; try { i({}, "", {}); } catch (e) { i = 0; } _regeneratorDefine2 = function _regeneratorDefine(e, r, n, t) { function o(r, n) { _regeneratorDefine2(e, r, function (e) { return this._invoke(r, n, e); }); } r ? i ? i(e, r, { value: n, enumerable: !t, configurable: !t, writable: !t }) : e[r] = n : (o("next", 0), o("throw", 1), o("return", 2)); }, _regeneratorDefine2(e, r, n, t); }
var _marked = /*#__PURE__*/_regenerator().m(gen);
function _createForOfIteratorHelperLoose(r, e) { var t = "undefined" != typeof Symbol && r[Symbol.iterator] || r["@@iterator"]; if (t) return (t = t.call(r)).next.bind(t); if (Array.isArray(r) || (t = _unsupportedIterableToArray(r)) || e && r && "number" == typeof r.length) { t && (r = t); var o = 0; return function () { return o >= r.length ? { done: !0 } : { done: !1, value: r[o++] }; }; } throw new TypeError("Invalid attempt to iterate non-iterable instance.\nIn order to be iterable, non-array objects must have a [Symbol.iterator]() method."); }
function _unsupportedIterableToArray(r, a) { if (r) { if ("string" == typeof r) return _arrayLikeToArray(r, a); var t = {}.toString.call(r).slice(8, -1); return "Object" === t && r.constructor && (t = r.constructor.name), "Map" === t || "Set" === t ? Array.from(r) : "Arguments" === t || /^(?:Ui|I)nt(?:8|16|32)(?:Clamped)?Array$/.test(t) ? _arrayLikeToArray(r, a) : void 0; } }
function _arrayLikeToArray(r, a) { (null == a || a > r.length) && (a = r.length); for (var e = 0, n = Array(a); e < a; e++) n[e] = r[e]; return n; }
var log = [];
function counted(name, items) {
  var _ref;
  return _ref = {}, _ref[Symbol.iterator] = function () {
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
  }, _ref;
}
for (var _iterator = _createForOfIteratorHelperLoose(counted("all", [1, 2])), _step; !(_step = _iterator()).done;) {
  var _v2 = _step.value;
  log.push("all=" + _v2);
}
for (var _iterator2 = _createForOfIteratorHelperLoose(counted("brk", [1, 2, 3])), _step2; !(_step2 = _iterator2()).done;) {
  var _v3 = _step2.value;
  if (_v3 === 2) break;
  log.push("brk=" + _v3);
}
function early() {
  for (var _iterator3 = _createForOfIteratorHelperLoose(counted("ret", [5, 6, 7])), _step3; !(_step3 = _iterator3()).done;) {
    var v = _step3.value;
    if (v === 6) return v;
  }
  return -1;
}
log.push("early=" + early());
try {
  for (var _iterator4 = _createForOfIteratorHelperLoose(counted("thr", [1, 2])), _step4; !(_step4 = _iterator4()).done;) {
    var v = _step4.value;
    if (v === 2) throw new Error("boom " + v);
  }
} catch (err) {
  log.push("caught " + err.message);
}
var skipped = 0;
for (var _iterator5 = _createForOfIteratorHelperLoose(counted("cont", [1, 2, 3])), _step5; !(_step5 = _iterator5()).done;) {
  var _v4 = _step5.value;
  if (_v4 === 2) continue;
  skipped += _v4;
}
log.push("cont=" + skipped);
outer: for (var _iterator6 = _createForOfIteratorHelperLoose(counted("outer", [1, 2])), _step6; !(_step6 = _iterator6()).done;) {
  var a = _step6.value;
  for (var _iterator1 = _createForOfIteratorHelperLoose(counted("inner", [10, 20])), _step1; !(_step1 = _iterator1()).done;) {
    var b = _step1.value;
    if (b === 20) continue outer;
    log.push(a + b);
  }
}
var last;
for (var _iterator7 = _createForOfIteratorHelperLoose(counted("assign", [7, 8])), _step7; !(_step7 = _iterator7()).done;) {
  last = _step7.value;
  log.push("seen " + last);
}
log.push("last=" + last);
var pairs = new Map([["x", 1], ["y", 2]]);
for (var _iterator8 = _createForOfIteratorHelperLoose(pairs), _step8; !(_step8 = _iterator8()).done;) {
  var _step8$value = _step8.value,
    key = _step8$value[0],
    value = _step8$value[1];
  log.push(key + value);
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
for (var _iterator9 = _createForOfIteratorHelperLoose(gen()), _step9; !(_step9 = _iterator9()).done;) {
  var _v5 = _step9.value;
  if (_v5 === 1) break;
}
try {
  for (var _iterator0 = _createForOfIteratorHelperLoose(counted("bad", [1])), _step0; !(_step0 = _iterator0()).done;) {
    var _v = _step0.value;
    log.push(_v.missing.field);
  }
} catch (err) {
  log.push(err.constructor.name);
}
var spreadSet = [].concat(new Set([4, 4, 5]));
var _Map$keys = new Map([["k", 1]]).keys(),
  firstKey = _Map$keys[0];
log.push(spreadSet.join("") + firstKey);
console.log(log.join("\n"));
