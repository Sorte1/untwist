"use strict";
var log = [];
var x = 3;
log.push(Math.pow(2, 10), Math.pow(x, 2));
var real = Math.pow;
Math.pow = function (a, b) {
  return real(a, b) + 1;
};
log.push(Math.pow(2, 10));
console.log(log.join(","));
