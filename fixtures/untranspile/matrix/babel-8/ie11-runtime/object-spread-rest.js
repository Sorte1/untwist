(function () {
  'use strict';

  function _typeof(o) {
    "@babel/helpers - typeof";

    return _typeof = "function" == typeof Symbol && "symbol" == typeof Symbol.iterator ? function (o) {
      return typeof o;
    } : function (o) {
      return o && "function" == typeof Symbol && o.constructor === Symbol && o !== Symbol.prototype ? "symbol" : typeof o;
    }, _typeof(o);
  }

  function toPrimitive(t, e) {
    if ("object" != _typeof(t) || !t) return t;
    var r;
    if ("undefined" != typeof Symbol && void 0 !== (r = t[Symbol.toPrimitive])) {
      var i = r.call(t, e);
      if ("object" != _typeof(i)) return i;
      throw new TypeError("@@toPrimitive must return a primitive value.");
    }
    return ("string" === e ? String : Number)(t);
  }

  function toPropertyKey(r) {
    var o = toPrimitive(r, "string");
    return "symbol" == _typeof(o) ? o : o + "";
  }

  function _defineProperty(r, t, o) {
    return (t = toPropertyKey(t)) in r ? Object.defineProperty(r, t, {
      value: o,
      enumerable: true,
      configurable: true,
      writable: true
    }) : r[t] = o, r;
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

  function ownKeys(e, r) {
    var t = Object.keys(e);
    if (Object.getOwnPropertySymbols) {
      var o = Object.getOwnPropertySymbols(e);
      r && (o = o.filter(function (r) {
        return Object.getOwnPropertyDescriptor(e, r).enumerable;
      })), t.push.apply(t, o);
    }
    return t;
  }
  function _objectSpread2(r) {
    for (var t = 1; t < arguments.length; t++) {
      var o = null != arguments[t] ? arguments[t] : {};
      t % 2 ? ownKeys(Object(o), true).forEach(function (t) {
        _defineProperty(r, t, o[t]);
      }) : Object.getOwnPropertyDescriptors ? Object.defineProperties(r, Object.getOwnPropertyDescriptors(o)) : ownKeys(Object(o)).forEach(function (e) {
        Object.defineProperty(r, e, Object.getOwnPropertyDescriptor(o, e));
      });
    }
    return r;
  }

  var _excluded = ["secret"];
  var base = {
    a: 1,
    b: 2
  };
  var extra = {
    c: 3
  };
  var merged = _objectSpread2(_objectSpread2(_objectSpread2({}, base), extra), {}, {
    d: 4
  });
  var overridden = _objectSpread2(_objectSpread2({}, base), {}, {
    a: 9
  });
  console.log(JSON.stringify(merged), JSON.stringify(overridden));
  var a = merged.a,
    others = _objectWithoutProperties(merged, ["a"]);
  console.log(a, JSON.stringify(others));
  function without(_ref) {
    _ref.secret;
      var pub = _objectWithoutProperties(_ref, _excluded);
    return pub;
  }
  console.log(JSON.stringify(without({
    secret: 1,
    open: 2
  })));
  var withGetter = _objectSpread2({}, {
    get g() {
      return "got";
    }
  });
  console.log(Object.getOwnPropertyDescriptor(withGetter, "g").value);
  var sym = Symbol("s");
  var symObj = _objectSpread2({}, _defineProperty(_defineProperty({}, sym, 1), "plain", 2));
  console.log(symObj[sym], symObj.plain);
  var picked = symObj[sym],
    restSym = _objectWithoutProperties(symObj, [sym].map(toPropertyKey));
  console.log(picked, Object.keys(restSym).join(","));
  console.log(JSON.stringify(_objectSpread2(_objectSpread2(_objectSpread2(_objectSpread2({}, null), undefined), "hi"), [5])));
  var order = [];
  var tracked = {
    get x() {
      order.push("x");
      return 1;
    },
    get y() {
      order.push("y");
      return 2;
    }
  };
  var copy = _objectSpread2({}, tracked);
  console.log(order.join(","), copy.x + copy.y);

})();
