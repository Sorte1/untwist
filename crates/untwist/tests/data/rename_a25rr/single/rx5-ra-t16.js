function f(a) { return a + 1; }
var fn = function () { return 99; };
var args = [globalThis, "f", fn]; Reflect.set(...args);
console.log(f(1), f(2));
