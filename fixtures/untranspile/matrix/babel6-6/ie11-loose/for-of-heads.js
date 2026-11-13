"use strict";

var calls = [];
var handlers = [function () {
  return calls.push("a");
}, function () {
  return calls.push("b");
}];
for (var _iterator = new Set(handlers), _isArray = Array.isArray(_iterator), _i = 0, _iterator = _isArray ? _iterator : _iterator[Symbol.iterator]();;) {
  var _ref;

  if (_isArray) {
    if (_i >= _iterator.length) break;
    _ref = _iterator[_i++];
  } else {
    _i = _iterator.next();
    if (_i.done) break;
    _ref = _i.value;
  }

  var handler = _ref;
  handler();
}var counts = {};
for (var _iterator2 = new Set(["x", "y", "x"]), _isArray2 = Array.isArray(_iterator2), _i2 = 0, _iterator2 = _isArray2 ? _iterator2 : _iterator2[Symbol.iterator]();;) {
  var _ref2;

  if (_isArray2) {
    if (_i2 >= _iterator2.length) break;
    _ref2 = _iterator2[_i2++];
  } else {
    _i2 = _iterator2.next();
    if (_i2.done) break;
    _ref2 = _i2.value;
  }

  var key = _ref2;
  counts[key] = (counts[key] || 0) + 1;
}for (var _iterator3 = new Set([{ list: [1, 2] }, { list: [3] }]), _isArray3 = Array.isArray(_iterator3), _i3 = 0, _iterator3 = _isArray3 ? _iterator3 : _iterator3[Symbol.iterator]();;) {
  var _ref3;

  if (_isArray3) {
    if (_i3 >= _iterator3.length) break;
    _ref3 = _iterator3[_i3++];
  } else {
    _i3 = _iterator3.next();
    if (_i3.done) break;
    _ref3 = _i3.value;
  }

  var entry = _ref3;
  for (var _iterator7 = entry.list, _isArray7 = Array.isArray(_iterator7), _i7 = 0, _iterator7 = _isArray7 ? _iterator7 : _iterator7[Symbol.iterator]();;) {
    var _ref7;

    if (_isArray7) {
      if (_i7 >= _iterator7.length) break;
      _ref7 = _iterator7[_i7++];
    } else {
      _i7 = _iterator7.next();
      if (_i7.done) break;
      _ref7 = _i7.value;
    }

    var n = _ref7;
    calls.push(n);
  }
}var removed = { a: 1, b: 2, c: 3 };
for (var _iterator4 = new Set(["a", "c"]), _isArray4 = Array.isArray(_iterator4), _i4 = 0, _iterator4 = _isArray4 ? _iterator4 : _iterator4[Symbol.iterator]();;) {
  var _ref4;

  if (_isArray4) {
    if (_i4 >= _iterator4.length) break;
    _ref4 = _iterator4[_i4++];
  } else {
    _i4 = _iterator4.next();
    if (_i4.done) break;
    _ref4 = _i4.value;
  }

  var _key = _ref4;
  delete removed[_key];
}var total = 0;
var weights = { p: 2, q: 5 };
for (var _iterator5 = new Set(["p", "q", "r"]), _isArray5 = Array.isArray(_iterator5), _i5 = 0, _iterator5 = _isArray5 ? _iterator5 : _iterator5[Symbol.iterator]();;) {
  var _ref5;

  if (_isArray5) {
    if (_i5 >= _iterator5.length) break;
    _ref5 = _iterator5[_i5++];
  } else {
    _i5 = _iterator5.next();
    if (_i5.done) break;
    _ref5 = _i5.value;
  }

  var name = _ref5;
  total += weights[name] || 0;
}for (var _iterator6 = new Set([{ run: function run(k) {
    return calls.push(k);
  } }]), _isArray6 = Array.isArray(_iterator6), _i6 = 0, _iterator6 = _isArray6 ? _iterator6 : _iterator6[Symbol.iterator]();;) {
  var _ref6;

  if (_isArray6) {
    if (_i6 >= _iterator6.length) break;
    _ref6 = _iterator6[_i6++];
  } else {
    _i6 = _iterator6.next();
    if (_i6.done) break;
    _ref6 = _i6.value;
  }

  var item = _ref6;
  item.run("ran");
}console.log(calls.join(), JSON.stringify(counts), JSON.stringify(removed), total);
