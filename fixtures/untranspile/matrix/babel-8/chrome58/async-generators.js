function _asyncIterator(r) { var n, t, o, e = 2; for ("undefined" != typeof Symbol && (t = Symbol.asyncIterator, o = Symbol.iterator); e--;) { if (t && null != (n = r[t])) return n.call(r); if (o && null != (n = r[o])) return new AsyncFromSyncIterator(n.call(r)); t = "@@asyncIterator", o = "@@iterator"; } throw new TypeError("Object is not async iterable"); }
function AsyncFromSyncIterator(r) { function AsyncFromSyncIteratorContinuation(r) { if (Object(r) !== r) return Promise.reject(new TypeError(r + " is not an object.")); var n = r.done; return Promise.resolve(r.value).then(function (r) { return { value: r, done: n }; }); } return AsyncFromSyncIterator = function (r) { this.s = r, this.n = r.next; }, AsyncFromSyncIterator.prototype = { s: null, n: null, next: function () { return AsyncFromSyncIteratorContinuation(this.n.apply(this.s, arguments)); }, return: function (r) { var n = this.s.return; return void 0 === n ? Promise.resolve({ value: r, done: !0 }) : AsyncFromSyncIteratorContinuation(n.apply(this.s, arguments)); }, throw: function (r) { var n = this.s.throw; return void 0 === n ? Promise.reject(r) : AsyncFromSyncIteratorContinuation(n.apply(this.s, arguments)); } }, new AsyncFromSyncIterator(r); }
function _awaitAsyncGenerator(r) { return new _OverloadYield(r, 0); }
function _wrapAsyncGenerator(e) { return function () { return new AsyncGenerator(e.apply(this, arguments)); }; }
function AsyncGenerator(t) { var n, r; function resume(n, r) { try { var o = t[n](r), u = o.value, i = u instanceof _OverloadYield; Promise.resolve(i ? u.v : u).then(function (e) { if (i) { var r = "return" === n && u.k ? n : "next"; if (!u.k || e.done) return resume(r, e); e = t[r](e).value; } settle(!!o.done, e); }, function (e) { resume("throw", e); }); } catch (e) { settle(2, e); } } function settle(e, t) { 2 === e ? n.reject(t) : n.resolve({ value: t, done: e }), (n = n.next) ? resume(n.key, n.arg) : r = null; } this._invoke = function (e, t) { return new Promise(function (o, u) { var i = { key: e, arg: t, resolve: o, reject: u, next: null }; r ? r = r.next = i : (n = r = i, resume(e, t)); }); }, "function" != typeof t.return && (this.return = void 0); }
AsyncGenerator.prototype["function" == typeof Symbol && Symbol.asyncIterator || "@@asyncIterator"] = function () { return this; }, AsyncGenerator.prototype.next = function (e) { return this._invoke("next", e); }, AsyncGenerator.prototype.throw = function (e) { return this._invoke("throw", e); }, AsyncGenerator.prototype.return = function (e) { return this._invoke("return", e); };
function _OverloadYield(e, d) { this.v = e, this.k = d; }
function ticker(_x) {
  return _ticker.apply(this, arguments);
}
function _ticker() {
  _ticker = _wrapAsyncGenerator(function* (n) {
    for (let i = 0; i < n; i++) {
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
        const v = _step.value;
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
async function main() {
  const seen = [];
  var _iteratorAbruptCompletion2 = false;
  var _didIteratorError2 = false;
  var _iteratorError2;
  try {
    for (var _iterator2 = _asyncIterator(relay(ticker(3))), _step2; _iteratorAbruptCompletion2 = !(_step2 = await _iterator2.next()).done; _iteratorAbruptCompletion2 = false) {
      const v = _step2.value;
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
        await _iterator2.return();
      }
    } finally {
      if (_didIteratorError2) {
        throw _iteratorError2;
      }
    }
  }
  console.log(seen.join(","));
  const it = ticker(5);
  console.log(JSON.stringify([await it.next(), await it.return("x"), await it.next()]));
  const mixed = [];
  var _iteratorAbruptCompletion3 = false;
  var _didIteratorError3 = false;
  var _iteratorError3;
  try {
    for (var _iterator3 = _asyncIterator([Promise.resolve("a"), "b", later("c")]), _step3; _iteratorAbruptCompletion3 = !(_step3 = await _iterator3.next()).done; _iteratorAbruptCompletion3 = false) {
      const v = _step3.value;
      {
        mixed.push(v);
      }
    }
  } catch (err) {
    _didIteratorError3 = true;
    _iteratorError3 = err;
  } finally {
    try {
      if (_iteratorAbruptCompletion3 && _iterator3.return != null) {
        await _iterator3.return();
      }
    } finally {
      if (_didIteratorError3) {
        throw _iteratorError3;
      }
    }
  }
  console.log(mixed.join(""));
}
function later(v) {
  return new Promise(resolve => setTimeout(() => resolve(v), 0));
}
main();
