"use strict";

function _typeof(obj) { "@babel/helpers - typeof"; return _typeof = "function" == typeof Symbol && "symbol" == typeof Symbol.iterator ? function (obj) { return typeof obj; } : function (obj) { return obj && "function" == typeof Symbol && obj.constructor === Symbol && obj !== Symbol.prototype ? "symbol" : typeof obj; }, _typeof(obj); }
function _createForOfIteratorHelper(o, allowArrayLike) { var it = typeof Symbol !== "undefined" && o[Symbol.iterator] || o["@@iterator"]; if (!it) { if (Array.isArray(o) || (it = _unsupportedIterableToArray(o)) || allowArrayLike && o && typeof o.length === "number") { if (it) o = it; var i = 0; var F = function F() {}; return { s: F, n: function n() { if (i >= o.length) return { done: true }; return { done: false, value: o[i++] }; }, e: function e(_e) { throw _e; }, f: F }; } throw new TypeError("Invalid attempt to iterate non-iterable instance.\nIn order to be iterable, non-array objects must have a [Symbol.iterator]() method."); } var normalCompletion = true, didErr = false, err; return { s: function s() { it = it.call(o); }, n: function n() { var step = it.next(); normalCompletion = step.done; return step; }, e: function e(_e2) { didErr = true; err = _e2; }, f: function f() { try { if (!normalCompletion && it.return != null) it.return(); } finally { if (didErr) throw err; } } }; }
function _unsupportedIterableToArray(o, minLen) { if (!o) return; if (typeof o === "string") return _arrayLikeToArray(o, minLen); var n = Object.prototype.toString.call(o).slice(8, -1); if (n === "Object" && o.constructor) n = o.constructor.name; if (n === "Map" || n === "Set") return Array.from(o); if (n === "Arguments" || /^(?:Ui|I)nt(?:8|16|32)(?:Clamped)?Array$/.test(n)) return _arrayLikeToArray(o, minLen); }
function _arrayLikeToArray(arr, len) { if (len == null || len > arr.length) len = arr.length; for (var i = 0, arr2 = new Array(len); i < len; i++) arr2[i] = arr[i]; return arr2; }
var fns = [];
var _loop = function _loop(i) {
  fns.push(function () {
    return i;
  });
};
for (var i = 0; i < 3; i++) {
  _loop(i);
}
console.log(fns.map(function (f) {
  return f();
}).join(","));
var later = [];
var _loop2 = function _loop2(i) {
  if (i === 1) return "continue";
  if (i === 3) return "break";
  later.push(function () {
    return i * 10;
  });
};
for (var _i = 0; _i < 4; _i++) {
  var _ret = _loop2(_i);
  if (_ret === "continue") continue;
  if (_ret === "break") break;
}
console.log(later.map(function (f) {
  return f();
}).join(","));
function find(list) {
  var _iterator = _createForOfIteratorHelper(list),
    _step;
  try {
    var _loop3 = function _loop3() {
      var item = _step.value;
      var handler = function handler() {
        return item.id;
      };
      if (item.ok) return {
        v: handler()
      };
    };
    for (_iterator.s(); !(_step = _iterator.n()).done;) {
      var _ret2 = _loop3();
      if (_typeof(_ret2) === "object") return _ret2.v;
    }
  } catch (err) {
    _iterator.e(err);
  } finally {
    _iterator.f();
  }
  return -1;
}
console.log(find([{
  id: 1
}, {
  id: 2,
  ok: true
}]));
var x = "outer";
{
  var _x = "inner";
  var y = _x + "!";
  console.log(_x, y);
}
console.log(x);
var ws = [];
var j = 0;
var _loop4 = function _loop4() {
  var k = j;
  ws.push(function () {
    return k;
  });
  j++;
};
while (j < 3) {
  _loop4();
}
console.log(ws.map(function (f) {
  return f();
}).join(","));
var _loop5 = function _loop5(i) {
  setTimeout(function () {
    return console.log("timer", i);
  }, 0);
};
for (var _i2 = 0, n = 2; _i2 < n; _i2++) {
  _loop5(_i2);
}
switch (1) {
  case 1:
    {
      var s = "case";
      console.log(s);
    }
}
