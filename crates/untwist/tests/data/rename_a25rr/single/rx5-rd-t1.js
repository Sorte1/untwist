function f(a) { return a + 1; }
var S = Reflect.set;
S(globalThis, "f", function () { return 99; });
console.log(f(1), f(2));
