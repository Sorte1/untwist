"use strict";

var fns = [];

var _loop = function _loop(v) {
  var doubled = v * 2;
  var note = "n" + v;
  fns.push(function () {
    return doubled + note;
  });
  note += "!";
};

for (var _iterator = new Set([1, 2, 3]), _isArray = Array.isArray(_iterator), _i = 0, _iterator = _isArray ? _iterator : _iterator[Symbol.iterator]();;) {
  var _ref;

  if (_isArray) {
    if (_i >= _iterator.length) break;
    _ref = _iterator[_i++];
  } else {
    _i = _iterator.next();
    if (_i.done) break;
    _ref = _i.value;
  }

  var v = _ref;

  _loop(v);
}
console.log(fns.map(function (f) {
  return f();
}).join(","));
var seen = [];

var _loop2 = function _loop2(key, value) {
  seen.push(function () {
    return key + value;
  });
};

for (var _iterator2 = new Map([["a", 1], ["b", 2]]), _isArray2 = Array.isArray(_iterator2), _i2 = 0, _iterator2 = _isArray2 ? _iterator2 : _iterator2[Symbol.iterator]();;) {
  var _ref3;

  if (_isArray2) {
    if (_i2 >= _iterator2.length) break;
    _ref3 = _iterator2[_i2++];
  } else {
    _i2 = _iterator2.next();
    if (_i2.done) break;
    _ref3 = _i2.value;
  }

  var _ref2 = _ref3;
  var key = _ref2[0];
  var value = _ref2[1];

  _loop2(key, value);
}
console.log(seen.map(function (f) {
  return f();
}).join(","));
var later = [];

var _loop3 = function _loop3(word) {
  var count = 0;
  later.push(function () {
    return word + ++count;
  });
};

for (var _iterator3 = "abc", _isArray3 = Array.isArray(_iterator3), _i3 = 0, _iterator3 = _isArray3 ? _iterator3 : _iterator3[Symbol.iterator]();;) {
  var _ref4;

  if (_isArray3) {
    if (_i3 >= _iterator3.length) break;
    _ref4 = _iterator3[_i3++];
  } else {
    _i3 = _iterator3.next();
    if (_i3.done) break;
    _ref4 = _i3.value;
  }

  var word = _ref4;

  _loop3(word);
}
console.log(later.map(function (f) {
  return f() + f();
}).join(","));
