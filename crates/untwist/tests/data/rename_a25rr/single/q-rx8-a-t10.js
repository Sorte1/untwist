function f(a) { return a + 1; }
var k = "x"; (function (n) { globalThis[n] = "f"; })("k"); globalThis[k] = function () { return 99; };
console.log(f(1), f(2));
