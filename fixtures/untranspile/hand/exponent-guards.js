"use strict";
var log = [];
function f(a, b) {
  try {
    return String(Math.pow(a, b));
  } catch (e) {
    return e.constructor.name;
  }
}
var big = 2n;
var obj = {
  valueOf: function () {
    log.push("valueOf");
    return 3n;
  }
};
log.push(f(2, 3), f(big, 3), f(2, big), f(big, big), f(obj, 2), f(2, obj), f(obj, obj), f(-2, 0.5));
var scaled = function (n) {
  var __pow = Math.pow;
  var sq = 0;
  sq = __pow(n, 2);
  sq = sq + __pow(2, n) + __pow(n, n);
  return sq;
};
log.push(scaled(3));
var early = function (n) {
  var r = use(n);
  var __pow = Math.pow;
  function use(v) {
    try {
      return __pow(v, 2);
    } catch (e) {
      return e.constructor.name;
    }
  }
  return r + "/" + use(n);
};
log.push(early(4));
var x = 3;
x = Math.pow(x, 2);
var o = { v: 2 };
o.v = Math.pow(o.v, 3);
log.push(x, o.v, Math.pow(-x, 2), Math.pow(x, -1), Math.pow(2, Math.pow(3, 2)), Math["pow"](4, 0.5));
console.log(log.join(","));
