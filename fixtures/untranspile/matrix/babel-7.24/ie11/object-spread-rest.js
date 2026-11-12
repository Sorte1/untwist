"use strict";

var _excluded = ["secret"];
function _typeof(o) { "@babel/helpers - typeof"; return _typeof = "function" == typeof Symbol && "symbol" == typeof Symbol.iterator ? function (o) { return typeof o; } : function (o) { return o && "function" == typeof Symbol && o.constructor === Symbol && o !== Symbol.prototype ? "symbol" : typeof o; }, _typeof(o); }
function _objectWithoutProperties(e, t) { if (null == e) return {}; var o, r, i = _objectWithoutPropertiesLoose(e, t); if (Object.getOwnPropertySymbols) { var s = Object.getOwnPropertySymbols(e); for (r = 0; r < s.length; r++) o = s[r], t.includes(o) || {}.propertyIsEnumerable.call(e, o) && (i[o] = e[o]); } return i; }
function _objectWithoutPropertiesLoose(r, e) { if (null == r) return {}; var t = {}; for (var n in r) if ({}.hasOwnProperty.call(r, n)) { if (e.includes(n)) continue; t[n] = r[n]; } return t; }
function ownKeys(e, r) { var t = Object.keys(e); if (Object.getOwnPropertySymbols) { var o = Object.getOwnPropertySymbols(e); r && (o = o.filter(function (r) { return Object.getOwnPropertyDescriptor(e, r).enumerable; })), t.push.apply(t, o); } return t; }
function _objectSpread(e) { for (var r = 1; r < arguments.length; r++) { var t = null != arguments[r] ? arguments[r] : {}; r % 2 ? ownKeys(Object(t), !0).forEach(function (r) { _defineProperty(e, r, t[r]); }) : Object.getOwnPropertyDescriptors ? Object.defineProperties(e, Object.getOwnPropertyDescriptors(t)) : ownKeys(Object(t)).forEach(function (r) { Object.defineProperty(e, r, Object.getOwnPropertyDescriptor(t, r)); }); } return e; }
function _defineProperty(e, r, t) { return (r = _toPropertyKey(r)) in e ? Object.defineProperty(e, r, { value: t, enumerable: !0, configurable: !0, writable: !0 }) : e[r] = t, e; }
function _toPropertyKey(t) { var i = _toPrimitive(t, "string"); return "symbol" == _typeof(i) ? i : i + ""; }
function _toPrimitive(t, r) { if ("object" != _typeof(t) || !t) return t; var e = t[Symbol.toPrimitive]; if (void 0 !== e) { var i = e.call(t, r || "default"); if ("object" != _typeof(i)) return i; throw new TypeError("@@toPrimitive must return a primitive value."); } return ("string" === r ? String : Number)(t); }
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
var symObj = _objectSpread({}, _defineProperty(_defineProperty({}, sym, 1), "plain", 2));
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
