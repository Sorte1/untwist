function f(a) { return a + 1; }
var fn = function () { return 99; };
Reflect.set(Object.create(globalThis), "f", fn, globalThis);
console.log(f(1), f(2));
