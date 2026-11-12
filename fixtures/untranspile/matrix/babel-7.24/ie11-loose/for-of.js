"use strict";

var _custom;
function _createForOfIteratorHelperLoose(r, e) { var t = "undefined" != typeof Symbol && r[Symbol.iterator] || r["@@iterator"]; if (t) return (t = t.call(r)).next.bind(t); if (Array.isArray(r) || (t = _unsupportedIterableToArray(r)) || e && r && "number" == typeof r.length) { t && (r = t); var o = 0; return function () { return o >= r.length ? { done: !0 } : { done: !1, value: r[o++] }; }; } throw new TypeError("Invalid attempt to iterate non-iterable instance.\nIn order to be iterable, non-array objects must have a [Symbol.iterator]() method."); }
function _unsupportedIterableToArray(r, a) { if (r) { if ("string" == typeof r) return _arrayLikeToArray(r, a); var t = {}.toString.call(r).slice(8, -1); return "Object" === t && r.constructor && (t = r.constructor.name), "Map" === t || "Set" === t ? Array.from(r) : "Arguments" === t || /^(?:Ui|I)nt(?:8|16|32)(?:Clamped)?Array$/.test(t) ? _arrayLikeToArray(r, a) : void 0; } }
function _arrayLikeToArray(r, a) { (null == a || a > r.length) && (a = r.length); for (var e = 0, n = Array(a); e < a; e++) n[e] = r[e]; return n; }
var out = [];
for (var _i = 0, _arr = [1, 2, 3]; _i < _arr.length; _i++) {
  var v = _arr[_i];
  out.push(v * 2);
}
for (var _iterator = _createForOfIteratorHelperLoose("héllo"), _step; !(_step = _iterator()).done;) {
  var ch = _step.value;
  out.push(ch);
}
for (var _iterator2 = _createForOfIteratorHelperLoose(new Map([["a", 1], ["b", 2]])), _step2; !(_step2 = _iterator2()).done;) {
  var _step2$value = _step2.value,
    k = _step2$value[0],
    _v = _step2$value[1];
  out.push(k + _v);
}
for (var _iterator3 = _createForOfIteratorHelperLoose(new Set([3, 3, 4])), _step3; !(_step3 = _iterator3()).done;) {
  var _v2 = _step3.value;
  out.push(_v2);
}
console.log(out.join(","));
var custom = (_custom = {
  from: 1,
  to: 4
}, _custom[Symbol.iterator] = function () {
  var cur = this.from;
  var last = this.to;
  return {
    next: function next() {
      return cur <= last ? {
        value: cur++,
        done: false
      } : {
        value: undefined,
        done: true
      };
    },
    return: function _return() {
      out.push("closed");
      return {
        done: true
      };
    }
  };
}, _custom);
var got = [];
for (var _iterator4 = _createForOfIteratorHelperLoose(custom), _step4; !(_step4 = _iterator4()).done;) {
  var _v3 = _step4.value;
  if (_v3 === 3) break;
  got.push(_v3);
}
console.log(got.join(","), out[out.length - 1]);
function args() {
  var xs = [];
  for (var _iterator5 = _createForOfIteratorHelperLoose(arguments), _step5; !(_step5 = _iterator5()).done;) {
    var a = _step5.value;
    xs.push(a);
  }
  return xs.join("+");
}
console.log(args(1, 2, 3));
var total = 0;
outer: for (var _i2 = 0, _arr2 = [[1, 2], [3, 4], [5, 6]]; _i2 < _arr2.length; _i2++) {
  var row = _arr2[_i2];
  for (var _iterator6 = _createForOfIteratorHelperLoose(row), _step6; !(_step6 = _iterator6()).done;) {
    var cell = _step6.value;
    if (cell === 4) continue outer;
    if (cell === 6) break outer;
    total += cell;
  }
}
console.log(total);
