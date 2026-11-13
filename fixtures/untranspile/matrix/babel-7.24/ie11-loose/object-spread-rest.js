"use strict";

var _extends2;
var _excluded = ["secret"];
function _toPropertyKey(t) { var i = _toPrimitive(t, "string"); return "symbol" == typeof i ? i : i + ""; }
function _toPrimitive(t, r) { if ("object" != typeof t || !t) return t; var e = t[Symbol.toPrimitive]; if (void 0 !== e) { var i = e.call(t, r || "default"); if ("object" != typeof i) return i; throw new TypeError("@@toPrimitive must return a primitive value."); } return ("string" === r ? String : Number)(t); }
function _objectWithoutPropertiesLoose(r, e) { if (null == r) return {}; var t = {}; for (var n in r) if ({}.hasOwnProperty.call(r, n)) { if (e.includes(n)) continue; t[n] = r[n]; } return t; }
function _extends() { return _extends = Object.assign ? Object.assign.bind() : function (n) { for (var e = 1; e < arguments.length; e++) { var t = arguments[e]; for (var r in t) ({}).hasOwnProperty.call(t, r) && (n[r] = t[r]); } return n; }, _extends.apply(null, arguments); }
var base = {
  a: 1,
  b: 2
};
var extra = {
  c: 3
};
var merged = _extends({}, base, extra, {
  d: 4
});
var overridden = _extends({}, base, {
  a: 9
});
console.log(JSON.stringify(merged), JSON.stringify(overridden));
var a = merged.a,
  others = _objectWithoutPropertiesLoose(merged, ["a"]);
console.log(a, JSON.stringify(others));
function without(_ref) {
  var secret = _ref.secret,
    pub = _objectWithoutPropertiesLoose(_ref, _excluded);
  return pub;
}
console.log(JSON.stringify(without({
  secret: 1,
  open: 2
})));
var withGetter = _extends({}, {
  get g() {
    return "got";
  }
});
console.log(Object.getOwnPropertyDescriptor(withGetter, "g").value);
var sym = Symbol("s");
var symObj = _extends({}, (_extends2 = {}, _extends2[sym] = 1, _extends2.plain = 2, _extends2));
console.log(symObj[sym], symObj.plain);
var picked = symObj[sym],
  restSym = _objectWithoutPropertiesLoose(symObj, [sym].map(_toPropertyKey));
console.log(picked, Object.keys(restSym).join(","));
console.log(JSON.stringify(_extends({}, null, undefined, "hi", [5])));
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
var copy = _extends({}, tracked);
console.log(order.join(","), copy.x + copy.y);
