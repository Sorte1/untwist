(function () {
  'use strict';

  function _typeof$1(o) {
    "@babel/helpers - typeof";

    return _typeof$1 = "function" == typeof Symbol && "symbol" == typeof Symbol.iterator ? function (o) {
      return typeof o;
    } : function (o) {
      return o && "function" == typeof Symbol && o.constructor === Symbol && o !== Symbol.prototype ? "symbol" : typeof o;
    }, _typeof$1(o);
  }

  function toPrimitive(t, r) {
    if ("object" != _typeof$1(t) || !t) return t;
    var e = t[Symbol.toPrimitive];
    if (void 0 !== e) {
      var i = e.call(t, r);
      if ("object" != _typeof$1(i)) return i;
      throw new TypeError("@@toPrimitive must return a primitive value.");
    }
    return ("string" === r ? String : Number)(t);
  }

  function toPropertyKey(t) {
    var i = toPrimitive(t, "string");
    return "symbol" == _typeof$1(i) ? i : i + "";
  }

  function _defineProperty(e, r, t) {
    return (r = toPropertyKey(r)) in e ? Object.defineProperty(e, r, {
      value: t,
      enumerable: true,
      configurable: true,
      writable: true
    }) : e[r] = t, e;
  }

  function _arrayLikeToArray$1(r, a) {
    (null == a || a > r.length) && (a = r.length);
    for (var e = 0, n = Array(a); e < a; e++) n[e] = r[e];
    return n;
  }

  function _arrayWithoutHoles(r) {
    if (Array.isArray(r)) return _arrayLikeToArray$1(r);
  }

  function _iterableToArray(r) {
    if ("undefined" != typeof Symbol && null != r[Symbol.iterator] || null != r["@@iterator"]) return Array.from(r);
  }

  function _unsupportedIterableToArray$1(r, a) {
    if (r) {
      if ("string" == typeof r) return _arrayLikeToArray$1(r, a);
      var t = {}.toString.call(r).slice(8, -1);
      return "Object" === t && r.constructor && (t = r.constructor.name), "Map" === t || "Set" === t ? Array.from(r) : "Arguments" === t || /^(?:Ui|I)nt(?:8|16|32)(?:Clamped)?Array$/.test(t) ? _arrayLikeToArray$1(r, a) : void 0;
    }
  }

  function _nonIterableSpread() {
    throw new TypeError("Invalid attempt to spread non-iterable instance.\nIn order to be iterable, non-array objects must have a [Symbol.iterator]() method.");
  }

  function _toConsumableArray(r) {
    return _arrayWithoutHoles(r) || _iterableToArray(r) || _unsupportedIterableToArray$1(r) || _nonIterableSpread();
  }

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
  		          i || (c ? c < 3 ? (c > 1 && (G.n = -1), d(c, u)) : G.n = u : G.v = u);
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

  function _createForOfIteratorHelper(r, e) { var t = "undefined" != typeof Symbol && r[Symbol.iterator] || r["@@iterator"]; if (!t) { if (Array.isArray(r) || (t = _unsupportedIterableToArray(r)) || e) { t && (r = t); var _n = 0, F = function F() {}; return { s: F, n: function n() { return _n >= r.length ? { done: true } : { done: false, value: r[_n++] }; }, e: function e(r) { throw r; }, f: F }; } throw new TypeError("Invalid attempt to iterate non-iterable instance.\nIn order to be iterable, non-array objects must have a [Symbol.iterator]() method."); } var o, a = true, u = false; return { s: function s() { t = t.call(r); }, n: function n() { var r = t.next(); return a = r.done, r; }, e: function e(r) { u = true, o = r; }, f: function f() { try { a || null == t.return || t.return(); } finally { if (u) throw o; } } }; }
  function _unsupportedIterableToArray(r, a) { if (r) { if ("string" == typeof r) return _arrayLikeToArray(r, a); var t = {}.toString.call(r).slice(8, -1); return "Object" === t && r.constructor && (t = r.constructor.name), "Map" === t || "Set" === t ? Array.from(r) : "Arguments" === t || /^(?:Ui|I)nt(?:8|16|32)(?:Clamped)?Array$/.test(t) ? _arrayLikeToArray(r, a) : void 0; } }
  function _arrayLikeToArray(r, a) { (null == a || a > r.length) && (a = r.length); for (var e = 0, n = Array(a); e < a; e++) n[e] = r[e]; return n; }
  var _marked = /*#__PURE__*/_regeneratorRuntime.mark(inner),
    _marked2 = /*#__PURE__*/_regeneratorRuntime.mark(outer),
    _marked3 = /*#__PURE__*/_regeneratorRuntime.mark(thrower),
    _marked4 = /*#__PURE__*/_regeneratorRuntime.mark(fib);
  function range(start, end) {
    var step = arguments.length > 2 && arguments[2] !== undefined ? arguments[2] : 1;
    return /*#__PURE__*/_regeneratorRuntime.mark(function _callee() {
      var i;
      return _regeneratorRuntime.wrap(function (_context) {
        while (1) switch (_context.prev = _context.next) {
          case 0:
            i = start;
          case 1:
            if (!(i < end)) {
              _context.next = 3;
              break;
            }
            _context.next = 2;
            return i;
          case 2:
            i += step;
            _context.next = 1;
            break;
          case 3:
          case "end":
            return _context.stop();
        }
      }, _callee);
    })();
  }
  function inner() {
    var got;
    return _regeneratorRuntime.wrap(function (_context2) {
      while (1) switch (_context2.prev = _context2.next) {
        case 0:
          _context2.next = 1;
          return "first";
        case 1:
          got = _context2.sent;
          _context2.next = 2;
          return "got " + got;
        case 2:
          return _context2.abrupt("return", "inner-done");
        case 3:
        case "end":
          return _context2.stop();
      }
    }, _marked);
  }
  function outer() {
    var r;
    return _regeneratorRuntime.wrap(function (_context3) {
      while (1) switch (_context3.prev = _context3.next) {
        case 0:
          return _context3.delegateYield(inner(), "t0", 1);
        case 1:
          r = _context3.t0;
          _context3.next = 2;
          return r;
        case 2:
          _context3.prev = 2;
          _context3.next = 3;
          return "try";
        case 3:
          _context3.prev = 3;
          _context3.next = 4;
          return "cleanup";
        case 4:
          return _context3.finish(3);
        case 5:
        case "end":
          return _context3.stop();
      }
    }, _marked2, null, [[2,, 3, 5]]);
  }
  console.log(_toConsumableArray(range(0, 5)).join(","), Array.from(range(1, 10, 3)).join(","));
  var it = outer();
  console.log(JSON.stringify([it.next(), it.next("x"), it.next(), it.next(), it.return(7), it.next(), it.next()]));
  var obj = _defineProperty({}, Symbol.iterator, /*#__PURE__*/_regeneratorRuntime.mark(function _callee2() {
    return _regeneratorRuntime.wrap(function (_context4) {
      while (1) switch (_context4.prev = _context4.next) {
        case 0:
          _context4.next = 1;
          return 1;
        case 1:
          _context4.next = 2;
          return 2;
        case 2:
        case "end":
          return _context4.stop();
      }
    }, _callee2);
  }));
  console.log(_toConsumableArray(obj).join(","));
  function thrower() {
    var _t;
    return _regeneratorRuntime.wrap(function (_context5) {
      while (1) switch (_context5.prev = _context5.next) {
        case 0:
          _context5.prev = 0;
          _context5.next = 1;
          return 1;
        case 1:
          _context5.next = 3;
          break;
        case 2:
          _context5.prev = 2;
          _t = _context5["catch"](0);
          _context5.next = 3;
          return "caught " + _t;
        case 3:
        case "end":
          return _context5.stop();
      }
    }, _marked3, null, [[0, 2]]);
  }
  var t = thrower();
  t.next();
  console.log(JSON.stringify(t.throw("boom")), JSON.stringify(t.next()));
  function fib() {
    var a, b, _ref;
    return _regeneratorRuntime.wrap(function (_context6) {
      while (1) switch (_context6.prev = _context6.next) {
        case 0:
          a = 0, b = 1;
        case 1:
          _context6.next = 2;
          return a;
        case 2:
          _ref = [b, a + b];
          a = _ref[0];
          b = _ref[1];
          _context6.next = 1;
          break;
        case 3:
        case "end":
          return _context6.stop();
      }
    }, _marked4);
  }
  var f = fib();
  var first = [];
  var _iterator = _createForOfIteratorHelper(f),
    _step;
  try {
    for (_iterator.s(); !(_step = _iterator.n()).done;) {
      var v = _step.value;
      if (v > 20) break;
      first.push(v);
    }
  } catch (err) {
    _iterator.e(err);
  } finally {
    _iterator.f();
  }
  console.log(first.join(","));

})();
