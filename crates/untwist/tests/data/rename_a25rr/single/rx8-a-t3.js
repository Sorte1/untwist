function f(a) { return a + 1; }
var o = {}; (function (g) { g.eval("o = g"); })(globalThis); o.f = function () { return 99; };
console.log(f(1), f(2));
