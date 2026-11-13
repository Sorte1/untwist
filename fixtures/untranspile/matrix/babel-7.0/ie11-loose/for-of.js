"use strict";

var _custom;

var out = [];
var _arr = [1, 2, 3];

for (var _i = 0; _i < _arr.length; _i++) {
  var v = _arr[_i];
  out.push(v * 2);
}

for (var _iterator = "héllo", _isArray = Array.isArray(_iterator), _i2 = 0, _iterator = _isArray ? _iterator : _iterator[Symbol.iterator]();;) {
  var _ref;

  if (_isArray) {
    if (_i2 >= _iterator.length) break;
    _ref = _iterator[_i2++];
  } else {
    _i2 = _iterator.next();
    if (_i2.done) break;
    _ref = _i2.value;
  }

  var ch = _ref;
  out.push(ch);
}

for (var _iterator2 = new Map([["a", 1], ["b", 2]]), _isArray2 = Array.isArray(_iterator2), _i3 = 0, _iterator2 = _isArray2 ? _iterator2 : _iterator2[Symbol.iterator]();;) {
  var _ref2;

  if (_isArray2) {
    if (_i3 >= _iterator2.length) break;
    _ref2 = _iterator2[_i3++];
  } else {
    _i3 = _iterator2.next();
    if (_i3.done) break;
    _ref2 = _i3.value;
  }

  var _ref7 = _ref2,
      k = _ref7[0],
      _v = _ref7[1];
  out.push(k + _v);
}

for (var _iterator3 = new Set([3, 3, 4]), _isArray3 = Array.isArray(_iterator3), _i4 = 0, _iterator3 = _isArray3 ? _iterator3 : _iterator3[Symbol.iterator]();;) {
  var _ref3;

  if (_isArray3) {
    if (_i4 >= _iterator3.length) break;
    _ref3 = _iterator3[_i4++];
  } else {
    _i4 = _iterator3.next();
    if (_i4.done) break;
    _ref3 = _i4.value;
  }

  var _v2 = _ref3;
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

for (var _iterator4 = custom, _isArray4 = Array.isArray(_iterator4), _i5 = 0, _iterator4 = _isArray4 ? _iterator4 : _iterator4[Symbol.iterator]();;) {
  var _ref4;

  if (_isArray4) {
    if (_i5 >= _iterator4.length) break;
    _ref4 = _iterator4[_i5++];
  } else {
    _i5 = _iterator4.next();
    if (_i5.done) break;
    _ref4 = _i5.value;
  }

  var _v3 = _ref4;
  if (_v3 === 3) break;
  got.push(_v3);
}

console.log(got.join(","), out[out.length - 1]);

function args() {
  var xs = [];

  for (var _iterator5 = arguments, _isArray5 = Array.isArray(_iterator5), _i6 = 0, _iterator5 = _isArray5 ? _iterator5 : _iterator5[Symbol.iterator]();;) {
    var _ref5;

    if (_isArray5) {
      if (_i6 >= _iterator5.length) break;
      _ref5 = _iterator5[_i6++];
    } else {
      _i6 = _iterator5.next();
      if (_i6.done) break;
      _ref5 = _i6.value;
    }

    var a = _ref5;
    xs.push(a);
  }

  return xs.join("+");
}

console.log(args(1, 2, 3));
var total = 0;
var _arr2 = [[1, 2], [3, 4], [5, 6]];

outer: for (var _i7 = 0; _i7 < _arr2.length; _i7++) {
  var row = _arr2[_i7];

  for (var _iterator6 = row, _isArray6 = Array.isArray(_iterator6), _i8 = 0, _iterator6 = _isArray6 ? _iterator6 : _iterator6[Symbol.iterator]();;) {
    var _ref6;

    if (_isArray6) {
      if (_i8 >= _iterator6.length) break;
      _ref6 = _iterator6[_i8++];
    } else {
      _i8 = _iterator6.next();
      if (_i8.done) break;
      _ref6 = _i8.value;
    }

    var cell = _ref6;
    if (cell === 4) continue outer;
    if (cell === 6) break outer;
    total += cell;
  }
}

console.log(total);
