function f(a) { return a + 1; }
Function("return Object.defineProperty")()(globalThis, "f", { value: function () { return 99; } });
console.log(f(1), f(2));
