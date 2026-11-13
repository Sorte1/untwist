"use strict";

var _o, _obj;
function _get() { return _get = "undefined" != typeof Reflect && Reflect.get ? Reflect.get.bind() : function (e, t, r) { var p = _superPropBase(e, t); if (p) { var n = Object.getOwnPropertyDescriptor(p, t); return n.get ? n.get.call(arguments.length < 3 ? e : r) : n.value; } }, _get.apply(null, arguments); }
function _superPropBase(t, o) { for (; !{}.hasOwnProperty.call(t, o) && null !== (t = _getPrototypeOf(t));); return t; }
function _getPrototypeOf(t) { return _getPrototypeOf = Object.setPrototypeOf ? Object.getPrototypeOf.bind() : function (t) { return t.__proto__ || Object.getPrototypeOf(t); }, _getPrototypeOf(t); }
function _defineAccessor(e, r, n, t) { var c = { configurable: !0, enumerable: !0 }; return c[e] = t, Object.defineProperty(r, n, c); }
var k = "dyn";
var x = 1;
var o = (_o = {
  x: x
}, _o[k + "1"] = 2, _o.method = function method() {
  return "m" + this.x;
}, _defineAccessor("get", _o, "g", function () {
  return "get";
}), _o["t" + k] = 3, _o.__proto__ = {
  inherited: true
}, _o);
console.log(o.x, o.dyn1, o.method(), o.g, o.tdyn, o.inherited, Object.keys(o).join(","));
var sup = {
  hello: function hello() {
    return "base";
  }
};
var sub = _obj = {
  __proto__: sup,
  hello: function hello() {
    return _get(_getPrototypeOf(_obj), "hello", this).call(this) + "+sub";
  }
};
console.log(sub.hello());
