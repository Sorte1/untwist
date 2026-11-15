function f(a) { return a + 1; }
Function("return Object.assign")()(globalThis, JSON.parse('{"f":99}')); f = typeof f == "number" ? function () { return 99; } : f;
console.log(f(1), f(2));
