"use strict";

var _obj;
function _superPropGet(t, o, e, r) { var p = _get(_getPrototypeOf(1 & r ? t.prototype : t), o, e); return 2 & r && "function" == typeof p ? function (t) { return p.apply(e, t); } : p; }
function _get() { return _get = "undefined" != typeof Reflect && Reflect.get ? Reflect.get.bind() : function (e, t, r) { var p = _superPropBase(e, t); if (p) { var n = Object.getOwnPropertyDescriptor(p, t); return n.get ? n.get.call(arguments.length < 3 ? e : r) : n.value; } }, _get.apply(null, arguments); }
function _superPropBase(t, o) { for (; !{}.hasOwnProperty.call(t, o) && null !== (t = _getPrototypeOf(t));); return t; }
function _getPrototypeOf(t) { return _getPrototypeOf = Object.setPrototypeOf ? Object.getPrototypeOf.bind() : function (t) { return t.__proto__ || Object.getPrototypeOf(t); }, _getPrototypeOf(t); }
var k = "dyn";
var x = 1;
var o = {
  x,
  [k + "1"]: 2,
  method() {
    return "m" + this.x;
  },
  get g() {
    return "get";
  },
  ["t".concat(k)]: 3,
  __proto__: {
    inherited: true
  }
};
console.log(o.x, o.dyn1, o.method(), o.g, o.tdyn, o.inherited, Object.keys(o).join(","));
var sup = {
  hello() {
    return "base";
  }
};
var sub = _obj = {
  __proto__: sup,
  hello() {
    return _superPropGet(_obj, "hello", this, 2)([]) + "+sub";
  }
};
console.log(sub.hello());
