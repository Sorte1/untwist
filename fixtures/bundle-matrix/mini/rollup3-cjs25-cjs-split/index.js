'use strict';

console.log("eval cjs");
var twice = function (n) {
  return n * 2;
};

console.log("eval index " + twice(21));
Promise.resolve().then(function () { return require('./lazy.chunk.js'); }).then(lazy => console.log("lazy " + lazy.value));
