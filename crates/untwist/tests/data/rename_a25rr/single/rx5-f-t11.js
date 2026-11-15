function f(a) { return a + 1; }
var G = Reflect.get; G(globalThis, "eval")("f = function () { return 99; }");
console.log(f(1), f(2));
