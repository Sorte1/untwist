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

  function _iterableToArrayLimit(r, t) {
    var e = null == r ? null : "undefined" != typeof Symbol && r[Symbol.iterator] || r["@@iterator"];
    if (Array.isArray(r) && (null == e || e === ("undefined" != typeof Symbol && [][Symbol.iterator]) || e === []["@@iterator"])) return r;
    if (null != e) {
      var l,
        n,
        i,
        a,
        u = [],
        o = true,
        f = false;
      try {
        if (i = (e = e.call(r)).next, 0 === t) {
          if (Object(e) !== e) return;
          o = !1;
        } else for (; !(o = (l = i.call(e)).done) && (u.push(l.value), u.length !== t); o = !0);
      } catch (r) {
        f = true, n = r;
      } finally {
        try {
          if (!o && null != e["return"] && (a = e["return"](), Object(a) !== a)) return;
        } finally {
          if (f) throw n;
        }
      }
      return u;
    }
  }

  function _arrayLikeToArray(r, a) {
    (null == a || a > r.length) && (a = r.length);
    for (var e = 0, n = Array(a); e < a; e++) n[e] = r[e];
    return n;
  }

  function _unsupportedIterableToArray(t, e) {
    if (t) {
      if ("string" == typeof t) return _arrayLikeToArray(t, e);
      var o = {}.toString.call(t).slice(8, -1);
      return "Object" === o && t.constructor && (o = t.constructor.name), "Map" === o || "Set" === o ? Array.from(t) : "Arguments" === o || /^(?:Ui|I)nt(?:8|16|32)(?:Clamped)?Array$/.test(o) ? _arrayLikeToArray(t, e) : void 0;
    }
  }

  function _nonIterableRest() {
    throw new TypeError("Invalid attempt to destructure non-iterable instance.\nIn order to be iterable, non-array objects must have a [Symbol.iterator]() method.");
  }

  function _slicedToArray(e, i) {
    return _iterableToArrayLimit(e, i) || _unsupportedIterableToArray(e, i) || _nonIterableRest();
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

  function _objectWithoutProperties(t, r) {
    if (null == t) return {};
    var o,
      n,
      i = _objectWithoutPropertiesLoose(t, r);
    if (Object.getOwnPropertySymbols) {
      var l = Object.getOwnPropertySymbols(t);
      for (n = 0; n < l.length; n++) o = l[n], -1 === r.indexOf(o) && {}.propertyIsEnumerable.call(t, o) && (i[o] = t[o]);
    }
    return i;
  }

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
  var _ref9 = /*#__PURE__*/_regenerator().m(function _callee() {
      return _regenerator().w(function (_context) {
        while (1) switch (_context.n) {
          case 0:
            _context.n = 1;
            return "a";
          case 1:
            _context.n = 2;
            return "b";
          case 2:
            _context.n = 3;
            return "c";
          case 3:
            return _context.a(2);
        }
      }, _callee);
    })(),
    _ref0 = _slicedToArray(_ref9, 2),
    g1 = _ref0[0],
    g2 = _ref0[1];
  console.log(g1, g2);

})();
