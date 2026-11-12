var _obj;
function _superPropGet(r, e, p, f) { var n = _get(_getPrototypeOf(1 & f ? r.prototype : r), e, p); return 2 & f && "function" == typeof n ? function (t) { return n.apply(p, t); } : n; }
function _get() { return _get = "undefined" != typeof Reflect && Reflect.get ? Reflect.get.bind() : function (t, r, n) { var l = _superPropBase(t, r); if (l) { var f = Object.getOwnPropertyDescriptor(l, r); return f.get ? f.get.call(arguments.length < 3 ? t : n) : f.value; } }, _get.apply(null, arguments); }
function _superPropBase(o, t) { for (; !{}.hasOwnProperty.call(o, t) && null !== (o = _getPrototypeOf(o));); return o; }
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
  [`t${k}`]: 3,
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
