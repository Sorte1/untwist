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

  function _OverloadYield(e, d) {
    this.v = e, this.k = d;
  }

  function _awaitAsyncGenerator(e) {
    return new _OverloadYield(e, 0);
  }

  function _wrapAsyncGenerator(e) {
    return function () {
      return new AsyncGenerator(e.apply(this, arguments));
    };
  }
  function AsyncGenerator(e) {
    var t, n;
    function resume(t, n) {
      try {
        var r = e[t](n),
          o = r.value,
          u = o instanceof _OverloadYield;
        Promise.resolve(u ? o.v : o).then(function (n) {
          if (u) {
            var i = "return" === t && o.k ? t : "next";
            if (!o.k || n.done) return resume(i, n);
            n = e[i](n).value;
          }
          settle(!!r.done, n);
        }, function (e) {
          resume("throw", e);
        });
      } catch (e) {
        settle(2, e);
      }
    }
    function settle(e, r) {
      2 === e ? t.reject(r) : t.resolve({
        value: r,
        done: e
      }), (t = t.next) ? resume(t.key, t.arg) : n = null;
    }
    this._invoke = function (e, r) {
      return new Promise(function (o, u) {
        var i = {
          key: e,
          arg: r,
          resolve: o,
          reject: u,
          next: null
        };
        n ? n = n.next = i : (t = n = i, resume(e, r));
      });
    }, "function" != typeof e["return"] && (this["return"] = void 0);
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

  function getDefaultExportFromCjs (x) {
  	return x && x.__esModule && Object.prototype.hasOwnProperty.call(x, 'default') ? x['default'] : x;
  }

  var regeneratorRuntime$1 = {exports: {}};

  var OverloadYield = {exports: {}};

  var hasRequiredOverloadYield;

  function requireOverloadYield () {
  	if (hasRequiredOverloadYield) return OverloadYield.exports;
  	hasRequiredOverloadYield = 1;
  	(function (module) {
  		function _OverloadYield(e, d) {
  		  this.v = e, this.k = d;
  		}
  		module.exports = _OverloadYield, module.exports.__esModule = true, module.exports["default"] = module.exports; 
  	} (OverloadYield));
  	return OverloadYield.exports;
  }

  var regenerator$1 = {exports: {}};

  var regeneratorDefine = {exports: {}};

  var hasRequiredRegeneratorDefine;

  function requireRegeneratorDefine () {
  	if (hasRequiredRegeneratorDefine) return regeneratorDefine.exports;
  	hasRequiredRegeneratorDefine = 1;
  	(function (module) {
  		function _regeneratorDefine(e, r, n, t) {
  		  var i = Object.defineProperty;
  		  try {
  		    i({}, "", {});
  		  } catch (e) {
  		    i = 0;
  		  }
  		  module.exports = _regeneratorDefine = function regeneratorDefine(e, r, n, t) {
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
  		  }, module.exports.__esModule = true, module.exports["default"] = module.exports, _regeneratorDefine(e, r, n, t);
  		}
  		module.exports = _regeneratorDefine, module.exports.__esModule = true, module.exports["default"] = module.exports; 
  	} (regeneratorDefine));
  	return regeneratorDefine.exports;
  }

  var hasRequiredRegenerator$1;

  function requireRegenerator$1 () {
  	if (hasRequiredRegenerator$1) return regenerator$1.exports;
  	hasRequiredRegenerator$1 = 1;
  	(function (module) {
  		var regeneratorDefine = requireRegeneratorDefine();
  		function _regenerator() {
  		  /*! regenerator-runtime -- Copyright (c) 2014-present, Facebook, Inc. -- license (MIT): https://github.com/babel/babel/blob/main/packages/babel-helpers/LICENSE */
  		  var e,
  		    t,
  		    r = "function" == typeof Symbol ? Symbol : {},
  		    n = r.iterator || "@@iterator",
  		    o = r.toStringTag || "@@toStringTag";
  		  function i(r, n, o, i) {
  		    var c = n && n.prototype instanceof Generator ? n : Generator,
  		      u = Object.create(c.prototype);
  		    return regeneratorDefine(u, "_invoke", function (r, n, o) {
  		      var i,
  		        c,
  		        u,
  		        f = 0,
  		        p = o || [],
  		        y = false,
  		        G = {
  		          p: 0,
  		          n: 0,
  		          v: e,
  		          a: d,
  		          f: d.bind(e, 4),
  		          d: function d(t, r) {
  		            return i = t, c = 0, u = e, G.n = r, a;
  		          }
  		        };
  		      function d(r, n) {
  		        for (c = r, u = n, t = 0; !y && f && !o && t < p.length; t++) {
  		          var o,
  		            i = p[t],
  		            d = G.p,
  		            l = i[2];
  		          r > 3 ? (o = l === n) && (u = i[(c = i[4]) ? 5 : (c = 3, 3)], i[4] = i[5] = e) : i[0] <= d && ((o = r < 2 && d < i[1]) ? (c = 0, G.v = n, G.n = i[1]) : d < l && (o = r < 3 || i[0] > n || n > l) && (i[4] = r, i[5] = n, G.n = l, c = 0));
  		        }
  		        if (o || r > 1) return a;
  		        throw y = true, n;
  		      }
  		      return function (o, p, l) {
  		        if (f > 1) throw TypeError("Generator is already running");
  		        for (y && 1 === p && d(p, l), c = p, u = l; (t = c < 2 ? e : u) || !y;) {
  		          i || (c ? (c > 1 && (G.n = c < 3 ? -1 : u), d(c, u)) : G.v = u);
  		          try {
  		            if (f = 2, i) {
  		              if (c || (o = "next"), t = i[o]) {
  		                if (!(t = t.call(i, u))) throw TypeError("iterator result is not an object");
  		                if (!t.done) return t;
  		                u = t.value, c < 2 && (c = 0);
  		              } else 1 === c && (t = i["return"]) && t.call(i), c < 2 && (u = TypeError("The iterator does not provide a '" + o + "' method"), c = 1);
  		              i = e;
  		            } else if ((t = (y = G.n < 0) ? u : r.call(n, G)) !== a) break;
  		          } catch (t) {
  		            i = e, c = 1, u = t;
  		          } finally {
  		            f = 1;
  		          }
  		        }
  		        return {
  		          value: t,
  		          done: y
  		        };
  		      };
  		    }(r, o, i), true), u;
  		  }
  		  var a = {};
  		  function Generator() {}
  		  function GeneratorFunction() {}
  		  function GeneratorFunctionPrototype() {}
  		  t = Object.getPrototypeOf;
  		  var c = [][n] ? t(t([][n]())) : (regeneratorDefine(t = {}, n, function () {
  		      return this;
  		    }), t),
  		    u = GeneratorFunctionPrototype.prototype = Generator.prototype = Object.create(c);
  		  function f(e) {
  		    return Object.setPrototypeOf ? Object.setPrototypeOf(e, GeneratorFunctionPrototype) : (e.__proto__ = GeneratorFunctionPrototype, regeneratorDefine(e, o, "GeneratorFunction")), e.prototype = Object.create(u), e;
  		  }
  		  return GeneratorFunction.prototype = GeneratorFunctionPrototype, regeneratorDefine(u, "constructor", GeneratorFunctionPrototype), regeneratorDefine(GeneratorFunctionPrototype, "constructor", GeneratorFunction), GeneratorFunction.displayName = "GeneratorFunction", regeneratorDefine(GeneratorFunctionPrototype, o, "GeneratorFunction"), regeneratorDefine(u), regeneratorDefine(u, o, "Generator"), regeneratorDefine(u, n, function () {
  		    return this;
  		  }), regeneratorDefine(u, "toString", function () {
  		    return "[object Generator]";
  		  }), (module.exports = _regenerator = function _regenerator() {
  		    return {
  		      w: i,
  		      m: f
  		    };
  		  }, module.exports.__esModule = true, module.exports["default"] = module.exports)();
  		}
  		module.exports = _regenerator, module.exports.__esModule = true, module.exports["default"] = module.exports; 
  	} (regenerator$1));
  	return regenerator$1.exports;
  }

  var regeneratorAsync = {exports: {}};

  var regeneratorAsyncGen = {exports: {}};

  var regeneratorAsyncIterator = {exports: {}};

  var hasRequiredRegeneratorAsyncIterator;

  function requireRegeneratorAsyncIterator () {
  	if (hasRequiredRegeneratorAsyncIterator) return regeneratorAsyncIterator.exports;
  	hasRequiredRegeneratorAsyncIterator = 1;
  	(function (module) {
  		var OverloadYield = requireOverloadYield();
  		var regeneratorDefine = requireRegeneratorDefine();
  		function AsyncIterator(t, e) {
  		  function n(r, o, i, f) {
  		    try {
  		      var c = t[r](o),
  		        u = c.value;
  		      return u instanceof OverloadYield ? e.resolve(u.v).then(function (t) {
  		        n("next", t, i, f);
  		      }, function (t) {
  		        n("throw", t, i, f);
  		      }) : e.resolve(u).then(function (t) {
  		        c.value = t, i(c);
  		      }, function (t) {
  		        return n("throw", t, i, f);
  		      });
  		    } catch (t) {
  		      f(t);
  		    }
  		  }
  		  var r;
  		  this.next || (regeneratorDefine(AsyncIterator.prototype), regeneratorDefine(AsyncIterator.prototype, "function" == typeof Symbol && Symbol.asyncIterator || "@asyncIterator", function () {
  		    return this;
  		  })), regeneratorDefine(this, "_invoke", function (t, o, i) {
  		    function f() {
  		      return new e(function (e, r) {
  		        n(t, i, e, r);
  		      });
  		    }
  		    return r = r ? r.then(f, f) : f();
  		  }, true);
  		}
  		module.exports = AsyncIterator, module.exports.__esModule = true, module.exports["default"] = module.exports; 
  	} (regeneratorAsyncIterator));
  	return regeneratorAsyncIterator.exports;
  }

  var hasRequiredRegeneratorAsyncGen;

  function requireRegeneratorAsyncGen () {
  	if (hasRequiredRegeneratorAsyncGen) return regeneratorAsyncGen.exports;
  	hasRequiredRegeneratorAsyncGen = 1;
  	(function (module) {
  		var regenerator = requireRegenerator$1();
  		var regeneratorAsyncIterator = requireRegeneratorAsyncIterator();
  		function _regeneratorAsyncGen(r, e, t, o, n) {
  		  return new regeneratorAsyncIterator(regenerator().w(r, e, t, o), n || Promise);
  		}
  		module.exports = _regeneratorAsyncGen, module.exports.__esModule = true, module.exports["default"] = module.exports; 
  	} (regeneratorAsyncGen));
  	return regeneratorAsyncGen.exports;
  }

  var hasRequiredRegeneratorAsync;

  function requireRegeneratorAsync () {
  	if (hasRequiredRegeneratorAsync) return regeneratorAsync.exports;
  	hasRequiredRegeneratorAsync = 1;
  	(function (module) {
  		var regeneratorAsyncGen = requireRegeneratorAsyncGen();
  		function _regeneratorAsync(n, e, r, t, o) {
  		  var a = regeneratorAsyncGen(n, e, r, t, o);
  		  return a.next().then(function (n) {
  		    return n.done ? n.value : a.next();
  		  });
  		}
  		module.exports = _regeneratorAsync, module.exports.__esModule = true, module.exports["default"] = module.exports; 
  	} (regeneratorAsync));
  	return regeneratorAsync.exports;
  }

  var regeneratorKeys = {exports: {}};

  var hasRequiredRegeneratorKeys;

  function requireRegeneratorKeys () {
  	if (hasRequiredRegeneratorKeys) return regeneratorKeys.exports;
  	hasRequiredRegeneratorKeys = 1;
  	(function (module) {
  		function _regeneratorKeys(e) {
  		  var n = Object(e),
  		    r = [];
  		  for (var t in n) r.unshift(t);
  		  return function e() {
  		    for (; r.length;) if ((t = r.pop()) in n) return e.value = t, e.done = false, e;
  		    return e.done = true, e;
  		  };
  		}
  		module.exports = _regeneratorKeys, module.exports.__esModule = true, module.exports["default"] = module.exports; 
  	} (regeneratorKeys));
  	return regeneratorKeys.exports;
  }

  var regeneratorValues = {exports: {}};

  var _typeof = {exports: {}};

  var hasRequired_typeof;

  function require_typeof () {
  	if (hasRequired_typeof) return _typeof.exports;
  	hasRequired_typeof = 1;
  	(function (module) {
  		function _typeof(o) {
  		  "@babel/helpers - typeof";

  		  return module.exports = _typeof = "function" == typeof Symbol && "symbol" == typeof Symbol.iterator ? function (o) {
  		    return typeof o;
  		  } : function (o) {
  		    return o && "function" == typeof Symbol && o.constructor === Symbol && o !== Symbol.prototype ? "symbol" : typeof o;
  		  }, module.exports.__esModule = true, module.exports["default"] = module.exports, _typeof(o);
  		}
  		module.exports = _typeof, module.exports.__esModule = true, module.exports["default"] = module.exports; 
  	} (_typeof));
  	return _typeof.exports;
  }

  var hasRequiredRegeneratorValues;

  function requireRegeneratorValues () {
  	if (hasRequiredRegeneratorValues) return regeneratorValues.exports;
  	hasRequiredRegeneratorValues = 1;
  	(function (module) {
  		var _typeof = require_typeof()["default"];
  		function _regeneratorValues(e) {
  		  if (null != e) {
  		    var t = e["function" == typeof Symbol && Symbol.iterator || "@@iterator"],
  		      r = 0;
  		    if (t) return t.call(e);
  		    if ("function" == typeof e.next) return e;
  		    if (!isNaN(e.length)) return {
  		      next: function next() {
  		        return e && r >= e.length && (e = void 0), {
  		          value: e && e[r++],
  		          done: !e
  		        };
  		      }
  		    };
  		  }
  		  throw new TypeError(_typeof(e) + " is not iterable");
  		}
  		module.exports = _regeneratorValues, module.exports.__esModule = true, module.exports["default"] = module.exports; 
  	} (regeneratorValues));
  	return regeneratorValues.exports;
  }

  var hasRequiredRegeneratorRuntime;

  function requireRegeneratorRuntime () {
  	if (hasRequiredRegeneratorRuntime) return regeneratorRuntime$1.exports;
  	hasRequiredRegeneratorRuntime = 1;
  	(function (module) {
  		var OverloadYield = requireOverloadYield();
  		var regenerator = requireRegenerator$1();
  		var regeneratorAsync = requireRegeneratorAsync();
  		var regeneratorAsyncGen = requireRegeneratorAsyncGen();
  		var regeneratorAsyncIterator = requireRegeneratorAsyncIterator();
  		var regeneratorKeys = requireRegeneratorKeys();
  		var regeneratorValues = requireRegeneratorValues();
  		function _regeneratorRuntime() {

  		  var r = regenerator(),
  		    e = r.m(_regeneratorRuntime),
  		    t = (Object.getPrototypeOf ? Object.getPrototypeOf(e) : e.__proto__).constructor;
  		  function n(r) {
  		    var e = "function" == typeof r && r.constructor;
  		    return !!e && (e === t || "GeneratorFunction" === (e.displayName || e.name));
  		  }
  		  var o = {
  		    "throw": 1,
  		    "return": 2,
  		    "break": 3,
  		    "continue": 3
  		  };
  		  function a(r) {
  		    var e, t;
  		    return function (n) {
  		      e || (e = {
  		        stop: function stop() {
  		          return t(n.a, 2);
  		        },
  		        "catch": function _catch() {
  		          return n.v;
  		        },
  		        abrupt: function abrupt(r, e) {
  		          return t(n.a, o[r], e);
  		        },
  		        delegateYield: function delegateYield(r, o, a) {
  		          return e.resultName = o, t(n.d, regeneratorValues(r), a);
  		        },
  		        finish: function finish(r) {
  		          return t(n.f, r);
  		        }
  		      }, t = function t(r, _t, o) {
  		        n.p = e.prev, n.n = e.next;
  		        try {
  		          return r(_t, o);
  		        } finally {
  		          e.next = n.n;
  		        }
  		      }), e.resultName && (e[e.resultName] = n.v, e.resultName = void 0), e.sent = n.v, e.next = n.n;
  		      try {
  		        return r.call(this, e);
  		      } finally {
  		        n.p = e.prev, n.n = e.next;
  		      }
  		    };
  		  }
  		  return (module.exports = _regeneratorRuntime = function _regeneratorRuntime() {
  		    return {
  		      wrap: function wrap(e, t, n, o) {
  		        return r.w(a(e), t, n, o && o.reverse());
  		      },
  		      isGeneratorFunction: n,
  		      mark: r.m,
  		      awrap: function awrap(r, e) {
  		        return new OverloadYield(r, e);
  		      },
  		      AsyncIterator: regeneratorAsyncIterator,
  		      async: function async(r, e, t, o, u) {
  		        return (n(e) ? regeneratorAsyncGen : regeneratorAsync)(a(r), e, t, o, u);
  		      },
  		      keys: regeneratorKeys,
  		      values: regeneratorValues
  		    };
  		  }, module.exports.__esModule = true, module.exports["default"] = module.exports)();
  		}
  		module.exports = _regeneratorRuntime, module.exports.__esModule = true, module.exports["default"] = module.exports; 
  	} (regeneratorRuntime$1));
  	return regeneratorRuntime$1.exports;
  }

  var regenerator;
  var hasRequiredRegenerator;

  function requireRegenerator () {
  	if (hasRequiredRegenerator) return regenerator;
  	hasRequiredRegenerator = 1;
  	// TODO(Babel 8): Remove this file.

  	var runtime = requireRegeneratorRuntime()();
  	regenerator = runtime;

  	// Copied from https://github.com/facebook/regenerator/blob/main/packages/runtime/runtime.js#L736=
  	try {
  	  regeneratorRuntime = runtime;
  	} catch (accidentalStrictMode) {
  	  if (typeof globalThis === "object") {
  	    globalThis.regeneratorRuntime = runtime;
  	  } else {
  	    Function("r", "regeneratorRuntime = r")(runtime);
  	  }
  	}
  	return regenerator;
  }

  var regeneratorExports = requireRegenerator();
  var _regeneratorRuntime = /*@__PURE__*/getDefaultExportFromCjs(regeneratorExports);

  function _asyncIterator(r) { var n, t, o, e = 2; for ("undefined" != typeof Symbol && (t = Symbol.asyncIterator, o = Symbol.iterator); e--;) { if (t && null != (n = r[t])) return n.call(r); if (o && null != (n = r[o])) return new AsyncFromSyncIterator(n.call(r)); t = "@@asyncIterator", o = "@@iterator"; } throw new TypeError("Object is not async iterable"); }
  function AsyncFromSyncIterator(r) { function AsyncFromSyncIteratorContinuation(r) { if (Object(r) !== r) return Promise.reject(new TypeError(r + " is not an object.")); var n = r.done; return Promise.resolve(r.value).then(function (r) { return { value: r, done: n }; }); } return AsyncFromSyncIterator = function AsyncFromSyncIterator(r) { this.s = r, this.n = r.next; }, AsyncFromSyncIterator.prototype = { s: null, n: null, next: function next() { return AsyncFromSyncIteratorContinuation(this.n.apply(this.s, arguments)); }, return: function _return(r) { var n = this.s.return; return void 0 === n ? Promise.resolve({ value: r, done: true }) : AsyncFromSyncIteratorContinuation(n.apply(this.s, arguments)); }, throw: function _throw(r) { var n = this.s.throw; return void 0 === n ? Promise.reject(r) : AsyncFromSyncIteratorContinuation(n.apply(this.s, arguments)); } }, new AsyncFromSyncIterator(r); }
  function ticker(_x) {
    return _ticker.apply(this, arguments);
  }
  function _ticker() {
    _ticker = _wrapAsyncGenerator(/*#__PURE__*/_regeneratorRuntime.mark(function _callee(n) {
      var i;
      return _regeneratorRuntime.wrap(function (_context) {
        while (1) switch (_context.prev = _context.next) {
          case 0:
            i = 0;
          case 1:
            if (!(i < n)) {
              _context.next = 4;
              break;
            }
            _context.next = 2;
            return _awaitAsyncGenerator(null);
          case 2:
            _context.next = 3;
            return i * 10;
          case 3:
            i++;
            _context.next = 1;
            break;
          case 4:
          case "end":
            return _context.stop();
        }
      }, _callee);
    }));
    return _ticker.apply(this, arguments);
  }
  function relay(_x2) {
    return _relay.apply(this, arguments);
  }
  function _relay() {
    _relay = _wrapAsyncGenerator(/*#__PURE__*/_regeneratorRuntime.mark(function _callee2(src) {
      var _iteratorAbruptCompletion, _didIteratorError, _iteratorError, _iterator, _step, v, _t;
      return _regeneratorRuntime.wrap(function (_context2) {
        while (1) switch (_context2.prev = _context2.next) {
          case 0:
            _iteratorAbruptCompletion = false;
            _didIteratorError = false;
            _context2.prev = 1;
            _iterator = _asyncIterator(src);
          case 2:
            _context2.next = 3;
            return _awaitAsyncGenerator(_iterator.next());
          case 3:
            if (!(_iteratorAbruptCompletion = !(_step = _context2.sent).done)) {
              _context2.next = 5;
              break;
            }
            v = _step.value;
            _context2.next = 4;
            return v + 1;
          case 4:
            _iteratorAbruptCompletion = false;
            _context2.next = 2;
            break;
          case 5:
            _context2.next = 7;
            break;
          case 6:
            _context2.prev = 6;
            _t = _context2["catch"](1);
            _didIteratorError = true;
            _iteratorError = _t;
          case 7:
            _context2.prev = 7;
            _context2.prev = 8;
            if (!(_iteratorAbruptCompletion && _iterator.return != null)) {
              _context2.next = 9;
              break;
            }
            _context2.next = 9;
            return _awaitAsyncGenerator(_iterator.return());
          case 9:
            _context2.prev = 9;
            if (!_didIteratorError) {
              _context2.next = 10;
              break;
            }
            throw _iteratorError;
          case 10:
            return _context2.finish(9);
          case 11:
            return _context2.finish(7);
          case 12:
          case "end":
            return _context2.stop();
        }
      }, _callee2, null, [[1, 6, 7, 12], [8,, 9, 11]]);
    }));
    return _relay.apply(this, arguments);
  }
  function main() {
    return _main.apply(this, arguments);
  }
  function _main() {
    _main = _asyncToGenerator(/*#__PURE__*/_regeneratorRuntime.mark(function _callee3() {
      var seen, _iteratorAbruptCompletion2, _didIteratorError2, _iteratorError2, _iterator2, _step2, v, it, mixed, _iteratorAbruptCompletion3, _didIteratorError3, _iteratorError3, _iterator3, _step3, _v, _t2, _t3, _t4, _t5, _t6, _t7, _t8;
      return _regeneratorRuntime.wrap(function (_context3) {
        while (1) switch (_context3.prev = _context3.next) {
          case 0:
            seen = [];
            _iteratorAbruptCompletion2 = false;
            _didIteratorError2 = false;
            _context3.prev = 1;
            _iterator2 = _asyncIterator(relay(ticker(3)));
          case 2:
            _context3.next = 3;
            return _iterator2.next();
          case 3:
            if (!(_iteratorAbruptCompletion2 = !(_step2 = _context3.sent).done)) {
              _context3.next = 5;
              break;
            }
            v = _step2.value;
            seen.push(v);
          case 4:
            _iteratorAbruptCompletion2 = false;
            _context3.next = 2;
            break;
          case 5:
            _context3.next = 7;
            break;
          case 6:
            _context3.prev = 6;
            _t2 = _context3["catch"](1);
            _didIteratorError2 = true;
            _iteratorError2 = _t2;
          case 7:
            _context3.prev = 7;
            _context3.prev = 8;
            if (!(_iteratorAbruptCompletion2 && _iterator2.return != null)) {
              _context3.next = 9;
              break;
            }
            _context3.next = 9;
            return _iterator2.return();
          case 9:
            _context3.prev = 9;
            if (!_didIteratorError2) {
              _context3.next = 10;
              break;
            }
            throw _iteratorError2;
          case 10:
            return _context3.finish(9);
          case 11:
            return _context3.finish(7);
          case 12:
            console.log(seen.join(","));
            it = ticker(5);
            _t3 = console;
            _t4 = JSON;
            _context3.next = 13;
            return it.next();
          case 13:
            _t5 = _context3.sent;
            _context3.next = 14;
            return it.return("x");
          case 14:
            _t6 = _context3.sent;
            _context3.next = 15;
            return it.next();
          case 15:
            _t7 = _context3.sent;
            _t3.log.call(_t3, _t4.stringify.call(_t4, [_t5, _t6, _t7]));
            mixed = [];
            _iteratorAbruptCompletion3 = false;
            _didIteratorError3 = false;
            _context3.prev = 16;
            _iterator3 = _asyncIterator([Promise.resolve("a"), "b", later("c")]);
          case 17:
            _context3.next = 18;
            return _iterator3.next();
          case 18:
            if (!(_iteratorAbruptCompletion3 = !(_step3 = _context3.sent).done)) {
              _context3.next = 20;
              break;
            }
            _v = _step3.value;
            mixed.push(_v);
          case 19:
            _iteratorAbruptCompletion3 = false;
            _context3.next = 17;
            break;
          case 20:
            _context3.next = 22;
            break;
          case 21:
            _context3.prev = 21;
            _t8 = _context3["catch"](16);
            _didIteratorError3 = true;
            _iteratorError3 = _t8;
          case 22:
            _context3.prev = 22;
            _context3.prev = 23;
            if (!(_iteratorAbruptCompletion3 && _iterator3.return != null)) {
              _context3.next = 24;
              break;
            }
            _context3.next = 24;
            return _iterator3.return();
          case 24:
            _context3.prev = 24;
            if (!_didIteratorError3) {
              _context3.next = 25;
              break;
            }
            throw _iteratorError3;
          case 25:
            return _context3.finish(24);
          case 26:
            return _context3.finish(22);
          case 27:
            console.log(mixed.join(""));
          case 28:
          case "end":
            return _context3.stop();
        }
      }, _callee3, null, [[1, 6, 7, 12], [8,, 9, 11], [16, 21, 22, 27], [23,, 24, 26]]);
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
