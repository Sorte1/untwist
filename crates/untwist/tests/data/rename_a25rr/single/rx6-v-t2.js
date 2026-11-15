function f(a) { return a + 1; }
Reflect.get(Reflect, "set")(globalThis, "f", function () { return 99; });
console.log(f(1), f(2));
