var _excluded = ["secret"];
function _typeof(o) { "@babel/helpers - typeof"; return _typeof = "function" == typeof Symbol && "symbol" == typeof Symbol.iterator ? function (o) { return typeof o; } : function (o) { return o && "function" == typeof Symbol && o.constructor === Symbol && o !== Symbol.prototype ? "symbol" : typeof o; }, _typeof(o); }
function _objectWithoutProperties(t, r) { if (null == t) return {}; var o, n, i = _objectWithoutPropertiesLoose(t, r); if (Object.getOwnPropertySymbols) { var l = Object.getOwnPropertySymbols(t); for (n = 0; n < l.length; n++) o = l[n], -1 === r.indexOf(o) && {}.propertyIsEnumerable.call(t, o) && (i[o] = t[o]); } return i; }
function _objectWithoutPropertiesLoose(r, e) { if (null == r) return {}; var t = {}; for (var n in r) if ({}.hasOwnProperty.call(r, n)) { if (-1 !== e.indexOf(n)) continue; t[n] = r[n]; } return t; }
function ownKeys(e, r) { var t = Object.keys(e); if (Object.getOwnPropertySymbols) { var o = Object.getOwnPropertySymbols(e); r && (o = o.filter(function (r) { return Object.getOwnPropertyDescriptor(e, r).enumerable; })), t.push.apply(t, o); } return t; }
function _objectSpread(r) { for (var t = 1; t < arguments.length; t++) { var o = null != arguments[t] ? arguments[t] : {}; t % 2 ? ownKeys(Object(o), !0).forEach(function (t) { _defineProperty(r, t, o[t]); }) : Object.getOwnPropertyDescriptors ? Object.defineProperties(r, Object.getOwnPropertyDescriptors(o)) : ownKeys(Object(o)).forEach(function (e) { Object.defineProperty(r, e, Object.getOwnPropertyDescriptor(o, e)); }); } return r; }
function _defineProperty(r, t, o) { return (t = _toPropertyKey(t)) in r ? Object.defineProperty(r, t, { value: o, enumerable: !0, configurable: !0, writable: !0 }) : r[t] = o, r; }
function _toPropertyKey(r) { var o = _toPrimitive(r, "string"); return "symbol" == _typeof(o) ? o : o + ""; }
function _toPrimitive(t, e) { if ("object" != _typeof(t) || !t) return t; var r; if ("undefined" != typeof Symbol && void 0 !== (r = t[Symbol.toPrimitive])) { var i = r.call(t, e || "default"); if ("object" != _typeof(i)) return i; throw new TypeError("@@toPrimitive must return a primitive value."); } return ("string" === e ? String : Number)(t); }
var base = {
  a: 1,
  b: 2
};
var extra = {
  c: 3
};
var merged = _objectSpread(_objectSpread(_objectSpread({}, base), extra), {}, {
  d: 4
});
var overridden = _objectSpread(_objectSpread({}, base), {}, {
  a: 9
});
console.log(JSON.stringify(merged), JSON.stringify(overridden));
var a = merged.a,
  others = _objectWithoutProperties(merged, ["a"]);
console.log(a, JSON.stringify(others));
function without(_ref) {
  var secret = _ref.secret,
    pub = _objectWithoutProperties(_ref, _excluded);
  return pub;
}
console.log(JSON.stringify(without({
  secret: 1,
  open: 2
})));
var withGetter = _objectSpread({}, {
  get g() {
    return "got";
  }
});
console.log(Object.getOwnPropertyDescriptor(withGetter, "g").value);
var sym = Symbol("s");
var symObj = _objectSpread({}, {
  [sym]: 1,
  plain: 2
});
console.log(symObj[sym], symObj.plain);
var picked = symObj[sym],
  restSym = _objectWithoutProperties(symObj, [sym].map(_toPropertyKey));
console.log(picked, Object.keys(restSym).join(","));
console.log(JSON.stringify(_objectSpread(_objectSpread(_objectSpread(_objectSpread({}, null), undefined), "hi"), [5])));
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
var copy = _objectSpread({}, tracked);
console.log(order.join(","), copy.x + copy.y);
