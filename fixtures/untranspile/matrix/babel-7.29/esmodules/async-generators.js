"use strict";

function asyncGeneratorStep(n, t, e, r, o, a, c) { try { var i = n[a](c), u = i.value; } catch (n) { return void e(n); } i.done ? t(u) : Promise.resolve(u).then(r, o); }
function _asyncToGenerator(n) { return function () { var t = this, e = arguments; return new Promise(function (r, o) { var a = n.apply(t, e); function _next(n) { asyncGeneratorStep(a, r, o, _next, _throw, "next", n); } function _throw(n) { asyncGeneratorStep(a, r, o, _next, _throw, "throw", n); } _next(void 0); }); }; }
function _asyncIterator(r) { var n, t, o, e = 2; for ("undefined" != typeof Symbol && (t = Symbol.asyncIterator, o = Symbol.iterator); e--;) { if (t && null != (n = r[t])) return n.call(r); if (o && null != (n = r[o])) return new AsyncFromSyncIterator(n.call(r)); t = "@@asyncIterator", o = "@@iterator"; } throw new TypeError("Object is not async iterable"); }
function AsyncFromSyncIterator(r) { function AsyncFromSyncIteratorContinuation(r) { if (Object(r) !== r) return Promise.reject(new TypeError(r + " is not an object.")); var n = r.done; return Promise.resolve(r.value).then(function (r) { return { value: r, done: n }; }); } return AsyncFromSyncIterator = function AsyncFromSyncIterator(r) { this.s = r, this.n = r.next; }, AsyncFromSyncIterator.prototype = { s: null, n: null, next: function next() { return AsyncFromSyncIteratorContinuation(this.n.apply(this.s, arguments)); }, return: function _return(r) { var n = this.s.return; return void 0 === n ? Promise.resolve({ value: r, done: !0 }) : AsyncFromSyncIteratorContinuation(n.apply(this.s, arguments)); }, throw: function _throw(r) { var n = this.s.throw; return void 0 === n ? Promise.reject(r) : AsyncFromSyncIteratorContinuation(n.apply(this.s, arguments)); } }, new AsyncFromSyncIterator(r); }
function _awaitAsyncGenerator(e) { return new _OverloadYield(e, 0); }
function _wrapAsyncGenerator(e) { return function () { return new AsyncGenerator(e.apply(this, arguments)); }; }
function AsyncGenerator(e) { var t, n; function resume(t, n) { try { var r = e[t](n), o = r.value, u = o instanceof _OverloadYield; Promise.resolve(u ? o.v : o).then(function (n) { if (u) { var i = "return" === t && o.k ? t : "next"; if (!o.k || n.done) return resume(i, n); n = e[i](n).value; } settle(!!r.done, n); }, function (e) { resume("throw", e); }); } catch (e) { settle(2, e); } } function settle(e, r) { 2 === e ? t.reject(r) : t.resolve({ value: r, done: e }), (t = t.next) ? resume(t.key, t.arg) : n = null; } this._invoke = function (e, r) { return new Promise(function (o, u) { var i = { key: e, arg: r, resolve: o, reject: u, next: null }; n ? n = n.next = i : (t = n = i, resume(e, r)); }); }, "function" != typeof e.return && (this.return = void 0); }
AsyncGenerator.prototype["function" == typeof Symbol && Symbol.asyncIterator || "@@asyncIterator"] = function () { return this; }, AsyncGenerator.prototype.next = function (e) { return this._invoke("next", e); }, AsyncGenerator.prototype.throw = function (e) { return this._invoke("throw", e); }, AsyncGenerator.prototype.return = function (e) { return this._invoke("return", e); };
function _OverloadYield(e, d) { this.v = e, this.k = d; }
function ticker(_x) {
  return _ticker.apply(this, arguments);
}
function _ticker() {
  _ticker = _wrapAsyncGenerator(function* (n) {
    for (var i = 0; i < n; i++) {
      yield _awaitAsyncGenerator(null);
      yield i * 10;
    }
  });
  return _ticker.apply(this, arguments);
}
function relay(_x2) {
  return _relay.apply(this, arguments);
}
function _relay() {
  _relay = _wrapAsyncGenerator(function* (src) {
    var _iteratorAbruptCompletion = false;
    var _didIteratorError = false;
    var _iteratorError;
    try {
      for (var _iterator = _asyncIterator(src), _step; _iteratorAbruptCompletion = !(_step = yield _awaitAsyncGenerator(_iterator.next())).done; _iteratorAbruptCompletion = false) {
        var v = _step.value;
        {
          yield v + 1;
        }
      }
    } catch (err) {
      _didIteratorError = true;
      _iteratorError = err;
    } finally {
      try {
        if (_iteratorAbruptCompletion && _iterator.return != null) {
          yield _awaitAsyncGenerator(_iterator.return());
        }
      } finally {
        if (_didIteratorError) {
          throw _iteratorError;
        }
      }
    }
  });
  return _relay.apply(this, arguments);
}
function main() {
  return _main.apply(this, arguments);
}
function _main() {
  _main = _asyncToGenerator(function* () {
    var seen = [];
    var _iteratorAbruptCompletion2 = false;
    var _didIteratorError2 = false;
    var _iteratorError2;
    try {
      for (var _iterator2 = _asyncIterator(relay(ticker(3))), _step2; _iteratorAbruptCompletion2 = !(_step2 = yield _iterator2.next()).done; _iteratorAbruptCompletion2 = false) {
        var v = _step2.value;
        {
          seen.push(v);
        }
      }
    } catch (err) {
      _didIteratorError2 = true;
      _iteratorError2 = err;
    } finally {
      try {
        if (_iteratorAbruptCompletion2 && _iterator2.return != null) {
          yield _iterator2.return();
        }
      } finally {
        if (_didIteratorError2) {
          throw _iteratorError2;
        }
      }
    }
    console.log(seen.join(","));
    var it = ticker(5);
    console.log(JSON.stringify([yield it.next(), yield it.return("x"), yield it.next()]));
    var mixed = [];
    var _iteratorAbruptCompletion3 = false;
    var _didIteratorError3 = false;
    var _iteratorError3;
    try {
      for (var _iterator3 = _asyncIterator([Promise.resolve("a"), "b", later("c")]), _step3; _iteratorAbruptCompletion3 = !(_step3 = yield _iterator3.next()).done; _iteratorAbruptCompletion3 = false) {
        var _v = _step3.value;
        {
          mixed.push(_v);
        }
      }
    } catch (err) {
      _didIteratorError3 = true;
      _iteratorError3 = err;
    } finally {
      try {
        if (_iteratorAbruptCompletion3 && _iterator3.return != null) {
          yield _iterator3.return();
        }
      } finally {
        if (_didIteratorError3) {
          throw _iteratorError3;
        }
      }
    }
    console.log(mixed.join(""));
  });
  return _main.apply(this, arguments);
}
function later(v) {
  return new Promise(resolve => setTimeout(() => resolve(v), 0));
}
main();
