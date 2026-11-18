function f(a) { return a + 1; }
var o = {}; (function (k) { setTimeout(function () { globalThis[k] = globalThis; o.f = function () { return 99; }; }); })("o");
console.log(f(1), f(2)); setTimeout(function () { console.log(f(1), f(2)); }, 5);
