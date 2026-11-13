"use strict";

var _extends2;

var _extends = Object.assign || function (target) { for (var i = 1; i < arguments.length; i++) { var source = arguments[i]; for (var key in source) { if (Object.prototype.hasOwnProperty.call(source, key)) { target[key] = source[key]; } } } return target; };

function _objectWithoutProperties(obj, keys) { var target = {}; for (var i in obj) { if (keys.indexOf(i) >= 0) continue; if (!Object.prototype.hasOwnProperty.call(obj, i)) continue; target[i] = obj[i]; } return target; }

var base = { a: 1, b: 2 };
var extra = { c: 3 };
var merged = _extends({}, base, extra, { d: 4 });
var overridden = _extends({}, base, { a: 9 });
console.log(JSON.stringify(merged), JSON.stringify(overridden));

var a = merged.a,
    others = _objectWithoutProperties(merged, ["a"]);

console.log(a, JSON.stringify(others));
function without(_ref) {
  var secret = _ref.secret,
      pub = _objectWithoutProperties(_ref, ["secret"]);

  return pub;
}
console.log(JSON.stringify(without({ secret: 1, open: 2 })));
var withGetter = _extends({ get g() {
    return "got";
  } });
console.log(Object.getOwnPropertyDescriptor(withGetter, "g").value);
var sym = Symbol("s");
var symObj = _extends((_extends2 = {}, _extends2[sym] = 1, _extends2.plain = 2, _extends2));
console.log(symObj[sym], symObj.plain);

var picked = symObj[sym],
    restSym = _objectWithoutProperties(symObj, [sym]);

console.log(picked, Object.keys(restSym).join(","));
console.log(JSON.stringify(_extends({}, null, undefined, "hi", [5])));
var order = [];
var tracked = { get x() {
    order.push("x");return 1;
  }, get y() {
    order.push("y");return 2;
  } };
var copy = _extends({}, tracked);
console.log(order.join(","), copy.x + copy.y);
