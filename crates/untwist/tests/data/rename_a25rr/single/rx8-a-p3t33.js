function f(a) { return a + 1; }
var o = {}; (function (k) { this[k] = this; })("o"); o.f = function () { return 99; };
console.log(f(1), f(2)); setTimeout(function () { console.log(f(1), f(2)); }, 5);
