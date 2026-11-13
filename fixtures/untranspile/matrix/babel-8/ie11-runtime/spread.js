(function () {
  'use strict';

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

  function _isNativeReflectConstruct() {
    try {
      var t = !Boolean.prototype.valueOf.call(Reflect.construct(Boolean, [], function () {}));
    } catch (t) {}
    return (_isNativeReflectConstruct = function _isNativeReflectConstruct() {
      return !!t;
    })();
  }

  function _construct(e, o, p) {
    if (_isNativeReflectConstruct()) return Reflect.construct.apply(null, arguments);
    var n = [null];
    n.push.apply(n, o);
    var l = new (e.bind.apply(e, n))();
    return l;
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

  function _arrayWithoutHoles(a) {
    if (Array.isArray(a)) return _arrayLikeToArray(a);
  }

  function _iterableToArray(r) {
    var e = "undefined" != typeof Symbol && r[Symbol.iterator] || r["@@iterator"];
    if (Array.isArray(r) && (null == e || e === ("undefined" != typeof Symbol && [][Symbol.iterator]) || e === []["@@iterator"])) return r;
    if (null != e) {
      for (var t, o = [], a = e.call(r); !(t = a.next()).done;) o.push(t.value);
      return o;
    }
  }

  function _unsupportedIterableToArray(t, e) {
    if (t) {
      if ("string" == typeof t) return _arrayLikeToArray(t, e);
      var o = {}.toString.call(t).slice(8, -1);
      return "Object" === o && t.constructor && (o = t.constructor.name), "Map" === o || "Set" === o ? Array.from(t) : "Arguments" === o || /^(?:Ui|I)nt(?:8|16|32)(?:Clamped)?Array$/.test(o) ? _arrayLikeToArray(t, e) : void 0;
    }
  }

  function _nonIterableSpread() {
    throw new TypeError("Invalid attempt to spread non-iterable instance.\nIn order to be iterable, non-array objects must have a [Symbol.iterator]() method.");
  }

  function _toConsumableArray(a) {
    return _arrayWithoutHoles(a) || _iterableToArray(a) || _unsupportedIterableToArray(a) || _nonIterableSpread();
  }

  var _obj$vals;
  var _marked = /*#__PURE__*/_regenerator().m(g);
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
    return _regenerator().w(function (_context) {
      while (1) switch (_context.n) {
        case 0:
          _context.n = 1;
          return "x";
        case 1:
          _context.n = 2;
          return "y";
        case 2:
          return _context.a(2);
      }
    }, _marked);
  }
  attempt("generator", function () {
    return [].concat(_toConsumableArray(g()), _toConsumableArray(g())).join("");
  });

})();
