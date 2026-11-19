function f(a) { return a + 1; }
function A() { return {}; } Function("\\u0041 = function () { return globalThis; }")(); A().f = function () { return 99; };
console.log(f(1), f(2));
