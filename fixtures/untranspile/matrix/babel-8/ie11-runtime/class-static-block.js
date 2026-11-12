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

  function _createClass(e, r, t) {
    return Object.defineProperty(e, "prototype", {
      writable: false
    }), e;
  }

  function _classCallCheck(a, n) {
    if (!(a instanceof n)) throw new TypeError("Cannot call a class as a function");
  }

  function _defineProperty(r, t, o) {
    return (t = toPropertyKey(t)) in r ? Object.defineProperty(r, t, {
      value: o,
      enumerable: true,
      configurable: true,
      writable: true
    }) : r[t] = o, r;
  }

  var _Config;
  var Config = /*#__PURE__*/_createClass(function Config() {
    _classCallCheck(this, Config);
  });
  _Config = Config;
  _defineProperty(Config, "defaults", {
    mode: "fast"
  });
  _defineProperty(Config, "ready", void 0);
  (function () {
    _Config.ready = Object.keys(_Config.defaults).length > 0;
    _Config.loaded = "yes";
  })();
  (function () {
    var local = 3;
    _Config.triple = local * 3;
  })();
  console.log(Config.ready, Config.loaded, Config.triple, typeof local === "undefined" ? "undefined" : _typeof(local));

})();
