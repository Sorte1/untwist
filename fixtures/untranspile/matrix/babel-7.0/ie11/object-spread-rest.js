"use strict";

var _objectSpread2;

function _objectWithoutProperties(source, excluded) { if (source == null) return {}; var target = _objectWithoutPropertiesLoose(source, excluded); var key, i; if (Object.getOwnPropertySymbols) { var sourceSymbolKeys = Object.getOwnPropertySymbols(source); for (i = 0; i < sourceSymbolKeys.length; i++) { key = sourceSymbolKeys[i]; if (excluded.indexOf(key) >= 0) continue; if (!Object.prototype.propertyIsEnumerable.call(source, key)) continue; target[key] = source[key]; } } return target; }

function _objectWithoutPropertiesLoose(source, excluded) { if (source == null) return {}; var target = {}; var sourceKeys = Object.keys(source); var key, i; for (i = 0; i < sourceKeys.length; i++) { key = sourceKeys[i]; if (excluded.indexOf(key) >= 0) continue; target[key] = source[key]; } return target; }

function _objectSpread(target) { for (var i = 1; i < arguments.length; i++) { var source = arguments[i] != null ? arguments[i] : {}; var ownKeys = Object.keys(source); if (typeof Object.getOwnPropertySymbols === 'function') { ownKeys = ownKeys.concat(Object.getOwnPropertySymbols(source).filter(function (sym) { return Object.getOwnPropertyDescriptor(source, sym).enumerable; })); } ownKeys.forEach(function (key) { _defineProperty(target, key, source[key]); }); } return target; }

function _defineProperty(obj, key, value) { if (key in obj) { Object.defineProperty(obj, key, { value: value, enumerable: true, configurable: true, writable: true }); } else { obj[key] = value; } return obj; }

var base = {
  a: 1,
  b: 2
};
var extra = {
  c: 3
};

var merged = _objectSpread({}, base, extra, {
  d: 4
});

var overridden = _objectSpread({}, base, {
  a: 9
});

console.log(JSON.stringify(merged), JSON.stringify(overridden));

var a = merged.a,
    others = _objectWithoutProperties(merged, ["a"]);

console.log(a, JSON.stringify(others));

function without(_ref) {
  var secret = _ref.secret,
      pub = _objectWithoutProperties(_ref, ["secret"]);

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

var symObj = _objectSpread({}, (_objectSpread2 = {}, _defineProperty(_objectSpread2, sym, 1), _defineProperty(_objectSpread2, "plain", 2), _objectSpread2));

console.log(symObj[sym], symObj.plain);

var picked = symObj[sym],
    restSym = _objectWithoutProperties(symObj, [sym]);

console.log(picked, Object.keys(restSym).join(","));
console.log(JSON.stringify(_objectSpread({}, null, undefined, "hi", [5])));
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
