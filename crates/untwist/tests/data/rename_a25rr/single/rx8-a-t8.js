function f(a) { return a + 1; }
var o = {}; (function (k) { globalThis[k] = 1; })("zz"); globalThis.o = globalThis; o.f = function () { return 99; };
console.log(f(1), f(2));
