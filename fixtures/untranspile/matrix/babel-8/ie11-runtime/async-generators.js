(function () {
  'use strict';

  function asyncGeneratorStep(n, t, e, r, o, a, c) {
    try {
      var i = n[a](c),
        u = i.value;
    } catch (n) {
      return void e(n);
    }
    i.done ? t(u) : Promise.resolve(u).then(r, o);
  }
  function _asyncToGenerator(n) {
    return function () {
      var t = this,
        e = arguments;
      return new Promise(function (r, o) {
        var a = n.apply(t, e);
        function _next(n) {
          asyncGeneratorStep(a, r, o, _next, _throw, "next", n);
        }
        function _throw(n) {
          asyncGeneratorStep(a, r, o, _next, _throw, "throw", n);
        }
        _next(void 0);
      });
    };
  }

  function _regeneratorDefine(e, r, n, t) {
    var i = Object.defineProperty;
    try {
      i({}, "", {});
    } catch (e) {
      i = 0;
    }
    _regeneratorDefine = function regeneratorDefine(e, r, n, t) {
      function o(r, n) {
        _regeneratorDefine(e, r, function (e) {
          return this._invoke(r, n, e);
        });
      }
      r ? i ? i(e, r, {
        value: n,
        enumerable: !t,
        configurable: !t,
        writable: !t
      }) : e[r] = n : (o("next", 0), o("throw", 1), o("return", 2));
    }, _regeneratorDefine(e, r, n, t);
  }

  function _regenerator() {
    /*! regenerator-runtime -- Copyright (c) 2014-present, Facebook, Inc. -- license (MIT): https://github.com/babel/babel/blob/main/packages/babel-helpers/LICENSE */
    var r,
      e,
      o = "function" == typeof Symbol ? Symbol : {},
      n = o.iterator || "@@iterator",
      a = o.toStringTag || "@@toStringTag";
    function i(o, n, a, i) {
      var u = n && n.prototype instanceof Generator ? n : Generator,
        f = Object.create(u.prototype);
      return _regeneratorDefine(f, "_invoke", function (t, o, n) {
        var a,
          i,
          u,
          f = 0,
          p = n || [],
          y = false,
          G = {
            p: 0,
            n: 0,
            v: r,
            a: l,
            f: l.bind(r, 4),
            d: function d(t, e) {
              return a = t, i = 0, u = r, G.n = e, c;
            }
          };
        function l(t, o) {
          for (i = t, u = o, e = 0; !y && f && !n && e < p.length; e++) {
            var n,
              a = p[e],
              l = G.p,
              F = a[2];
            t > 3 ? (n = F === o) && (u = a[(i = a[4]) ? 5 : (i = 3, 3)], a[4] = a[5] = r) : a[0] <= l && ((n = t < 2 && l < a[1]) ? (i = 0, G.v = o, G.n = a[1]) : l < F && (n = t < 3 || a[0] > o || o > F) && (a[4] = t, a[5] = o, G.n = F, i = 0));
          }
          if (n || t > 1) return c;
          throw y = true, o;
        }
        return function (n, p, F) {
          if (f > 1) throw TypeError("Generator is already running");
          for (y && 1 === p && l(p, F), i = p, u = F; (e = i < 2 ? r : u) || !y;) {
            a || (i ? (i > 1 && (G.n = i < 3 ? -1 : u), l(i, u)) : G.v = u);
            try {
              if (f = 2, a) {
                if (i || (n = "next"), e = a[n]) {
                  if (!(e = e.call(a, u))) throw TypeError("iterator result is not an object");
                  if (!e.done) return e;
                  u = e.value, i < 2 && (i = 0);
                } else 1 === i && (e = a["return"]) && e.call(a), i < 2 && (u = TypeError("The iterator does not provide a '" + n + "' method"), i = 1);
                a = r;
              } else if ((e = (y = G.n < 0) ? u : t.call(o, G)) !== c) break;
            } catch (t) {
              a = r, i = 1, u = t;
            } finally {
              f = 1;
            }
          }
          return {
            value: e,
            done: y
          };
        };
      }(o, a, i), true), f;
    }
    var c = {};
    function Generator() {}
    function GeneratorFunction() {}
    function GeneratorFunctionPrototype() {}
    e = Object.getPrototypeOf;
    var u = [][n] ? e(e([][n]())) : (_regeneratorDefine(e = {}, n, function () {
        return this;
      }), e),
      f = GeneratorFunctionPrototype.prototype = Generator.prototype = Object.create(u);
    function p(r) {
      return Object.setPrototypeOf ? Object.setPrototypeOf(r, GeneratorFunctionPrototype) : (r.__proto__ = GeneratorFunctionPrototype, _regeneratorDefine(r, a, "GeneratorFunction")), r.prototype = Object.create(f), r;
    }
    return GeneratorFunction.prototype = GeneratorFunctionPrototype, _regeneratorDefine(f, "constructor", GeneratorFunctionPrototype), _regeneratorDefine(GeneratorFunctionPrototype, "constructor", GeneratorFunction), GeneratorFunction.displayName = "GeneratorFunction", _regeneratorDefine(GeneratorFunctionPrototype, a, "GeneratorFunction"), _regeneratorDefine(f), _regeneratorDefine(f, a, "Generator"), _regeneratorDefine(f, n, function () {
      return this;
    }), _regeneratorDefine(f, "toString", function () {
      return "[object Generator]";
    }), (_regenerator = function _regenerator() {
      return {
        w: i,
        m: p
      };
    })();
  }

  function _asyncIterator(r) {
    var n,
      t,
      o,
      e = 2;
    for ("undefined" != typeof Symbol && (t = Symbol.asyncIterator, o = Symbol.iterator); e--;) {
      if (t && null != (n = r[t])) return n.call(r);
      if (o && null != (n = r[o])) return new AsyncFromSyncIterator(n.call(r));
      t = "@@asyncIterator", o = "@@iterator";
    }
    throw new TypeError("Object is not async iterable");
  }
  function AsyncFromSyncIterator(r) {
    function AsyncFromSyncIteratorContinuation(r) {
      if (Object(r) !== r) return Promise.reject(new TypeError(r + " is not an object."));
      var n = r.done;
      return Promise.resolve(r.value).then(function (r) {
        return {
          value: r,
          done: n
        };
      });
    }
    return AsyncFromSyncIterator = function AsyncFromSyncIterator(r) {
      this.s = r, this.n = r.next;
    }, AsyncFromSyncIterator.prototype = {
      s: null,
      n: null,
      next: function next() {
        return AsyncFromSyncIteratorContinuation(this.n.apply(this.s, arguments));
      },
      "return": function _return(r) {
        var n = this.s["return"];
        return void 0 === n ? Promise.resolve({
          value: r,
          done: true
        }) : AsyncFromSyncIteratorContinuation(n.apply(this.s, arguments));
      },
      "throw": function _throw(r) {
        var n = this.s["throw"];
        return void 0 === n ? Promise.reject(r) : AsyncFromSyncIteratorContinuation(n.apply(this.s, arguments));
      }
    }, new AsyncFromSyncIterator(r);
  }

  function _OverloadYield(e, d) {
    this.v = e, this.k = d;
  }

  function _awaitAsyncGenerator(r) {
    return new _OverloadYield(r, 0);
  }

  function _wrapAsyncGenerator(e) {
    return function () {
      return new AsyncGenerator(e.apply(this, arguments));
    };
  }
  function AsyncGenerator(t) {
    var n, r;
    function resume(n, r) {
      try {
        var o = t[n](r),
          u = o.value,
          i = u instanceof _OverloadYield;
        Promise.resolve(i ? u.v : u).then(function (e) {
          if (i) {
            var r = "return" === n && u.k ? n : "next";
            if (!u.k || e.done) return resume(r, e);
            e = t[r](e).value;
          }
          settle(!!o.done, e);
        }, function (e) {
          resume("throw", e);
        });
      } catch (e) {
        settle(2, e);
      }
    }
    function settle(e, t) {
      2 === e ? n.reject(t) : n.resolve({
        value: t,
        done: e
      }), (n = n.next) ? resume(n.key, n.arg) : r = null;
    }
    this._invoke = function (e, t) {
      return new Promise(function (o, u) {
        var i = {
          key: e,
          arg: t,
          resolve: o,
          reject: u,
          next: null
        };
        r ? r = r.next = i : (n = r = i, resume(e, t));
      });
    }, "function" != typeof t["return"] && (this["return"] = void 0);
  }
  AsyncGenerator.prototype["function" == typeof Symbol && Symbol.asyncIterator || "@@asyncIterator"] = function () {
    return this;
  }, AsyncGenerator.prototype.next = function (e) {
    return this._invoke("next", e);
  }, AsyncGenerator.prototype["throw"] = function (e) {
    return this._invoke("throw", e);
  }, AsyncGenerator.prototype["return"] = function (e) {
    return this._invoke("return", e);
  };

  function ticker(_x) {
    return _ticker.apply(this, arguments);
  }
  function _ticker() {
    _ticker = _wrapAsyncGenerator(/*#__PURE__*/_regenerator().m(function _callee(n) {
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
            return i * 10;
          case 3:
            i++;
            _context.n = 1;
            break;
          case 4:
            return _context.a(2);
        }
      }, _callee);
    }));
    return _ticker.apply(this, arguments);
  }
  function relay(_x2) {
    return _relay.apply(this, arguments);
  }
  function _relay() {
    _relay = _wrapAsyncGenerator(/*#__PURE__*/_regenerator().m(function _callee2(src) {
      var _iteratorAbruptCompletion, _didIteratorError, _iteratorError, _iterator, _step, v, _t;
      return _regenerator().w(function (_context2) {
        while (1) switch (_context2.p = _context2.n) {
          case 0:
            _iteratorAbruptCompletion = false;
            _didIteratorError = false;
            _context2.p = 1;
            _iterator = _asyncIterator(src);
          case 2:
            _context2.n = 3;
            return _awaitAsyncGenerator(_iterator.next());
          case 3:
            if (!(_iteratorAbruptCompletion = !(_step = _context2.v).done)) {
              _context2.n = 5;
              break;
            }
            v = _step.value;
            _context2.n = 4;
            return v + 1;
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
            return _awaitAsyncGenerator(_iterator.return());
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
            return _context2.a(2);
        }
      }, _callee2, null, [[8,, 9, 11], [1, 6, 7, 12]]);
    }));
    return _relay.apply(this, arguments);
  }
  function main() {
    return _main.apply(this, arguments);
  }
  function _main() {
    _main = _asyncToGenerator(/*#__PURE__*/_regenerator().m(function _callee3() {
      var seen, _iteratorAbruptCompletion2, _didIteratorError2, _iteratorError2, _iterator2, _step2, v, it, mixed, _iteratorAbruptCompletion3, _didIteratorError3, _iteratorError3, _iterator3, _step3, _v, _t2, _t3, _t4, _t5, _t6, _t7, _t8, _t9, _t0;
      return _regenerator().w(function (_context3) {
        while (1) switch (_context3.p = _context3.n) {
          case 0:
            seen = [];
            _iteratorAbruptCompletion2 = false;
            _didIteratorError2 = false;
            _context3.p = 1;
            _iterator2 = _asyncIterator(relay(ticker(3)));
          case 2:
            _context3.n = 3;
            return _iterator2.next();
          case 3:
            if (!(_iteratorAbruptCompletion2 = !(_step2 = _context3.v).done)) {
              _context3.n = 5;
              break;
            }
            v = _step2.value;
            seen.push(v);
          case 4:
            _iteratorAbruptCompletion2 = false;
            _context3.n = 2;
            break;
          case 5:
            _context3.n = 7;
            break;
          case 6:
            _context3.p = 6;
            _t2 = _context3.v;
            _didIteratorError2 = true;
            _iteratorError2 = _t2;
          case 7:
            _context3.p = 7;
            _context3.p = 8;
            if (!(_iteratorAbruptCompletion2 && _iterator2.return != null)) {
              _context3.n = 9;
              break;
            }
            _context3.n = 9;
            return _iterator2.return();
          case 9:
            _context3.p = 9;
            if (!_didIteratorError2) {
              _context3.n = 10;
              break;
            }
            throw _iteratorError2;
          case 10:
            return _context3.f(9);
          case 11:
            return _context3.f(7);
          case 12:
            console.log(seen.join(","));
            it = ticker(5);
            _t3 = console;
            _t4 = _t3.log;
            _t5 = JSON;
            _t6 = _t5.stringify;
            _context3.n = 13;
            return it.next();
          case 13:
            _t7 = _context3.v;
            _context3.n = 14;
            return it.return("x");
          case 14:
            _t8 = _context3.v;
            _context3.n = 15;
            return it.next();
          case 15:
            _t9 = _context3.v;
            _t4.call(_t3, _t6.call(_t5, [_t7, _t8, _t9]));
            mixed = [];
            _iteratorAbruptCompletion3 = false;
            _didIteratorError3 = false;
            _context3.p = 16;
            _iterator3 = _asyncIterator([Promise.resolve("a"), "b", later("c")]);
          case 17:
            _context3.n = 18;
            return _iterator3.next();
          case 18:
            if (!(_iteratorAbruptCompletion3 = !(_step3 = _context3.v).done)) {
              _context3.n = 20;
              break;
            }
            _v = _step3.value;
            mixed.push(_v);
          case 19:
            _iteratorAbruptCompletion3 = false;
            _context3.n = 17;
            break;
          case 20:
            _context3.n = 22;
            break;
          case 21:
            _context3.p = 21;
            _t0 = _context3.v;
            _didIteratorError3 = true;
            _iteratorError3 = _t0;
          case 22:
            _context3.p = 22;
            _context3.p = 23;
            if (!(_iteratorAbruptCompletion3 && _iterator3.return != null)) {
              _context3.n = 24;
              break;
            }
            _context3.n = 24;
            return _iterator3.return();
          case 24:
            _context3.p = 24;
            if (!_didIteratorError3) {
              _context3.n = 25;
              break;
            }
            throw _iteratorError3;
          case 25:
            return _context3.f(24);
          case 26:
            return _context3.f(22);
          case 27:
            console.log(mixed.join(""));
          case 28:
            return _context3.a(2);
        }
      }, _callee3, null, [[23,, 24, 26], [16, 21, 22, 27], [8,, 9, 11], [1, 6, 7, 12]]);
    }));
    return _main.apply(this, arguments);
  }
  function later(v) {
    return new Promise(function (resolve) {
      return setTimeout(function () {
        return resolve(v);
      }, 0);
    });
  }
  main();

})();
