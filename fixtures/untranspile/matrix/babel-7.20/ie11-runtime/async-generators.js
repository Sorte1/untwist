(function () {
  'use strict';

  function asyncGeneratorStep(gen, resolve, reject, _next, _throw, key, arg) {
    try {
      var info = gen[key](arg);
      var value = info.value;
    } catch (error) {
      reject(error);
      return;
    }
    if (info.done) {
      resolve(value);
    } else {
      Promise.resolve(value).then(_next, _throw);
    }
  }
  function _asyncToGenerator(fn) {
    return function () {
      var self = this,
        args = arguments;
      return new Promise(function (resolve, reject) {
        var gen = fn.apply(self, args);
        function _next(value) {
          asyncGeneratorStep(gen, resolve, reject, _next, _throw, "next", value);
        }
        function _throw(err) {
          asyncGeneratorStep(gen, resolve, reject, _next, _throw, "throw", err);
        }
        _next(undefined);
      });
    };
  }

  function _OverloadYield(value, kind) {
    this.v = value, this.k = kind;
  }

  function _awaitAsyncGenerator(value) {
    return new _OverloadYield(value, 0);
  }

  function AsyncGenerator(gen) {
    var front, back;
    function resume(key, arg) {
      try {
        var result = gen[key](arg),
          value = result.value,
          overloaded = value instanceof _OverloadYield;
        Promise.resolve(overloaded ? value.v : value).then(function (arg) {
          if (overloaded) {
            var nextKey = "return" === key ? "return" : "next";
            if (!value.k || arg.done) return resume(nextKey, arg);
            arg = gen[nextKey](arg).value;
          }
          settle(result.done ? "return" : "normal", arg);
        }, function (err) {
          resume("throw", err);
        });
      } catch (err) {
        settle("throw", err);
      }
    }
    function settle(type, value) {
      switch (type) {
        case "return":
          front.resolve({
            value: value,
            done: true
          });
          break;
        case "throw":
          front.reject(value);
          break;
        default:
          front.resolve({
            value: value,
            done: false
          });
      }
      (front = front.next) ? resume(front.key, front.arg) : back = null;
    }
    this._invoke = function (key, arg) {
      return new Promise(function (resolve, reject) {
        var request = {
          key: key,
          arg: arg,
          resolve: resolve,
          reject: reject,
          next: null
        };
        back ? back = back.next = request : (front = back = request, resume(key, arg));
      });
    }, "function" != typeof gen["return"] && (this["return"] = void 0);
  }
  AsyncGenerator.prototype["function" == typeof Symbol && Symbol.asyncIterator || "@@asyncIterator"] = function () {
    return this;
  }, AsyncGenerator.prototype.next = function (arg) {
    return this._invoke("next", arg);
  }, AsyncGenerator.prototype["throw"] = function (arg) {
    return this._invoke("throw", arg);
  }, AsyncGenerator.prototype["return"] = function (arg) {
    return this._invoke("return", arg);
  };

  function _wrapAsyncGenerator(fn) {
    return function () {
      return new AsyncGenerator(fn.apply(this, arguments));
    };
  }

  function getDefaultExportFromCjs (x) {
  	return x && x.__esModule && Object.prototype.hasOwnProperty.call(x, 'default') ? x['default'] : x;
  }

  var regeneratorRuntime$1 = {exports: {}};

  var _typeof = {exports: {}};

  var hasRequired_typeof;

  function require_typeof () {
  	if (hasRequired_typeof) return _typeof.exports;
  	hasRequired_typeof = 1;
  	(function (module) {
  		function _typeof(obj) {
  		  "@babel/helpers - typeof";

  		  return (module.exports = _typeof = "function" == typeof Symbol && "symbol" == typeof Symbol.iterator ? function (obj) {
  		    return typeof obj;
  		  } : function (obj) {
  		    return obj && "function" == typeof Symbol && obj.constructor === Symbol && obj !== Symbol.prototype ? "symbol" : typeof obj;
  		  }, module.exports.__esModule = true, module.exports["default"] = module.exports), _typeof(obj);
  		}
  		module.exports = _typeof, module.exports.__esModule = true, module.exports["default"] = module.exports; 
  	} (_typeof));
  	return _typeof.exports;
  }

  var hasRequiredRegeneratorRuntime;

  function requireRegeneratorRuntime () {
  	if (hasRequiredRegeneratorRuntime) return regeneratorRuntime$1.exports;
  	hasRequiredRegeneratorRuntime = 1;
  	(function (module) {
  		var _typeof = require_typeof()["default"];
  		function _regeneratorRuntime() {
  		  module.exports = _regeneratorRuntime = function _regeneratorRuntime() {
  		    return exports;
  		  }, module.exports.__esModule = true, module.exports["default"] = module.exports;
  		  var exports = {},
  		    Op = Object.prototype,
  		    hasOwn = Op.hasOwnProperty,
  		    defineProperty = Object.defineProperty || function (obj, key, desc) {
  		      obj[key] = desc.value;
  		    },
  		    $Symbol = "function" == typeof Symbol ? Symbol : {},
  		    iteratorSymbol = $Symbol.iterator || "@@iterator",
  		    asyncIteratorSymbol = $Symbol.asyncIterator || "@@asyncIterator",
  		    toStringTagSymbol = $Symbol.toStringTag || "@@toStringTag";
  		  function define(obj, key, value) {
  		    return Object.defineProperty(obj, key, {
  		      value: value,
  		      enumerable: true,
  		      configurable: true,
  		      writable: true
  		    }), obj[key];
  		  }
  		  try {
  		    define({}, "");
  		  } catch (err) {
  		    define = function define(obj, key, value) {
  		      return obj[key] = value;
  		    };
  		  }
  		  function wrap(innerFn, outerFn, self, tryLocsList) {
  		    var protoGenerator = outerFn && outerFn.prototype instanceof Generator ? outerFn : Generator,
  		      generator = Object.create(protoGenerator.prototype),
  		      context = new Context(tryLocsList || []);
  		    return defineProperty(generator, "_invoke", {
  		      value: makeInvokeMethod(innerFn, self, context)
  		    }), generator;
  		  }
  		  function tryCatch(fn, obj, arg) {
  		    try {
  		      return {
  		        type: "normal",
  		        arg: fn.call(obj, arg)
  		      };
  		    } catch (err) {
  		      return {
  		        type: "throw",
  		        arg: err
  		      };
  		    }
  		  }
  		  exports.wrap = wrap;
  		  var ContinueSentinel = {};
  		  function Generator() {}
  		  function GeneratorFunction() {}
  		  function GeneratorFunctionPrototype() {}
  		  var IteratorPrototype = {};
  		  define(IteratorPrototype, iteratorSymbol, function () {
  		    return this;
  		  });
  		  var getProto = Object.getPrototypeOf,
  		    NativeIteratorPrototype = getProto && getProto(getProto(values([])));
  		  NativeIteratorPrototype && NativeIteratorPrototype !== Op && hasOwn.call(NativeIteratorPrototype, iteratorSymbol) && (IteratorPrototype = NativeIteratorPrototype);
  		  var Gp = GeneratorFunctionPrototype.prototype = Generator.prototype = Object.create(IteratorPrototype);
  		  function defineIteratorMethods(prototype) {
  		    ["next", "throw", "return"].forEach(function (method) {
  		      define(prototype, method, function (arg) {
  		        return this._invoke(method, arg);
  		      });
  		    });
  		  }
  		  function AsyncIterator(generator, PromiseImpl) {
  		    function invoke(method, arg, resolve, reject) {
  		      var record = tryCatch(generator[method], generator, arg);
  		      if ("throw" !== record.type) {
  		        var result = record.arg,
  		          value = result.value;
  		        return value && "object" == _typeof(value) && hasOwn.call(value, "__await") ? PromiseImpl.resolve(value.__await).then(function (value) {
  		          invoke("next", value, resolve, reject);
  		        }, function (err) {
  		          invoke("throw", err, resolve, reject);
  		        }) : PromiseImpl.resolve(value).then(function (unwrapped) {
  		          result.value = unwrapped, resolve(result);
  		        }, function (error) {
  		          return invoke("throw", error, resolve, reject);
  		        });
  		      }
  		      reject(record.arg);
  		    }
  		    var previousPromise;
  		    defineProperty(this, "_invoke", {
  		      value: function value(method, arg) {
  		        function callInvokeWithMethodAndArg() {
  		          return new PromiseImpl(function (resolve, reject) {
  		            invoke(method, arg, resolve, reject);
  		          });
  		        }
  		        return previousPromise = previousPromise ? previousPromise.then(callInvokeWithMethodAndArg, callInvokeWithMethodAndArg) : callInvokeWithMethodAndArg();
  		      }
  		    });
  		  }
  		  function makeInvokeMethod(innerFn, self, context) {
  		    var state = "suspendedStart";
  		    return function (method, arg) {
  		      if ("executing" === state) throw new Error("Generator is already running");
  		      if ("completed" === state) {
  		        if ("throw" === method) throw arg;
  		        return doneResult();
  		      }
  		      for (context.method = method, context.arg = arg;;) {
  		        var delegate = context.delegate;
  		        if (delegate) {
  		          var delegateResult = maybeInvokeDelegate(delegate, context);
  		          if (delegateResult) {
  		            if (delegateResult === ContinueSentinel) continue;
  		            return delegateResult;
  		          }
  		        }
  		        if ("next" === context.method) context.sent = context._sent = context.arg;else if ("throw" === context.method) {
  		          if ("suspendedStart" === state) throw state = "completed", context.arg;
  		          context.dispatchException(context.arg);
  		        } else "return" === context.method && context.abrupt("return", context.arg);
  		        state = "executing";
  		        var record = tryCatch(innerFn, self, context);
  		        if ("normal" === record.type) {
  		          if (state = context.done ? "completed" : "suspendedYield", record.arg === ContinueSentinel) continue;
  		          return {
  		            value: record.arg,
  		            done: context.done
  		          };
  		        }
  		        "throw" === record.type && (state = "completed", context.method = "throw", context.arg = record.arg);
  		      }
  		    };
  		  }
  		  function maybeInvokeDelegate(delegate, context) {
  		    var methodName = context.method,
  		      method = delegate.iterator[methodName];
  		    if (undefined === method) return context.delegate = null, "throw" === methodName && delegate.iterator["return"] && (context.method = "return", context.arg = undefined, maybeInvokeDelegate(delegate, context), "throw" === context.method) || "return" !== methodName && (context.method = "throw", context.arg = new TypeError("The iterator does not provide a '" + methodName + "' method")), ContinueSentinel;
  		    var record = tryCatch(method, delegate.iterator, context.arg);
  		    if ("throw" === record.type) return context.method = "throw", context.arg = record.arg, context.delegate = null, ContinueSentinel;
  		    var info = record.arg;
  		    return info ? info.done ? (context[delegate.resultName] = info.value, context.next = delegate.nextLoc, "return" !== context.method && (context.method = "next", context.arg = undefined), context.delegate = null, ContinueSentinel) : info : (context.method = "throw", context.arg = new TypeError("iterator result is not an object"), context.delegate = null, ContinueSentinel);
  		  }
  		  function pushTryEntry(locs) {
  		    var entry = {
  		      tryLoc: locs[0]
  		    };
  		    1 in locs && (entry.catchLoc = locs[1]), 2 in locs && (entry.finallyLoc = locs[2], entry.afterLoc = locs[3]), this.tryEntries.push(entry);
  		  }
  		  function resetTryEntry(entry) {
  		    var record = entry.completion || {};
  		    record.type = "normal", delete record.arg, entry.completion = record;
  		  }
  		  function Context(tryLocsList) {
  		    this.tryEntries = [{
  		      tryLoc: "root"
  		    }], tryLocsList.forEach(pushTryEntry, this), this.reset(true);
  		  }
  		  function values(iterable) {
  		    if (iterable) {
  		      var iteratorMethod = iterable[iteratorSymbol];
  		      if (iteratorMethod) return iteratorMethod.call(iterable);
  		      if ("function" == typeof iterable.next) return iterable;
  		      if (!isNaN(iterable.length)) {
  		        var i = -1,
  		          next = function next() {
  		            for (; ++i < iterable.length;) if (hasOwn.call(iterable, i)) return next.value = iterable[i], next.done = false, next;
  		            return next.value = undefined, next.done = true, next;
  		          };
  		        return next.next = next;
  		      }
  		    }
  		    return {
  		      next: doneResult
  		    };
  		  }
  		  function doneResult() {
  		    return {
  		      value: undefined,
  		      done: true
  		    };
  		  }
  		  return GeneratorFunction.prototype = GeneratorFunctionPrototype, defineProperty(Gp, "constructor", {
  		    value: GeneratorFunctionPrototype,
  		    configurable: true
  		  }), defineProperty(GeneratorFunctionPrototype, "constructor", {
  		    value: GeneratorFunction,
  		    configurable: true
  		  }), GeneratorFunction.displayName = define(GeneratorFunctionPrototype, toStringTagSymbol, "GeneratorFunction"), exports.isGeneratorFunction = function (genFun) {
  		    var ctor = "function" == typeof genFun && genFun.constructor;
  		    return !!ctor && (ctor === GeneratorFunction || "GeneratorFunction" === (ctor.displayName || ctor.name));
  		  }, exports.mark = function (genFun) {
  		    return Object.setPrototypeOf ? Object.setPrototypeOf(genFun, GeneratorFunctionPrototype) : (genFun.__proto__ = GeneratorFunctionPrototype, define(genFun, toStringTagSymbol, "GeneratorFunction")), genFun.prototype = Object.create(Gp), genFun;
  		  }, exports.awrap = function (arg) {
  		    return {
  		      __await: arg
  		    };
  		  }, defineIteratorMethods(AsyncIterator.prototype), define(AsyncIterator.prototype, asyncIteratorSymbol, function () {
  		    return this;
  		  }), exports.AsyncIterator = AsyncIterator, exports.async = function (innerFn, outerFn, self, tryLocsList, PromiseImpl) {
  		    void 0 === PromiseImpl && (PromiseImpl = Promise);
  		    var iter = new AsyncIterator(wrap(innerFn, outerFn, self, tryLocsList), PromiseImpl);
  		    return exports.isGeneratorFunction(outerFn) ? iter : iter.next().then(function (result) {
  		      return result.done ? result.value : iter.next();
  		    });
  		  }, defineIteratorMethods(Gp), define(Gp, toStringTagSymbol, "Generator"), define(Gp, iteratorSymbol, function () {
  		    return this;
  		  }), define(Gp, "toString", function () {
  		    return "[object Generator]";
  		  }), exports.keys = function (val) {
  		    var object = Object(val),
  		      keys = [];
  		    for (var key in object) keys.push(key);
  		    return keys.reverse(), function next() {
  		      for (; keys.length;) {
  		        var key = keys.pop();
  		        if (key in object) return next.value = key, next.done = false, next;
  		      }
  		      return next.done = true, next;
  		    };
  		  }, exports.values = values, Context.prototype = {
  		    constructor: Context,
  		    reset: function reset(skipTempReset) {
  		      if (this.prev = 0, this.next = 0, this.sent = this._sent = undefined, this.done = false, this.delegate = null, this.method = "next", this.arg = undefined, this.tryEntries.forEach(resetTryEntry), !skipTempReset) for (var name in this) "t" === name.charAt(0) && hasOwn.call(this, name) && !isNaN(+name.slice(1)) && (this[name] = undefined);
  		    },
  		    stop: function stop() {
  		      this.done = true;
  		      var rootRecord = this.tryEntries[0].completion;
  		      if ("throw" === rootRecord.type) throw rootRecord.arg;
  		      return this.rval;
  		    },
  		    dispatchException: function dispatchException(exception) {
  		      if (this.done) throw exception;
  		      var context = this;
  		      function handle(loc, caught) {
  		        return record.type = "throw", record.arg = exception, context.next = loc, caught && (context.method = "next", context.arg = undefined), !!caught;
  		      }
  		      for (var i = this.tryEntries.length - 1; i >= 0; --i) {
  		        var entry = this.tryEntries[i],
  		          record = entry.completion;
  		        if ("root" === entry.tryLoc) return handle("end");
  		        if (entry.tryLoc <= this.prev) {
  		          var hasCatch = hasOwn.call(entry, "catchLoc"),
  		            hasFinally = hasOwn.call(entry, "finallyLoc");
  		          if (hasCatch && hasFinally) {
  		            if (this.prev < entry.catchLoc) return handle(entry.catchLoc, true);
  		            if (this.prev < entry.finallyLoc) return handle(entry.finallyLoc);
  		          } else if (hasCatch) {
  		            if (this.prev < entry.catchLoc) return handle(entry.catchLoc, true);
  		          } else {
  		            if (!hasFinally) throw new Error("try statement without catch or finally");
  		            if (this.prev < entry.finallyLoc) return handle(entry.finallyLoc);
  		          }
  		        }
  		      }
  		    },
  		    abrupt: function abrupt(type, arg) {
  		      for (var i = this.tryEntries.length - 1; i >= 0; --i) {
  		        var entry = this.tryEntries[i];
  		        if (entry.tryLoc <= this.prev && hasOwn.call(entry, "finallyLoc") && this.prev < entry.finallyLoc) {
  		          var finallyEntry = entry;
  		          break;
  		        }
  		      }
  		      finallyEntry && ("break" === type || "continue" === type) && finallyEntry.tryLoc <= arg && arg <= finallyEntry.finallyLoc && (finallyEntry = null);
  		      var record = finallyEntry ? finallyEntry.completion : {};
  		      return record.type = type, record.arg = arg, finallyEntry ? (this.method = "next", this.next = finallyEntry.finallyLoc, ContinueSentinel) : this.complete(record);
  		    },
  		    complete: function complete(record, afterLoc) {
  		      if ("throw" === record.type) throw record.arg;
  		      return "break" === record.type || "continue" === record.type ? this.next = record.arg : "return" === record.type ? (this.rval = this.arg = record.arg, this.method = "return", this.next = "end") : "normal" === record.type && afterLoc && (this.next = afterLoc), ContinueSentinel;
  		    },
  		    finish: function finish(finallyLoc) {
  		      for (var i = this.tryEntries.length - 1; i >= 0; --i) {
  		        var entry = this.tryEntries[i];
  		        if (entry.finallyLoc === finallyLoc) return this.complete(entry.completion, entry.afterLoc), resetTryEntry(entry), ContinueSentinel;
  		      }
  		    },
  		    "catch": function _catch(tryLoc) {
  		      for (var i = this.tryEntries.length - 1; i >= 0; --i) {
  		        var entry = this.tryEntries[i];
  		        if (entry.tryLoc === tryLoc) {
  		          var record = entry.completion;
  		          if ("throw" === record.type) {
  		            var thrown = record.arg;
  		            resetTryEntry(entry);
  		          }
  		          return thrown;
  		        }
  		      }
  		      throw new Error("illegal catch attempt");
  		    },
  		    delegateYield: function delegateYield(iterable, resultName, nextLoc) {
  		      return this.delegate = {
  		        iterator: values(iterable),
  		        resultName: resultName,
  		        nextLoc: nextLoc
  		      }, "next" === this.method && (this.arg = undefined), ContinueSentinel;
  		    }
  		  }, exports;
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

  function _asyncIterator(iterable) { var method, async, sync, retry = 2; for ("undefined" != typeof Symbol && (async = Symbol.asyncIterator, sync = Symbol.iterator); retry--;) { if (async && null != (method = iterable[async])) return method.call(iterable); if (sync && null != (method = iterable[sync])) return new AsyncFromSyncIterator(method.call(iterable)); async = "@@asyncIterator", sync = "@@iterator"; } throw new TypeError("Object is not async iterable"); }
  function AsyncFromSyncIterator(s) { function AsyncFromSyncIteratorContinuation(r) { if (Object(r) !== r) return Promise.reject(new TypeError(r + " is not an object.")); var done = r.done; return Promise.resolve(r.value).then(function (value) { return { value: value, done: done }; }); } return AsyncFromSyncIterator = function AsyncFromSyncIterator(s) { this.s = s, this.n = s.next; }, AsyncFromSyncIterator.prototype = { s: null, n: null, next: function next() { return AsyncFromSyncIteratorContinuation(this.n.apply(this.s, arguments)); }, return: function _return(value) { var ret = this.s.return; return void 0 === ret ? Promise.resolve({ value: value, done: true }) : AsyncFromSyncIteratorContinuation(ret.apply(this.s, arguments)); }, throw: function _throw(value) { var thr = this.s.return; return void 0 === thr ? Promise.reject(value) : AsyncFromSyncIteratorContinuation(thr.apply(this.s, arguments)); } }, new AsyncFromSyncIterator(s); }
  function ticker(_x) {
    return _ticker.apply(this, arguments);
  }
  function _ticker() {
    _ticker = _wrapAsyncGenerator( /*#__PURE__*/_regeneratorRuntime.mark(function _callee(n) {
      var i;
      return _regeneratorRuntime.wrap(function _callee$(_context) {
        while (1) switch (_context.prev = _context.next) {
          case 0:
            i = 0;
          case 1:
            if (!(i < n)) {
              _context.next = 9;
              break;
            }
            _context.next = 4;
            return _awaitAsyncGenerator(null);
          case 4:
            _context.next = 6;
            return i * 10;
          case 6:
            i++;
            _context.next = 1;
            break;
          case 9:
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
    _relay = _wrapAsyncGenerator( /*#__PURE__*/_regeneratorRuntime.mark(function _callee2(src) {
      var _iteratorAbruptCompletion, _didIteratorError, _iteratorError, _iterator, _step, v;
      return _regeneratorRuntime.wrap(function _callee2$(_context2) {
        while (1) switch (_context2.prev = _context2.next) {
          case 0:
            _iteratorAbruptCompletion = false;
            _didIteratorError = false;
            _context2.prev = 2;
            _iterator = _asyncIterator(src);
          case 4:
            _context2.next = 6;
            return _awaitAsyncGenerator(_iterator.next());
          case 6:
            if (!(_iteratorAbruptCompletion = !(_step = _context2.sent).done)) {
              _context2.next = 13;
              break;
            }
            v = _step.value;
            _context2.next = 10;
            return v + 1;
          case 10:
            _iteratorAbruptCompletion = false;
            _context2.next = 4;
            break;
          case 13:
            _context2.next = 19;
            break;
          case 15:
            _context2.prev = 15;
            _context2.t0 = _context2["catch"](2);
            _didIteratorError = true;
            _iteratorError = _context2.t0;
          case 19:
            _context2.prev = 19;
            _context2.prev = 20;
            if (!(_iteratorAbruptCompletion && _iterator.return != null)) {
              _context2.next = 24;
              break;
            }
            _context2.next = 24;
            return _awaitAsyncGenerator(_iterator.return());
          case 24:
            _context2.prev = 24;
            if (!_didIteratorError) {
              _context2.next = 27;
              break;
            }
            throw _iteratorError;
          case 27:
            return _context2.finish(24);
          case 28:
            return _context2.finish(19);
          case 29:
          case "end":
            return _context2.stop();
        }
      }, _callee2, null, [[2, 15, 19, 29], [20,, 24, 28]]);
    }));
    return _relay.apply(this, arguments);
  }
  function main() {
    return _main.apply(this, arguments);
  }
  function _main() {
    _main = _asyncToGenerator( /*#__PURE__*/_regeneratorRuntime.mark(function _callee3() {
      var seen, _iteratorAbruptCompletion2, _didIteratorError2, _iteratorError2, _iterator2, _step2, v, it, mixed, _iteratorAbruptCompletion3, _didIteratorError3, _iteratorError3, _iterator3, _step3, _v;
      return _regeneratorRuntime.wrap(function _callee3$(_context3) {
        while (1) switch (_context3.prev = _context3.next) {
          case 0:
            seen = [];
            _iteratorAbruptCompletion2 = false;
            _didIteratorError2 = false;
            _context3.prev = 3;
            _iterator2 = _asyncIterator(relay(ticker(3)));
          case 5:
            _context3.next = 7;
            return _iterator2.next();
          case 7:
            if (!(_iteratorAbruptCompletion2 = !(_step2 = _context3.sent).done)) {
              _context3.next = 13;
              break;
            }
            v = _step2.value;
            seen.push(v);
          case 10:
            _iteratorAbruptCompletion2 = false;
            _context3.next = 5;
            break;
          case 13:
            _context3.next = 19;
            break;
          case 15:
            _context3.prev = 15;
            _context3.t0 = _context3["catch"](3);
            _didIteratorError2 = true;
            _iteratorError2 = _context3.t0;
          case 19:
            _context3.prev = 19;
            _context3.prev = 20;
            if (!(_iteratorAbruptCompletion2 && _iterator2.return != null)) {
              _context3.next = 24;
              break;
            }
            _context3.next = 24;
            return _iterator2.return();
          case 24:
            _context3.prev = 24;
            if (!_didIteratorError2) {
              _context3.next = 27;
              break;
            }
            throw _iteratorError2;
          case 27:
            return _context3.finish(24);
          case 28:
            return _context3.finish(19);
          case 29:
            console.log(seen.join(","));
            it = ticker(5);
            _context3.t1 = console;
            _context3.t2 = JSON;
            _context3.next = 35;
            return it.next();
          case 35:
            _context3.t3 = _context3.sent;
            _context3.next = 38;
            return it.return("x");
          case 38:
            _context3.t4 = _context3.sent;
            _context3.next = 41;
            return it.next();
          case 41:
            _context3.t5 = _context3.sent;
            _context3.t6 = [_context3.t3, _context3.t4, _context3.t5];
            _context3.t7 = _context3.t2.stringify.call(_context3.t2, _context3.t6);
            _context3.t1.log.call(_context3.t1, _context3.t7);
            mixed = [];
            _iteratorAbruptCompletion3 = false;
            _didIteratorError3 = false;
            _context3.prev = 48;
            _iterator3 = _asyncIterator([Promise.resolve("a"), "b", later("c")]);
          case 50:
            _context3.next = 52;
            return _iterator3.next();
          case 52:
            if (!(_iteratorAbruptCompletion3 = !(_step3 = _context3.sent).done)) {
              _context3.next = 58;
              break;
            }
            _v = _step3.value;
            mixed.push(_v);
          case 55:
            _iteratorAbruptCompletion3 = false;
            _context3.next = 50;
            break;
          case 58:
            _context3.next = 64;
            break;
          case 60:
            _context3.prev = 60;
            _context3.t8 = _context3["catch"](48);
            _didIteratorError3 = true;
            _iteratorError3 = _context3.t8;
          case 64:
            _context3.prev = 64;
            _context3.prev = 65;
            if (!(_iteratorAbruptCompletion3 && _iterator3.return != null)) {
              _context3.next = 69;
              break;
            }
            _context3.next = 69;
            return _iterator3.return();
          case 69:
            _context3.prev = 69;
            if (!_didIteratorError3) {
              _context3.next = 72;
              break;
            }
            throw _iteratorError3;
          case 72:
            return _context3.finish(69);
          case 73:
            return _context3.finish(64);
          case 74:
            console.log(mixed.join(""));
          case 75:
          case "end":
            return _context3.stop();
        }
      }, _callee3, null, [[3, 15, 19, 29], [20,, 24, 28], [48, 60, 64, 74], [65,, 69, 73]]);
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
