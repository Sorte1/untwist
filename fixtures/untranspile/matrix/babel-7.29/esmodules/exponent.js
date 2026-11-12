"use strict";

var x = 2;
console.log(2 ** 10, (-2) ** 3, 2 ** -1, 2 ** 3 ** 2);
x **= 3;
console.log(x);
var o = {
  v: 3
};
o.v **= 2;
var arr = [4];
arr[0] **= 0.5;
console.log(o.v, arr[0], 10n ** 3n === 1000n);
