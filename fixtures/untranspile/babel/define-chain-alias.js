function _defineProperty(e, r, t) { return (r = _toPropertyKey(r)) in e ? Object.defineProperty(e, r, { value: t, enumerable: !0, configurable: !0, writable: !0 }) : e[r] = t, e; }
function _toPropertyKey(t) { var i = _toPrimitive(t, "string"); return "symbol" == _typeof(i) ? i : i + ""; }
function _toPrimitive(t, r) { if ("object" != _typeof(t) || !t) return t; var e = t[Symbol.toPrimitive]; if (void 0 !== e) { var i = e.call(t, r || "default"); if ("object" != _typeof(i)) return i; throw new TypeError("@@toPrimitive must return a primitive value."); } return ("string" === r ? String : Number)(t); }
function _typeof(o) { "@babel/helpers - typeof"; return _typeof = "function" == typeof Symbol && "symbol" == typeof Symbol.iterator ? function (o) { return typeof o; } : function (o) { return o && "function" == typeof Symbol && o.constructor === Symbol && o !== Symbol.prototype ? "symbol" : typeof o; }, _typeof(o); }
function show(label, o) {
  var parts = Object.getOwnPropertyNames(o).map(function (name) {
    var d = Object.getOwnPropertyDescriptor(o, name);
    return name + "=" + JSON.stringify(d.value) + (d.enumerable ? "e" : "") + (d.writable ? "w" : "") + (d.configurable ? "c" : "");
  });
  console.log(label, parts.join(" "));
}
var O = Object;
var define = O.defineProperty;
O.defineProperty = function (o, k, d) {
  return define(o, k, { value: "patched:" + d.value, enumerable: true, writable: true, configurable: true });
};
show("inherited-keys", _defineProperty(_defineProperty({}, "toString", 1), "valueOf", 2));
show("own-keys", _defineProperty(_defineProperty({}, "a", 1), "b", 2));
