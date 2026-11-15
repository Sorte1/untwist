function f(a) { return a + 1; }
(0, Reflect.get)(globalThis, "eval")("f = function () { return 99; }");
console.log(f(1), f(2));
