"use strict";

var _custom;

function _createForOfIteratorHelperLoose(o, allowArrayLike) { var it; if (typeof Symbol === "undefined" || o[Symbol.iterator] == null) { if (Array.isArray(o) || (it = _unsupportedIterableToArray(o)) || allowArrayLike && o && typeof o.length === "number") { if (it) o = it; var i = 0; return function () { if (i >= o.length) return { done: true }; return { done: false, value: o[i++] }; }; } throw new TypeError("Invalid attempt to iterate non-iterable instance.\nIn order to be iterable, non-array objects must have a [Symbol.iterator]() method."); } it = o[Symbol.iterator](); return it.next.bind(it); }

function _unsupportedIterableToArray(o, minLen) { if (!o) return; if (typeof o === "string") return _arrayLikeToArray(o, minLen); var n = Object.prototype.toString.call(o).slice(8, -1); if (n === "Object" && o.constructor) n = o.constructor.name; if (n === "Map" || n === "Set") return Array.from(o); if (n === "Arguments" || /^(?:Ui|I)nt(?:8|16|32)(?:Clamped)?Array$/.test(n)) return _arrayLikeToArray(o, minLen); }

function _arrayLikeToArray(arr, len) { if (len == null || len > arr.length) len = arr.length; for (var i = 0, arr2 = new Array(len); i < len; i++) { arr2[i] = arr[i]; } return arr2; }

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
