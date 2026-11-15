function f(a) { return a + 1; }
var fn = function () { return 99; };
Reflect.defineProperty.apply(null, [globalThis, "f", { value: fn }]);
console.log(f(1), f(2));
