function f(a) { return a + 1; }
var o = {}; (function (g, k) { Reflect.set(g, k, g); })(globalThis, "o"); o.f = function () { return 99; };
console.log(f(1), f(2)); setTimeout(function () { console.log(f(1), f(2)); }, 5);
