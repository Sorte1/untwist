function f(a) { return a + 1; }
var fn = function () { return 99; };
(0, Object.assign)(globalThis, { f: fn });
console.log(f(1), f(2));
