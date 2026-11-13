"use strict";

var fns = [];

var _loop = function _loop() {
  if (_isArray) {
    if (_i >= _iterator.length) return "break";
    _ref = _iterator[_i++];
  } else {
    _i = _iterator.next();
    if (_i.done) return "break";
    _ref = _i.value;
  }

  var v = _ref;
  var doubled = v * 2;
  var note = "n" + v;
  fns.push(function () {
    return doubled + note;
  });
  note += "!";
};

for (var _iterator = new Set([1, 2, 3]), _isArray = Array.isArray(_iterator), _i = 0, _iterator = _isArray ? _iterator : _iterator[Symbol.iterator]();;) {
  var _ref;

  var _ret = _loop();

  if (_ret === "break") break;
}

console.log(fns.map(function (f) {
  return f();
}).join(","));
var seen = [];

var _loop2 = function _loop2() {
  if (_isArray2) {
    if (_i2 >= _iterator2.length) return "break";
    _ref2 = _iterator2[_i2++];
  } else {
    _i2 = _iterator2.next();
    if (_i2.done) return "break";
    _ref2 = _i2.value;
  }

  var _ref4 = _ref2,
      key = _ref4[0],
      value = _ref4[1];
  seen.push(function () {
    return key + value;
  });
};

for (var _iterator2 = new Map([["a", 1], ["b", 2]]), _isArray2 = Array.isArray(_iterator2), _i2 = 0, _iterator2 = _isArray2 ? _iterator2 : _iterator2[Symbol.iterator]();;) {
  var _ref2;

  var _ret2 = _loop2();

  if (_ret2 === "break") break;
}

console.log(seen.map(function (f) {
  return f();
}).join(","));
var later = [];

var _loop3 = function _loop3() {
  if (_isArray3) {
    if (_i3 >= _iterator3.length) return "break";
    _ref3 = _iterator3[_i3++];
  } else {
    _i3 = _iterator3.next();
    if (_i3.done) return "break";
    _ref3 = _i3.value;
  }

  var word = _ref3;
  var count = 0;
  later.push(function () {
    return word + ++count;
  });
};

for (var _iterator3 = "abc", _isArray3 = Array.isArray(_iterator3), _i3 = 0, _iterator3 = _isArray3 ? _iterator3 : _iterator3[Symbol.iterator]();;) {
  var _ref3;

  var _ret3 = _loop3();

  if (_ret3 === "break") break;
}

console.log(later.map(function (f) {
  return f() + f();
}).join(","));
