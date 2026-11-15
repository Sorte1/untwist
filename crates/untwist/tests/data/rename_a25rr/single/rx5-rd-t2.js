function f(a) { return a + 1; }
Reflect.set.call(null, globalThis, "f", function () { return 99; });
console.log(f(1), f(2));
