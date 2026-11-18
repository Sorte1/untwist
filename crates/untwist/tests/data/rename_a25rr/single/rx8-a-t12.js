function f(a) { return a + 1; }
function A(x) { for (var x of [globalThis]); return x; } A({}).f = function () { return 99; };
console.log(f(1), f(2));
