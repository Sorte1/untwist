(function () {
  'use strict';

  function _classCallCheck(a, n) {
    if (!(a instanceof n)) throw new TypeError("Cannot call a class as a function");
  }

  function _typeof$1(o) {
    "@babel/helpers - typeof";

    return _typeof$1 = "function" == typeof Symbol && "symbol" == typeof Symbol.iterator ? function (o) {
      return typeof o;
    } : function (o) {
      return o && "function" == typeof Symbol && o.constructor === Symbol && o !== Symbol.prototype ? "symbol" : typeof o;
    }, _typeof$1(o);
  }

  function toPrimitive(t, e) {
    if ("object" != _typeof$1(t) || !t) return t;
    var r;
    if ("undefined" != typeof Symbol && void 0 !== (r = t[Symbol.toPrimitive])) {
      var i = r.call(t, e);
      if ("object" != _typeof$1(i)) return i;
      throw new TypeError("@@toPrimitive must return a primitive value.");
    }
    return (String )(t);
  }

  function toPropertyKey(t) {
    var i = toPrimitive(t, "string");
    return "symbol" == _typeof$1(i) ? i : i + "";
  }

  function _defineProperties(e, r) {
    for (var t = 0; t < r.length; t++) {
      var o = r[t];
      o.enumerable = o.enumerable || false, o.configurable = true, "value" in o && (o.writable = true), Object.defineProperty(e, toPropertyKey(o.key), o);
    }
  }
  function _createClass(e, r, t) {
    return r && _defineProperties(e.prototype, r), Object.defineProperty(e, "prototype", {
      writable: false
    }), e;
  }

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

  var log = [];
  var later = function later(v) {
    return new Promise(function (resolve) {
      return setTimeout(function () {
        return resolve(v);
      }, 0);
    });
  };
  function fetchTwice(_x) {
    return _fetchTwice.apply(this, arguments);
  }
  function _fetchTwice() {
    _fetchTwice = _asyncToGenerator(/*#__PURE__*/_regeneratorRuntime.mark(function _callee4(x) {
      var a, b;
      return _regeneratorRuntime.wrap(function (_context4) {
        while (1) switch (_context4.prev = _context4.next) {
          case 0:
            log.push("start " + x);
            _context4.next = 1;
            return later(x);
          case 1:
            a = _context4.sent;
            _context4.next = 2;
            return later(a + 1);
          case 2:
            b = _context4.sent;
            return _context4.abrupt("return", a + b);
          case 3:
          case "end":
            return _context4.stop();
        }
      }, _callee4);
    }));
    return _fetchTwice.apply(this, arguments);
  }
  function guarded() {
    return _guarded.apply(this, arguments);
  }
  function _guarded() {
    _guarded = _asyncToGenerator(/*#__PURE__*/_regeneratorRuntime.mark(function _callee5() {
      var _t2;
      return _regeneratorRuntime.wrap(function (_context5) {
        while (1) switch (_context5.prev = _context5.next) {
          case 0:
            _context5.prev = 0;
            _context5.next = 1;
            return later(0);
          case 1:
            throw new TypeError("t");
          case 2:
            _context5.prev = 2;
            _t2 = _context5["catch"](0);
            log.push("caught " + _t2.name);
            return _context5.abrupt("return", "r");
          case 3:
            _context5.prev = 3;
            log.push("finally");
            return _context5.finish(3);
          case 4:
          case "end":
            return _context5.stop();
        }
      }, _callee5, null, [[0, 2, 3, 4]]);
    }));
    return _guarded.apply(this, arguments);
  }
  var service = {
    base: 100,
    total: function total(n) {
      var _arguments = arguments,
        _this = this;
      return _asyncToGenerator(/*#__PURE__*/_regeneratorRuntime.mark(function _callee2() {
        var add, sum, i, _t;
        return _regeneratorRuntime.wrap(function (_context2) {
          while (1) switch (_context2.prev = _context2.next) {
            case 0:
              add = /*#__PURE__*/function () {
                var _ref = _asyncToGenerator(/*#__PURE__*/_regeneratorRuntime.mark(function _callee(k) {
                  return _regeneratorRuntime.wrap(function (_context) {
                    while (1) switch (_context.prev = _context.next) {
                      case 0:
                        return _context.abrupt("return", _this.base + k + _arguments.length);
                      case 1:
                      case "end":
                        return _context.stop();
                    }
                  }, _callee);
                }));
                return function add(_x2) {
                  return _ref.apply(this, arguments);
                };
              }();
              sum = 0;
              i = 0;
            case 1:
              if (!(i < n)) {
                _context2.next = 4;
                break;
              }
              _t = sum;
              _context2.next = 2;
              return add(i);
            case 2:
              sum = _t += _context2.sent;
            case 3:
              i++;
              _context2.next = 1;
              break;
            case 4:
              return _context2.abrupt("return", sum);
            case 5:
            case "end":
              return _context2.stop();
          }
        }, _callee2);
      }))();
    }
  };
  var Loader = /*#__PURE__*/function () {
    function Loader() {
      _classCallCheck(this, Loader);
      this.items = [];
    }
    return _createClass(Loader, [{
      key: "load",
      value: function () {
        var _load = _asyncToGenerator(/*#__PURE__*/_regeneratorRuntime.mark(function _callee3(v) {
          return _regeneratorRuntime.wrap(function (_context3) {
            while (1) switch (_context3.prev = _context3.next) {
              case 0:
                _context3.next = 1;
                return null;
              case 1:
                this.items.push(v);
                return _context3.abrupt("return", this.items.length);
              case 2:
              case "end":
                return _context3.stop();
            }
          }, _callee3, this);
        }));
        function load(_x3) {
          return _load.apply(this, arguments);
        }
        return load;
      }()
    }]);
  }();
  function main() {
    return _main.apply(this, arguments);
  }
  function _main() {
    _main = _asyncToGenerator(/*#__PURE__*/_regeneratorRuntime.mark(function _callee7() {
      var p, l, arrow, _t4, _t5, _t6, _t7, _t8, _t9, _t0;
      return _regeneratorRuntime.wrap(function (_context7) {
        while (1) switch (_context7.prev = _context7.next) {
          case 0:
            p = fetchTwice(1);
            log.push("after call");
            _t4 = console;
            _context7.next = 1;
            return p;
          case 1:
            _t5 = _context7.sent;
            _context7.next = 2;
            return guarded();
          case 2:
            _t6 = _context7.sent;
            _context7.next = 3;
            return service.total(3);
          case 3:
            _t4.log.call(_t4, _t5, _t6, _context7.sent);
            l = new Loader();
            _t7 = console;
            _context7.next = 4;
            return l.load("a");
          case 4:
            _t8 = _context7.sent;
            _context7.next = 5;
            return l.load("b");
          case 5:
            _t7.log.call(_t7, _t8, _context7.sent);
            arrow = /*#__PURE__*/function () {
              var _ref2 = _asyncToGenerator(/*#__PURE__*/_regeneratorRuntime.mark(function _callee6(n) {
                var _t3;
                return _regeneratorRuntime.wrap(function (_context6) {
                  while (1) switch (_context6.prev = _context6.next) {
                    case 0:
                      _context6.next = 1;
                      return later(n);
                    case 1:
                      _t3 = _context6.sent;
                      return _context6.abrupt("return", _t3 * 2);
                    case 2:
                    case "end":
                      return _context6.stop();
                  }
                }, _callee6);
              }));
              return function arrow(_x4) {
                return _ref2.apply(this, arguments);
              };
            }();
            _t9 = console;
            _context7.next = 6;
            return arrow(4);
          case 6:
            _t0 = _context7.sent;
            _context7.next = 7;
            return Promise.all([arrow(1), fetchTwice(2)]);
          case 7:
            _t9.log.call(_t9, _t0, _context7.sent.join(","));
            console.log(log.join("; "));
          case 8:
          case "end":
            return _context7.stop();
        }
      }, _callee7);
    }));
    return _main.apply(this, arguments);
  }
  main().catch(function (e) {
    return console.log("fail", e);
  });

})();
