(function () {
  'use strict';

  function _isNativeReflectConstruct() {
    try {
      var t = !Boolean.prototype.valueOf.call(Reflect.construct(Boolean, [], function () {}));
    } catch (t) {}
    return (_isNativeReflectConstruct = function _isNativeReflectConstruct() {
      return !!t;
    })();
  }

  function _construct(t, e, r) {
    if (_isNativeReflectConstruct()) return Reflect.construct.apply(null, arguments);
    var o = [null];
    o.push.apply(o, e);
    var p = new (t.bind.apply(t, o))();
    return p;
  }

  function _createClass(e, r, t) {
    return Object.defineProperty(e, "prototype", {
      writable: false
    }), e;
  }

  function _classCallCheck(a, n) {
    if (!(a instanceof n)) throw new TypeError("Cannot call a class as a function");
  }

  function _arrayLikeToArray(r, a) {
    (null == a || a > r.length) && (a = r.length);
    for (var e = 0, n = Array(a); e < a; e++) n[e] = r[e];
    return n;
  }

  function _arrayWithoutHoles(r) {
    if (Array.isArray(r)) return _arrayLikeToArray(r);
  }

  function _iterableToArray(r) {
    if ("undefined" != typeof Symbol && null != r[Symbol.iterator] || null != r["@@iterator"]) return Array.from(r);
  }

  function _unsupportedIterableToArray(r, a) {
    if (r) {
      if ("string" == typeof r) return _arrayLikeToArray(r, a);
      var t = {}.toString.call(r).slice(8, -1);
      return "Object" === t && r.constructor && (t = r.constructor.name), "Map" === t || "Set" === t ? Array.from(r) : "Arguments" === t || /^(?:Ui|I)nt(?:8|16|32)(?:Clamped)?Array$/.test(t) ? _arrayLikeToArray(r, a) : void 0;
    }
  }

  function _nonIterableSpread() {
    throw new TypeError("Invalid attempt to spread non-iterable instance.\nIn order to be iterable, non-array objects must have a [Symbol.iterator]() method.");
  }

  function _toConsumableArray(r) {
    return _arrayWithoutHoles(r) || _iterableToArray(r) || _unsupportedIterableToArray(r) || _nonIterableSpread();
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

  var _obj$vals;
  var _marked = /*#__PURE__*/_regeneratorRuntime.mark(g);
  function sum() {
    for (var _len = arguments.length, nums = new Array(_len), _key = 0; _key < _len; _key++) {
      nums[_key] = arguments[_key];
    }
    return nums.reduce(function (a, b) {
      return a + b;
    }, 0);
  }
  var a = [1, 2];
  var b = [3, 4];
  console.log([].concat(a, b, [5]).join(","), sum.apply(void 0, a.concat(b)), sum.apply(void 0, [0].concat(a)));
  function attempt(label, f) {
    try {
      console.log(label, f());
    } catch (e) {
      console.log(label, e.constructor.name);
    }
  }
  attempt("string", function () {
    return _toConsumableArray("abc").join("-");
  });
  attempt("set", function () {
    return _toConsumableArray(new Set([1, 1, 2])).join(",");
  });
  var holes = [, 1];
  console.log([].concat(holes).length, 0 in [].concat(holes));
  console.log(Math.max.apply(Math, a.concat(b)));
  var Box = /*#__PURE__*/_createClass(function Box() {
    _classCallCheck(this, Box);
    for (var _len2 = arguments.length, items = new Array(_len2), _key2 = 0; _key2 < _len2; _key2++) {
      items[_key2] = arguments[_key2];
    }
    this.items = items;
  });
  console.log(_construct(Box, a.concat([9])).items.join(","));
  var obj = {
    vals: [7, 8],
    total: function total() {
      return sum.apply(void 0, _toConsumableArray(this.vals));
    }
  };
  console.log(obj.total(), (_obj$vals = obj.vals).concat.apply(_obj$vals, [[9], [10]]).join(","));
  function g() {
    return _regeneratorRuntime.wrap(function (_context) {
      while (1) switch (_context.prev = _context.next) {
        case 0:
          _context.next = 1;
          return "x";
        case 1:
          _context.next = 2;
          return "y";
        case 2:
        case "end":
          return _context.stop();
      }
    }, _marked);
  }
  attempt("generator", function () {
    return [].concat(_toConsumableArray(g()), _toConsumableArray(g())).join("");
  });

})();
