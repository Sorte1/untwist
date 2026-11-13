(function () {
  'use strict';

  function _arrayWithHoles(r) {
    if (Array.isArray(r)) return r;
  }

  function _iterableToArrayLimit(r, l) {
    var t = null == r ? null : "undefined" != typeof Symbol && r[Symbol.iterator] || r["@@iterator"];
    if (null != t) {
      var e,
        n,
        i,
        u,
        a = [],
        f = true,
        o = false;
      try {
        if (i = (t = t.call(r)).next, 0 === l) {
          if (Object(t) !== t) return;
          f = !1;
        } else for (; !(f = (e = i.call(t)).done) && (a.push(e.value), a.length !== l); f = !0);
      } catch (r) {
        o = true, n = r;
      } finally {
        try {
          if (!f && null != t["return"] && (u = t["return"](), Object(u) !== u)) return;
        } finally {
          if (o) throw n;
        }
      }
      return a;
    }
  }

  function _arrayLikeToArray$1(r, a) {
    (null == a || a > r.length) && (a = r.length);
    for (var e = 0, n = Array(a); e < a; e++) n[e] = r[e];
    return n;
  }

  function _unsupportedIterableToArray(r, a) {
    if (r) {
      if ("string" == typeof r) return _arrayLikeToArray$1(r, a);
      var t = {}.toString.call(r).slice(8, -1);
      return "Object" === t && r.constructor && (t = r.constructor.name), "Map" === t || "Set" === t ? Array.from(r) : "Arguments" === t || /^(?:Ui|I)nt(?:8|16|32)(?:Clamped)?Array$/.test(t) ? _arrayLikeToArray$1(r, a) : void 0;
    }
  }

  function _nonIterableRest() {
    throw new TypeError("Invalid attempt to destructure non-iterable instance.\nIn order to be iterable, non-array objects must have a [Symbol.iterator]() method.");
  }

  function _slicedToArray(r, e) {
    return _arrayWithHoles(r) || _iterableToArrayLimit(r, e) || _unsupportedIterableToArray(r, e) || _nonIterableRest();
  }

  function _objectWithoutPropertiesLoose(r, e) {
    if (null == r) return {};
    var t = {};
    for (var n in r) if ({}.hasOwnProperty.call(r, n)) {
      if (-1 !== e.indexOf(n)) continue;
      t[n] = r[n];
    }
    return t;
  }

  function _objectWithoutProperties(e, t) {
    if (null == e) return {};
    var o,
      r,
      i = _objectWithoutPropertiesLoose(e, t);
    if (Object.getOwnPropertySymbols) {
      var n = Object.getOwnPropertySymbols(e);
      for (r = 0; r < n.length; r++) o = n[r], -1 === t.indexOf(o) && {}.propertyIsEnumerable.call(e, o) && (i[o] = e[o]);
    }
    return i;
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

  function _arrayLikeToArray(r, a) { (null == a || a > r.length) && (a = r.length); for (var e = 0, n = Array(a); e < a; e++) n[e] = r[e]; return n; }
  var _ref = [1, 2, undefined, 4, 5],
    a = _ref[0],
    _ref$ = _ref[2],
    b = _ref$ === void 0 ? 10 : _ref$,
    rest = _arrayLikeToArray(_ref).slice(3);
  var _x$q$r = {
      x: 1,
      q: 9,
      r: 8
    },
    x = _x$q$r.x,
    _x$q$r$y = _x$q$r.y,
    why = _x$q$r$y === void 0 ? 2 : _x$q$r$y,
    _x$q$r$z = _x$q$r.z,
    _x$q$r$z2 = _x$q$r$z === void 0 ? {
      deep: "d"
    } : _x$q$r$z,
    deep = _x$q$r$z2.deep,
    others = _objectWithoutProperties(_x$q$r, ["x", "y", "z"]);
  console.log(a, b, rest.join(","), x, why, deep, JSON.stringify(others));
  function point() {
    var _ref2 = arguments.length > 0 && arguments[0] !== undefined ? arguments[0] : {},
      _ref2$x = _ref2.x,
      x = _ref2$x === void 0 ? 0 : _ref2$x,
      _ref2$y = _ref2.y,
      y = _ref2$y === void 0 ? 0 : _ref2$y;
    var _ref3 = arguments.length > 1 && arguments[1] !== undefined ? arguments[1] : [],
      _ref4 = _slicedToArray(_ref3, 2),
      first = _ref4[0],
      second = _ref4[1];
    return x + y + (first || 0) + (second || 0);
  }
  console.log(point({
    x: 1
  }, [2, 3]), point());
  var m = 1;
  var n = 2;
  var _ref5 = [n, m];
  m = _ref5[0];
  n = _ref5[1];
  console.log(m, n);
  var pairs = [[1, "one"], [2, "two"]];
  for (var _i = 0, _pairs = pairs; _i < _pairs.length; _i++) {
    var _pairs$_i = _slicedToArray(_pairs[_i], 2),
      num = _pairs$_i[0],
      word = _pairs$_i[1];
    console.log(num, word);
  }
  var _hello = "hello",
    length = _hello.length;
  var _hi = "hi",
    _hi2 = _slicedToArray(_hi, 2),
    c1 = _hi2[0],
    c2 = _hi2[1];
  console.log(length, c1, c2);
  var data = {
    list: [{
      id: 7,
      tags: ["t1", "t2"]
    }]
  };
  var _data$list = _slicedToArray(data.list, 1),
    _data$list$ = _data$list[0],
    id = _data$list$.id,
    _data$list$$tags = _slicedToArray(_data$list$.tags, 2),
    second = _data$list$$tags[1];
  console.log(id, second);
  var key = "dyn";
  var _ref6 = {},
    _ref6$key = _ref6[key],
    dynamic = _ref6$key === void 0 ? "fallback" : _ref6$key;
  console.log(dynamic);
  function swap(_ref7) {
    var _ref8 = _slicedToArray(_ref7, 2),
      p = _ref8[0],
      q = _ref8[1];
    return [q, p];
  }
  console.log(swap([1, 2]).join(","));
  var _ref9 = /*#__PURE__*/_regeneratorRuntime.mark(function _callee() {
      return _regeneratorRuntime.wrap(function (_context) {
        while (1) switch (_context.prev = _context.next) {
          case 0:
            _context.next = 1;
            return "a";
          case 1:
            _context.next = 2;
            return "b";
          case 2:
            _context.next = 3;
            return "c";
          case 3:
          case "end":
            return _context.stop();
        }
      }, _callee);
    })(),
    _ref0 = _slicedToArray(_ref9, 2),
    g1 = _ref0[0],
    g2 = _ref0[1];
  console.log(g1, g2);

})();
