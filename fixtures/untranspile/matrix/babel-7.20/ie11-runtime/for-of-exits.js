(function () {
  'use strict';

  function _arrayWithHoles(arr) {
    if (Array.isArray(arr)) return arr;
  }

  function _iterableToArrayLimit(arr, i) {
    var _i = null == arr ? null : "undefined" != typeof Symbol && arr[Symbol.iterator] || arr["@@iterator"];
    if (null != _i) {
      var _s,
        _e,
        _x,
        _r,
        _arr = [],
        _n = true,
        _d = false;
      try {
        if (_x = (_i = _i.call(arr)).next, 0 === i) {
          if (Object(_i) !== _i) return;
          _n = !1;
        } else for (; !(_n = (_s = _x.call(_i)).done) && (_arr.push(_s.value), _arr.length !== i); _n = !0);
      } catch (err) {
        _d = true, _e = err;
      } finally {
        try {
          if (!_n && null != _i["return"] && (_r = _i["return"](), Object(_r) !== _r)) return;
        } finally {
          if (_d) throw _e;
        }
      }
      return _arr;
    }
  }

  function _arrayLikeToArray$1(arr, len) {
    if (len == null || len > arr.length) len = arr.length;
    for (var i = 0, arr2 = new Array(len); i < len; i++) arr2[i] = arr[i];
    return arr2;
  }

  function _unsupportedIterableToArray$1(o, minLen) {
    if (!o) return;
    if (typeof o === "string") return _arrayLikeToArray$1(o, minLen);
    var n = Object.prototype.toString.call(o).slice(8, -1);
    if (n === "Object" && o.constructor) n = o.constructor.name;
    if (n === "Map" || n === "Set") return Array.from(o);
    if (n === "Arguments" || /^(?:Ui|I)nt(?:8|16|32)(?:Clamped)?Array$/.test(n)) return _arrayLikeToArray$1(o, minLen);
  }

  function _nonIterableRest() {
    throw new TypeError("Invalid attempt to destructure non-iterable instance.\nIn order to be iterable, non-array objects must have a [Symbol.iterator]() method.");
  }

  function _slicedToArray(arr, i) {
    return _arrayWithHoles(arr) || _iterableToArrayLimit(arr, i) || _unsupportedIterableToArray$1(arr, i) || _nonIterableRest();
  }

  function _arrayWithoutHoles(arr) {
    if (Array.isArray(arr)) return _arrayLikeToArray$1(arr);
  }

  function _iterableToArray(iter) {
    if (typeof Symbol !== "undefined" && iter[Symbol.iterator] != null || iter["@@iterator"] != null) return Array.from(iter);
  }

  function _nonIterableSpread() {
    throw new TypeError("Invalid attempt to spread non-iterable instance.\nIn order to be iterable, non-array objects must have a [Symbol.iterator]() method.");
  }

  function _toConsumableArray(arr) {
    return _arrayWithoutHoles(arr) || _iterableToArray(arr) || _unsupportedIterableToArray$1(arr) || _nonIterableSpread();
  }

  function _typeof$1(obj) {
    "@babel/helpers - typeof";

    return _typeof$1 = "function" == typeof Symbol && "symbol" == typeof Symbol.iterator ? function (obj) {
      return typeof obj;
    } : function (obj) {
      return obj && "function" == typeof Symbol && obj.constructor === Symbol && obj !== Symbol.prototype ? "symbol" : typeof obj;
    }, _typeof$1(obj);
  }

  function _toPrimitive(input, hint) {
    if (_typeof$1(input) !== "object" || input === null) return input;
    var prim = input[Symbol.toPrimitive];
    if (prim !== undefined) {
      var res = prim.call(input, hint);
      if (_typeof$1(res) !== "object") return res;
      throw new TypeError("@@toPrimitive must return a primitive value.");
    }
    return (hint === "string" ? String : Number)(input);
  }

  function _toPropertyKey(arg) {
    var key = _toPrimitive(arg, "string");
    return _typeof$1(key) === "symbol" ? key : String(key);
  }

  function _defineProperty(obj, key, value) {
    key = _toPropertyKey(key);
    if (key in obj) {
      Object.defineProperty(obj, key, {
        value: value,
        enumerable: true,
        configurable: true,
        writable: true
      });
    } else {
      obj[key] = value;
    }
    return obj;
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

  var _marked = /*#__PURE__*/_regeneratorRuntime.mark(gen);
  function _createForOfIteratorHelper(o, allowArrayLike) { var it = typeof Symbol !== "undefined" && o[Symbol.iterator] || o["@@iterator"]; if (!it) { if (Array.isArray(o) || (it = _unsupportedIterableToArray(o)) || allowArrayLike) { if (it) o = it; var i = 0; var F = function F() {}; return { s: F, n: function n() { if (i >= o.length) return { done: true }; return { done: false, value: o[i++] }; }, e: function e(_e) { throw _e; }, f: F }; } throw new TypeError("Invalid attempt to iterate non-iterable instance.\nIn order to be iterable, non-array objects must have a [Symbol.iterator]() method."); } var normalCompletion = true, didErr = false, err; return { s: function s() { it = it.call(o); }, n: function n() { var step = it.next(); normalCompletion = step.done; return step; }, e: function e(_e2) { didErr = true; err = _e2; }, f: function f() { try { if (!normalCompletion && it.return != null) it.return(); } finally { if (didErr) throw err; } } }; }
  function _unsupportedIterableToArray(o, minLen) { if (!o) return; if (typeof o === "string") return _arrayLikeToArray(o, minLen); var n = Object.prototype.toString.call(o).slice(8, -1); if (n === "Object" && o.constructor) n = o.constructor.name; if (n === "Map" || n === "Set") return Array.from(o); if (n === "Arguments" || /^(?:Ui|I)nt(?:8|16|32)(?:Clamped)?Array$/.test(n)) return _arrayLikeToArray(o, minLen); }
  function _arrayLikeToArray(arr, len) { if (len == null || len > arr.length) len = arr.length; for (var i = 0, arr2 = new Array(len); i < len; i++) arr2[i] = arr[i]; return arr2; }
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
      var _iterator11 = _createForOfIteratorHelper(counted("inner", [10, 20])),
        _step11;
      try {
        for (_iterator11.s(); !(_step11 = _iterator11.n()).done;) {
          var b = _step11.value;
          if (b === 20) continue outer;
          log.push(a + b);
        }
      } catch (err) {
        _iterator11.e(err);
      } finally {
        _iterator11.f();
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
    return _regeneratorRuntime.wrap(function gen$(_context) {
      while (1) switch (_context.prev = _context.next) {
        case 0:
          _context.prev = 0;
          _context.next = 3;
          return 1;
        case 3:
          _context.next = 5;
          return 2;
        case 5:
          _context.prev = 5;
          log.push("gen:finally");
          return _context.finish(5);
        case 8:
        case "end":
          return _context.stop();
      }
    }, _marked, null, [[0,, 5, 8]]);
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
    var _iterator10 = _createForOfIteratorHelper(counted("bad", [1])),
      _step10;
    try {
      for (_iterator10.s(); !(_step10 = _iterator10.n()).done;) {
        var _v = _step10.value;
        log.push(_v.missing.field);
      }
    } catch (err) {
      _iterator10.e(err);
    } finally {
      _iterator10.f();
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

})();
