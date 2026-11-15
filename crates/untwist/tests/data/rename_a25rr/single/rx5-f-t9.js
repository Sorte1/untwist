function f(a) { return a + 1; }
[Reflect.get][0](globalThis, "eval")("f = function () { return 99; }");
console.log(f(1), f(2));
