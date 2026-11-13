"use strict";

function _createForOfIteratorHelperLoose(r, e) { var t = "undefined" != typeof Symbol && r[Symbol.iterator] || r["@@iterator"]; if (t) return (t = t.call(r)).next.bind(t); if (Array.isArray(r) || (t = _unsupportedIterableToArray(r)) || e && r && "number" == typeof r.length) { t && (r = t); var o = 0; return function () { return o >= r.length ? { done: !0 } : { done: !1, value: r[o++] }; }; } throw new TypeError("Invalid attempt to iterate non-iterable instance.\nIn order to be iterable, non-array objects must have a [Symbol.iterator]() method."); }
function _unsupportedIterableToArray(r, a) { if (r) { if ("string" == typeof r) return _arrayLikeToArray(r, a); var t = {}.toString.call(r).slice(8, -1); return "Object" === t && r.constructor && (t = r.constructor.name), "Map" === t || "Set" === t ? Array.from(r) : "Arguments" === t || /^(?:Ui|I)nt(?:8|16|32)(?:Clamped)?Array$/.test(t) ? _arrayLikeToArray(r, a) : void 0; } }
function _arrayLikeToArray(r, a) { (null == a || a > r.length) && (a = r.length); for (var e = 0, n = Array(a); e < a; e++) n[e] = r[e]; return n; }
var calls = [];
var handlers = [function () {
  return calls.push("a");
}, function () {
  return calls.push("b");
}];
for (var _iterator = _createForOfIteratorHelperLoose(new Set(handlers)), _step; !(_step = _iterator()).done;) {
  var handler = _step.value;
  handler();
}
var counts = {};
for (var _iterator2 = _createForOfIteratorHelperLoose(new Set(["x", "y", "x"])), _step2; !(_step2 = _iterator2()).done;) {
  var key = _step2.value;
  counts[key] = (counts[key] || 0) + 1;
}
for (var _iterator3 = _createForOfIteratorHelperLoose(new Set([{
    list: [1, 2]
  }, {
    list: [3]
  }])), _step3; !(_step3 = _iterator3()).done;) {
  var entry = _step3.value;
  for (var _iterator7 = _createForOfIteratorHelperLoose(entry.list), _step7; !(_step7 = _iterator7()).done;) {
    var n = _step7.value;
    calls.push(n);
  }
}
var removed = {
  a: 1,
  b: 2,
  c: 3
};
for (var _iterator4 = _createForOfIteratorHelperLoose(new Set(["a", "c"])), _step4; !(_step4 = _iterator4()).done;) {
  var _key = _step4.value;
  delete removed[_key];
}
var total = 0;
var weights = {
  p: 2,
  q: 5
};
for (var _iterator5 = _createForOfIteratorHelperLoose(new Set(["p", "q", "r"])), _step5; !(_step5 = _iterator5()).done;) {
  var name = _step5.value;
  total += weights[name] || 0;
}
for (var _iterator6 = _createForOfIteratorHelperLoose(new Set([{
    run: function run(k) {
      return calls.push(k);
    }
  }])), _step6; !(_step6 = _iterator6()).done;) {
  var item = _step6.value;
  item.run("ran");
}
console.log(calls.join(), JSON.stringify(counts), JSON.stringify(removed), total);
