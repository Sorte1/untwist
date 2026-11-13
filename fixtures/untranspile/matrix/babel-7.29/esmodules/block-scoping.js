"use strict";

var fns = [];
var _loop = function _loop(i) {
  fns.push(() => i);
};
for (var i = 0; i < 3; i++) {
  _loop(i);
}
console.log(fns.map(f => f()).join(","));
var later = [];
var _loop2 = function _loop2(_i) {
    if (_i === 1) return 0; // continue
    if (_i === 3) return 1; // break
    later.push(function () {
      return _i * 10;
    });
  },
  _ret;
for (var _i = 0; _i < 4; _i++) {
  _ret = _loop2(_i);
  if (_ret === 0) continue;
  if (_ret === 1) break;
}
console.log(later.map(f => f()).join(","));
function find(list) {
  var _loop3 = function _loop3(item) {
      var handler = () => item.id;
      if (item.ok) return {
        v: handler()
      };
    },
    _ret2;
  for (var item of list) {
    _ret2 = _loop3(item);
    if (_ret2) return _ret2.v;
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
  ws.push(() => k);
  j++;
};
while (j < 3) {
  _loop4();
}
console.log(ws.map(f => f()).join(","));
var _loop5 = function _loop5(_i2) {
  setTimeout(() => console.log("timer", _i2), 0);
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
