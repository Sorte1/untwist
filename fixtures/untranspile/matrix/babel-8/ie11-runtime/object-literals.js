(function () {
  'use strict';

  function _getPrototypeOf(t) {
    return _getPrototypeOf = Object.setPrototypeOf ? Object.getPrototypeOf.bind() : function (t) {
      return t.__proto__ || Object.getPrototypeOf(t);
    }, _getPrototypeOf(t);
  }

  function _superPropBase(o, t) {
    for (; !{}.hasOwnProperty.call(o, t) && null !== (o = _getPrototypeOf(o)););
    return o;
  }

  function _get() {
    return _get = "undefined" != typeof Reflect && Reflect.get ? Reflect.get.bind() : function (t, r, n) {
      var l = _superPropBase(t, r);
      if (l) {
        var f = Object.getOwnPropertyDescriptor(l, r);
        return f.get ? f.get.call(arguments.length < 3 ? t : n) : f.value;
      }
    }, _get.apply(null, arguments);
  }

  function _superPropGet(r, e, p, f) {
    var n = _get(_getPrototypeOf(r), e, p);
    return "function" == typeof n ? function (t) {
      return n.apply(p, t);
    } : n;
  }

  function _defineAccessor(e, r, n, t) {
    var c = {
      configurable: true,
      enumerable: true
    };
    return c[e] = t, Object.defineProperty(r, n, c);
  }

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

  var _obj;
  var k = "dyn";
  var x = 1;
  var o = _defineProperty(_defineProperty(_defineAccessor("get", _defineProperty(_defineProperty({
    x: x
  }, k + "1", 2), "method", function method() {
    return "m" + this.x;
  }), "g", function () {
    return "get";
  }), "t".concat(k), 3), "__proto__", {
    inherited: true
  });
  console.log(o.x, o.dyn1, o.method(), o.g, o.tdyn, o.inherited, Object.keys(o).join(","));
  var sup = {
    hello: function hello() {
      return "base";
    }
  };
  var sub = _obj = {
    __proto__: sup,
    hello: function hello() {
      return _superPropGet(_obj, "hello", this)([]) + "+sub";
    }
  };
  console.log(sub.hello());

})();
