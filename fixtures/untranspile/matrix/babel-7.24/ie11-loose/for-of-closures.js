"use strict";

function _createForOfIteratorHelperLoose(r, e) { var t = "undefined" != typeof Symbol && r[Symbol.iterator] || r["@@iterator"]; if (t) return (t = t.call(r)).next.bind(t); if (Array.isArray(r) || (t = _unsupportedIterableToArray(r)) || e && r && "number" == typeof r.length) { t && (r = t); var o = 0; return function () { return o >= r.length ? { done: !0 } : { done: !1, value: r[o++] }; }; } throw new TypeError("Invalid attempt to iterate non-iterable instance.\nIn order to be iterable, non-array objects must have a [Symbol.iterator]() method."); }
function _unsupportedIterableToArray(r, a) { if (r) { if ("string" == typeof r) return _arrayLikeToArray(r, a); var t = {}.toString.call(r).slice(8, -1); return "Object" === t && r.constructor && (t = r.constructor.name), "Map" === t || "Set" === t ? Array.from(r) : "Arguments" === t || /^(?:Ui|I)nt(?:8|16|32)(?:Clamped)?Array$/.test(t) ? _arrayLikeToArray(r, a) : void 0; } }
function _arrayLikeToArray(r, a) { (null == a || a > r.length) && (a = r.length); for (var e = 0, n = Array(a); e < a; e++) n[e] = r[e]; return n; }
var fns = [];
var _loop = function _loop() {
  var v = _step.value;
  var doubled = v * 2;
  var note = "n" + v;
  fns.push(function () {
    return doubled + note;
  });
  note += "!";
};
for (var _iterator = _createForOfIteratorHelperLoose(new Set([1, 2, 3])), _step; !(_step = _iterator()).done;) {
  _loop();
}
console.log(fns.map(function (f) {
  return f();
}).join(","));
var seen = [];
var _loop2 = function _loop2() {
  var _step2$value = _step2.value,
    key = _step2$value[0],
    value = _step2$value[1];
  seen.push(function () {
    return key + value;
  });
};
for (var _iterator2 = _createForOfIteratorHelperLoose(new Map([["a", 1], ["b", 2]])), _step2; !(_step2 = _iterator2()).done;) {
  _loop2();
}
console.log(seen.map(function (f) {
  return f();
}).join(","));
var later = [];
var _loop3 = function _loop3() {
  var word = _step3.value;
  var count = 0;
  later.push(function () {
    return word + ++count;
  });
};
for (var _iterator3 = _createForOfIteratorHelperLoose("abc"), _step3; !(_step3 = _iterator3()).done;) {
  _loop3();
}
console.log(later.map(function (f) {
  return f() + f();
}).join(","));
