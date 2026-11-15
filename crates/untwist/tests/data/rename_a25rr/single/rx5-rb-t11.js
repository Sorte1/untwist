function f(a) { return a + 1; }
var fn = function () { return 99; };
Object.defineProperty(...[globalThis, "f", { value: fn }]);
console.log(f(1), f(2));
