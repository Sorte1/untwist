"use strict";

var _o, _obj;
function _get() { if (typeof Reflect !== "undefined" && Reflect.get) { _get = Reflect.get.bind(); } else { _get = function _get(target, property, receiver) { var base = _superPropBase(target, property); if (!base) return; var desc = Object.getOwnPropertyDescriptor(base, property); if (desc.get) { return desc.get.call(arguments.length < 3 ? target : receiver); } return desc.value; }; } return _get.apply(this, arguments); }
function _superPropBase(object, property) { while (!Object.prototype.hasOwnProperty.call(object, property)) { object = _getPrototypeOf(object); if (object === null) break; } return object; }
function _getPrototypeOf(o) { _getPrototypeOf = Object.setPrototypeOf ? Object.getPrototypeOf.bind() : function _getPrototypeOf(o) { return o.__proto__ || Object.getPrototypeOf(o); }; return _getPrototypeOf(o); }
function _defineAccessor(type, obj, key, fn) { var desc = { configurable: !0, enumerable: !0 }; return desc[type] = fn, Object.defineProperty(obj, key, desc); }
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
