function asyncGeneratorStep(n, t, e, r, o, a, c) { try { var i = n[a](c), u = i.value; } catch (n) { return void e(n); } i.done ? t(u) : Promise.resolve(u).then(r, o); }
function _asyncToGenerator(n) { return function () { var t = this, e = arguments; return new Promise(function (r, o) { var a = n.apply(t, e); function _next(n) { asyncGeneratorStep(a, r, o, _next, _throw, "next", n); } function _throw(n) { asyncGeneratorStep(a, r, o, _next, _throw, "throw", n); } _next(void 0); }); }; }
function _regenerator() { /*! regenerator-runtime -- Copyright (c) 2014-present, Facebook, Inc. -- license (MIT): https://github.com/babel/babel/blob/main/packages/babel-helpers/LICENSE */ var e, t, r = "function" == typeof Symbol ? Symbol : {}, n = r.iterator || "@@iterator", o = r.toStringTag || "@@toStringTag"; function i(r, n, o, i) { var c = n && n.prototype instanceof Generator ? n : Generator, u = Object.create(c.prototype); return _regeneratorDefine2(u, "_invoke", function (r, n, o) { var i, c, u, f = 0, p = o || [], y = !1, G = { p: 0, n: 0, v: e, a: d, f: d.bind(e, 4), d: function d(t, r) { return i = t, c = 0, u = e, G.n = r, a; } }; function d(r, n) { for (c = r, u = n, t = 0; !y && f && !o && t < p.length; t++) { var o, i = p[t], d = G.p, l = i[2]; r > 3 ? (o = l === n) && (u = i[(c = i[4]) ? 5 : (c = 3, 3)], i[4] = i[5] = e) : i[0] <= d && ((o = r < 2 && d < i[1]) ? (c = 0, G.v = n, G.n = i[1]) : d < l && (o = r < 3 || i[0] > n || n > l) && (i[4] = r, i[5] = n, G.n = l, c = 0)); } if (o || r > 1) return a; throw y = !0, n; } return function (o, p, l) { if (f > 1) throw TypeError("Generator is already running"); for (y && 1 === p && d(p, l), c = p, u = l; (t = c < 2 ? e : u) || !y;) { i || (c ? c < 3 ? (c > 1 && (G.n = -1), d(c, u)) : G.n = u : G.v = u); try { if (f = 2, i) { if (c || (o = "next"), t = i[o]) { if (!(t = t.call(i, u))) throw TypeError("iterator result is not an object"); if (!t.done) return t; u = t.value, c < 2 && (c = 0); } else 1 === c && (t = i.return) && t.call(i), c < 2 && (u = TypeError("The iterator does not provide a '" + o + "' method"), c = 1); i = e; } else if ((t = (y = G.n < 0) ? u : r.call(n, G)) !== a) break; } catch (t) { i = e, c = 1, u = t; } finally { f = 1; } } return { value: t, done: y }; }; }(r, o, i), !0), u; } var a = {}; function Generator() {} function GeneratorFunction() {} function GeneratorFunctionPrototype() {} t = Object.getPrototypeOf; var c = [][n] ? t(t([][n]())) : (_regeneratorDefine2(t = {}, n, function () { return this; }), t), u = GeneratorFunctionPrototype.prototype = Generator.prototype = Object.create(c); function f(e) { return Object.setPrototypeOf ? Object.setPrototypeOf(e, GeneratorFunctionPrototype) : (e.__proto__ = GeneratorFunctionPrototype, _regeneratorDefine2(e, o, "GeneratorFunction")), e.prototype = Object.create(u), e; } return GeneratorFunction.prototype = GeneratorFunctionPrototype, _regeneratorDefine2(u, "constructor", GeneratorFunctionPrototype), _regeneratorDefine2(GeneratorFunctionPrototype, "constructor", GeneratorFunction), GeneratorFunction.displayName = "GeneratorFunction", _regeneratorDefine2(GeneratorFunctionPrototype, o, "GeneratorFunction"), _regeneratorDefine2(u), _regeneratorDefine2(u, o, "Generator"), _regeneratorDefine2(u, n, function () { return this; }), _regeneratorDefine2(u, "toString", function () { return "[object Generator]"; }), (_regenerator = function _regenerator() { return { w: i, m: f }; })(); }
function _regeneratorDefine2(e, r, n, t) { var i = Object.defineProperty; try { i({}, "", {}); } catch (e) { i = 0; } _regeneratorDefine2 = function _regeneratorDefine(e, r, n, t) { function o(r, n) { _regeneratorDefine2(e, r, function (e) { return this._invoke(r, n, e); }); } r ? i ? i(e, r, { value: n, enumerable: !t, configurable: !t, writable: !t }) : e[r] = n : (o("next", 0), o("throw", 1), o("return", 2)); }, _regeneratorDefine2(e, r, n, t); }
function _asyncIterator(r) { var n, t, o, e = 2; for ("undefined" != typeof Symbol && (t = Symbol.asyncIterator, o = Symbol.iterator); e--;) { if (t && null != (n = r[t])) return n.call(r); if (o && null != (n = r[o])) return new AsyncFromSyncIterator(n.call(r)); t = "@@asyncIterator", o = "@@iterator"; } throw new TypeError("Object is not async iterable"); }
function AsyncFromSyncIterator(r) { function AsyncFromSyncIteratorContinuation(r) { if (Object(r) !== r) return Promise.reject(new TypeError(r + " is not an object.")); var n = r.done; return Promise.resolve(r.value).then(function (r) { return { value: r, done: n }; }); } return AsyncFromSyncIterator = function AsyncFromSyncIterator(r) { this.s = r, this.n = r.next; }, AsyncFromSyncIterator.prototype = { s: null, n: null, next: function next() { return AsyncFromSyncIteratorContinuation(this.n.apply(this.s, arguments)); }, return: function _return(r) { var n = this.s.return; return void 0 === n ? Promise.resolve({ value: r, done: !0 }) : AsyncFromSyncIteratorContinuation(n.apply(this.s, arguments)); }, throw: function _throw(r) { var n = this.s.return; return void 0 === n ? Promise.reject(r) : AsyncFromSyncIteratorContinuation(n.apply(this.s, arguments)); } }, new AsyncFromSyncIterator(r); }
function _awaitAsyncGenerator(e) { return new _OverloadYield(e, 0); }
function _wrapAsyncGenerator(e) { return function () { return new AsyncGenerator(e.apply(this, arguments)); }; }
function AsyncGenerator(e) { var t, n; function resume(t, n) { try { var r = e[t](n), o = r.value, u = o instanceof _OverloadYield; Promise.resolve(u ? o.v : o).then(function (n) { if (u) { var i = "return" === t && o.k ? t : "next"; if (!o.k || n.done) return resume(i, n); n = e[i](n).value; } settle(!!r.done, n); }, function (e) { resume("throw", e); }); } catch (e) { settle(2, e); } } function settle(e, r) { 2 === e ? t.reject(r) : t.resolve({ value: r, done: e }), (t = t.next) ? resume(t.key, t.arg) : n = null; } this._invoke = function (e, r) { return new Promise(function (o, u) { var i = { key: e, arg: r, resolve: o, reject: u, next: null }; n ? n = n.next = i : (t = n = i, resume(e, r)); }); }, "function" != typeof e.return && (this.return = void 0); }
AsyncGenerator.prototype["function" == typeof Symbol && Symbol.asyncIterator || "@@asyncIterator"] = function () { return this; }, AsyncGenerator.prototype.next = function (e) { return this._invoke("next", e); }, AsyncGenerator.prototype.throw = function (e) { return this._invoke("throw", e); }, AsyncGenerator.prototype.return = function (e) { return this._invoke("return", e); };
function _OverloadYield(e, d) { this.v = e, this.k = d; }
function ticks(_x) {
  return _ticks.apply(this, arguments);
}
function _ticks() {
  _ticks = _wrapAsyncGenerator(/*#__PURE__*/_regenerator().m(function _callee(n) {
    var i;
    return _regenerator().w(function (_context) {
      while (1) switch (_context.n) {
        case 0:
          i = 0;
        case 1:
          if (!(i < n)) {
            _context.n = 4;
            break;
          }
          _context.n = 2;
          return _awaitAsyncGenerator(null);
        case 2:
          _context.n = 3;
          return i;
        case 3:
          i++;
          _context.n = 1;
          break;
        case 4:
          return _context.a(2);
      }
    }, _callee);
  }));
  return _ticks.apply(this, arguments);
}
function main() {
  return _main.apply(this, arguments);
}
function _main() {
  _main = _asyncToGenerator(/*#__PURE__*/_regenerator().m(function _callee2() {
    var out, _iteratorAbruptCompletion, _didIteratorError, _iteratorError, _iterator, _step, v, it, fromSync, _iteratorAbruptCompletion2, _didIteratorError2, _iteratorError2, _iterator2, _step2, _v, _t, _t2, _t3, _t4, _t5, _t6, _t7, _t8;
    return _regenerator().w(function (_context2) {
      while (1) switch (_context2.p = _context2.n) {
        case 0:
          out = [];
          _iteratorAbruptCompletion = false;
          _didIteratorError = false;
          _context2.p = 1;
          _iterator = _asyncIterator(ticks(3));
        case 2:
          _context2.n = 3;
          return _iterator.next();
        case 3:
          if (!(_iteratorAbruptCompletion = !(_step = _context2.v).done)) {
            _context2.n = 5;
            break;
          }
          v = _step.value;
          out.push(v);
        case 4:
          _iteratorAbruptCompletion = false;
          _context2.n = 2;
          break;
        case 5:
          _context2.n = 7;
          break;
        case 6:
          _context2.p = 6;
          _t = _context2.v;
          _didIteratorError = true;
          _iteratorError = _t;
        case 7:
          _context2.p = 7;
          _context2.p = 8;
          if (!(_iteratorAbruptCompletion && _iterator.return != null)) {
            _context2.n = 9;
            break;
          }
          _context2.n = 9;
          return _iterator.return();
        case 9:
          _context2.p = 9;
          if (!_didIteratorError) {
            _context2.n = 10;
            break;
          }
          throw _iteratorError;
        case 10:
          return _context2.f(9);
        case 11:
          return _context2.f(7);
        case 12:
          console.log(out.join(","));
          it = ticks(2);
          _t2 = console;
          _t3 = JSON;
          _context2.n = 13;
          return it.next();
        case 13:
          _t4 = _t3.stringify.call(_t3, _context2.v);
          _t5 = JSON;
          _context2.n = 14;
          return it.return(9);
        case 14:
          _t6 = _t5.stringify.call(_t5, _context2.v);
          _t7 = JSON;
          _context2.n = 15;
          return it.next();
        case 15:
          _t2.log.call(_t2, _t4, _t6, _t7.stringify.call(_t7, _context2.v));
          fromSync = [];
          _iteratorAbruptCompletion2 = false;
          _didIteratorError2 = false;
          _context2.p = 16;
          _iterator2 = _asyncIterator([Promise.resolve("a"), "b"]);
        case 17:
          _context2.n = 18;
          return _iterator2.next();
        case 18:
          if (!(_iteratorAbruptCompletion2 = !(_step2 = _context2.v).done)) {
            _context2.n = 20;
            break;
          }
          _v = _step2.value;
          fromSync.push(_v);
        case 19:
          _iteratorAbruptCompletion2 = false;
          _context2.n = 17;
          break;
        case 20:
          _context2.n = 22;
          break;
        case 21:
          _context2.p = 21;
          _t8 = _context2.v;
          _didIteratorError2 = true;
          _iteratorError2 = _t8;
        case 22:
          _context2.p = 22;
          _context2.p = 23;
          if (!(_iteratorAbruptCompletion2 && _iterator2.return != null)) {
            _context2.n = 24;
            break;
          }
          _context2.n = 24;
          return _iterator2.return();
        case 24:
          _context2.p = 24;
          if (!_didIteratorError2) {
            _context2.n = 25;
            break;
          }
          throw _iteratorError2;
        case 25:
          return _context2.f(24);
        case 26:
          return _context2.f(22);
        case 27:
          console.log(fromSync.join(""));
        case 28:
          return _context2.a(2);
      }
    }, _callee2, null, [[23,, 24, 26], [16, 21, 22, 27], [8,, 9, 11], [1, 6, 7, 12]]);
  }));
  return _main.apply(this, arguments);
}
main();
