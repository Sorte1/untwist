(function () {
  'use strict';

  function _getPrototypeOf(t) {
    return _getPrototypeOf = Object.setPrototypeOf ? Object.getPrototypeOf.bind() : function (t) {
      return t.__proto__ || Object.getPrototypeOf(t);
    }, _getPrototypeOf(t);
  }

  function _superPropBase(t, o) {
    for (; !{}.hasOwnProperty.call(t, o) && null !== (t = _getPrototypeOf(t)););
    return t;
  }

  function _get() {
    return _get = "undefined" != typeof Reflect && Reflect.get ? Reflect.get.bind() : function (e, t, r) {
      var p = _superPropBase(e, t);
      if (p) {
        var n = Object.getOwnPropertyDescriptor(p, t);
        return n.get ? n.get.call(arguments.length < 3 ? e : r) : n.value;
      }
    }, _get.apply(null, arguments);
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

  function toPropertyKey(t) {
    var i = toPrimitive(t, "string");
    return "symbol" == _typeof(i) ? i : i + "";
  }

  function _defineProperty(e, r, t) {
    return (r = toPropertyKey(r)) in e ? Object.defineProperty(e, r, {
      value: t,
      enumerable: true,
      configurable: true,
      writable: true
    }) : e[r] = t, e;
  }

  var _obj;
  function _superPropGet(t, o, e, r) { var p = _get(_getPrototypeOf(t), o, e); return "function" == typeof p ? function (t) { return p.apply(e, t); } : p; }
  function _defineAccessor(e, r, n, t) { var c = { configurable: true, enumerable: true }; return c[e] = t, Object.defineProperty(r, n, c); }
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
