"use strict";
var total = 0;
for (var i = 1; i <= 4; i++) {
  total += i * i;
}
console.log("squares", total, (function () { return typeof this; })());
