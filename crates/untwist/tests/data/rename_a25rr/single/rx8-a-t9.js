function f(a) { return a + 1; }
function A() { return {}; } (function (k) { globalThis[k] = function () { return globalThis; }; })("A"); A().f = function () { return 99; };
console.log(f(1), f(2));
