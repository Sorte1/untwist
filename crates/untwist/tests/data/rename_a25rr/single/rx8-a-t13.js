function f(a) { return a + 1; }
function A(x) { try { throw globalThis; } catch (e) { var x = e; } return x; } A({}).f = function () { return 99; };
console.log(f(1), f(2));
