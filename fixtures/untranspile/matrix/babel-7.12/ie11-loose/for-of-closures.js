"use strict";

function _createForOfIteratorHelperLoose(o, allowArrayLike) { var it; if (typeof Symbol === "undefined" || o[Symbol.iterator] == null) { if (Array.isArray(o) || (it = _unsupportedIterableToArray(o)) || allowArrayLike && o && typeof o.length === "number") { if (it) o = it; var i = 0; return function () { if (i >= o.length) return { done: true }; return { done: false, value: o[i++] }; }; } throw new TypeError("Invalid attempt to iterate non-iterable instance.\nIn order to be iterable, non-array objects must have a [Symbol.iterator]() method."); } it = o[Symbol.iterator](); return it.next.bind(it); }

function _unsupportedIterableToArray(o, minLen) { if (!o) return; if (typeof o === "string") return _arrayLikeToArray(o, minLen); var n = Object.prototype.toString.call(o).slice(8, -1); if (n === "Object" && o.constructor) n = o.constructor.name; if (n === "Map" || n === "Set") return Array.from(o); if (n === "Arguments" || /^(?:Ui|I)nt(?:8|16|32)(?:Clamped)?Array$/.test(n)) return _arrayLikeToArray(o, minLen); }

function _arrayLikeToArray(arr, len) { if (len == null || len > arr.length) len = arr.length; for (var i = 0, arr2 = new Array(len); i < len; i++) { arr2[i] = arr[i]; } return arr2; }

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
