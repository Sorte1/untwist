"use strict";

var _get = function get(object, property, receiver) { if (object === null) object = Function.prototype; var desc = Object.getOwnPropertyDescriptor(object, property); if (desc === undefined) { var parent = Object.getPrototypeOf(object); if (parent === null) { return undefined; } else { return get(parent, property, receiver); } } else if ("value" in desc) { return desc.value; } else { var getter = desc.get; if (getter === undefined) { return undefined; } return getter.call(receiver); } };

var _g, _o, _mutatorMap, _obj;

function _defineEnumerableProperties(obj, descs) { for (var key in descs) { var desc = descs[key]; desc.configurable = desc.enumerable = true; if ("value" in desc) desc.writable = true; Object.defineProperty(obj, key, desc); } return obj; }

function _defineProperty(obj, key, value) { if (key in obj) { Object.defineProperty(obj, key, { value: value, enumerable: true, configurable: true, writable: true }); } else { obj[key] = value; } return obj; }

var k = "dyn";
var x = 1;
var o = (_o = {
  x: x
}, _defineProperty(_o, k + "1", 2), _defineProperty(_o, "method", function method() {
  return "m" + this.x;
}), _g = "g", _mutatorMap = {}, _mutatorMap[_g] = _mutatorMap[_g] || {}, _mutatorMap[_g].get = function () {
  return "get";
}, _defineProperty(_o, "t" + k, 3), _o.__proto__ = { inherited: true }, _defineEnumerableProperties(_o, _mutatorMap), _o);
console.log(o.x, o.dyn1, o.method(), o.g, o.tdyn, o.inherited, Object.keys(o).join(","));
var sup = {
  hello: function hello() {
    return "base";
  }
};
var sub = _obj = {
  __proto__: sup,
  hello: function hello() {
    return _get(_obj.__proto__ || Object.getPrototypeOf(_obj), "hello", this).call(this) + "+sub";
  }
};
console.log(sub.hello());
